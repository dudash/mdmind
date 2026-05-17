# Agent-Facing CLI Contract

This document is the design target for using `mdm` from coding agents, scripts,
and eval harnesses. It records the current behavior where that behavior matters,
then defines the stable contract future CLI work should implement and test.

The goal is not to turn `mdm` into an agent-only tool. The goal is to preserve a
friendly terminal experience while giving agents deterministic commands,
parseable output, explicit failure semantics, and a small command surface they
can discover without reading the entire repo.

## Design Inputs

The contract follows three recurring patterns from current agent-friendly CLI
guidance:

- Speakeasy recommends keeping human workflows intact while adding
  non-interactive escape hatches, structured output, quiet/low-noise modes, and
  focused skills for agents:
  <https://www.speakeasy.com/blog/engineering-agent-friendly-cli>
- The DeployHQ CLI writeup frames the core invariant as stdout for data, stderr
  for human messages, exit codes for branching, self-discovery for command
  cataloging, and consistent JSON errors:
  <https://dev.to/martakar/building-an-ai-agent-friendly-cli-lessons-from-deployhq-cli-5p3>
- Google's Agents CLI announcement is a useful product precedent for a CLI that
  is directly usable by both agents and humans, with an agent-oriented,
  machine-readable path through setup, evaluation, and deployment:
  <https://developers.googleblog.com/agents-cli-in-agent-platform-create-to-production-in-one-cli/>

## Contract Goals

Agents must be able to:

- know which commands are safe to run non-interactively
- know which commands read files, write files, open the TUI, or access the
  network
- parse stdout without filtering progress messages, spinners, or warnings
- branch on exit codes without scraping human text
- get JSON errors in the same shape across commands
- discover the command catalog and common workflows locally
- recover from common failures with explicit next actions

Humans must still get:

- readable default output
- normal `--help` and version behavior
- interactive `mdmind` and `mdm open` flows
- stderr status messages for operations that write files or fetch remote input

## Mode Decision

`mdm` should not add a broad `--agent` mode yet.

The stable contract should instead use explicit, composable modes:

- default mode for human-readable terminal output
- `--plain` for compact line-oriented output where grep-friendly output is
  useful
- `--json` for parseable command results
- `mdm export --format json` for raw document export
- future `mdm commands --json` for command discovery

If a future command needs to suppress prompts, add a clear non-interactive flag
such as `--non-interactive` or command-specific required flags. Avoid making
`--agent` mean several hidden behaviors at once.

## Global Output Rules

Stdout is command data.

Stderr is human-facing status, warnings, diagnostics, and errors.

For agent-oriented modes:

- `--json` stdout must be valid JSON and must not include progress or decorative
  text.
- `--plain` stdout must be line-oriented command data and must not include
  progress or decorative text.
- writing commands should print the primary resulting path or identifier to
  stdout and status text to stderr.
- warnings from lossy or risky operations belong on stderr.
- `--help` and `--version` keep normal Clap behavior and are not treated as
  machine contracts.

Current behavior mostly follows this split:

- `init`, `import`, and `examples copy` print created paths to stdout.
- `init` and `import` print human status to stderr.
- `import --report` prints preservation and validation stats to stderr.
- runtime errors print `error: ...` to stderr.

## Exit Codes

Stable exit-code semantics:

| Code | Meaning | Examples |
| --- | --- | --- |
| `0` | Success | command completed, `validate` found no errors, help/version displayed |
| `1` | Runtime, data, or user-action failure | missing file, parse error, unresolved deep link, refused overwrite, unsupported import, validation errors |
| `2` | Invalid CLI usage | missing required argument, invalid flag value, mutually exclusive flags |

`mdm validate` treats warnings as success and errors as failure. In JSON mode,
validation diagnostics should still be emitted before returning code `1` for
error diagnostics.

## JSON Contract

Command-style `--json` output uses a consistent envelope. The command-specific
payload lives in `data`:

