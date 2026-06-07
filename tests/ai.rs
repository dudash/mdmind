use mdmind::ai::{
    AiAdapterType, AiAuthScheme, AiIntent, AiOutput, AiProfile, AiProfilesConfig, AiRequest,
    AiScope, AiSuggestedChange, AiSuggestion, AiSuggestionTarget, CLAUDE_LOCAL_QUICK_ADD_ID,
    CODEX_LOCAL_QUICK_ADD_ID, NVIDIA_NIM_LOCAL_SECRET_ID, NVIDIA_NIM_QUICK_ADD_ID,
    OLLAMA_LOCAL_QUICK_ADD_ID, ai_profile_is_claude_local_bridge, ai_profile_is_codex_local_bridge,
    ai_profile_is_ollama_local, ai_profiles_path_for_config_dir, ai_secrets_path_for_config_dir,
    claude_code_stream_json_event_text, claude_local_exec_args, codex_exec_json_event_text,
    codex_local_exec_args, contextual_presets_for_node, load_ai_profiles_from_path,
    load_local_ai_secrets_from_path, local_ai_secret_ref, local_claude_bridge_prompt,
    local_codex_bridge_prompt, map_assistant_system_prompt, openai_compatible_chat_body,
    openai_compatible_chat_body_with_stream, openai_compatible_chat_url,
    parse_reviewable_map_suggestions, quick_add_presets, quick_add_profile,
    recommended_ollama_model, resolve_local_ai_secret_from_path, save_ai_profiles_to_path,
    save_local_ai_secret_to_path, send_ai_chat_streaming_cancellable,
    split_reviewable_map_suggestions, split_reviewable_map_suggestions_with_warning,
};
use mdmind::parser::parse_document;
use std::io::{Read, Write};

fn temp_path(name: &str) -> std::path::PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("mdmind-ai-{nonce}-{name}"))
}

#[test]
fn ai_request_trims_empty_instruction_and_profile() {
    let request = AiRequest::new(
        AiIntent::Extract,
        AiScope::CurrentBranch,
        AiOutput::ReviewableMapEdits,
    )
    .with_instruction("  Find TODOs and risks.  ")
    .with_profile("  local-llama  ");

    assert_eq!(
        request.instruction.as_deref(),
        Some("Find TODOs and risks.")
    );
    assert_eq!(request.profile.as_deref(), Some("local-llama"));

    let request = request.with_instruction("   ").with_profile("");
    assert_eq!(request.instruction, None);
    assert_eq!(request.profile, None);
}

#[test]
fn ai_suggestions_model_add_update_and_remove_operations() {
    let target = AiSuggestionTarget::new(vec![0, 2], Some("launch/risks".to_string()), "Risks");
    let suggestion = AiSuggestion::new(
        target.clone(),
        "NVIDIA NIM",
        "Review this branch.",
        vec![
            AiSuggestedChange::AddChild {
                target: None,
                fragment: "Missing fallback path #risk".to_string(),
                detail: "Provider failure needs a visible recovery path.".to_string(),
            },
            AiSuggestedChange::UpdateNode {
                target: target.clone(),
                fragment: Some("Risks and recovery".to_string()),
                detail: None,
            },
            AiSuggestedChange::RemoveNode { target },
        ],
    );

    let json = serde_json::to_string(&suggestion).expect("suggestion should serialize");
    assert!(json.contains("\"operation\":\"add_child\""));
    assert!(json.contains("\"operation\":\"update_node\""));
    assert!(json.contains("\"operation\":\"remove_node\""));

    let decoded: AiSuggestion = serde_json::from_str(&json).expect("suggestion should deserialize");
    assert_eq!(decoded.changes.len(), 3);
    assert_eq!(decoded.answer, None);
    assert_eq!(decoded.changes[0].operation_label(), "add child");
    assert_eq!(decoded.changes[1].operation_label(), "update node");
    assert_eq!(decoded.changes[2].operation_label(), "remove node");
}

#[test]
fn ai_suggestions_can_hold_conversational_answers_without_edits() {
    let suggestion = AiSuggestion::answer(
        AiSuggestionTarget::new(vec![0], Some("launch".to_string()), "Launch"),
        "NVIDIA NIM",
        "What gaps do you see?",
        "Add acceptance criteria.",
    );

    assert!(suggestion.is_conversational_answer());
    assert_eq!(
        suggestion.answer.as_deref(),
        Some("Add acceptance criteria.")
    );
    assert!(suggestion.changes.is_empty());
}

#[test]
fn map_assistant_system_prompt_asks_for_reviewable_operations() {
    let prompt = map_assistant_system_prompt();

    assert!(prompt.contains("Default to conversational answers"));
    assert!(prompt.contains("The word \"suggest\" is an explicit signal"));
    assert!(prompt.contains("suggest new characters"));
    assert!(prompt.contains("explicitly asks"));
    assert!(prompt.contains("reviewable map edits"));
    assert!(prompt.contains("local conventions"));
    assert!(prompt.contains("Prefer repairing or enriching existing nodes"));
    assert!(prompt.contains("Do not claim you changed the map directly"));
}

