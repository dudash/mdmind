use mdmind::ai::{
    AiAdapterType, AiAuthScheme, AiIntent, AiOutput, AiProfile, AiProfilesConfig, AiRequest,
    AiScope, AiSuggestedChange, AiSuggestion, AiSuggestionTarget, CODEX_LOCAL_QUICK_ADD_ID,
    NVIDIA_NIM_LOCAL_SECRET_ID, NVIDIA_NIM_QUICK_ADD_ID, ai_profile_is_codex_local_bridge,
    ai_profiles_path_for_config_dir, ai_secrets_path_for_config_dir, codex_exec_json_event_text,
    codex_local_exec_args, contextual_presets_for_node, load_ai_profiles_from_path,
    load_local_ai_secrets_from_path, local_ai_secret_ref, local_codex_bridge_prompt,
    map_assistant_system_prompt, openai_compatible_chat_body,
    openai_compatible_chat_body_with_stream, openai_compatible_chat_url,
    parse_reviewable_map_suggestions, quick_add_presets, quick_add_profile,
    resolve_local_ai_secret_from_path, save_ai_profiles_to_path, save_local_ai_secret_to_path,
    send_ai_chat_streaming_cancellable, split_reviewable_map_suggestions,
    split_reviewable_map_suggestions_with_warning,
};
use mdmind::parser::parse_document;

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
    assert!(prompt.contains("Do not claim you changed the map directly"));
}

#[test]
fn reviewable_map_suggestion_blocks_parse_and_strip_from_chat_answer() {
    let contract = mdmind::ai::reviewable_map_suggestion_contract();
    assert!(contract.contains("tool-call-like output"));
    assert!(contract.contains("Omit target"));
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
fn quick_add_presets_include_nvidia_nim_and_local_codex() {
    let presets = quick_add_presets();
    let ids = presets.iter().map(|preset| preset.id).collect::<Vec<_>>();

    assert!(ids.contains(&NVIDIA_NIM_QUICK_ADD_ID));
    assert!(ids.contains(&CODEX_LOCAL_QUICK_ADD_ID));

    let nvidia = presets
        .iter()
        .find(|preset| preset.id == NVIDIA_NIM_QUICK_ADD_ID)
        .expect("NVIDIA NIM preset should exist");
    assert_eq!(nvidia.default_secret_ref, Some("env:NVIDIA_API_KEY"));
    assert_eq!(
        nvidia.default_model,
        Some("nvidia/llama-3.3-nemotron-super-49b-v1.5")
    );
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
fn local_codex_bridge_prompt_is_read_only_and_preserves_contract() {
    let prompt = local_codex_bridge_prompt("system", "user payload");

    assert!(prompt.contains("read-only"));
    assert!(prompt.contains("Do not modify files"));
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
