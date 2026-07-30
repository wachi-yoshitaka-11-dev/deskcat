# GitHub Pages source

このdirectoryには、GitHub Pages固有の入口、Jekyll設定、404 page、page表示用assetだけを置く。

技術文書の正本はrootのMarkdownと`docs/`である。Pages用に技術仕様、ADR、runbook、hardware値、live statusを複製しない。

`scripts/prepare-pages.ps1`は、次を`.pages-src/`へwhitelist copyする。

- `pages/_config.yml`
- `pages/index.md`
- `pages/404.md`
- `pages/assets-manifest.psd1`が列挙した`pages/assets/`配下のasset
- Rootの公開Markdown
- `docs/`配下のMarkdown

`pages/assets/`には、入口pageが参照するassetだけを置く。公開対象は`pages/assets-manifest.psd1`が列挙したexact pathに限られ、列挙外のfileを置くとbuildが失敗する。Assetを追加する手順は次のとおり。

1. [公開asset register](../docs/governance/published-asset-register.md)へ出所と再配布許諾を登録する。
2. Imageは表示寸法の2倍程度へ縮小する。1 fileの上限は1 MiBである。
3. `pages/assets-manifest.psd1`へpathを追加する。Binaryはあわせて`Sha256`を記録する。
4. Gitへ追跡させる。追跡外のfileはmanifestへ書いても公開されない。

Hardware写真や技術図のような文書向けimageはここへ置かない。境界の回帰testは`scripts/test-pages-guards.ps1`にある。

`pages/assets/css/style.scss`は、Cayman themeを差し替えずに上書きするtheme override stylesheetである。対象は配色、typography、table、blockquote、code block、responsive layoutである。Themeの差し替えはdependency reviewが必要なため、[ADR-0003](../docs/decisions/0003-public-documentation-publishing.md)に従って独立した変更として扱う。

このstylesheetは`@import "{{ site.theme }}"`でCaymanのSCSSを読み込む。CaymanのSCSSはGoogle Fontsへの`@import url(...)`を含み、Caymanのlayoutも同じfontを`<link>`で読み込む。よってPagesは外部fontを取得する。`font-family`の上書きでは、読み込み自体は止まらない。停止するにはtheme SCSSとlayoutを自前で持つ必要があり、theme更新の恩恵を失う。

`.pages-src/`と`_site/`は生成物であり、commitしない。

方針は[ADR-0003](../docs/decisions/0003-public-documentation-publishing.md)、操作手順は[GitHub Pages公開runbook](../docs/runbooks/github-pages-publishing.md)を参照する。