#[test]
fn reviewable_map_suggestion_blocks_parse_and_strip_from_chat_answer() {
    let contract = mdmind::ai::reviewable_map_suggestion_contract();
    assert!(contract.contains("tool-call-like output"));
    assert!(contract.contains("Omit target"));
    assert!(contract.contains("smallest honest operation"));
    assert!(contract.contains("Do not express every idea as add_child"));
    assert!(contract.contains("update_node"));
    assert!(contract.contains("remove_node"));
    assert!(contract.contains("Match nearby branch conventions"));
    assert!(contract.contains("detail lines, relations, and external references"));

    let answer = "I found two additions.\n```mdmind-suggestions\n{\n  \"changes\": [\n    {\"operation\":\"add_child\",\"fragment\":\"Acceptance criteria #todo\",\"detail\":\"Define done.\"},\n    {\"operation\":\"add_child\",\"fragment\":\"Rollout risks #risk\"}\n  ]\n}\n```\nOpen Review Suggestions.";

    let (chat, changes) = split_reviewable_map_suggestions(answer);

    assert_eq!(changes.len(), 2);
    assert_eq!(changes[0].operation_label(), "add child");
    assert!(chat.contains("I found two additions."));
    assert!(chat.contains("Open Review Suggestions."));
    assert!(!chat.contains("mdmind-suggestions"));
}

#[test]
fn empty_reviewable_map_suggestions_are_a_clean_noop() {
    let direct = split_reviewable_map_suggestions_with_warning(r#"{"changes":[]}"#);
    assert!(direct.answer.is_empty());
    assert!(direct.changes.is_empty());
    assert!(direct.warning.is_none());

    let fenced = split_reviewable_map_suggestions_with_warning(
        "Nothing useful to add.\n```mdmind-suggestions\n{\"changes\":[]}\n```\n",
    );
    assert_eq!(fenced.answer, "Nothing useful to add.");
    assert!(fenced.changes.is_empty());
    assert!(fenced.warning.is_none());
}

#[test]
fn reviewable_map_suggestion_parser_filters_empty_rows() {
    let changes = parse_reviewable_map_suggestions(
        r#"{
          "changes": [
            {"operation":"add_child","fragment":"","detail":"ignore"},
            {"operation":"add_child","fragment":"Real node","detail":""}
          ]
        }"#,
    )
    .expect("suggestion JSON should parse");

    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].operation_label(), "add child");
}

#[test]
fn reviewable_map_suggestion_parser_accepts_targeted_add_child_rows() {
    let changes = parse_reviewable_map_suggestions(
        r#"{
          "changes": [
            {
              "operation": "add_child",
              "target": {"id": "product/tasks", "label": "Tasks"},
              "fragment": "Spell check new task #ai",
              "detail": "Check spelling before committing."
            }
          ]
        }"#,
    )
    .expect("targeted suggestion JSON should parse");

    assert_eq!(changes.len(), 1);
    match &changes[0] {
        AiSuggestedChange::AddChild {
            target: Some(target),
            fragment,
            detail,
        } => {
            assert_eq!(target.id.as_deref(), Some("product/tasks"));
            assert!(target.path.is_empty());
            assert_eq!(target.label, "Tasks");
            assert_eq!(fragment, "Spell check new task #ai");
            assert_eq!(detail, "Check spelling before committing.");
        }
        other => panic!("expected targeted add child, got {other:?}"),
    }
}

#[test]
fn invalid_reviewable_map_suggestion_block_stays_in_chat_as_warning() {
    let extraction = split_reviewable_map_suggestions_with_warning(
        "Looks good.\n```mdmind-suggestions\n{\"changes\":[{\"operation\":\"add_child\"}]}\n```\n",
    );

    assert!(extraction.changes.is_empty());
    assert!(extraction.warning.is_some());
    assert!(extraction.answer.contains("Could not stage suggestions"));
    assert!(!extraction.answer.contains("mdmind-suggestions"));
}

#[test]
fn contextual_presets_are_dynamic_for_rough_detailed_branches() {
    let parsed = parse_document(
        "- Launch Plan [id:launch]\n  | Need support FAQ and rollout owner.\n  | Risk: billing approval is not done.\n",
    );
    let node = &parsed.document.nodes[0];
    let presets = contextual_presets_for_node(node);
    let ids = presets
        .iter()
        .map(|preset| preset.id.as_str())
        .collect::<Vec<_>>();

    assert!(ids.contains(&"extract-from-details"));
    assert!(ids.contains(&"summarize-details"));
    assert!(ids.contains(&"prepare-handoff"));
    assert!(
        presets
            .iter()
            .any(|preset| preset.request.intent == AiIntent::Extract
                && preset.request.output == AiOutput::ReviewableMapEdits)
    );
}

