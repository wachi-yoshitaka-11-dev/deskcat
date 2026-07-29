# GitHub Pages source

このdirectoryには、GitHub Pages固有の入口、Jekyll設定、404 page、入口pageのimageだけを置く。

技術文書の正本はrootのMarkdownと`docs/`である。Pages用に技術仕様、ADR、runbook、hardware値、live statusを複製しない。

`scripts/prepare-pages.ps1`は、次を`.pages-src/`へwhitelist copyする。

- `pages/_config.yml`
- `pages/index.md`
- `pages/404.md`
- `pages/assets/`配下のimageとstylesheet
- Rootの公開Markdown
- `docs/`配下のMarkdown

`pages/assets/`には、再配布権を確認済みで、入口pageが参照するimageだけを置く。1 file 1 MiBを上限とし、hardware写真や技術図のような文書向けimageはここへ置かない。

`pages/assets/css/style.scss`は、Cayman themeを差し替えずに配色だけを上書きするstylesheetである。Themeの変更はdependency reviewが必要なため、[ADR-0003](../docs/decisions/0003-public-documentation-publishing.md)に従って独立した変更として扱う。

`.pages-src/`と`_site/`は生成物であり、commitしない。

方針は[ADR-0003](../docs/decisions/0003-public-documentation-publishing.md)、操作手順は[GitHub Pages公開runbook](../docs/runbooks/github-pages-publishing.md)を参照する。
