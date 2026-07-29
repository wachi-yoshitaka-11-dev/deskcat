# GitHub Pages source

このdirectoryには、GitHub Pages固有の入口、Jekyll設定、404 page、page表示用assetだけを置く。

技術文書の正本はrootのMarkdownと`docs/`である。Pages用に技術仕様、ADR、runbook、hardware値、live statusを複製しない。

`scripts/prepare-pages.ps1`は、次を`.pages-src/`へwhitelist copyする。

- `pages/_config.yml`
- `pages/index.md`
- `pages/404.md`
- `pages/assets/`配下のimageとstylesheet
- Rootの公開Markdown
- `docs/`配下のMarkdown

`pages/assets/`には、入口pageが参照するimageとstylesheetだけを置く。Imageは[公開asset register](../docs/governance/published-asset-register.md)へ出所と再配布許諾を登録したものに限り、表示寸法の2倍程度へ縮小してから追加する。1 file 1 MiBを上限とし、hardware写真や技術図のような文書向けimageはここへ置かない。

`pages/assets/css/style.scss`は、Cayman themeを差し替えずに上書きするtheme override stylesheetである。対象は配色、typography、table、blockquote、code block、responsive layoutである。Themeの差し替えはdependency reviewが必要なため、[ADR-0003](../docs/decisions/0003-public-documentation-publishing.md)に従って独立した変更として扱う。

このstylesheetは`@import "{{ site.theme }}"`でCaymanのSCSSを読み込む。CaymanのSCSSはGoogle Fontsへの`@import url(...)`を含み、Caymanのlayoutも同じfontを`<link>`で読み込む。よってPagesは外部fontを取得する。`font-family`の上書きでは、読み込み自体は止まらない。停止するにはtheme SCSSとlayoutを自前で持つ必要があり、theme更新の恩恵を失う。

`.pages-src/`と`_site/`は生成物であり、commitしない。

方針は[ADR-0003](../docs/decisions/0003-public-documentation-publishing.md)、操作手順は[GitHub Pages公開runbook](../docs/runbooks/github-pages-publishing.md)を参照する。