#[test]
fn contextual_presets_offer_audit_for_task_or_risk_heavy_branches() {
    let parsed = parse_document(
        "- Release\n  - [ ] Draft release notes #todo\n  - Support load risk #risk\n  - Publish package @status:active\n  - Confirm approvals\n",
    );
    let node = &parsed.document.nodes[0];
    let presets = contextual_presets_for_node(node);

    let audit = presets
        .iter()
        .find(|preset| preset.id == "audit-gaps")
        .expect("structured branch should get an audit preset");

    assert_eq!(audit.request.intent, AiIntent::Audit);
    assert_eq!(audit.request.scope, AiScope::CurrentBranch);
    assert_eq!(audit.request.output, AiOutput::Observations);
}

#[test]
fn profiles_round_trip_without_secret_material() {
    let root = temp_path("config-root");
    let path = ai_profiles_path_for_config_dir(&root);
    let mut config = AiProfilesConfig {
        enabled: true,
        default_profile: Some("openai".to_string()),
        profiles: vec![
            AiProfile::openai_compatible(
                "openai",
                "OpenAI",
                "https://api.openai.com/v1",
                "gpt-5.2",
            ),
            AiProfile::local_cli("codex", "Codex Local", "codex"),
        ],
    };
    config.profiles[0].secret_ref = Some("mdmind.ai.openai".to_string());

    save_ai_profiles_to_path(&path, &config).expect("AI profile config should save");
    let loaded = load_ai_profiles_from_path(&path).expect("AI profile config should load");

    assert_eq!(loaded, config);
    assert_eq!(
        loaded
            .default_profile()
            .map(|profile| profile.label.as_str()),
        Some("OpenAI")
    );
    assert_eq!(
        loaded.profile("codex").map(|profile| profile.adapter_type),
        Some(AiAdapterType::LocalCli)
    );

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn legacy_ai_profiles_config_migrates_codex_local_profile() {
    let root = temp_path("legacy-config-root");
    let path = ai_profiles_path_for_config_dir(&root);
    std::fs::create_dir_all(path.parent().expect("config path should have a parent"))
        .expect("config directory should be created");
    std::fs::write(
        &path,
        r#"{
  "active_profile_id": "codex-local",
  "profiles": [
    {
      "id": "codex-local",
      "label": "Codex Local",
      "provider": "codex-local",
      "endpoint": "codex",
      "model": "Codex CLI default"
    }
  ]
}"#,
    )
    .expect("legacy AI profile config should be written");

    let loaded = load_ai_profiles_from_path(&path).expect("legacy AI profile config should load");
    let profile = loaded
        .profile(CODEX_LOCAL_QUICK_ADD_ID)
        .expect("Codex Local profile should migrate");

    assert!(loaded.enabled);
    assert_eq!(loaded.default_profile.as_deref(), Some("codex-local"));
    assert_eq!(profile.adapter_type, AiAdapterType::LocalCli);
    assert_eq!(profile.command.as_deref(), Some("codex"));
    assert_eq!(profile.command_args, vec!["exec"]);
    assert_eq!(profile.model, None);
    assert!(ai_profile_is_codex_local_bridge(profile));

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn legacy_ai_profiles_config_maps_short_default_ids_to_canonical_profiles() {
    let root = temp_path("legacy-short-default-config-root");
    let path = ai_profiles_path_for_config_dir(&root);
    std::fs::create_dir_all(path.parent().expect("config path should have a parent"))
        .expect("config directory should be created");
    std::fs::write(
        &path,
        r#"{
  "active_profile_id": "codex",
  "profiles": [
    {
      "id": "nvidia-nim",
      "label": "NVIDIA NIM",
      "provider": "nvidia-nim",
      "endpoint": "https://integrate.api.nvidia.com/v1",
      "model": "nvidia/llama-3.3-nemotron-super-49b-v1.5"
    },
    {
      "id": "codex",
      "label": "Codex Local",
      "provider": "codex",
      "endpoint": "codex"
    }
  ]
}"#,
    )
    .expect("legacy AI profile config should be written");

    let loaded = load_ai_profiles_from_path(&path).expect("legacy AI profile config should load");

    assert_eq!(loaded.default_profile.as_deref(), Some("codex-local"));
    assert!(loaded.profile(NVIDIA_NIM_QUICK_ADD_ID).is_some());
    assert!(loaded.profile(CODEX_LOCAL_QUICK_ADD_ID).is_some());

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn legacy_ai_profiles_config_can_be_saved_after_nvidia_quick_add() {
    let root = temp_path("legacy-nim-config-root");
    let path = ai_profiles_path_for_config_dir(&root);
    std::fs::create_dir_all(path.parent().expect("config path should have a parent"))
        .expect("config directory should be created");
    std::fs::write(
        &path,
        r#"{
  "active_profile_id": "codex-local",
  "profiles": [
    {
      "id": "codex-local",
      "label": "Codex Local",
      "provider": "codex-local",
      "endpoint": "codex",
      "model": "Codex CLI default"
    }
  ]
}"#,
    )
    .expect("legacy AI profile config should be written");

    let mut config =
        load_ai_profiles_from_path(&path).expect("legacy AI profile config should load");
    config.upsert_profile(
        quick_add_profile(
            NVIDIA_NIM_QUICK_ADD_ID,
            Some(local_ai_secret_ref(NVIDIA_NIM_LOCAL_SECRET_ID)),
        )
        .expect("NVIDIA NIM quick-add should build"),
        true,
    );
    save_ai_profiles_to_path(&path, &config).expect("migrated AI profile config should save");

    let contents =
        std::fs::read_to_string(&path).expect("migrated AI profile config should be readable");
    assert!(contents.contains("\"default_profile\": \"nvidia-nim\""));
    assert!(contents.contains("\"adapter_type\""));
    assert!(!contents.contains("active_profile_id"));

    let reloaded =
        load_ai_profiles_from_path(&path).expect("migrated AI profile config should reload");
    assert!(reloaded.profile(CODEX_LOCAL_QUICK_ADD_ID).is_some());
    assert_eq!(
        reloaded
            .profile(NVIDIA_NIM_QUICK_ADD_ID)
            .and_then(|profile| profile.secret_ref.as_deref()),
        Some("local:mdmind.ai.nvidia-nim")
    );

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn profiles_refuse_raw_key_material() {
    let root = temp_path("unsafe-config-root");
    let path = ai_profiles_path_for_config_dir(&root);
    let mut profile =
        AiProfile::openai_compatible("openai", "OpenAI", "https://api.openai.com/v1", "gpt-5.2");
    profile
        .env
        .insert("OPENAI_API_KEY".to_string(), "sk-secret".to_string());

    let error = save_ai_profiles_to_path(
        &path,
        &AiProfilesConfig {
            enabled: true,
            default_profile: Some("openai".to_string()),
            profiles: vec![profile],
        },
    )
    .expect_err("raw key-like material should be rejected");

    assert!(error.message().contains("secret_ref"));
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn local_secret_store_round_trips_with_restricted_file_permissions() {
    let root = temp_path("secrets-root");
    let path = ai_secrets_path_for_config_dir(&root);

    save_local_ai_secret_to_path(&path, NVIDIA_NIM_LOCAL_SECRET_ID, "  nvapi-test-key  ")
        .expect("local AI secret should save");
    let loaded = load_local_ai_secrets_from_path(&path).expect("local AI secrets should load");

    assert_eq!(
        loaded
            .secrets
            .get(NVIDIA_NIM_LOCAL_SECRET_ID)
            .map(String::as_str),
        Some("nvapi-test-key")
    );
    assert_eq!(
        local_ai_secret_ref(NVIDIA_NIM_LOCAL_SECRET_ID),
        "local:mdmind.ai.nvidia-nim"
    );
    assert_eq!(
        resolve_local_ai_secret_from_path(&path, NVIDIA_NIM_LOCAL_SECRET_ID)
            .expect("local AI secret should resolve"),
        "nvapi-test-key"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mode = std::fs::metadata(&path)
            .expect("secret file metadata should load")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn openai_compatible_chat_request_uses_profile_defaults() {
    let profile = quick_add_profile(NVIDIA_NIM_QUICK_ADD_ID, Some("local:test".to_string()))
        .expect("NVIDIA NIM profile should build");
    let url = openai_compatible_chat_url(&profile).expect("chat URL should build");
    let body =
        openai_compatible_chat_body(&profile, "system", "user").expect("chat body should build");

    assert_eq!(url, "https://integrate.api.nvidia.com/v1/chat/completions");
    assert_eq!(body["model"], "nvidia/llama-3.3-nemotron-super-49b-v1.5");
    assert_eq!(body["messages"][0]["role"], "system");
    assert_eq!(body["messages"][1]["content"], "user");
    assert_eq!(body["temperature"], 0.6);
    assert_eq!(body["top_p"], 0.95);
    assert_eq!(body["max_tokens"], 65_536);
    assert_eq!(body["stream"], false);
}

#[test]
fn openai_compatible_chat_request_can_force_streaming() {
    let profile = quick_add_profile(NVIDIA_NIM_QUICK_ADD_ID, Some("local:test".to_string()))
        .expect("NVIDIA NIM profile should build");
    let body = openai_compatible_chat_body_with_stream(&profile, "system", "user", Some(true))
        .expect("streaming chat body should build");

    assert_eq!(body["stream"], true);
    assert_eq!(body["model"], "nvidia/llama-3.3-nemotron-super-49b-v1.5");
}

#[test]
fn quick_add_presets_include_nvidia_nim_local_ollama_codex_and_claude() {
    let presets = quick_add_presets();
    let ids = presets.iter().map(|preset| preset.id).collect::<Vec<_>>();

    assert!(ids.contains(&NVIDIA_NIM_QUICK_ADD_ID));
    assert!(ids.contains(&CODEX_LOCAL_QUICK_ADD_ID));
    assert!(ids.contains(&CLAUDE_LOCAL_QUICK_ADD_ID));
    assert!(ids.contains(&OLLAMA_LOCAL_QUICK_ADD_ID));

    let nvidia = presets
        .iter()
        .find(|preset| preset.id == NVIDIA_NIM_QUICK_ADD_ID)
        .expect("NVIDIA NIM preset should exist");
    assert_eq!(nvidia.default_secret_ref, Some("env:NVIDIA_API_KEY"));
    assert_eq!(
        nvidia.default_model,
        Some("nvidia/llama-3.3-nemotron-super-49b-v1.5")
    );

    let ollama = presets
        .iter()
        .find(|preset| preset.id == OLLAMA_LOCAL_QUICK_ADD_ID)
        .expect("Ollama preset should exist");
    assert_eq!(ollama.adapter_type, AiAdapterType::LocalHttp);
    assert_eq!(ollama.default_secret_ref, None);

    let claude = presets
        .iter()
        .find(|preset| preset.id == CLAUDE_LOCAL_QUICK_ADD_ID)
        .expect("Claude preset should exist");
    assert_eq!(claude.adapter_type, AiAdapterType::LocalCli);
    assert_eq!(claude.default_secret_ref, None);
}

#[test]
fn nvidia_nim_quick_add_uses_chat_completions_profile_without_storing_key() {
    let profile = quick_add_profile(NVIDIA_NIM_QUICK_ADD_ID, None)
        .expect("NVIDIA NIM quick-add should build a profile");

    assert_eq!(profile.id, "nvidia-nim");
    assert_eq!(profile.label, "NVIDIA NIM");
    assert_eq!(profile.adapter_type, AiAdapterType::OpenAiCompatibleHttp);
    assert_eq!(profile.auth_scheme, AiAuthScheme::Bearer);
    assert_eq!(
        profile.endpoint.as_deref(),
        Some("https://integrate.api.nvidia.com/v1")
    );
    assert_eq!(
        profile.model.as_deref(),
        Some("nvidia/llama-3.3-nemotron-super-49b-v1.5")
    );
    assert_eq!(profile.secret_ref.as_deref(), Some("env:NVIDIA_API_KEY"));
    assert_eq!(profile.model_parameters.temperature.as_deref(), Some("0.6"));
    assert_eq!(profile.model_parameters.top_p.as_deref(), Some("0.95"));
    assert_eq!(profile.model_parameters.max_tokens, Some(65_536));
    assert_eq!(profile.model_parameters.stream, Some(false));
    assert!(!profile.has_secret_material());
}

#[test]
fn codex_quick_add_uses_local_exec_bridge() {
    let profile = quick_add_profile(CODEX_LOCAL_QUICK_ADD_ID, None)
        .expect("Codex quick-add should build a profile");

    assert_eq!(profile.id, "codex-local");
    assert_eq!(profile.adapter_type, AiAdapterType::LocalCli);
    assert_eq!(profile.command.as_deref(), Some("codex"));
    assert_eq!(profile.command_args, vec!["exec"]);
    assert!(profile.capabilities.local_execution);
    assert!(profile.capabilities.filesystem_access);
    assert_eq!(profile.secret_ref, None);
    assert!(ai_profile_is_codex_local_bridge(&profile));
}

#[test]
fn claude_quick_add_uses_local_print_bridge() {
    let profile = quick_add_profile(CLAUDE_LOCAL_QUICK_ADD_ID, None)
        .expect("Claude quick-add should build a profile");

    assert_eq!(profile.id, "claude-local");
    assert_eq!(profile.adapter_type, AiAdapterType::LocalCli);
    assert_eq!(profile.command.as_deref(), Some("claude"));
    assert_eq!(profile.command_args, vec!["-p"]);
    assert!(profile.capabilities.local_execution);
    assert!(!profile.capabilities.filesystem_access);
    assert_eq!(profile.secret_ref, None);
    assert!(ai_profile_is_claude_local_bridge(&profile));
}

#[test]
fn ollama_quick_add_uses_local_http_without_a_secret() {
    let profile = quick_add_profile(OLLAMA_LOCAL_QUICK_ADD_ID, None)
        .expect("Ollama quick-add should build a profile");

    assert_eq!(profile.id, "ollama-local");
    assert_eq!(profile.label, "Ollama Local");
    assert_eq!(profile.adapter_type, AiAdapterType::LocalHttp);
    assert_eq!(profile.auth_scheme, AiAuthScheme::None);
    assert_eq!(
        profile.endpoint.as_deref(),
        Some("http://127.0.0.1:11434/v1")
    );
    assert_eq!(profile.model.as_deref(), Some("llama3.2:latest"));
    assert_eq!(profile.secret_ref, None);
    assert!(ai_profile_is_ollama_local(&profile));
    assert!(!profile.has_secret_material());
}

#[test]
fn local_http_chat_urls_must_stay_local() {
    let mut profile = quick_add_profile(OLLAMA_LOCAL_QUICK_ADD_ID, None)
        .expect("Ollama quick-add should build a profile");
    profile.endpoint = Some("https://example.com/v1".to_string());

    let error =
        openai_compatible_chat_url(&profile).expect_err("remote local-http URLs should be blocked");
    assert!(error.message().contains("not local"));
}

#[test]
fn ollama_local_http_streaming_uses_local_chat_completions_endpoint() {
    let listener =
        std::net::TcpListener::bind("127.0.0.1:0").expect("fake local AI server should bind");
    let endpoint = format!(
        "http://{}/v1",
        listener
            .local_addr()
            .expect("fake local AI server address should be available")
    );
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .expect("fake local AI server should accept one request");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .expect("fake local AI server should set a read timeout");

        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let read = stream
                .read(&mut buffer)
                .expect("fake local AI server should read request bytes");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            let header_end = request
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .map(|index| index + 4);
            if let Some(header_end) = header_end {
                let headers = String::from_utf8_lossy(&request[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix("Content-Length:")
                            .or_else(|| line.strip_prefix("content-length:"))
                    })
                    .and_then(|value| value.trim().parse::<usize>().ok())
                    .unwrap_or(0);
                if request.len() >= header_end + content_length {
                    break;
                }
            }
        }

        let request_text = String::from_utf8_lossy(&request).to_string();
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"local \"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"Ollama\"}}]}\n\n",
            "data: [DONE]\n\n"
        );
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("fake local AI server should write response");
        request_text
    });

    let mut profile = quick_add_profile(OLLAMA_LOCAL_QUICK_ADD_ID, None)
        .expect("Ollama quick-add should build a profile");
    profile.endpoint = Some(endpoint);
    let mut streamed = String::new();
    let answer = send_ai_chat_streaming_cancellable(
        &profile,
        "system",
        "user",
        None,
        |delta| streamed.push_str(delta),
        || false,
    )
    .expect("fake Ollama-compatible server should stream an answer");
    let request_text = server
        .join()
        .expect("fake local AI server thread should not panic");

    assert!(request_text.starts_with("POST /v1/chat/completions "));
    assert!(request_text.contains("\"model\":\"llama3.2:latest\""));
    assert!(request_text.contains("\"stream\":true"));
    assert_eq!(streamed, "local Ollama");
    assert_eq!(answer, "local Ollama");
}

