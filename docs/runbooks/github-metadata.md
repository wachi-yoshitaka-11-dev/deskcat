# GitHub metadataの操作と読み戻し

規則の正本は[CONTRIBUTINGの起票項目](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#起票時に設定する項目)と
[PRの手順](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#pull-request)。
`scripts/github_metadata.py`は既存の認証済み`gh`とPython標準ライブラリを使う。
tokenや認証headerをplan、journal、報告へ保存しない。新規依存・常駐botはない。

## 検査の意味

| 対象 | 機械で比較するもの | 根拠・判断が必要なもの |
|---|---|---|
| Issue | milestoneの存在とM0–M6、type/priority各1個、label語彙、assigneeの存在 | milestoneの選択、担当者、条件付きarea/needs/blocked |
| PR | milestone禁止、area/type/priorityの存在。type/priorityを各1個に制限しない | 対応Issueの指定。指定されたIssue群のlabel/assigneeの和集合と照合する |
| 両方 | 指定owner/number/titleのProject所属、Status、日付の存在・形式 | openのTodo/In Progressの選択、Issue予定・実着手日、openの完了予定 |
| PR Start | created_atをJSTへ変換した作成実績日 | IssueのStartを作成日から推定しない |
| closed Target | PRはmerged_at（未mergeならclosed_at）、Issueは最新closed_atのJST日 | 昔の予定値から実績を推定しない |
| reopen | openでDoneを拒否。操作planに新しい予定とactive Statusを要求 | 同じ日を再計画する場合もあり、古い日付だけでは再計画漏れと断定しない |

全件監査は**現在値を現行規則へ照合する**。起票当時からの違反とは断定しない。
古い予定は`notice`、確定した不一致は`violation`、値の決定に根拠が要る欠落は`pending`。
`coverage_pending`は本文の意味・当時の担当・Issue実着手日など未検証範囲であり、空の違反一覧と区別する。
`context`の根拠の真偽は機械が認証しない。閉じたIssueのStartが記入済みでも実績である証明にはならない。

## 読み取りと操作

```bash
python3 scripts/github_metadata.py audit --output before.local.json
python3 scripts/github_metadata.py check --number 466
python3 scripts/github_metadata.py check --number 466 --expect update.local.json
python3 scripts/github_metadata.py apply --plan update.local.json --journal update-journal.local.json
python3 scripts/github_metadata.py audit --output after.local.json
```

`audit`と`check`は読み取りだけ。全件取得はRESTのIssue一覧（PRを含む）とPR一覧を照合し、
RESTのidで重複を除外する。値の異なる重複は失敗にする。Project items/fieldsはcursorの終端まで取得し、
totalCount、重複id、cursorの進行を検査する。3つの値は`fieldValueByName`で取得し、
fieldValuesの先頭pageだけを見る形を避ける。Projectの名前・field型・optionをAPIで解決し、IDを固定しない。
取得失敗・null content・権限不足・GraphQL部分errorはexit 2。完全なsnapshotとは報告しない。
全件snapshotは同時刻のtransactionではないため、他操作と競合した場合は取り直す。

exit 0は**指定した検査範囲で非noticeの指摘なし**、1はviolation/pendingあり、2は取得・操作・入力失敗。
CIのexit 0はProjects合格ではない。`coverage_pending`があるときは全面準拠と報告しない。
操作結果の期待値・実値・対象番号・同じjournalによる再開方法を出力する。正しい他fieldは書き換えない。

更新plan例（値とsourceは実際の判断に置き換える。例は承認ではない）:

```json
{
  "repo": "wachi-yoshitaka-11-dev/deskcat",
  "kind": "issue",
  "number": 466,
  "source": "対象Issueのユーザー依頼と着手判断へのURL",
  "expected": {"Status": "In Progress", "Target date": "2026-09-30"},
  "context": {"areas": [], "blocked": false, "needs": [], "source": "対象範囲・依存・必要検証の根拠"}
}
```

`expected`には意図して変更・照合するfieldだけを書く。`labels`/`assignees`は正確な集合、
`milestone`は番号（PRから外すならnull）、`state`はopen/closed。
`before`へ同じfieldの監査時実値を渡せば、適用前に他者が変えた値は上書きせず停止する。
初回にbeforeを指定しなければ、その時の実値をjournalへ保存する。再実行でも保持する。
Project所属が無ければ追加するが、既存itemの削除・archive解除・他Projectへの移動はしない。

新規作成は`number`の代わりに`create: {"title": "...", "body": "..."}`を指定する。
PRではさらに明示的な`base`/`head`が必要。baseは作成後にも読み戻す。
bodyは既存templateに従って完成させる。既存hookのtemplate選択・見出し検査を再利用する。
Issue templateの表示名を明示する場合はplanの`template`へ書く。
完全なexpected（label/assignee/Status/両日付、Issueならmilestone）とcontextが必要。
PRのStart dateには`@created`を使える。contextの`related_issues`へ対応Issue番号の配列を
指定すると現在のlabel/assigneeをAPIで照合する。昇格で複数typeになることを拒否しない。
対応Issueや条件付きlabelの意味を本文の正規表現だけから推定しない。

closeは`state: closed`、`Status: Done`、`Target date: @closed`を同じplanへ書く。
DoneはIssueを自動closeし得るため、stateを省いたDone更新は拒否する。
reopenは`state: open`、active Status、新しいTarget dateのplanと新しいjournalを用意する。
再closeは再び`@closed`を使う。日付は操作後のtimestampを読み戻して決める。
PR merge自体は既存の承認・trailer手順で実施し、その後にcheck/applyで実績を確認する。
このCLIはmerge権限を追加しない。

## 部分成功・反映待ちからの復旧

create前にmarkerとattemptをjournalへ原子的に保存する。作成成功後は番号を保存し、
Project更新が失敗しても**同じplan・journal**で既存番号から再開する。
新規Project itemのworkflow既定値は初回取得時にjournalへ記録し、その後の再開でも保持する。
create応答を失った場合は全pageからbodyのmarkerを検索する。1件だけなら採用する。
0件または複数なら停止し、もう一度createを送らない。反映待ちか通信前失敗かを機械で断定しない。
後で同じjournalを再実行するか、GitHub上の結果を人が確認して対応する。
journalを捨てて別のcreate planを作る操作、別clone・別journalからの同時起票までは防げない。
指定更新を読み戻したjournal（`updates_verified`）の再実行は読み取りだけにする。
別のmetadata指摘が残っても、この記録を再更新の指示にしない。指摘があれば終了codeは非ゼロのままで、
更新済みと全面準拠は別である。他者による後の編集やreopenを元へ戻さず不一致として返す。
再更新には、新しい番号指定planとjournalを作る。

write成功は合格ではない。最大3回、2秒間隔でread-backし、期待値の不一致が残ればexit 1。
この待ち時間はAPI反映の保証上限ではない。API自体の失敗は即exit 2であり、空配列へ置換しない。
同一journalへの同時実行はlockで拒否。停止processの確認前にlockを消さない。
GitHubとのcompare-and-swapは無いためread/write間の競合余地は残る。read-backで検出し、
自動rollbackは行わない。Statusとstateへの介入は他担当と同時に行わない。

## CIと強制力

| 経路 | 実効性と限界 |
|---|---|
| ローカルapply | 既存認証でAPI更新と再取得。不一致・API失敗で完了を止める。このcommandを通った操作に効く |
| ローカルcheck/audit | CLI/MCP/UIなど経路を問わず保存済みの実値を事後検出。実行時点だけを見る |
| Metadata audit CI | Issue/PRイベントと手動dispatchでrepository内fieldだけをread-back。Project・日付・条件の意味は未検査。trusted baseのcodeを使い、PR headをtoken付きで実行しない |
| 既存Bash hook | create option/template、merge trailerの事前検査。実値検査と同義ではなく、MCP/UI/Codexを網羅しない |
| merge前後 | merge実施者がcheckを行う。CI checkは現在必須ではなく、管理者を含む全merge経路の強制停止にはならない |

CIは短期`GITHUB_TOKEN`のread権限のみ。Project tokenや新しいsecretは追加しない。
イベントを受けないProject UI変更、無効化したworkflow、GitHubが再イベントを抑止する操作は
継続監視されない。ローカル全件auditで検出する。workflowはbaseへmerge・必要なmain昇格後に
有効になり、このPR自身の未merge版で実効性を主張しない。

将来CIを必須にする判断材料: label/assignee/milestoneの欠落をmerge条件にできる一方、
Projectsや意味の検証は保証しない。現在の`Verify repository metadata (Projects excluded)`は
`pull_request_target`のbaseで動くため、PR headの検証証拠ではない。**そのままrequired checkに
登録しない。**必須化には、trusted codeを維持して対象PR headへcheckを供給する設計、
forkやlabel更新の再実行、管理者例外、両baseでのcheck供給を別途確認する。
本変更ではbranch protection/ruleset/enforce_adminsを変更しない。

APIの契約は[GitHub Projects API](https://docs.github.com/en/issues/planning-and-tracking-with-projects/automating-your-project/using-the-api-to-manage-projects)と
[GraphQL pagination](https://docs.github.com/en/graphql/guides/using-pagination-in-the-graphql-api)を参照した。

## 証拠を残す

before/afterの取得時刻、件数、重複処理、修正した番号・field・旧値・新値・根拠、判断待ちと理由、
実行checkと未実行範囲を既存Issue/PRへ保存する。ローカルだけのjournalを引き継ぎの前提にしない。
公開するのは必要なmetadataの差分と非秘密の結果。token・認証header・個人pathは載せない。
新たな台帳やテストIssueを量産せず、失敗・再開は`scripts/test_github_metadata.py`のfixtureで検証する。
