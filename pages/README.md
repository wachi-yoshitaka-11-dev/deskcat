# GitHub Pages source

このdirectoryには、GitHub Pages固有の入口、Jekyll設定、404 page、layout、page表示用assetだけを置く。

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
- `scripts/prepare_pages.py`の`PORTAL_LAYOUTS`が列挙した`pages/_layouts/`配下のlayout
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
4. Gitへ追跡させる。追跡外のfileをmanifestへ書くと、`prepare_pages.py`が`Declared asset is not tracked by Git`で失敗する。そのassetが公開されないだけでなく、staging全体が止まる。

Hardware写真や技術図のような文書向けimageはここへ置かない。境界の回帰testは`scripts/test_pages_guards.py`にある。

## Layoutとstylesheet

[ADR-0009](../docs/decisions/0009-pages-own-layout.md)に従い、**layoutとstylesheetの正本はこのdirectoryにある。**themeのlayoutもSCSSも生成siteへ届かない。

`pages/_layouts/`には次の3枚だけを置く。公開対象は`scripts/prepare_pages.py`の`PORTAL_LAYOUTS`が列挙したexact pathに限られ、列挙外のfileを置くとbuildが失敗する。

| layout | 当たるpage | 機構 |
|---|---|---|
| `default.html` | `pages/404.md`とfallback | front matterの`layout: default` |
| `home.html` | `pages/index.md` | front matterの`layout: home` |
| `page.html` | `docs/`配下の約40 pageとroot直下の公開文書 | GitHub Pages既定pluginの`jekyll-default-layout`が注入する |

front matterを持たないMarkdownへlayoutを割り当てるのは`jekyll-default-layout`である。**fallbackは横並びの1本ではなく、文書の種別ごとに決まる。**

| 文書の種別 | 探す順 |
|---|---|
| 入口page（`url == "/"`） | `home` → `page` → `default` |
| page（`Jekyll::Page`） | `page` → `default` |
| post | `post` → `default` |
| collection document | collection名 → `default` |

pageが`post`へ落ちることはなく、postが`page`へ落ちることもない。どれも存在しなければlayoutは付かない。

**`page.html`があるおかげで、`docs/`配下のfileを1行も変更せずに文書用layoutが当たる。**`default.html`を欠かすと、`page.html`も消したときにlayoutなしのHTMLが生成される。

`pages/_config.yml`の`theme: jekyll-theme-cayman`は**削除しない。**`github-pages` gemの`Configuration::DEFAULTS`が`theme => jekyll-theme-primer`を持つため、keyを消すとreviewしていないprimerが暗黙に有効化される。review済みのCaymanをinertなbaseとして残す方が安全である。`{% seo %}`（jekyll-seo-tag）が使えるのも、このgemの依存だからである。

`pages/assets/css/style.scss`はthemeのSCSSを読み込まない。配色、typography、chrome、card、table、blockquote、code block、responsive layout、a11yを自前で持つ。文字色と背景色の組は、明暗いずれのmodeでもWCAG AAのcontrast比（4.5:1、24 px以上または18.66 px以上のboldは3:1）を満たす。Sassのcompileはこの端末では実行できず、GitHub ActionsのPR buildが唯一の検証経路である。

外部fontはGoogle FontsのM PLUS Rounded 1c（SIL OFL 1.1）を`_layouts/default.html`の`<link>`で読み込む。日本語glyphを持つ丸ゴシックであり、和文と欧文を1書体で通せるため、混在見出しで書体が割れない。Cayman由来のOpen Sans依存はlayoutの自前化で消えている。

`favicon.ico`は`prepare_pages.py`がpixel artから生成し、`default.html`が`<link rel="icon">`で参照する。Caymanのlayoutはこのlinkをコメントアウトしたまま配信していたため、以前のfaviconはどのpageからも参照されていなかった。

`.pages-src/`と`_site/`は生成物であり、commitしない。

## Link基準の注意

`index.md`と`404.md`の相対linkは、**`.pages-src/`のroot基準**で書く。staging後にこの2 fileはroot直下へ置かれるため、`docs/governance/README.md`のように書く。

このため、`pages/index.md`をGitHubのrepository画面で開くと、相対linkはこのdirectory基準で解決されて404になる。これはlinkの誤りではなく、生成siteでのみ有効な記法である。

`scripts/validate_doc_links.py`はrepository全体の追跡Markdownを検査する。そのうちlinkをstaging root基準で解決するのは、この2 fileだけである。他のfileのlinkは、そのfile自身の位置から解決する。link先を変更したら同scriptを実行する。

方針は[ADR-0003](../docs/decisions/0003-public-documentation-publishing.md)、操作手順は[GitHub Pages公開runbook](../docs/runbooks/github-pages-publishing.md)を参照する。