#[test]
fn ollama_model_recommendation_prefers_chat_models_over_embeddings() {
    let recommendation = recommended_ollama_model([
        "nomic-embed-text:latest",
        "all-minilm:latest",
        "llama3.2:latest",
        "qwen2.5:7b-instruct",
    ])
    .expect("a chat-capable local model should be recommended");

    assert_eq!(recommendation, "qwen2.5:7b-instruct");
    assert_eq!(
        recommended_ollama_model(["nomic-embed-text:latest", "mxbai-embed-large:latest"]),
        None
    );
}

#[test]
fn codex_local_exec_args_force_read_only_ephemeral_chat() {
    let profile = quick_add_profile(CODEX_LOCAL_QUICK_ADD_ID, None)
        .expect("Codex quick-add should build a profile");
    let output_path = temp_path("codex-output.txt");
    let workspace = temp_path("workspace");
    let args = codex_local_exec_args(&profile, Some(&workspace), Some(&output_path))
        .expect("Codex args should build");

    assert_eq!(args.first().map(String::as_str), Some("exec"));
    assert!(args.contains(&"--json".to_string()));
    assert!(args.contains(&"--ephemeral".to_string()));
    assert!(
        args.windows(2)
            .any(|pair| pair[0] == "--sandbox" && pair[1] == "read-only")
    );
    assert!(
        args.windows(2)
            .any(|pair| pair[0] == "--color" && pair[1] == "never")
    );
    assert!(
        args.windows(2)
            .any(|pair| pair[0] == "--output-last-message"
                && pair[1] == output_path.display().to_string())
    );
    assert!(args.windows(2).any(|pair| pair[0] == "--cd"));
    assert_eq!(args.last().map(String::as_str), Some("-"));
}

