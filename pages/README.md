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

`scripts/prepare_pages.py`は、次を`.pages-src/`へwhitelist copyする。

- `pages/_config.yml`
- `pages/index.md`
- `pages/404.md`
- `pages/assets-manifest.json`が列挙した`pages/assets/`配下のasset
- Rootの公開文書（`README.md`、`AGENTS.md`、`CONTRIBUTING.md`、`SECURITY.md`、`LICENSE`）。`LICENSE`はMarkdownではないため、`prepare_pages.py`が個別に扱う
- `docs/`配下の**Markdownだけ**

`pages/_config.yml`の`baseurl`は、top-levelのplain key `baseurl:`として1件だけ定義する。colonと非空valueの間にはspaceまたはtabを置き、valueは単一行scalarとする。空value、single／double quote、引用符の外側にあるinline comment、末尾slashはvalidatorが明示的に扱う。Quoted key、anchor／alias、block scalar、flow mappingで`baseurl`を定義しない。

複製したfileには、extensionを問わず1 fileあたり1 MiBのsize上限を適用する。

`docs/`配下の非Markdown fileは複製しない。画像はbinaryのため公開前の内容scanが効かず、EXIFや写り込みを検出できない。またGitが追跡していないfileも複製しない。localの下書きが公開へ混ざらないようにし、localとCIのstaging結果を一致させるためである。

文書向けの図版を公開する必要が生じた場合は、`docs/`へ置かず、下記のasset追加手順に従って`pages/assets/`へ登録する。

`pages/assets/`には、入口pageが参照するassetだけを置く。公開対象は`pages/assets-manifest.json`が列挙したexact pathに限られ、列挙外のfileを置くとbuildが失敗する。Assetを追加する手順は次のとおり。

1. [公開asset register](../docs/governance/published-asset-register.md)へ出所と再配布許諾を登録する。
2. Imageは表示寸法の2倍程度へ縮小する。1 fileの上限は1 MiBである。
3. `pages/assets-manifest.json`へpathを追加する。Binaryはあわせて`sha256`を記録する。
4. Gitへ追跡させる。追跡外のfileはmanifestへ書いても公開されない。

Hardware写真や技術図のような文書向けimageはここへ置かない。境界の回帰testは`scripts/test_pages_guards.py`にある。

`pages/assets/css/style.scss`は、Cayman themeを差し替えずに上書きするtheme override stylesheetである。対象は配色、typography、table、blockquote、code block、responsive layoutである。Themeの差し替えはdependency reviewが必要なため、[ADR-0003](../docs/decisions/0003-public-documentation-publishing.md)に従って独立した変更として扱う。

このstylesheetは`@import "{{ site.theme }}"`でCaymanのSCSSを読み込む。CaymanのSCSSはGoogle Fontsへの`@import url(...)`を含み、Caymanのlayoutも同じfontを`<link>`で読み込む。よってPagesは外部fontとしてOpen Sansを取得する。これは意図した依存であり、stylesheet側もLatinへOpen Sansを当てて実際に使用する。読み込むが使わない状態を避けるためである。日本語はOSのUI fontへ落ちる。

外部fontの読み込み自体を止める場合は、`font-family`の上書きでは足りず、theme SCSSとlayoutを自前で持つ必要がある。theme更新の恩恵を失うため、独立した変更として判断する。

`.pages-src/`と`_site/`は生成物であり、commitしない。

## Link基準の注意

`index.md`と`404.md`の相対linkは、**`.pages-src/`のroot基準**で書く。staging後にこの2 fileはroot直下へ置かれるため、`docs/governance/README.md`のように書く。

このため、`pages/index.md`をGitHubのrepository画面で開くと、相対linkはこのdirectory基準で解決されて404になる。これはlinkの誤りではなく、生成siteでのみ有効な記法である。

`scripts/validate_doc_links.py`は、この2 fileだけをstaging root基準で検査する。link先を変更したら同scriptを実行する。

方針は[ADR-0003](../docs/decisions/0003-public-documentation-publishing.md)、操作手順は[GitHub Pages公開runbook](../docs/runbooks/github-pages-publishing.md)を参照する。
