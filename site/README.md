# mdmind Website

This is a small static marketing site for GitHub Pages.

Current pages:

- `index.html`
- `downloads/index.html`

Agent and crawler discovery:

- `robots.txt`
- `sitemap.xml`
- `llms.txt`
- `.well-known/agent.json`
- `.well-known/agent-skills/index.json`
- `.well-known/agent-skills/*.tar.gz`

When the checked-in skills change, rebuild the `.well-known/agent-skills/*.tar.gz`
archives and update their SHA-256 values in the skills index.

Assets:

- `assets/novel-focus-branch.png`

If you want to publish it with GitHub Pages, the included workflow deploys the `site/` directory as a Pages artifact.