#[test]
fn codex_local_exec_args_reject_dangerous_profile_flags() {
    let mut profile = quick_add_profile(CODEX_LOCAL_QUICK_ADD_ID, None)
        .expect("Codex quick-add should build a profile");
    profile.command_args = vec![
        "exec".to_string(),
        "--dangerously-bypass-approvals-and-sandbox".to_string(),
    ];

    let error = codex_local_exec_args(&profile, None, None)
        .expect_err("dangerous Codex flags should be rejected");
    assert!(error.message().contains("dangerous Codex flags"));
}

#[test]
fn claude_local_exec_args_force_constrained_print_mode() {
    let mut profile = quick_add_profile(CLAUDE_LOCAL_QUICK_ADD_ID, None)
        .expect("Claude quick-add should build a profile");
    profile.model = Some("sonnet".to_string());
    let args = claude_local_exec_args(&profile).expect("Claude args should build");

    assert_eq!(args.first().map(String::as_str), Some("-p"));
    assert!(
        args.windows(2)
            .any(|pair| pair[0] == "--output-format" && pair[1] == "stream-json")
    );
    assert!(args.contains(&"--verbose".to_string()));
    assert!(args.contains(&"--include-partial-messages".to_string()));
    assert!(
        args.windows(2)
            .any(|pair| pair[0] == "--permission-mode" && pair[1] == "plan")
    );
    assert!(
        args.windows(2)
            .any(|pair| pair[0] == "--tools" && pair[1].is_empty())
    );
    assert!(
        args.windows(2)
            .any(|pair| pair[0] == "--max-turns" && pair[1] == "1")
    );
    assert!(args.contains(&"--no-session-persistence".to_string()));
    assert!(
        args.windows(2)
            .any(|pair| pair[0] == "--model" && pair[1] == "sonnet")
    );
    assert!(
        args.last()
            .expect("Claude query should be appended")
            .contains("stdin")
    );
}