| Command | Envelope `format` | `data` payload |
| --- | --- | --- |
| `view --json` | `export_document.v1` | `ExportDocument` object |
| `open --json` | `export_document.v1` | `ExportDocument` object |
| `find --json` | `search_matches.v1` | `SearchMatch[]` |
| `tags --json` | `tag_counts.v1` | `TagCount[]` |
| `kv --json` | `metadata_rows.v1` | `MetadataRow[]` |
| `links --json` | `link_entries.v1` | `LinkEntry[]` |
| `refs --json` | `reference_rows.v1` | `ReferenceRow[]` |
| `relations --json` | `relation_rows.v1` | `RelationRow[]` |
| `validate --json` | `diagnostics.v1` | `Diagnostic[]` |
| `export --format json` | raw export | `ExportDocument` object |

`mdm export --format json` intentionally remains raw document export because
downstream tools use it as document data, not command metadata.

Target success envelope:

```json
{
  "ok": true,
  "command": "validate",
  "format": "diagnostics.v1",
  "target": "map.md",
  "data": [],
  "summary": {
    "errors": 0,
    "warnings": 0
  },
  "next_actions": []
}
```

Target error envelope:

```json
{
  "ok": false,
  "command": "view",
  "format": "error.v1",
  "target": "missing.md",
  "error": {
    "code": "file_not_found",
    "category": "runtime",
    "message": "Could not read 'missing.md': No such file or directory"
  },
  "next_actions": [
    {
      "label": "List available examples",
      "command": ["mdm", "examples", "list"],
      "writes": false
    }
  ]
}
```

Envelope fields:

- `ok`: boolean success marker.
- `command`: canonical subcommand name.
- `format`: schema name and version for `data` or `error`.
- `target`: target path or deep link when one exists.
- `data`: command result for successful command-style JSON.
- `summary`: optional compact counts or status useful before reading `data`.
- `error`: stable machine-readable failure object when `ok` is false.
- `next_actions`: optional safe follow-up commands.

Error object fields:

- `code`: stable snake_case error code.
- `category`: one of `usage`, `runtime`, `parse`, `validation`, `network`, or
  `filesystem`.
- `message`: concise human-readable message.
- `path`: optional file path.
- `line`: optional 1-based line number.
- `details`: optional object for command-specific context.

Migration risk: pre-MDM-18 consumers may expect top-level arrays from
`find --json`, `tags --json`, `kv --json`, `links --json`, `refs --json`,
`relations --json`, and `validate --json`. The envelope is the stable pre-1.0
agent contract from MDM-18 onward. `mdm export --format json` remains a raw
export object.

## Plain Output Contract

`--plain` is for line-oriented inspection, not for full-fidelity data.

Commands with `--plain` should keep these properties:

- one record per line when possible
- stable field order for grep and simple parsing
- no color, table borders, spinners, or progress indicators
- no stderr chatter except warnings and errors

`--plain` is appropriate for quick scans and shell pipelines. Agents should use
`--json` when they need reliable structured fields.

## Non-Interactive Rules

Commands must never prompt unless they are explicitly classified as
interactive.

Current interactive commands:

- `mdm open <target>` without `--preview` or `--json`
- `mdm check-keys`
- `mdmind <target>` without `--preview`
- `mdmind --check-keys`

All other current commands are non-interactive when required arguments are
provided. Writing commands must fail fast instead of prompting for overwrite,
confirmation, missing destination, or credentials. Existing examples:

- `init` refuses to overwrite unless `--force` is provided.
- `import` refuses to overwrite unless `--force` is provided.
- `examples copy` refuses to overwrite unless `--force` is provided.

Future commands that might prompt must provide a non-interactive path and must be
listed as interactive in `mdm commands --json`.

## Command Safety Table

