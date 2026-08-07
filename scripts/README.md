# 開発script

このディレクトリには、project作業を再現するための小さくreview可能な補助scriptを置く。

## 現在のscript

| Script | 用途 | 実行元 |
|---|---|---|
| `validate_doc_links.py` | リポジトリ全体のMarkdown相対linkを検査する | Pages workflowとlocal |
| `prepare_pages.py` | 公開対象を`.pages-src/`へ複製し、公開禁止情報を検査する。診断のfile pathはstaging-root相対で出力する | Pages workflowとlocal |
| `validate_pages_output.py` | 生成済み`_site/`のlinkと公開禁止情報を検査する。診断のfile pathはsite-root相対で出力する | Pages workflowとlocal |
| `test_link_validators.py` | source／生成siteのanchor、Pages baseurl（引用、YAML comment、末尾slashを含む）、時間制限付きHTML解析、local URL解決（encoding、unsafe scheme、directory、曖昧候補、case、reparse point、非HTML assetを含む）、公開禁止pattern・local path・値の非露出、Markdown link抽出、追跡file／symlink helperの0・1・複数件、PathSpec、Git quoting前提、非ASCII pathを検証する。link作成不可の環境では対象caseをskipする | Pages workflowとlocal |
| `test_pages_guards.py` | 公開境界の回帰test（未宣言asset、追跡外file、hash不一致、size超過、公開禁止patternとlocal staging pathの非露出、**Gitのmode 120000によるsymlink除外**、**file属性のreparse point除外**、拡張子）を検証する。symlinkの2 caseは、どちらのguardが働いたかをskip理由で確認する | Pages workflowとlocal |
| `lib/publish_guards.py` | secret／個人path pattern、path containmentとroot相対表記、追跡file列挙（`core.quotePath=false`で非ASCII pathをescapeさせない）、Gitのmodeによるsymlink判定、見出しanchor生成、Markdown link抽出、null安全な読み出し、reparse pointを跨がないtree走査。上の5 scriptがimportする | import専用 |

`lib/publish_guards.py`は単体で実行しない。secretや個人pathのpatternはこのfileだけで定義し、各scriptへ複製しない。

## 実行環境の前提

**Python 3の標準ライブラリだけ**を前提とする（[ADR-0006](../docs/decisions/0006-validation-script-language.md)）。
サードパーティpackageを導入しない。virtualenvも要らない。

Localでのbuild前検査:

```bash
python3 scripts/validate_doc_links.py
python3 scripts/test_link_validators.py
python3 scripts/prepare_pages.py
python3 scripts/test_pages_guards.py
```

test harnessは`unittest`であり、`unittest`のrunnerからも実行できる。

```bash
python3 -m unittest discover --start-directory scripts --pattern "test_*.py" --verbose
```

Pages CIは`test_link_validators.py`と`test_pages_guards.py`をrunnerの一時directoryから
絶対pathで起動する。両harnessはrepository root以外のcurrent directoryでも成功し、
`PAGES_SOURCE=.pages-src`をrepository root基準で解決しなければならない。

`validate_pages_output.py`は生成済みの`_site/`を対象とするため、上記の検査だけでは実行できない。
localで実行するにはJekyll build（Ruby、Jekyll、GitHub Pages gem）が必要である。

```bash
# Jekyll buildを実行できる環境の場合
jekyll build --source .pages-src --destination _site
python3 scripts/validate_pages_output.py --site-root ./_site
```

`bundle exec`は使わない。このrepositoryは`Gemfile`を追跡しておらず、Bundlerが解決する対象が無い。
CIのPages buildは`actions/jekyll-build-pages`が内部のGemfileで実行するため、repository側の
`Gemfile`は使われない。localで版を固定したい場合は各自の環境で`Gemfile`を用意する。
**その`Gemfile`はCIのbuild環境とは一致しない**ため、localの成功をCIの成功の根拠にしない。

Jekyll環境を用意しない端末では、出力検査はPull RequestのCIに任せる。
その場合、build後にしか分からない問題（`.md` linkの未変換、生成siteでの404）は
CIで初めて検出される。

## PowerShellからの移行中

[ADR-0006](../docs/decisions/0006-validation-script-language.md)に従いPythonへ移行している。
公開境界のguardを含むため、旧`.ps1`をすぐには消さず、二重化して同等性を確認する。

- Pages CIが新旧を両方実行する
- `validate-doc-links.ps1`と`validate_doc_links.py`が出力する行（`DIGEST=`を含む）の一致を必須にする
- CIで一致を確認した後、`.ps1`と`pages/assets-manifest.psd1`を削除する

移行が完了するまで、判定logicを変更する場合は新旧の両方へ同じ変更を入れる。
片方だけを変えると、同等性checkがそこで落ちる。

規則:

- 前提条件と使用方法を記載する。
- 安全側で失敗し、error時は0以外のstatusを返す。
- 秘密情報を埋め込まない。
- 固定されていないremote contentをdownload・実行しない。
- project固有の必要性がない限り、単純な標準Cargo／ESP-IDF commandを複製しない。
