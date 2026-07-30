# GitHub Pages source

このdirectoryには、GitHub Pages固有の入口、Jekyll設定、404 page、page表示用assetだけを置く。

技術文書の正本はrootのMarkdownと`docs/`である。Pages用に技術仕様、ADR、runbook、hardware値、live statusを複製しない。

## 入口pageの責務

`pages/index.md`は、公開文書への導線と、projectの第一印象を担う。技術情報の置き場でも、進捗の掲示板でもない。

- Projectが何であるかを短く示す。
- 主要文書への導線を、読み手の目的別に並べる。
- 技術仕様、設計値、ADR、runbook、hardware値を書かない。正本へlinkする。
- 進捗とlive statusを書かない。GitHub Issuesへlinkする。特定時点の状況を本文へ書き写すと、更新漏れがそのまま公開情報として残る。
- Concept素材を置いてよい。ただし次の両方を満たすものに限る。
  - 実機の外観、部品構成、動作を示すものではない旨を、page上へ明記する。
  - [公開asset register](../docs/governance/published-asset-register.md)へ出所と再配布許諾を登録する。

Concept素材を許すのは、[ADR-0003](../docs/decisions/0003-public-documentation-publishing.md)の判断要因「新規参加者が主要文書へ辿りやすくする」に資するためである。装飾のために技術的な断りを省略してはならない。

## 公開対象

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

このstylesheetは`@import "{{ site.theme }}"`でCaymanのSCSSを読み込む。CaymanのSCSSはGoogle Fontsへの`@import url(...)`を含み、Caymanのlayoutも同じfontを`<link>`で読み込む。よってPagesは外部fontとしてOpen Sansを取得する。これは意図した依存であり、stylesheet側もLatinへOpen Sansを当てて実際に使用する。読み込むが使わない状態を避けるためである。日本語はOSのUI fontへ落ちる。

外部fontの読み込み自体を止める場合は、`font-family`の上書きでは足りず、theme SCSSとlayoutを自前で持つ必要がある。theme更新の恩恵を失うため、独立した変更として判断する。

`.pages-src/`と`_site/`は生成物であり、commitしない。

方針は[ADR-0003](../docs/decisions/0003-public-documentation-publishing.md)、操作手順は[GitHub Pages公開runbook](../docs/runbooks/github-pages-publishing.md)を参照する。
