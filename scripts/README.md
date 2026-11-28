# 開発script

このディレクトリには、project作業を再現するための小さくreview可能な補助scriptを置く。

## GitHub 投稿と commit の前の公開情報検査

`check_publication.py`は投稿本文、commit message、staged additionsを同じ判定で検査する。
Issue／PR本文、コメント、review返信は本文をfileへ保存してから投稿し、直前に次を実行する。

```bash
python3 scripts/check_publication.py --file <body-file>
python3 scripts/check_publication.py --staged
```

標準入力から渡す場合は`--stdin`を使う。終了codeは検出なしが0、公開禁止情報の検出が1、
入力を読めない・設定不正・staged binaryを検査できない場合が2である。
診断には種別と行番号だけを出し、検出値は再掲しない。

個人に固有の名前、メール、SSID、その他の識別子は、repository rootの`.publication.local.json`へ置く。
この名前は既存の`*.local.json`でgitignoreされる。fileはUTF-8のJSONで、次の4つの配列を持つ。
実際の値はこのfileだけに記入し、例やtest、Issue、PRへ転記しない。
各値はUnicode正規化と大文字・小文字の差を吸収して照合する。全配列の合計は256件まで。

```json
{"names": [], "emails": [], "ssids": [], "identifiers": []}
```

このfileが無ければ、共通pattern（指定されたプライベートIPv4範囲、区切り付きMAC／BSSID、
秘密情報、証明書、一般的なメール形式）は引き続き検査し、固有文字列だけを省いたことを表示する。
fileが読めない、構造が不正、または入力が読めない場合は失敗する。

commit前の自動検査は`.githooks/pre-commit`と`.githooks/commit-msg`が呼ぶ。
**追跡されているだけではGit hookは動かない。**専用cloneでは
`git config --local core.hooksPath .githooks`で有効にする。
複数worktreeが同じGit設定を共有する環境では、他のworktreeにも効くため、
この設定を無断で変更しない。設定できない場合はcommit時に
`git -c core.hooksPath=.githooks commit ...`を使い、両hookを起動する。
新しいfileもstaged additionsへ含める。binaryの中身は判定できないため止める。
`git commit -v`でeditor用messageに付く差分は、Gitのmarkerとstaged差分が一致した場合だけ
commit messageの走査から外す。Gitの`--cleanup`指定はhookから確定できないため、
comment行は残して走査する。判定できない形は保守的に止める。

Claude Codeでは`.claude/settings.json`が`publication_guard_hook.py`を起動する。
`gh`の本文引数／読める本文file／API fieldを対象にするが、shell展開、対話editor、
GitHub UI、MCP、その他のclientはhookから事前検査できない。これらの経路では、
送信する本文をfileへ保存して手動commandを通す。Codexでも同じ手動commandを使う。
本文を特定できない`gh`の投稿はhookが拒否する。

文書用IPv4範囲は指定のプライベート範囲でないため許可する。プライベートIPv4の例示値を
広く許可せず、文書ではplaceholderへ置き換える。メールは文書用domain、および
bot／noreply形式のmailboxを例外とする。接尾辞だけがbot形のmailboxはcommit trailer内に限り例外とし、
実在の個人メールはcommit trailer内でも検出する。
秘密値の例示は既知のplaceholder語だけを許し、任意の山括弧内の値は許可しない。
例外と検出は`test_publication_guard.py`の合成値で検証する。

## 現在のscript

