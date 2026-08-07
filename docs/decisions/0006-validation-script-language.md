# ADR-0006: 検証scriptの実装言語をPythonとする

> 状態: Accepted
> 日付: 2026-08-06

## 背景

`scripts/`配下の検証scriptはPowerShell（`.ps1`）で実装し、Pages workflowも`shell: pwsh`で
実行していた。判断時点の内訳は次のとおりである。

| Script | 行数 |
|---|---|
| `test-link-validators.ps1` | 1354 |
| `validate-pages-output.ps1` | 761 |
| `test-pages-guards.ps1` | 449 |
| `lib/publish-guards.ps1` | 318 |
| `prepare-pages.ps1` | 291 |
| `validate-doc-links.ps1` | 291 |

`pwsh`はどのOSにもプリインストールされない。Windowsに入るのはWindows PowerShell 5.1であり、
`scripts/README.md`はそれを対象外と明記していた。

この前提は[ADR-0002](0002-role-based-development-environments.md)と矛盾する。ADR-0002は
Docs / Review profileの必須要件を「Git、Markdownを扱えるエディタ」とし、不要なものにRustを挙げている。
そこには文書検証scriptを実行するruntimeの記載がなく、記述どおりに用意した端末では
`./scripts/validate-doc-links.ps1`を実行できなかった。

[ADR-0005](0005-standard-development-os.md)で開発環境の標準OSを実機Linuxとした結果、
素の端末へ追加させるruntimeを見直す必要が生じた。

## 判断要因

- Docs / Review profileの必須要件を、ADR-0002の水準から不必要に引き上げない
- 標準OS（実機Linux）とCI（`ubuntu-24.04`）の双方で、追加導入なしに実行できる
- 他profileが既に必須化しているruntimeと重複させ、端末が抱えるruntimeの総数を増やさない
- 公開境界のguardを含むため、サードパーティ依存を持ち込まない
- 手書きのtest harnessを、言語標準の仕組みへ置き換える

## 検討した選択肢

### PowerShellを継続する

移行costが不要で、判定logicの同等性を確認する手間もかからない。
しかし`pwsh`の導入をDocs / Review profileの必須要件へ追加することになり、
ADR-0002が「Git、Markdownを扱えるエディタ」で足りるとした前提を引き上げる。
`ubuntu-24.04`のGitHub-hosted runnerには`pwsh`が入っているが、開発端末には入っていない。

### Python 3の標準ライブラリだけで実装する

Python 3はLinuxとmacOSでほぼプリインストールされる。加えてESP32 Build profileでは
ESP-IDFが既にPythonを必須化している（`idf_tools.py`が`python -m venv`を使う）。
`pathlib`、`os`、`stat`、`shutil`、`subprocess`、`re`、`hashlib`、`json`、`html`、
`urllib.parse`、`unicodedata`、`argparse`で現行機能を賄える。標準ライブラリの`unittest`が、手書き1803行のtest harness
（`test-link-validators.ps1` + `test-pages-guards.ps1`）を置き換える。

MarkdownとYAMLには標準parserが無い。ただし現行のPowerShell実装も、そのために
線形走査のhelperを`lib/publish-guards.ps1`へ持っており、この点で後退しない。
asset manifestはPowerShell data file（`.psd1`）を`Import-PowerShellDataFile`で読んでいたが、
これに相当する標準ライブラリが無いため、`json`で読めるJSONへ変える。

### Rust製のCLIを作る

workspace内で完結し、型と所有権による保証が得られる。
しかしDocs / Review profileはRustを「不要なもの」として明示的に排除しており、
文書検証のためにRust toolchainを要求すると、その境界を壊す。
build生成物の配布と版固定も、文書検証のためだけには重い。

## 決定

`scripts/`配下の検証scriptをPython 3で実装し、**標準ライブラリのみを使う**。
サードパーティ依存を追加しない。

判定logicの仕様は変更しない。診断message、exit code、公開境界の判断は現行と同一にする。

secretと個人pathのpatternは`scripts/lib/publish_guards.py`だけで定義し、各scriptへ複製しない。
`lib/publish-guards.ps1`が持っていたSingle Source of Truthの方針を維持する。

## 影響

### 利点

- Docs / Review profileの必須要件が、ADR-0002の水準から大きく離れない
- `unittest`がtest実行、結果集計、skip管理を担い、harness自身の実装量が減る
- CI runnerと開発端末で同じruntimeを使える
- ESP32 Build profileが既に導入するruntimeと重なり、端末が抱えるruntimeの総数が増えない

### 欠点

- Docs / Review profileの必須要件へ`python3`を追加する必要がある（ADR-0002の記述だけでは足りない）
- 移行時に判定logicの同等性を確認する作業が発生する
- `.psd1` asset manifestをJSONへ変えるため、manifestのcommentを`documentation` keyへ移す

### リスクと対策

| リスク | 対策 |
|---|---|
| 移行で判定logicが変わり、公開境界が緩む | 二重化してCIで新旧を両方実行し、`validate-doc-links`が出す`DIGEST`の一致を必須にしてから旧実装を削除する |
| .NETとPythonの標準関数の差（ordinal比較、拡張子解釈、reparse point判定）で結果が環境ごとに変わる | 差の出る箇所を`publish_guards.py`のhelperへ集約し、旧実装と同じ規則を明示的に再現する |
| 端末に`python3`が無い | `machine-profiles.md`のDocs / Review profileへ必須要件として記載する |
| Pythonのversion差で挙動が変わる | 標準ライブラリのみを使い、`ubuntu-24.04`のCIを判定の基準とする |

## 意図的に一致させない挙動

判定logicは同一にするが、次の2点だけはPython実装が旧実装と異なる。
どちらも旧実装の「意図した拒否」ではなく、PowerShellの`[Parameter(Mandatory)][string]`が
空文字列をbindできずに検査そのものを中断していた事故である。同じ事故を再現しない。

| 入力 | PowerShell実装 | Python実装 |
|---|---|---|
| `pages/_config.yml`に値なしの`baseurl:`（colonの後ろが空） | `Cannot bind argument to parameter 'Value'`で中断 | `baseurl: ""`と同じくroot Pagesとして扱う。YAMLでもJekyllでもnull＝空文字列である |
| 生成siteに0 byteの`.html` | `Cannot bind argument to parameter 'Content'`で中断 | 空の走査結果として扱い、他のguardを最後まで実行する |

いずれもfail-openではない。中断した旧実装は残りのguardを一切実行しないため、
Python実装の方が検査範囲は広い。現在の`pages/_config.yml`は`baseurl: /deskcat`であり、
どちらの入力も現状のrepositoryでは発生しない。

## 検証

- `validate-doc-links`の新旧実装が、同一checkoutに対して同じ`MARKDOWN=` `LINKS=` `BROKEN=0` `DIGEST=`を出す
- `prepare-pages`の新旧実装が、`.pages-src/`へ同一のfile集合と同一内容を生成する
- 既存の回帰caseに相当する検証が`python3 -m unittest`で通る
- 上記を`ubuntu-24.04`のCIで確認する。localの結果だけをCIの根拠にしない

見直し条件: Docs / Review profileの必須要件から`python3`を外す必要が生じた場合、
または標準ライブラリだけでは賄えない検査が必要になった場合に再検討する。

## 置き換える決定

なし。[ADR-0002](0002-role-based-development-environments.md)のprofile定義を置き換えるものではなく、
そこに記載が無かった検証script実行runtimeを補う。
