# GitHub Pages source

このdirectoryには、GitHub Pages固有の入口、Jekyll設定、404 pageだけを置く。

技術文書の正本はrootのMarkdownと`docs/`である。Pages用に技術仕様、ADR、runbook、hardware値、live statusを複製しない。

`scripts/prepare-pages.ps1`は、次を`.pages-src/`へwhitelist copyする。

- `pages/_config.yml`
- `pages/index.md`
- `pages/404.md`
- Rootの公開Markdown
- `docs/`配下のMarkdown

`.pages-src/`と`_site/`は生成物であり、commitしない。

方針は[ADR-0003](../docs/decisions/0003-public-documentation-publishing.md)、操作手順は[GitHub Pages公開runbook](../docs/runbooks/github-pages-publishing.md)を参照する。