#[test]
fn claude_local_exec_args_reject_dangerous_profile_flags() {
    let mut profile = quick_add_profile(CLAUDE_LOCAL_QUICK_ADD_ID, None)
        .expect("Claude quick-add should build a profile");
    profile.command_args = vec![
        "-p".to_string(),
        "--dangerously-skip-permissions".to_string(),
    ];

    let error =
        claude_local_exec_args(&profile).expect_err("dangerous Claude flags should be rejected");
    assert!(error.message().contains("dangerous Claude flags"));

    let mut profile = quick_add_profile(CLAUDE_LOCAL_QUICK_ADD_ID, None)
        .expect("Claude quick-add should build a profile");
    profile.command_args = vec![
        "-p".to_string(),
        "--permission-mode".to_string(),
        "bypassPermissions".to_string(),
    ];
    let error =
        claude_local_exec_args(&profile).expect_err("Claude permission bypass should be rejected");
    assert!(error.message().contains("plan mode"));

    let mut profile = quick_add_profile(CLAUDE_LOCAL_QUICK_ADD_ID, None)
        .expect("Claude quick-add should build a profile");
    profile.command_args = vec!["-p".to_string(), "--tools=Read".to_string()];
    let error =
        claude_local_exec_args(&profile).expect_err("Claude tool overrides should be rejected");
    assert!(error.message().contains("tools disabled"));
}