| Command | Reads | Writes | Network | Interactive | Agent-safe usage |
| --- | --- | --- | --- | --- | --- |
| `mdm view <target>` | map | no | no | no | Use for readable context; add `--json` for structured export-shaped data. |
| `mdm find <target> <query>` | map | no | no | no | Use `--plain` for quick scans and `--json` for matches with lines, breadcrumbs, tags, and metadata. |
| `mdm tags <target>` | map | no | no | no | Use `--json` for counts or `--plain` for vocabulary scans. |
| `mdm kv <target>` | map | no | no | no | Use `--keys a,b` to narrow audits; use `--json` for rows. |
| `mdm links <target>` | map | no | no | no | Use before deep-linking if ids are unknown. |
| `mdm refs <target>` | map | no | no | no | Use for external Markdown links and images; do not confuse with relations. |
| `mdm relations <target>` | map | no | no | no | Whole-map target lists outgoing relations; deep-linked target lists incoming and outgoing context for that node. |
| `mdm validate <target>` | map | no | no | no | Run after generated or edited maps; exit `1` when diagnostics include errors. |
| `mdm export <target>` | map | no | no | no | Use `--format json`, `mermaid`, or `opml`; use `--query` for filtered exports. |
| `mdm init <path>` | templates | map file | no | no | Requires `--template`; use `--force` only when overwriting intentionally. |
| `mdm import <source>` | source | map file unless `--preview` | URL sources only | no | Prefer `--preview` for agent review; remote URL import is lossy and fetches the network. |
| `mdm examples list` | bundled examples | no | no | no | Human-readable list only today; future command catalog may cover machine discovery. |
| `mdm examples path` | installed examples | no | no | no | Prints examples directory when available. |
| `mdm examples copy <name>` | bundled examples | files | no | no | Use `all` to materialize examples; use `--force` only when intentional. |
| `mdm open <target>` | map | session/location sidecars in interactive mode | no | yes by default | Agents should use `--preview` or `--json`; avoid bare `open`. |
| `mdm check-keys` | terminal input | no | no | yes | Humans only; agents should not run it. |
| `mdm version` | no | no | no | no | Prints `mdm <version>`. |
| `mdmind <target>` | map | session/location sidecars and optional edits | no | yes by default | Humans only unless `--preview` is used. |
| `mdmind --preview <target>` | map | no | no | no | Static preview equivalent to a readable tree view. |
| `mdmind --check-keys` | terminal input | no | no | yes | Humans only; agents should not run it. |

## Command Discovery Target

`mdm commands --json` should expose the command surface in one local,
machine-readable result.

Minimum shape:

```json
{
  "version": "0.7.0",
  "commands": [
    {
      "name": "find",
      "summary": "Search labels, tags, metadata, and ids.",
      "reads": ["map"],
      "writes": [],
      "network": false,
      "interactive": false,
      "output_modes": ["pretty", "plain", "json"],
      "args": [
        {"name": "target", "required": true},
        {"name": "query", "required": true}
      ],
      "flags": [
        {"name": "--plain", "takes_value": false},
        {"name": "--json", "takes_value": false}
      ],
      "examples": [
        "mdm find TODO.md \"task:open\" --plain"
      ]
    }
  ]
}
```

The catalog should include safety metadata, not just help text. That lets an
agent choose commands without guessing which ones write files or need a TTY.

## Next Actions Target

JSON envelopes may include `next_actions` when the follow-up is mechanical and
safe.

Examples:

- after unresolved deep link: suggest `mdm links <file>`
- after validation warnings: suggest `mdm validate <file> --plain`
- after no search matches: suggest a broader `mdm tags` or `mdm kv` inspection
- after import preview: suggest validating the output path after writing
- after relation warnings: suggest `mdm links <file>` and
  `mdm relations <file> --plain`

Next actions should be command arrays, not shell strings, so callers do not need
to parse quoting:

```json
{
  "label": "Inspect available ids",
  "command": ["mdm", "links", "map.md", "--plain"],
  "writes": false
}
```

## Test Expectations

The implementation slices that follow this contract should add tests for:

- every non-interactive command exits without needing a TTY
- every `--json` command writes parseable JSON to stdout
- JSON error mode emits the same envelope shape across parse, filesystem,
  validation, usage, and network failures
- `--plain` and `--json` stay mutually exclusive where both exist
- writing commands refuse overwrite without `--force`
- `validate` exits `0` for warnings-only diagnostics and `1` for errors
- interactive commands fail clearly when run without a TTY
- no progress or warning text contaminates JSON stdout
- `mdm export --format json` remains raw document data

## Follow-Up Slices

Existing Linear slices are enough to implement the contract:

- `MDM-18`: implement consistent JSON success and error envelopes.
- `MDM-19`: add `mdm commands --json` self-discovery catalog.
- `MDM-20`: add agent breadcrumbs and next actions to structured output.
- `MDM-24`: use the contract as eval harness input for real agent workflows.

No new Linear issue is needed from this design unless the JSON migration needs a
separate compatibility/deprecation slice.
