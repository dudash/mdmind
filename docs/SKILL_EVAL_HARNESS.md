# Skill Eval Harness

MDM-24 adds a local, repeatable eval harness for mdmind agent workflows. The
goal is to measure whether skills make agents produce better maps, choose safer
CLI commands, and validate outputs more reliably.

## Research Summary

Current agent-eval practice points toward a few design constraints:

- Start with traces while behavior is still unclear, then move to repeatable
  datasets and eval runs once "good" is understood. OpenAI's agent eval guidance
  frames traces, graders, datasets, and eval runs as the core loop for improving
  agent quality.
- Evaluate agent paths, not just final text. LangChain AgentEvals distinguishes
  deterministic trajectory matching from LLM-as-judge grading, which maps well
  to mdmind's need to check command choice and output quality separately.
- Treat coding-agent and file-edit evals as integration tests. Promptfoo's
  coding-agent eval guidance recommends plain baselines, structured assertions,
  cost/latency thresholds, disposable workspaces, trace assertions when the path
  matters, and repeated runs for non-deterministic prompts.
- Agent Skills guidance recommends comparing `with_skill` and `without_skill`
  runs in clean contexts, recording outputs, timing, grading, and aggregate
  benchmark data per iteration.
- Reusable community tooling exists, but each option has tradeoffs:
  - Promptfoo is strong for provider orchestration, assertions, repeats, and
    Codex/Claude/OpenCode agent providers.
  - Skill Bench is a small MIT-licensed GitHub Action for Claude Code skills,
    YAML eval cases, evidence-backed grading, and PR reporting.
  - Inspect AI is broad and mature for frontier/coding/agent evals, including
    arbitrary external agents and sandboxes.
  - EvoSkill is promising for automated skill evolution, but it is heavier than
    the first mdmind harness slice.

Sources:

- OpenAI agent evals: <https://developers.openai.com/api/docs/guides/agent-evals>
- Agent Skills eval guidance: <https://agentskills.io/skill-creation/evaluating-skills>
- Promptfoo coding-agent evals: <https://www.promptfoo.dev/docs/guides/evaluate-coding-agents/>
- LangChain AgentEvals: <https://docs.langchain.com/oss/python/langchain/test/evals>
- Inspect AI: <https://inspect.aisi.org.uk/>
- Skill Bench: <https://skill-bench.dev/>
- Vercel skills CLI: <https://github.com/vercel-labs/skills>
- EvoSkill: <https://github.com/sentient-agi/EvoSkill>

## Design Choice

The first mdmind harness is file-based and dependency-light:

- Eval cases live in `evals/skill-workflows/cases.json`.
- Workspaces use the common `iteration-N/<case>/<config>/outputs/` pattern.
- Configs default to `with_skill` and `without_skill` for baseline comparison.
- `scripts/skill_eval.py` initializes workspaces and grades outputs.
- Checks prefer deterministic assertions and `mdm` command results before any
  model-graded rubric.

This gives mdmind a stable local foundation without requiring a specific model,
API key, or hosted eval product. Promptfoo, Skill Bench, Inspect, or a custom
Codex/Claude runner can later execute agents and place artifacts into the same
workspace layout.

## Case Shape

Each case includes:

- `id`: stable case identifier.
- `skill`: expected skill under test.
- `capability`: broad behavior such as `map_authoring` or `cli_inspection`.
- `prompt`: realistic user task.
- `inputs`: optional files copied into the run workspace.
- `expected_artifacts`: output filenames the agent should write.
- `checks`: deterministic grading steps.

Current check types:

- `file_exists`
- `contains`
- `json_valid`
- `mdm_validate`
- `mdm_find`
- `mdm_links`
- `max_label_chars`

## Running

Grade the checked-in golden outputs:

```bash
python3 scripts/skill_eval.py \
  --cases evals/skill-workflows/cases.json \
  --workspace evals/skill-workflows/golden/iteration-1 \
  --configs with_skill \
  --grade
```

Create a new workspace:

```bash
python3 scripts/skill_eval.py \
  --workspace /tmp/mdmind-skill-evals/iteration-1 \
  --init
```

Then run an agent or manual test against each generated `prompt.md`, save
artifacts under `outputs/`, and grade:

```bash
python3 scripts/skill_eval.py \
  --workspace /tmp/mdmind-skill-evals/iteration-1 \
  --grade
```

The grader writes `grading.json` beside each run and `benchmark.json` at the
workspace root.

## What To Measure Next

The first fixtures cover:

- map authoring from meeting notes
- research synthesis into themes/evidence/questions/actions
- narrow CLI inspection for blocked owned work
- choosing raw `mdm export --format json` instead of command `--json`

The next useful additions are:

- negative trigger cases where the skill should not activate
- repeated runs to measure variance
- runner adapters that launch Codex, Claude Code, or Promptfoo automatically
- optional LLM-as-judge grading for holistic map quality after deterministic
  checks pass
- passive `AGENTS.md` vs skills vs combined-context comparison for MDM-26