#[test]
fn codex_json_events_extract_assistant_text_only() {
    assert_eq!(
        codex_exec_json_event_text(r#"{"type":"agent_message_delta","delta":"Hel"}"#).as_deref(),
        Some("Hel")
    );
    assert_eq!(
        codex_exec_json_event_text(
            r#"{"type":"item.completed","item":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"lo"}]}}"#
        )
        .as_deref(),
        Some("lo")
    );
    assert_eq!(
        codex_exec_json_event_text(r#"{"type":"turn.started","message":"ignore"}"#),
        None
    );
}

#[test]
fn claude_stream_json_events_extract_text_deltas_only() {
    assert_eq!(
        claude_code_stream_json_event_text(
            r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hel"}}}"#
        )
        .as_deref(),
        Some("Hel")
    );
    assert_eq!(
        claude_code_stream_json_event_text(
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"lo"}}"#
        )
        .as_deref(),
        Some("lo")
    );
    assert_eq!(
        claude_code_stream_json_event_text(
            r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"input_json_delta","partial_json":"{}"}}}"#
        ),
        None
    );
}

#[test]
fn local_codex_bridge_prompt_is_read_only_and_preserves_contract() {
    let prompt = local_codex_bridge_prompt("system", "user payload");

    assert!(prompt.contains("read-only"));
    assert!(prompt.contains("Do not modify files"));
    assert!(prompt.contains("system"));
    assert!(prompt.contains("user payload"));
}

#[test]
fn local_claude_bridge_prompt_is_constrained_and_preserves_contract() {
    let prompt = local_claude_bridge_prompt("system", "user payload");

    assert!(prompt.contains("constrained"));
    assert!(prompt.contains("Do not modify files"));
    assert!(prompt.contains("run commands"));
    assert!(prompt.contains("system"));
    assert!(prompt.contains("user payload"));
}

