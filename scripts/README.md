# 開発script

このディレクトリには、project作業を再現するための小さくreview可能な補助scriptを置く。

## 現在のscript

| Script | 用途 | 実行元 |
|---|---|---|
| `validate_doc_links.py` | リポジトリ全体のMarkdown相対linkを検査する。公開対象の判定は`prepare_pages.py`と揃え、Gitのmode 120000のsymlinkを経由するpathは複製されないため未公開として扱う。**扱いが未確認の文字を含む見出しへのfragment linkを拒否する**（anchorが一致していても通さない。一致しているのはこちらの計算どうしであり、生成site側と一致する保証が無い。外した場合に落ちるのはJekyll buildの後であり、localでは分からない）。**走査に先立ち、閉じていないcode fenceを検出して打ち切る**（fenceが奇数個だと以降の行がすべてfence内と見なされ、linkも見出しも検査されないまま`BROKEN=0`になる） | Pages workflowとlocal |
| `prepare_pages.py` | 公開対象を`.pages-src/`へ複製し、公開禁止情報を検査する。診断のfile pathはstaging-root相対で出力する | Pages workflowとlocal |
| `validate_pages_output.py` | 生成済み`_site/`のlinkと公開禁止情報を検査する。拡張子allowlistとsize上限は`.pages-src/`側と同じ値を使う。診断のfile pathはsite-root相対で出力し、`EXTENSIONS=`／`UNSCANNED=`／`LARGEST=`で公開物の内訳を残す | Pages workflowとlocal |
| `test_link_validators.py` | source／生成siteのanchor、Pages baseurl（引用、YAML comment、末尾slashを含む）、時間制限付きHTML解析、local URL解決（encoding、unsafe scheme、directory、曖昧候補、case、reparse point、非HTML assetを含む）、公開禁止pattern・local path・値の非露出、Markdown link抽出、追跡file／symlink helperの0・1・複数件、PathSpec、Git quoting前提、非ASCII path、**symlinkを途中に挟むpathとsymlink自身へのlinkが未公開として報告されること**、**閉じていないcode fenceが、その後ろの壊れたlinkを隠す前に失敗すること**を検証する。link作成不可の環境では対象caseを成功件数と分けてskipする | Pages workflowとlocal |
| `test_pages_guards.py` | 公開境界の回帰test（未宣言asset、追跡外file、hash不一致、size超過、公開禁止patternとlocal staging pathの非露出、**Gitのmode 120000によるsymlink除外**、**file属性のreparse point除外**、拡張子）を検証する。あわせて`pages/_layouts/`の境界（`PORTAL_LAYOUTS`の列挙外、欠落、追跡外、symlink）と、faviconを検証する。**faviconは、encoderとは独立に書いた復号でICOをASCII artへ戻して突き合わせる**（生成bytesを`build_favicon()`と比べても両辺が同じ関数由来で常に一致し、channel順や行順の誤りを検出できない）。symlinkのcaseは、どちらのguardが働いたかをskip理由で確認する。**manifestはin-placeで書き換えるため、実行前に改変caseの前提を1回だけ検査し、実行後に元へ戻ったことを確認する。**前提には**宣言済みassetの実体があること**も含む。cleanupが戻らなかったmanifestは実体の無いentryを持ち、本番のstagingが`Declared asset is missing`で拒否するため、放置すると無関係なcaseがまとめて失敗する。前回の残骸から始めると、backupが残骸側になって静かに固定され、無関係なcaseが誤解を招くmessageで落ちる | Pages workflowとlocal |
| `validate_instruction_entrypoint.py` | `CLAUDE.md`がGit index上でmode 100644であり、内容が`@AGENTS.md`のimport stubと1 byteも違わないことを検査する。**working treeではなくindexを読む。**working treeの実体はcheckout環境で変わるため、mode 120000が記録されたままでもsymlinkを解決する環境では成功してしまう | Pages workflowとlocal |
| `validate_table_columns.py` | 追跡下の全Markdownを走査し、GFM表のheader行とセル数が食い違う行を検出して**merge前に止める**。不足セルは空で埋まり超過セルは捨てられるため、GFMは描画時に黙って吸収する（[#289](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/289)）。**セルはセル内の文字として1文字ずつ走査して割る。**`\|`はセル内の文字であり区切りではない（区切りと数えると存在しない不整合を報告する）。**末尾のパイプは省略できる**（パイプ数から1を引く数え方は省略した行で1セルずれ、実在の破損を誤検知にする）。fence内は表として読まない | Pages workflowとlocal |
| `test_table_columns.py` | 上記2つの穴（エスケープしたパイプを区切りと数える／末尾パイプ省略で1セルずれる）を実在した事故の形で再現する単体testと、fixture repositoryへ実際にcommitして子processで起動し、exit codeと診断（file:line、header列数、実列数）まで検査する回帰test | Pages workflowとlocal |
| `report_promotion_closing_keywords.py` | base が`main`の昇格Pull Requestで、範囲内commitに含まれる`Closes #N`等のclosing keywordを検出して**報告のみ**行う（[#289](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/289)）。`closingIssuesReferences`はPull Request本文からしか計算されず、範囲内の個々のcommitは見ない。**mergeを止めない。検出しても消せない**（共有branchの履歴を書き換えない）。`--check-issue-state`で参照先Issueの状態（OPEN／CLOSED）を`gh`で確認できる（既定では確認しない。CIのjobは`gh`の認証を持たないため付けない） | Review gate workflowとlocal |
| `test_promotion_closing_keywords.py` | closing keyword検出（大小文字、複数keyword、`disclose`等の語の一部を拾わないこと）の単体testと、fixture repositoryへ実際にcommitして子processで起動し、常にexit 0（報告のみ）であることを検証する回帰test | Pages workflowとlocal |
| `scan_procurement_mentions.py` | GitHubのIssue／Pull Request本文・commentから、調達状態を示す語（`docs/hardware/tbd-register.md`のcommand 3)群Aと同じ語）を走査し**報告のみ**行う（[#289](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/289)）。同文書のcommand 3)は`docs/hardware docs/decisions docs/toolchains`のtracked Markdownしか見ず、GitHub側のIssue本文・PR本文・commentはどの引数にも入らない。**走査語はtbd-register.mdの群A行から読み取り、ここへ複製しない**（複製すると同文書が警告する「片方だけを広げた状態で走査したと言えてしまう」形になる）。ヒットの分類は`tbd-register.md`の区別の基準に従って人が行う。**CIには組み込まない**（GitHub APIのrate limitが未測定であり、走査頻度は手動運用とする）。`gh`をsubprocessで呼ぶため実行には認証済みの`gh`が要る | local（手動実行） |
| `test_procurement_mentions.py` | 語の一覧抽出（実repositoryの`tbd-register.md`に対して行う）、pattern構築、`gh api graphql`のpagination・本文/comment両方のヒット検出を、`_run_gh`をmockして検証する回帰test。networkへは出ない | Pages workflowとlocal |
| `test_instruction_entrypoint.py` | このrepository自身のindexが契約を満たすことと、mode 120000のentry、link先のpath文字列だけの内容、CRLF、未追跡のそれぞれで上のscriptが失敗することを検証する。fixtureはmode 120000をGit indexへ直接登録するため、OSのsymlink作成権限に依存しない。**modeと内容は独立に検査する**（片方だけを直して通らないことを確認する） | Pages workflowとlocal |
| `review_gate.py` | 変更範囲を`minor`／`review-required`へ分類し、head commitのtrailer（分類、自己レビュー、指示source変更の宣言）を照合する。**`Change-Class`は3値を取る。`fixup`は宣言専用で`classify`は返さない**（後始末かどうかは差分から読めない。宣言したら`Refs`で対象を示すことを要求する。[ADR-0015](../docs/decisions/0015-fixup-class-and-direct-commit-scope.md)）。`gate`は`classify`・`receipt`・`instructions`をまとめて実行する。`history`は範囲の**各commit**の宣言を見る（起点は`DECLARATION_CUTOVER`。**`gate`には含めない。**feature branchの中間commitへ宣言を要求しないため）。**免除commitも指示source変更の宣言だけは問われる。**免除が飛ばすのは`Change-Class`と`Self-Review`であり、`Instruction-Change`は飛ばさない。免除commitが指示sourceを触る場合は`DECLARATION_EXEMPT`の記録（`instruction_reviewed`と文面。**SHAと同じ場所に置く**）が要る。**base が`main`のPull Requestでは`gate`を走らせない**（理由は[review-gate.yml](../.github/workflows/review-gate.yml)のcomment と CONTRIBUTING の「Merge方式」が持つ。**ここへ複製しない**）。**意味は判定しない。**規則を持つfileの列挙と、変更行の字句的なdeny規則だけで判定し、軽微と証明できないものはすべて`review-required`にする（[ADR-0010](../docs/decisions/0010-change-class-and-review-declaration.md)）。**未解決threadと必要CIは検証しない。**未解決threadは`required_conversation_resolution`が強制するため、同じ条件を2箇所で持たない。**必要CIを強制しているものは無い**（`main`の`required_status_checks`は`null`。判断は[Repository設定](../.github/REPOSITORY_SETTINGS.md)が持つ） | Review gate workflowとlocal |
| `test_review_gate.py` | 各deny規則（数値、inline code、link、autolink、表、見出し、checkbox、HTML comment、fence内）と、**規則の列挙とcaseの対応そのもの**（規則を足してcaseを忘れたら落ちる）と、**各caseが狙った規則に当たること**（`_deny_reason`は最初に当たった規則を返すため、行の書き方次第で狙いより前の規則を踏む）と、instruction source、非Markdown、追加file、空範囲が`minor`にならないことを検証する。**`fixup`については、`Refs`の有無で受理と拒否が分かれること、`minor`経路を主張しないこと、`Instruction-Change`を代替しないこと、`classify`の出力に現れないことを検証する。**あわせてtrailerの欠落、`minor`と宣言して範囲がreview必須の場合、review後のcommit追加で宣言が無効になること、path境界の前方一致を検証する。`history`については、起点が無いhistoryで検査しないこと、起点より前のcommitを蒸し返さないこと、各commitの分類・自己レビュー・指示source宣言の欠落を検出することを検証する。**免除commitについては、指示sourceを触って記録が無ければ落ちること、記録があれば通ること、指示sourceを触らなければ記録の真偽に関係なく通ること、commit自身が`Instruction-Change`を持つなら記録が要らないこと、`Change-Class`と`Self-Review`は従来どおり免除されること**を検証する。**`summary`の3値**（検査した件数、skipしたmerge commitの件数、免除した件数）が出ることと、**どの免除が指示sourceを触るかのcommentの列挙が実測と一致すること**も検証する（手で書いた列挙が2回遅れたため）。**fixtureはfileを実際にcommitして分類させる** | Pages workflowとlocal |
| `lib/publish_guards.py` | secret／個人path pattern、path containmentとroot相対表記、追跡file列挙（`core.quotePath=false`で非ASCII pathをescapeさせない）、Gitのmodeによるsymlink判定、見出しanchor生成（**生成siteで実証した文字だけをstripする。**扱いを確認していない文字は`_ANCHOR_UNVERIFIED`が持ち、それを含む見出しへのlinkは`validate_doc_links.py`が拒否する）、fence外行の抽出と閉じ忘れfenceの検出、Markdown link抽出、null安全な読み出し、reparse pointを跨がないtree走査。`validate_doc_links.py`、`prepare_pages.py`、`validate_pages_output.py`、`test_link_validators.py`、`test_pages_guards.py`、`test_instruction_entrypoint.py`、`test_review_gate.py`がimportする。`validate_instruction_entrypoint.py`と`review_gate.py`はimportしない。test harnessはいずれも、importとは別に対象scriptを子processとして起動し、exit codeと診断出力まで検査する | import専用 |
| `hooks/gh_metadata_guard.py` | Claude CodeのPreToolUse hook。`gh issue create`／`gh pr create`に`--project`（短縮形`-p`）が無い場合と、`gh pr merge`のsquash messageが`Change-Class`／`Self-Review`を持たない場合に**拒否**する。**判定は字句だけで行い、意味は判定しない。**stdin（`-F -`）で渡された本文は読めないため、読めないことを理由に止める。`DESKCAT_SKIP_GH_GUARD=1`で無効化できる | Claude Codeのhook |
| `hooks/branch_base_guard.py` | Claude CodeのPreToolUse hook。`git checkout -b`／`git switch -c`の基点が最新の`origin/develop`から遅れている場合に**拒否**する。**`git fetch`を伴う**（fetchしないとlocalの`origin/develop`自体が古いままで一致し、本当の失敗を検出できない）。基点を明示した場合と`hotfix/`で始まるbranchは対象外。`DESKCAT_SKIP_BASE_GUARD=1`で無効化できる | Claude Codeのhook |
| `hooks/merge_trailer_report.py` | Claude CodeのPostToolUse hook。`gh pr merge`の後にmerge commitのtrailerを実測して報告する。**判定を止めない。****確認できなかった場合は「入っている」と書かない。**事後の検査であり、入っていなければ履歴書き換えなしには直せないが、免除へ回す判断を次の昇格まで先延ばしにしないために報告する | Claude Codeのhook |
| `hooks/push_gate.py` | Claude CodeのPreToolUse hook。`develop`を更新する`git push`の前に`review_gate.py gate`を実行し、落ちた場合に**拒否**する。**直接pushはCIを通らないため、ここだけが検査の無い経路だった**（`b93b309`が`Instruction-Change`を持たないまま入った。push前に実行したのが`receipt`だけで、`receipt`は指示sourceの検査をしない）。**`git fetch`を伴う**（localの`origin/develop`が古いと、他人のcommitを自分の宣言漏れとして数える）。**直接commitしてよい基準そのものは検査しない**（trailerを落としたsquash commitをamendして直す手順を誤って止めるため）。`DESKCAT_SKIP_PUSH_GATE=1`で無効化できる | Claude Codeのhook |
| `hooks/command_line.py` | hookが受け取ったcommand文字列から、目的のprogramの呼び出しを取り出す。**語がcommand位置にあるかを見る**（単に語を探すと`echo gh pr merge`を呼び出しと読む。実際にそれで誤報告した）。区切り、絶対path、`sudo`／`env`等の前置語、`VAR=value`の代入を扱う。**shellの意味論は再現しない。**alias、function、変数展開経由の呼び出しは取れない。`hooks/gh_metadata_guard.py`、`hooks/branch_base_guard.py`、`hooks/merge_trailer_report.py`、`hooks/push_gate.py`がimportする | import専用 |
| `test_hooks.py` | `hooks/`配下の回帰test。hookを子processとして起動し、stdinへ入力JSONを渡して**拒否したか通したか**と診断文を検査する。基点の検査はfixture repositoryを作り、自分自身をoriginにして実際に`git fetch`させる（networkへ出ない）。`gh`と実際のPull Requestを要する経路は検査しない（落ちた理由がhookの誤りか環境かを区別できなくなるため） | Pages workflowとlocal |

`lib/publish_guards.py`は単体で実行しない。secretや個人pathのpatternはこのfileだけで定義し、各scriptへ複製しない。

`hooks/command_line.py`も単体で実行しない。commandの語がcommand位置にあるかの判定はこのfileだけで持ち、各hookへ複製しない。**`hooks/gh_metadata_guard.py`と`hooks/merge_trailer_report.py`は`review_gate.py`もimportする。**trailerの名前をhook側へ複製せず、正本から取るためである。名前がずれると、gateが要求するものとhookが見るものが食い違い、hookが素通りする。

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
python3 scripts/review_gate.py classify --base origin/develop --head HEAD
python3 scripts/test_review_gate.py
python3 scripts/test_hooks.py
python3 scripts/prepare_pages.py
python3 scripts/test_pages_guards.py
```

test harnessは`unittest`であり、`unittest`のrunnerからも実行できる。

```bash
python3 -m unittest discover --start-directory scripts --pattern "test_*.py" --verbose
```

Pages CIは`test_link_validators.py`、`test_pages_guards.py`、
`test_instruction_entrypoint.py`、`test_review_gate.py`、`test_hooks.py`をrunnerの一時directoryから
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
