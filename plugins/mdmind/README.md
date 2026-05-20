# mdmind Agent Plugin

This plugin bundles the two mdmind skills for agents:

- `mdmind-map-authoring`
- `mdm-cli-inspection`

The same plugin root is used for Claude Code and Codex:

- Claude Code manifest: `.claude-plugin/plugin.json`
- Codex manifest: `.codex-plugin/plugin.json`
- Bundled skills: `skills/`

## Sync Skills

The canonical skill sources still live under the repository root `skills/`
directory. After editing those skills, refresh this plugin package:

```bash
scripts/sync-plugin-skills.sh
```

## Claude Code

Validate the marketplace and plugin from the repository root:

```bash
claude plugin validate .
claude plugin validate plugins/mdmind
```

Add this repository as a marketplace from GitHub:

```bash
claude plugin marketplace add dudash/mdmind --sparse .claude-plugin plugins
claude plugin install mdmind@mdmind
```

For local testing before pushing:

```bash
claude plugin marketplace add . --scope local
claude plugin install mdmind@mdmind --scope local
```

Claude Code exposes plugin skills with the plugin namespace, for example:

```text
/mdmind:mdmind-map-authoring
/mdmind:mdm-cli-inspection
```

## Codex

Codex reads the repo marketplace from:

```text
.agents/plugins/marketplace.json
```

Restart Codex after changing the plugin or marketplace metadata, or register
the repo marketplace explicitly:

```bash
codex plugin marketplace add .
```

Then install or enable the `mdmind` plugin from the `mdmind` marketplace in the
plugin directory. The `.agents` folder is shared agent workspace convention, but
`.agents/plugins/marketplace.json` is the Codex plugin marketplace path.
