# ADR-0009: Pagesのlayoutとstylesheetを自前で保持する

> 状態: Accepted
> 日付: 2026-08-21

## 背景

[ADR-0003](0003-public-documentation-publishing.md)に従い、GitHub Pagesはrepositoryの
Markdownから生成する公開siteである。[#26](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/26)
の整備では、公式対応themeのCaymanを採用し、`pages/assets/css/style.scss`から
`@import "{{ site.theme }}"`でthemeのSCSSを読み込んで配色とtypographyだけを上書きしていた。
themeの差し替えはdependency reviewを要する独立した変更として扱う、という運用にしていた。

2026-08-21時点で、この構成には次の制約と不具合がある。

- **DOMがCaymanのlayoutに固定される。**`pages/index.md`は見出しと箇条書きしか置けず、
  入口pageにhero、目的別のnavigation card、mascotを作れない。stylesheetの上書きでは
  要素を増やせない。
- **`favicon.ico`がどのpageからも参照されていない。**`scripts/prepare_pages.py`が
  生成しているにもかかわらず、Caymanのlayoutは`<link rel="icon">`をコメントアウトした
  まま配信する。公開siteの実HTMLで確認した。browserが代わりに要求するdomain rootの
  `/favicon.ico`はHTTP 404であり、tabにiconが出ていない。
- **`<meta name="theme-color">`がCaymanの`#157878`のまま残る。**このsiteの配色ではない。
- Caymanのlayoutは`<!-- Setup Google Analytics -->`のような空commentを全pageへ出す。

## 判断要因

- 入口pageの表現の自由度を確保する
- reviewの対象になるdependencyを増やさない
- `prepare_pages.py`が持つ公開境界（whitelist、Git追跡確認、symlink拒否、size上限、
  拡張子allowlist）を弱めない
- 生成siteの`favicon`と`theme-color`を正しくする
- themeの更新から受けていた恩恵を失うことを受け入れられるか判断する
- 検証経路がPull RequestのCIだけである（この端末でJekyllとSassを実行できない）

## 検討した選択肢

### 選択肢A: stylesheetの上書きだけを続ける

現状維持である。dependencyは増えないが、DOMがCaymanのlayoutに固定されたままであり、
hero、navigation card、favicon、`theme-color`のいずれも実現できない。上の不具合も残る。

### 選択肢B: 別のthird-party themeへ差し替える

見た目の選択肢は増えるが、新規dependencyが増え、`remote_theme`ではbuild時に外部
repositoryを取得する。しかもDOMは新しいthemeのlayoutに固定されるため、制約の形が
変わるだけで無くならない。

### 選択肢C: `pages/_config.yml`から`theme`keyを削除してtheme無しにする

**実現しない。**`github-pages` gemの
[`Configuration::DEFAULTS`](https://github.com/github/pages-gem/blob/master/lib/github-pages/configuration.rb)
が`theme => jekyll-theme-primer`を持つため、keyを削除するとreviewしていないprimerが
暗黙に有効化される。「themeを持たない」ではなく「未reviewのthemeを持つ」状態になり、
選択肢Aより悪い。

### 選択肢D: `theme`宣言を残し、layoutとstylesheetを自前で持つ

`pages/_layouts/`を新設してlayoutの正本をrepositoryへ置き、`style.scss`から
`@import "{{ site.theme }}"`を外す。`theme: jekyll-theme-cayman`の宣言はreview済みの
inertなbaseとして残す。Caymanのlayoutもstylesheetも生成siteへ届かなくなり、
dependencyの増減はゼロである。あわせてCayman由来のOpen Sans外部font依存も消える。

## 決定

選択肢Dを採用する。

- `pages/_layouts/default.html`、`home.html`、`page.html`をlayoutの正本とする。
  front matterを持たないMarkdownへは、GitHub Pages既定pluginの
  [`jekyll-default-layout`](https://github.com/benbalter/jekyll-default-layout/blob/master/lib/jekyll-default-layout/generator.rb)
  が`page`を注入する。**`docs/`配下のfileは変更しない。**
- `pages/assets/css/style.scss`はthemeのSCSSを読み込まない。配色、typography、chrome、
  card、table、code block、responsive layout、a11yを自前で持つ。
- `pages/_config.yml`の`theme: jekyll-theme-cayman`は削除しない。削除するとprimerが
  暗黙に当たる。`{% seo %}`（jekyll-seo-tag）が使えるのも、このgemの依存だからである。
- `pages/_layouts/`の公開は`prepare_pages.py`の`PORTAL_LAYOUTS`が列挙したexact pathに
  限る。列挙外のfileがdirectoryにあればbuildを失敗させる。存在、Gitの追跡、symlinkと
  reparse point、拡張子の確認は`pages/assets/`と同じ規則を課す。
- `favicon.ico`は`prepare_pages.py`がpixel artから生成し、layoutが
  `<link rel="icon">`で参照する。
- third-party themeの導入と`theme`keyの削除は、今後も採らない。

## 影響

### 利点

- 入口pageと`docs/`の文書pageで別のlayoutを使える。`docs/`のfront matterは変更しない。
- `favicon`と`theme-color`が実際に効く。
- 生成CSSからCaymanの規則とOpen Sansの`@import url(...)`が消える。
- 新規dependencyがゼロのまま、DOMを完全に制御できる。

### 欠点

- layoutとstylesheetのa11yとresponsiveを自前で保守する。themeの更新は受けられない。
- `pages/_layouts/`の分だけ公開境界の対象が増える。
- 使っていないthemeの宣言が`_config.yml`に残る。理由をcommentで明記して補う。

### リスクと対策

| リスク | 対策 |
|---|---|
| reviewを経ていないlayoutが公開経路へ入る | `PORTAL_LAYOUTS`のexact path列挙とfail-closed、`test_pages_guards.py`の回帰test |
| `theme`keyを「使っていないから」と削除される | ADRと`_config.yml`のcommentへ、削除するとprimerが暗黙に当たる旨を記載する |
| 自前layoutの初回buildが失敗する | この端末で静的mockupを作りDOMとCSSを確認したうえで、Pull RequestのCIで数往復する前提を置く |
| 外部fontの読み込みが増える | 追加はM PLUS Rounded 1c（SIL OFL 1.1）1書体のみ。Cayman由来のOpen Sans依存が消えるため、通信先hostは増えない |
| a11yが退行する | contrast比4.5:1（大きい文字3:1）、`:focus-visible`、`prefers-reduced-motion`、44 pxのhit targetを実測で確認する |

## 検証

この決定は、次を満たすことで検証する。**確認できる時期が項目ごとに違う。**
`Upload Pages artifact`は`main`へのpushだけを対象にしているため、
**Pull Requestのrunからは生成物（`_site/`）を取り出せない。**生成HTMLとCSSの
中身に関する項目は、`develop`→`main`昇格後のread-backでしか確認できない。

Pull RequestのCIが毎回自動で見るもの:

| 項目 | 見ているもの |
|---|---|
| Pages workflowのbuildと`validate_pages_output.py`が成功する | job の結果 |
| `_site/`に`_layouts/`が出力されない | `EXTENSIONS=`の`.html`件数がpage数と一致すること |
| `REQUIRED_FILES`と`MINIMUM_PUBLISHED_COUNT`を割っていない | `validate_pages_output.py` |
| 生成siteのlinkが解決する | `BROKEN_LINKS=0` |
| `favicon.ico`が32 x 32と16 x 16の2枚を含み、ASCII artと1 pixel単位で一致する | `test_pages_guards.py`（encoderとは独立に復号して突き合わせる） |
| `pages/_layouts/`の公開境界がfail-closedである | `test_pages_guards.py` |

deploy後のread-backで見るもの（**CIでは代替できない**）:

| 項目 | 確認方法 |
|---|---|
| 生成HTMLが`<link rel="icon">`を持つ | 公開pageのHTMLを取得して確認する |
| `<meta name="theme-color">`が新paletteの値である | 同上 |
| 生成CSSにCayman由来の規則とOpen Sansの`@import url(...)`が含まれない | 公開`assets/css/style.css`を取得して確認する |
| 肉球SVGの`mask-image`が404にならない | CSSの`url()`は`validate_pages_output.py`のlink検査対象外である |

この端末で実測したもの（生成siteそのものではない）:

| 項目 | 方法 |
|---|---|
| 文字色と背景色の全組が、明暗いずれのmodeでもcontrast比の下限を満たす | layoutのliquidを展開した静的mockupを、desktop / mobile × light / dark で計測した |
| `docs/`配下のpageが`page` layoutで生成される | 現行公開siteのkramdown出力を`page.html`へ流して確認した。`docs/`のfileは変更していない（diffで確認できる） |

## 置き換える決定

なし。[ADR-0003](0003-public-documentation-publishing.md)は有効であり、本ADRは
その「site generatorのversion、保守状況、license、代替をreviewする」方針の内側で、
reviewの対象を増やさずに実装方式を定めるものである。
