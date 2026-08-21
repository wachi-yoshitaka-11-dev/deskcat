# GitHub Pages公開

> 状態: Verified — 2026-07-28に初回build／deployと公開結果を確認済み
> 対象Issue: [#26](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/26)
> 方針: [ADR-0003](../decisions/0003-public-documentation-publishing.md)

## 目的

RootのMarkdownと`docs/`を正本として維持し、公開対象だけをGitHub Pagesへ再現可能かつ最小権限で公開する。

## 採用方式

GitHub公式のJekyll Pages Actionを使用する。

- この端末へRuby、Jekyll、Bundlerを導入しない。
- `scripts/prepare_pages.py`で公開対象を`.pages-src/`へwhitelist copyする。
- GitHub Actions上でJekyll buildを実行する。
- `scripts/validate_pages_output.py`で必須page、local link、公開禁止patternを検査する。
- Pull Requestではbuildと検査だけを行い、artifactをdeployしない。
- `main`で成功した場合だけ`github-pages` environmentへdeployする。

## Dependency review

2026-07-28にGitHub公式repositoryと公式Pages文書を確認した。Workflowでは移動可能なmajor tagではなく、確認したcommit SHAへ固定する。

| Action | Major | 固定commit | License | 用途 |
|---|---|---|---|---|
| `actions/checkout` | `v6` | `d23441a48e516b6c34aea4fa41551a30e30af803` | MIT | Repository checkout |
| `actions/configure-pages` | `v5` | `983d7736d9b0ae728b81ab479565c72886d7745b` | MIT | Pages metadata設定 |
| `actions/jekyll-build-pages` | `v1` | `44a6e6beabd48582f863aeeb6cb2151cc1716697` | MIT | Jekyll build |
| `actions/upload-pages-artifact` | `v4` | `7b1f4a764d45c48632c6b24a0339c27f5614fb0b` | MIT | Pages artifact upload |
| `actions/deploy-pages` | `v4` | `d6db90164ac5ed86f2b6aed7e0febac5b3c0c03e` | MIT | Pages deploy |

いずれのAction repositoryもarchivedではない。`actions/jekyll-build-pages`が使用するGitHub Pages Jekyll runtimeはMIT License、`pages/_config.yml`が宣言するCayman themeはCC0-1.0で、いずれのrepositoryもarchivedではない。

[ADR-0009](../decisions/0009-pages-own-layout.md)以降、layoutとstylesheetの正本は`pages/_layouts/`と`pages/assets/css/style.scss`であり、**Cayman themeの出力は生成siteへ届かない。**それでも`theme`宣言を残すのは、keyを削除すると`github-pages` gemの`Configuration::DEFAULTS`が`jekyll-theme-primer`を当て、reviewしていないthemeが暗黙に有効化されるためである。review済みのCaymanをinertなbaseとして固定する。

自前layoutが読み込む外部fontは次の1件である。

| Font | 取得元 | License | 用途 |
|---|---|---|---|
| M PLUS Rounded 1c | Google Fonts（`fonts.googleapis.com`／`fonts.gstatic.com`） | SIL OFL 1.1 | 和文・欧文の本文と見出し |

repositoryへfont fileを同梱せず、`<link>`で取得する。Cayman由来のOpen Sansを同じ経路で取得していたため、**通信先hostは増えない。**代替は、外部fontを使わずOSのUI fontへ落とすことである。日本語の丸ゴシックはOSによって存在しないため、見た目が端末ごとに変わることを受け入れる場合に選ぶ。

### 検討した代替

| 選択肢 | 判断 |
|---|---|
| `docs/`をbranch sourceとして直接公開 | Root文書を含めにくく、公開対象の明示的検査とPR buildを組み込みにくいため不採用 |
| MkDocsと外部theme | Navigationは強力だがPython dependencyとtheme dependencyが増えるため初期段階では不採用 |
| Repository rootをJekyll sourceにする | Source codeや将来の生成物を意図せず公開するriskがあるため不採用 |
| Whitelist stagingとGitHub公式Jekyll Action | 追加toolを端末へ導入せず、公開対象と権限を限定できるため採用 |

## 公開対象

- `pages/_config.yml`
- `pages/index.md`
- `pages/404.md`
- `scripts/prepare_pages.py`の`PORTAL_LAYOUTS`が列挙した`pages/_layouts/`配下のlayout
- `pages/assets-manifest.json`が列挙した`pages/assets/`配下のasset
- Rootの`README.md`、`AGENTS.md`、`CONTRIBUTING.md`、`SECURITY.md`、`LICENSE`
- `docs/`配下の文書

### 生成siteに含まれるもの

Jekyllは`.md`をHTMLへ変換するだけでなく、**`.md`自体も`_site/`へ複製する**。
公開siteでは両方が配信される。

```text
GET /deskcat/docs/protocol/esp32-pi-protocol.html  -> HTTP 200 (text/html)
GET /deskcat/docs/protocol/esp32-pi-protocol.md    -> HTTP 200 (text/markdown)
```

内容はHTML版と同一の公開文書であり、`.md`はsecret／個人pathのscan対象でもある。
公開してよいものだけが`.pages-src/`へ入るため、これ自体は公開境界の問題ではない。
ただし**公開surfaceは想定の倍になる**ため、非公開にしたい内容を`docs/`へ置かない前提は
HTMLだけを見て判断しない。

なお`validate_pages_output.py`は`.md`への**link**を`Unconverted Markdown link`として
拒否する。生成siteでのnavigationはHTML側へ向ける、という規則であり、`.md`が配信されて
いること自体とは別の話である。

`_site/`の実際の内訳は、Pages CIのlogに毎回出る（`EXTENSIONS=`）。

`pages/assets/`のassetは、`pages/assets-manifest.json`が列挙したexact pathだけを公開する。`prepare_pages.py`は次を失敗として扱う。

- Manifestに無いfileが`pages/assets/`にある
- Manifestが列挙したfileがdisk上に無い、またはGitの追跡対象でない
- Binary assetのSHA-256がmanifestと一致しない、または未記録である
- Text asset（`.css`、`.scss`、`.svg`、`.txt`）が`sha256`を記録している。編集ごとに古くなるため記録しない
- Manifestの`path`が`..`、絶対path、rooted pathを含む
- 承認外の拡張子、または1 MiBを超えるfile

これらの回帰testは`scripts/test_pages_guards.py`にあり、Pages workflowで実行する。追加toolは不要である。

```bash
python3 scripts/test_pages_guards.py
```

Asset追加前に[公開asset register](../governance/published-asset-register.md)へ出所と再配布許諾を登録し、実機写真ではないimageにはその旨をpage上へ明記する。

`pages/_layouts/`のlayoutは、`prepare_pages.py`の`PORTAL_LAYOUTS`が列挙したexact pathだけを公開する。`prepare_pages.py`は次を失敗として扱う。

- 列挙外のfileが`pages/_layouts/`にある
- 列挙したfileがdisk上に無い、またはGitの追跡対象でない
- 列挙したfileがsymlinkまたはreparse pointである
- 拡張子が`.html`でない

`pages/assets/css/style.scss`はthemeのSCSSを読み込まず、配色、typography、chrome、card、table、blockquote、code block、responsive layout、a11yを自前で持つ（[ADR-0009](../decisions/0009-pages-own-layout.md)）。Sassのcompileはこの端末では実行できず、GitHub ActionsのPR buildが唯一の検証経路である。**layoutを変更したときは、CIで数往復する前提を置く。**この端末でできるのは、layoutのliquidを展開した静的mockupをbrowserで確認することまでである。

`favicon.ico`は`prepare_pages.py`が32 x 32と16 x 16のpixel artから組み立て、`_layouts/default.html`が`<link rel="icon">`で参照する。以前はCaymanのlayoutがこのlinkをコメントアウトしたまま配信していたため、生成したfaviconはどのpageからも参照されていなかった。

次は公開しない。

- `.git`、`.github`、local設定、credential
- Software／firmware source
- Build artifactとtool cache
- PDF
- 個人の絶対path
- 再配布権が確認できないimage、font、binary

## Workflow

`.github/workflows/pages.yml`は次の場合に実行する。

- Pages関連文書を変更するPull Request
- Pages関連文書を`main`へpush
- `main`を対象にした手動実行

権限はjobごとに分離する。

| Job | 権限 |
|---|---|
| `build` | `contents: read`、`pages: read` |
| `deploy` | `contents: read`、`pages: write`、`id-token: write` |

RepositoryのActions defaultは`read`である。`github-pages` environmentは`main`だけをdeploy元として許可する。

## ローカルで可能なcheck

Docs / Review端末では追加toolを導入せず、`python3`の標準ライブラリだけで公開sourceを生成・検査する。

```bash
python3 scripts/prepare_pages.py
```

期待する結果:

```text
PAGES_SOURCE=.pages-src
FILES=<count> MARKDOWN=<count> DOCS_COPIED=<count> DOCS_SKIPPED=<count>
```

Jekyll buildはこの端末では未実施である。GitHub Actionsで`_site/`が生成された後、次を実行する。

```bash
python3 scripts/validate_pages_output.py --site-root ./_site
```

## 初回公開後の確認

1. `build` jobが成功している。
2. Pull Request eventからdeployされていない。
3. `deploy` jobだけがPages write権限を持つ。
4. `https://wachi-yoshitaka-11-dev.github.io/deskcat/`がHTTPSで表示される。
5. Top、404、Architecture、Governance、安全、hardware、protocol、runbook、toolchainへ辿れる。
6. Source commitとdeploymentが対応する。
7. Secret、個人path、local専用資料、PDFが公開されていない。

### 初回公開の検証記録

2026-07-28に、source commit
[`5bd2ba3`](https://github.com/wachi-yoshitaka-11-dev/deskcat/commit/5bd2ba38648eae1d0c46696944e4d631b6db582a)
からの初回公開を確認した。

| 確認対象 | 結果 |
|---|---|
| Workflow run | [Pages run 30338761812](https://github.com/wachi-yoshitaka-11-dev/deskcat/actions/runs/30338761812)が成功 |
| Build | Jekyll build、出力検査、artifact uploadが成功 |
| Deploy | Deployment `5635863240`が`success` |
| Pages設定 | `build_type: workflow`、HTTPS強制、公開URLが設定済み |
| 公開URL | `https://wachi-yoshitaka-11-dev.github.io/deskcat/` |
| Read-back | Top、404、favicon、Architecture、Governance、安全、Hardware、Protocol、Runbook、ToolchainがHTTP 200 |
| 公開範囲 | Workflowのwhitelist検査でsecret様pattern、個人path、PDF、未承認形式を拒否 |

最初のrun
[`30337279379`](https://github.com/wachi-yoshitaka-11-dev/deskcat/actions/runs/30337279379)
では、Jekyllがdirectory内の`README.md`を`index.html`へ変換する点と、
themeが参照する`favicon.ico`を出力検査へ反映できておらず失敗した。
期待pathとfavicon生成を修正し、上記の成功runで再検証した。

## 失敗時

- Workflowを成功扱いにしない。
- Pages設定をbranch sourceへ切り替えて迂回しない。
- Logとartifact生成段階を確認し、#26へ記録する。
- Actionのmajor tagへ戻して固定を弱めない。
- Deploy済みsiteに公開禁止情報がある場合は、siteを非公開化または安全なversionへ戻す操作を優先し、原因と影響を記録する。

## 公式資料

- [Using custom workflows with GitHub Pages](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages)
- [About GitHub Pages and Jekyll](https://docs.github.com/en/pages/setting-up-a-github-pages-site-with-jekyll/about-github-pages-and-jekyll)
- [Adding a theme to your GitHub Pages site using Jekyll](https://docs.github.com/en/pages/setting-up-a-github-pages-site-with-jekyll/adding-a-theme-to-your-github-pages-site-using-jekyll)
- [Deploy Pages Action](https://github.com/actions/deploy-pages)