#[cfg(unix)]
#[test]
fn codex_local_bridge_runs_command_and_returns_final_message() {
    use std::os::unix::fs::PermissionsExt;

    let root = temp_path("fake-codex");
    std::fs::create_dir_all(&root).expect("temp root should be created");
    let command_path = root.join("codex");
    std::fs::write(
        &command_path,
        r#"#!/bin/sh
out=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--output-last-message" ]; then
    shift
    out="$1"
  fi
  shift
done
cat >/dev/null
printf '{"type":"agent_message_delta","delta":"streaming"}\n'
printf 'final from codex' > "$out"
"#,
    )
    .expect("fake codex should be written");
    let mut permissions = std::fs::metadata(&command_path)
        .expect("fake codex metadata should load")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&command_path, permissions).expect("fake codex should be executable");

    let mut profile = AiProfile::codex_local();
    profile.command = Some(command_path.display().to_string());
    let mut streamed = String::new();
    let answer = send_ai_chat_streaming_cancellable(
        &profile,
        "system",
        "user",
        Some(&root),
        |delta| streamed.push_str(delta),
        || false,
    )
    .expect("fake Codex bridge should answer");

    assert_eq!(streamed, "streaming");
    assert_eq!(answer, "final from codex");

    std::fs::remove_dir_all(root).ok();
}

#[cfg(unix)]
#[test]
fn claude_local_bridge_runs_command_and_streams_answer() {
    use std::os::unix::fs::PermissionsExt;

    let root = temp_path("fake-claude");
    std::fs::create_dir_all(&root).expect("temp root should be created");
    let command_path = root.join("claude");
    std::fs::write(
        &command_path,
        r#"#!/bin/sh
cat >/dev/null
printf '{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"stream"}}}\n'
printf '{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"ing"}}}\n'
"#,
    )
    .expect("fake claude should be written");
    let mut permissions = std::fs::metadata(&command_path)
        .expect("fake claude metadata should load")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&command_path, permissions).expect("fake claude should be executable");

    let mut profile = AiProfile::claude_local();
    profile.command = Some(command_path.display().to_string());
    let mut streamed = String::new();
    let answer = send_ai_chat_streaming_cancellable(
        &profile,
        "system",
        "user",
        Some(&root),
        |delta| streamed.push_str(delta),
        || false,
    )
    .expect("fake Claude bridge should answer");

    assert_eq!(streamed, "streaming");
    assert_eq!(answer, "streaming");

    std::fs::remove_dir_all(root).ok();
}

#[test]
fn profiles_upsert_quick_add_profile_and_set_default() {
    let mut config = AiProfilesConfig::default();
    let profile = quick_add_profile(
        NVIDIA_NIM_QUICK_ADD_ID,
        Some("env:NVIDIA_API_KEY".to_string()),
    )
    .expect("NVIDIA profile should build");

    config.upsert_profile(profile, true);

    assert_eq!(config.profiles.len(), 1);
    assert_eq!(config.default_profile.as_deref(), Some("nvidia-nim"));
    assert_eq!(
        config
            .default_profile()
            .and_then(|profile| profile.model.as_deref()),
        Some("nvidia/llama-3.3-nemotron-super-49b-v1.5")
    );

    let replacement = quick_add_profile(NVIDIA_NIM_QUICK_ADD_ID, Some("keychain:nim".to_string()))
        .expect("replacement NVIDIA profile should build");
    config.upsert_profile(replacement, false);

    assert_eq!(config.profiles.len(), 1);
    assert_eq!(
        config
            .profile("nvidia-nim")
            .and_then(|profile| profile.secret_ref.as_deref()),
        Some("keychain:nim")
    );
}

#[test]
fn profiles_can_switch_the_default_provider_explicitly() {
    let mut config = AiProfilesConfig::default();
    config.upsert_profile(
        quick_add_profile(NVIDIA_NIM_QUICK_ADD_ID, Some("local:nim".to_string()))
            .expect("NVIDIA profile should build"),
        true,
    );
    config.upsert_profile(
        quick_add_profile(CODEX_LOCAL_QUICK_ADD_ID, None).expect("Codex profile should build"),
        false,
    );

    config
        .set_default_profile(CODEX_LOCAL_QUICK_ADD_ID)
        .expect("Codex profile should be selectable");
    assert_eq!(
        config.default_profile.as_deref(),
        Some(CODEX_LOCAL_QUICK_ADD_ID)
    );
    assert_eq!(
        config
            .default_profile()
            .map(|profile| profile.label.as_str()),
        Some("Codex Local")
    );

    let error = config
        .set_default_profile("missing-provider")
        .expect_err("unknown providers should not be selectable");
    assert!(error.message().contains("No AI profile"));
}

#[test]
fn profiles_can_turn_ai_off_without_deleting_provider_setup() {
    let mut config = AiProfilesConfig::default();
    config.upsert_profile(
        quick_add_profile(NVIDIA_NIM_QUICK_ADD_ID, Some("local:nim".to_string()))
            .expect("NVIDIA profile should build"),
        true,
    );

    config.turn_off();

    assert!(!config.enabled);
    assert_eq!(config.profiles.len(), 1);
    assert_eq!(
        config.default_profile.as_deref(),
        Some(NVIDIA_NIM_QUICK_ADD_ID)
    );

    config
        .set_default_profile(NVIDIA_NIM_QUICK_ADD_ID)
        .expect("selecting a provider should turn AI back on");
    assert!(config.enabled);
}
