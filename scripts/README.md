# 開発script

このディレクトリには、project作業を再現するための小さくreview可能な補助scriptを置く。

## 現在のscript

| Script | 用途 | 実行元 |
|---|---|---|
| `validate-doc-links.ps1` | リポジトリ全体のMarkdown相対linkを検査する | Pages workflowとlocal |
| `prepare-pages.ps1` | 公開対象を`.pages-src/`へ複製し、公開禁止情報を検査する。診断のfile pathはstaging-root相対で出力する | Pages workflowとlocal |
| `validate-pages-output.ps1` | 生成済み`_site/`のlinkと公開禁止情報を検査する。診断のfile pathはsite-root相対で出力する | Pages workflowとlocal |
| `test-link-validators.ps1` | source／生成siteのanchor、Pages baseurl（引用、YAML comment、末尾slashを含む）、時間制限付きHTML解析、local URL解決（encoding、unsafe scheme、directory、曖昧候補、case、reparse point、非HTML assetを含む）、公開禁止pattern・local path・値の非露出、Markdown link抽出、追跡file／symlink helperの0・1・複数件、PathSpec、Git quoting前提、非ASCII pathを検証する。link作成不可の環境では対象caseを成功件数と分けてskipする | Pages workflowとlocal |
| `test-pages-guards.ps1` | 公開境界の回帰test（未宣言asset、追跡外file、hash不一致、size超過、公開禁止patternとlocal staging pathの非露出、**Gitのmode 120000によるsymlink除外**、**file属性のreparse point除外**、拡張子）を検証する。symlinkの2 caseは、どちらのguardが働いたかをskip理由で確認する | Pages workflowとlocal |
| `lib/publish-guards.ps1` | secret／個人path pattern、path containmentとroot相対表記、追跡file列挙（`core.quotePath=false`で非ASCII pathをescapeさせない）、Gitのmodeによるsymlink判定、見出しanchor生成、Markdown link抽出、null安全な読み出し。`validate-doc-links.ps1`、`prepare-pages.ps1`、`validate-pages-output.ps1`、`test-link-validators.ps1`がdot-sourceする（`test-pages-guards.ps1`は`prepare-pages.ps1`を子processとして起動するため直接は参照しない） | dot-source専用 |

`lib/publish-guards.ps1`は単体で実行しない。secretや個人pathのpatternはこのfileだけで定義し、各scriptへ複製しない。

## 実行環境の前提

PowerShell 7以降（`pwsh`）を前提とする。Pages workflowも`shell: pwsh`で実行する。

この前提により、非ASCIIを含むscriptにBOMを付けない。`pwsh`はBOMなしをUTF-8として
読む。Windows PowerShell 5.1はBOMなしをANSIとして読むため日本語コメントが壊れるが、
5.1は対象外とする。PSScriptAnalyzerの`PSUseBOMForUnicodeEncodedFile`はこの前提のもとで
意図的に満たしていない。5.1対応が必要になった時点でBOM付与を再検討する。

Localでのbuild前検査:

```powershell
./scripts/validate-doc-links.ps1
./scripts/test-link-validators.ps1
./scripts/prepare-pages.ps1
./scripts/test-pages-guards.ps1
```

Pages CIは`test-link-validators.ps1`と`test-pages-guards.ps1`をrunnerの一時directoryから
絶対pathで起動する。両harnessはrepository root以外のcurrent directoryでも成功し、
`PAGES_SOURCE=.pages-src`をrepository root基準で解決しなければならない。

`validate-pages-output.ps1`は生成済みの`_site/`を対象とするため、上記の検査だけでは実行できない。
localで実行するにはJekyll build（Ruby、Jekyll、GitHub Pages gem）が必要である。

```powershell
# Jekyll buildを実行できる環境の場合
jekyll build --source .pages-src --destination _site
./scripts/validate-pages-output.ps1 -SiteRoot ./_site
```

`bundle exec`は使わない。このrepositoryは`Gemfile`を追跡しておらず、Bundlerが解決する対象が無い。
CIのPages buildは`actions/jekyll-build-pages`が内部のGemfileで実行するため、repository側の
`Gemfile`は使われない。localで版を固定したい場合は各自の環境で`Gemfile`を用意する。
**その`Gemfile`はCIのbuild環境とは一致しない**ため、localの成功をCIの成功の根拠にしない。

Jekyll環境を用意しない端末では、出力検査はPull RequestのCIに任せる。
その場合、build後にしか分からない問題（`.md` linkの未変換、生成siteでの404）は
CIで初めて検出される。

規則:

- 前提条件と使用方法を記載する。
- 安全側で失敗し、error時は0以外のstatusを返す。
- 秘密情報を埋め込まない。
- 固定されていないremote contentをdownload・実行しない。
- project固有の必要性がない限り、単純な標準Cargo／ESP-IDF commandを複製しない。