| Script | 用途 | 実行元 |
|---|---|---|
| `check_publication.py` | 投稿本文・commit message・staged additionsを共通判定で検査する | 手動commandとgit hook |
| `test_publication_guard.py` | 合成データで判定、CLI、Claude Code hook、git hookを検証する | Pages workflowとlocal |
| `hooks/publication_guard_hook.py` | `gh`の本文・title・API fieldを投稿前に検査する | Claude Codeのhook |
| `lib/publication_guard.py` | 個人情報・ネットワーク識別子・秘密情報の共通判定とローカル設定を扱う | import専用 |
| `validate_doc_links.py` | リポジトリ全体のMarkdown相対linkを検査する。公開対象の判定は`prepare_pages.py`と揃え、Gitのmode 120000のsymlinkを経由するpathは複製されないため未公開として扱う。**扱いが未確認の文字を含む見出しへのfragment linkを拒否する**（anchorが一致していても通さない。一致しているのはこちらの計算どうしであり、生成site側と一致する保証が無い。外した場合に落ちるのはJekyll buildの後であり、localでは分からない）。**走査に先立ち、閉じていないcode fenceを検出して打ち切る**（fenceが奇数個だと以降の行がすべてfence内と見なされ、linkも見出しも検査されないまま`BROKEN=0`になる） | Pages workflowとlocal |
| `prepare_pages.py` | 公開対象を`.pages-src/`へ複製し、公開禁止情報を検査する。診断のfile pathはstaging-root相対で出力する | Pages workflowとlocal |
| `validate_pages_output.py` | 生成済み`_site/`のlinkと公開禁止情報を検査する。拡張子allowlistとsize上限は`.pages-src/`側と同じ値を使う。診断のfile pathはsite-root相対で出力し、`EXTENSIONS=`／`UNSCANNED=`／`LARGEST=`で公開物の内訳を残す | Pages workflowとlocal |
| `test_link_validators.py` | source／生成siteのanchor、Pages baseurl（引用、YAML comment、末尾slashを含む）、時間制限付きHTML解析、local URL解決（encoding、unsafe scheme、directory、曖昧候補、case、reparse point、非HTML assetを含む）、公開禁止pattern・local path・値の非露出、Markdown link抽出、追跡file／symlink helperの0・1・複数件、PathSpec、Git quoting前提、非ASCII path、**symlinkを途中に挟むpathとsymlink自身へのlinkが未公開として報告されること**、**閉じていないcode fenceが、その後ろの壊れたlinkを隠す前に失敗すること**を検証する。link作成不可の環境では対象caseを成功件数と分けてskipする | Pages workflowとlocal |
| `test_pages_guards.py` | 公開境界の回帰test（未宣言asset、追跡外file、hash不一致、size超過、公開禁止patternとlocal staging pathの非露出、**Gitのmode 120000によるsymlink除外**、**file属性のreparse point除外**、拡張子）を検証する。あわせて`pages/_layouts/`の境界（`PORTAL_LAYOUTS`の列挙外、欠落、追跡外、symlink）と、faviconを検証する。**faviconは、encoderとは独立に書いた復号でICOをASCII artへ戻して突き合わせる**（生成bytesを`build_favicon()`と比べても両辺が同じ関数由来で常に一致し、channel順や行順の誤りを検出できない）。symlinkのcaseは、どちらのguardが働いたかをskip理由で確認する。**manifestはin-placeで書き換えるため、実行前に改変caseの前提を1回だけ検査し、実行後に元へ戻ったことを確認する。**前提には**宣言済みassetの実体があること**も含む。cleanupが戻らなかったmanifestは実体の無いentryを持ち、本番のstagingが`Declared asset is missing`で拒否するため、放置すると無関係なcaseがまとめて失敗する。前回の残骸から始めると、backupが残骸側になって静かに固定され、無関係なcaseが誤解を招くmessageで落ちる | Pages workflowとlocal |
| `validate_instruction_entrypoint.py` | `CLAUDE.md`がGit index上でmode 100644であり、内容が`@AGENTS.md`のimport stubと1 byteも違わないことを検査する。**working treeではなくindexを読む。**working treeの実体はcheckout環境で変わるため、mode 120000が記録されたままでもsymlinkを解決する環境では成功してしまう | Pages workflowとlocal |
| `validate_table_columns.py` | 追跡下の全Markdownを走査し、GFM表のheader行とセル数が食い違う行を検出して**merge前に止める**。不足セルは空で埋まり超過セルは捨てられるため、GFMは描画時に黙って吸収する（[#289](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/289)）。**セルはセル内の文字として1文字ずつ走査して割る。**`\|`はセル内の文字であり区切りではない（区切りと数えると存在しない不整合を報告する）。**末尾のパイプは省略できる**（パイプ数から1を引く数え方は省略した行で1セルずれ、実在の破損を誤検知にする）。fence内は表として読まない | Pages workflowとlocal |
| `test_table_columns.py` | 上記2つの穴（エスケープしたパイプを区切りと数える／末尾パイプ省略で1セルずれる）を実在した事故の形で再現する単体testと、fixture repositoryへ実際にcommitして子processで起動し、exit codeと診断（file:line、header列数、実列数）まで検査する回帰test | Pages workflowとlocal |
| `report_promotion_closing_keywords.py` | base が`main`の昇格Pull Requestで、範囲内commitに含まれる`Closes #N`等のclosing keywordを検出して**報告のみ**行う（[#289](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/289)）。`closingIssuesReferences`はPull Request本文からしか計算されず、範囲内の個々のcommitは見ない。**mergeを止めない。検出しても消せない**（共有branchの履歴を書き換えない）。`--check-issue-state`で参照先Issueの状態（OPEN／CLOSED）を`gh`で確認できる（既定では確認しない。CIのjobは`gh`の認証を持たないため付けない） | Promotion closing keywords workflowとlocal |
| `test_promotion_closing_keywords.py` | closing keyword検出（大小文字、複数keyword、`disclose`等の語の一部を拾わないこと）の単体testと、fixture repositoryへ実際にcommitして子processで起動し、常にexit 0（報告のみ）であることを検証する回帰test | Pages workflowとlocal |
| `scan_procurement_mentions.py` | GitHubのIssue／Pull Request本文・commentから、調達状態を示す語（`docs/hardware/tbd-register.md`のcommand 3)群Aと同じ語）を走査し**報告のみ**行う（[#289](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/289)）。同文書のcommand 3)は`docs/hardware docs/decisions docs/toolchains`のtracked Markdownしか見ず、GitHub側のIssue本文・PR本文・commentはどの引数にも入らない。**走査語はtbd-register.mdの群A行から読み取り、ここへ複製しない**（複製すると同文書が警告する「片方だけを広げた状態で走査したと言えてしまう」形になる）。ヒットの分類は`tbd-register.md`の区別の基準に従って人が行う。**CIには組み込まない**（GitHub APIのrate limitが未測定であり、走査頻度は手動運用とする）。`gh`をsubprocessで呼ぶため実行には認証済みの`gh`が要る | local（手動実行） |
| `test_procurement_mentions.py` | 語の一覧抽出（実repositoryの`tbd-register.md`に対して行う）、pattern構築、`gh api graphql`のpagination・本文/comment両方のヒット検出を、`_run_gh`をmockして検証する回帰test。networkへは出ない | Pages workflowとlocal |
| `test_instruction_entrypoint.py` | このrepository自身のindexが契約を満たすことと、mode 120000のentry、link先のpath文字列だけの内容、CRLF、未追跡のそれぞれで上のscriptが失敗することを検証する。fixtureはmode 120000をGit indexへ直接登録するため、OSのsymlink作成権限に依存しない。**modeと内容は独立に検査する**（片方だけを直して通らないことを確認する） | Pages workflowとlocal |
| `check_wiring_figure_sync.py` | `docs/hardware/experiment-log.md`の`Wiring revision`の行が追加・変更され、`hardware/breadboard/layout.svg`が変わっていなければ落とす。**字句だけで判定し、図が記録と合うかは見ない。**図に関係しない変更は、Pull Request本文の`Wiring-Figure: not-applicable <理由>`（10字以上。`<...>`の雛形のままは不可）で通す。diffの起点はmerge base | Wiring figure sync workflowとlocal |
| `test_check_wiring_figure_sync.py` | 追加行・編集行で図が無ければ落ちること（見逃し）、他の行・文中の語・他file・行の削除・空範囲・base側の変更で落ちないこと（誤検知）、宣言の受理と拒否（理由なし・短い・雛形のまま・値違い・文中）を、fixtureへ実際にcommitして検証する | Pages workflowとlocal |
| `lib/publish_guards.py` | secret／個人path pattern、path containmentとroot相対表記、追跡file列挙（`core.quotePath=false`で非ASCII pathをescapeさせない）、Gitのmodeによるsymlink判定、見出しanchor生成（**生成siteで実証した文字だけをstripする。**扱いを確認していない文字は`_ANCHOR_UNVERIFIED`が持ち、それを含む見出しへのlinkは`validate_doc_links.py`が拒否する）、fence外行の抽出と閉じ忘れfenceの検出、Markdown link抽出、null安全な読み出し、reparse pointを跨がないtree走査。`validate_doc_links.py`、`prepare_pages.py`、`validate_pages_output.py`、`test_link_validators.py`、`test_pages_guards.py`、`test_instruction_entrypoint.py`がimportする。`validate_instruction_entrypoint.py`はimportしない。test harnessはいずれも、importとは別に対象scriptを子processとして起動し、exit codeと診断出力まで検査する | import専用 |
| `hooks/gh_metadata_guard.py` | Claude CodeのPreToolUse hook。`gh issue create`／`gh pr create`に`--project`（短縮形`-p`）が無い場合と、`gh pr merge`に`--subject`と本文が無い場合と、慎重に扱うpath（範囲は[CONTRIBUTING](../CONTRIBUTING.md#慎重に扱う-path)）を含む`gh pr merge`でCodeRabbitのreviewがどのcommitでも完走していない場合と、`gh pr merge`でレビューの記録がmerge-baseからheadまでつながっていない場合（本文の範囲が記録と一致しない、慎重に扱うpathで本文の全体が`承認 <headの40桁のSHA>`だけの人のコメントが無い、を含む。[#814](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/814)の作業リストの9）に**拒否**する。**判定は字句だけで行い、意味は判定しない**（review完走とレビューの記録の検査は、`gh`でPull Requestの状態を、gitで手元の履歴を読む）。stdin（`-F -`）で渡された本文は読めないため、読めないことを理由に止める。`DESKCAT_SKIP_GH_GUARD=1`で無効化できる。**ただしreview完走とレビューの記録の検査は無効化できない** | Claude Codeのhook |
| `hooks/review_record.py` | Claude CodeのPreToolUse／PostToolUse hook。backgroundのsubagentの中で範囲`A..B`を明示して走らせた`/code-review`を、gitの共通のdirの`deskcat-review-log.jsonl`に記録する（mainの文脈での実行は記録しない）。完了に数えるのは、報告の本文が空でないときだけである。**記録は止めない。**このlogへのWrite／Edit／MultiEdit／NotebookEdit、log名を含むBash／PowerShellを**拒否**する（[#814](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/814)の作業リストの9） | Claude Codeのhook |
| `review_coverage.py` | レビューの記録の範囲が、Pull Requestのbaseとのmerge-baseからheadまで途切れずにつながっているかを判定する。`hooks/review_record.py`、`hooks/gh_metadata_guard.py`（`gh pr merge`の前）、`review-coverage.yml`（本文の範囲だけ）が共有する | Review coverage workflow、hookからimport |
| `test_review_coverage.py` | 範囲の字句の読み取り、範囲のつながり、記録の組み、本文の範囲、記録のhook、`gh pr merge`の前の検査（`gh`を差し替え、gitは一時のrepositoryで本物を使う）、CIの入口を検査する | Pages workflowとlocal |
| `hooks/branch_base_guard.py` | Claude CodeのPreToolUse hook。`git checkout -b`／`git switch -c`の基点が最新の`origin/develop`から遅れている場合に**拒否**する。**`git fetch`を伴う**（fetchしないとlocalの`origin/develop`自体が古いままで一致し、本当の失敗を検出できない）。基点を明示した場合と`hotfix/`で始まるbranchは対象外。`DESKCAT_SKIP_BASE_GUARD=1`で無効化できる | Claude Codeのhook |
| `hooks/worktree_guard.py` | Claude CodeのPreToolUse hook。`git reset --hard`、checkout系（`git checkout`／`git switch`／`git restore`）による破棄、`git clean`の前に、**捨てる範囲に未commitの変更があるかを`git status`で見て`ask`する**（[#325](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/325)）。**汚れていなければ通す**（失うものが無い呼び出しで毎回止めると、hookごと無視される）。**`git clean`だけは状態を見ない**（`-x`が消す無視fileを`git status`で数え切れない）。`DESKCAT_SKIP_WORKTREE_GUARD=1`で無効化できる | Claude Codeのhook |
| `hooks/command_line.py` | hookが受け取ったcommand文字列から、目的のprogramの呼び出しを取り出す。**語がcommand位置にあるかを見る**（単に語を探すと`echo gh pr merge`を呼び出しと読む。実際にそれで誤報告した）。区切り、絶対path、`sudo`／`env`等の前置語、`VAR=value`の代入を扱う。**改行もcommandの区切りである**（#389。引用の外で開いたheredocのbodyと語頭の`#`以降は落とす）。**理由と取り切れない範囲は、このfileのdocstringとCONTRIBUTINGの「hookが止めたとき」が持つ。****shellの意味論は再現しない。**alias、function、変数展開経由の呼び出しは取れない。**`hooks/`配下の他のhookはすべてこれをimportする**（どの関数をどのhookが使うか、変えたときに動く範囲はdocstringが持つ） | import専用 |
| `test_hooks.py` | `hooks/`配下の回帰test。hookを子processとして起動し、stdinへ入力JSONを渡して**拒否したか通したか**と診断文を検査する。基点の検査はfixture repositoryを作り、自分自身をoriginにして実際に`git fetch`させる（networkへ出ない）。`gh`と実際のPull Requestを要する経路は検査しない（落ちた理由がhookの誤りか環境かを区別できなくなるため） | Pages workflowとlocal |

`lib/publish_guards.py`は単体で実行しない。secretや個人pathのpatternはこのfileだけで定義し、各scriptへ複製しない。

`hooks/command_line.py`も単体で実行しない。commandの語がcommand位置にあるかの判定はこのfileだけで持ち、各hookへ複製しない。

`hooks/`配下は Claude Code の hook として自動で起動される。**`.claude/settings.json`が起動元であり、hookの一覧と回避手順の正本は[CONTRIBUTINGの「hookが止めたとき」](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#hookが止めたとき)である。**stdinからhookの入力JSONを読み、`tool_input.command`だけを見る。**対象commandを実行しない。**

## 実行環境の前提

**Python 3の標準ライブラリだけ**を前提とする（[ADR-0006](../docs/decisions/0006-validation-script-language.md)）。
サードパーティpackageを導入しない。virtualenvも要らない。

Localでのbuild前検査:

```bash
python3 scripts/validate_doc_links.py
python3 scripts/test_link_validators.py
python3 scripts/validate_instruction_entrypoint.py
python3 scripts/test_instruction_entrypoint.py
python3 scripts/validate_table_columns.py
python3 scripts/test_table_columns.py
python3 scripts/test_promotion_closing_keywords.py
python3 scripts/test_procurement_mentions.py
python3 scripts/test_check_wiring_figure_sync.py
python3 scripts/test_hooks.py
python3 scripts/test_publication_guard.py
python3 scripts/prepare_pages.py
python3 scripts/test_pages_guards.py
```

test harnessは`unittest`であり、`unittest`のrunnerからも実行できる。

```bash
python3 -m unittest discover --start-directory scripts --pattern "test_*.py" --verbose
```

Pages CIは`test_link_validators.py`、`test_pages_guards.py`、
`test_instruction_entrypoint.py`、`test_check_wiring_figure_sync.py`、`test_hooks.py`、`test_publication_guard.py`をrunnerの一時directoryから
絶対pathで起動する。
いずれもrepository root以外のcurrent directoryで成功しなければならない。
`test_link_validators.py`と`test_pages_guards.py`は、あわせて`PAGES_SOURCE=.pages-src`を
repository root基準で解決しなければならない。

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

## 公開物のscan範囲

実際にGitHub Pagesへuploadされるartifactは`_site/`である。`.pages-src/`ではない。
`validate_pages_output.py`のsummaryが、その内訳を毎回logへ残す。

```text
FILES=121 HTML=58 BROKEN_LINKS=0
EXTENSIONS=(none)=1,.css=1,.html=58,.ico=1,.jpg=1,.md=58,.svg=1
UNSCANNED=.ico=1,.jpg=1
LARGEST=343587 docs/hardware/power-budget.html
```

`UNSCANNED=`はsecret／個人pathの内容scanが効かない拡張子である。**binaryは内容scanに
意味が無いため、意図して対象外にしている。** `favicon.ico`は`prepare_pages.py`が
sourceのASCII art（`FAVICON_ART_32`／`FAVICON_ART_16`）から組み立てるため、
内容はdiff reviewで読める。`.jpg`はmanifestがSHA-256で固定する。いずれも内容を
別の手段で押さえている。

`_site/`にも`.pages-src/`と同じ拡張子allowlistとsize上限を課す。上のsummaryは
2026-08-21時点の実測である（自前layout導入後。[ADR-0009](../docs/decisions/0009-pages-own-layout.md)）。
7種類すべてが許可済み・上限内であり、どちらの判定も現状no-opである。Jekyllやpluginが
将来別の拡張子を生成したときに、気付かないまま公開せず止めるために置いている。

`_layouts/`は`.pages-src/`にはあるが、Jekyllがunderscore始まりのdirectoryを出力へ
複製しないため`_site/`には現れない。`EXTENSIONS=`の`.html`件数にも入らない。

`TEXT_EXTENSIONS`へ`.xml`や`.json`を加えることは**しない**。`_site`にそれらは存在せず、
存在しない拡張子へ備えるのは推測になる。`EXTENSIONS=`が変化したら、その時点で判断する。

## 公開対象の判定を揃える

`validate_doc_links.py`（build前）と`prepare_pages.py`（staging）は、同じ「公開されるか」を
別々に判定する。ここが食い違うと、link検査を通ったlinkが生成siteで404になる。

食い違いの実体はsymlinkだった。`prepare_pages.py`はreparse point配下へ降りないため複製しないが、
`os.path.exists`はlinkを辿るため、link検査は「存在する」と判定していた。
どちらも`guards.get_tracked_symlinks`（Gitのmode 120000）を見るようにして揃えてある。

この件は生成site側でも検出できる。CIの`workflow_dispatch`で実際にJekyll buildを通したところ、
`Check documentation links`は成功し、`Validate output`が失敗した。

```text
Unconverted Markdown link in docs/governance/index.html: ../linkdir/target.md
```

多層で受けている以上、build前の判定は必須ではない。それでも直したのは、この
messageが「拡張子を`.html`へ直せ」と読め、真因である「そのpathは公開されない」へ
辿り着かないためである。**原因を出す層で報告する。**

規則:

- 前提条件と使用方法を記載する。
- 安全側で失敗し、error時は0以外のstatusを返す。
- 秘密情報を埋め込まない。
- 固定されていないremote contentをdownload・実行しない。
- project固有の必要性がない限り、単純な標準Cargo／ESP-IDF commandを複製しない。
