# mdmind Skill Workflow Evals

This directory contains outcome-oriented eval cases for mdmind agent skills.

The evals are runner-neutral:

1. `scripts/skill_eval.py --init` creates isolated case directories.
2. A human or agent runs each prompt and writes the requested artifacts into
   `outputs/`.
3. `scripts/skill_eval.py --grade` checks the artifacts with deterministic
   assertions and `mdm` commands.

Run the checked-in golden artifacts:

```bash
python3 scripts/skill_eval.py \
  --cases evals/skill-workflows/cases.json \
  --workspace evals/skill-workflows/golden/iteration-1 \
  --configs with_skill \
  --grade
```

Create a fresh workspace:

```bash
python3 scripts/skill_eval.py \
  --cases evals/skill-workflows/cases.json \
  --workspace /tmp/mdmind-skill-evals/iteration-1 \
  --init
```

The default config names are `with_skill` and `without_skill` so maintainers can
compare skill-guided behavior against a baseline or a previous skill snapshot.
