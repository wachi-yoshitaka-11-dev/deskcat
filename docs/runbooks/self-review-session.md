# 自己レビューの停止と再開

規則の正本は[CONTRIBUTINGの自己レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#自己レビュー)。
この手順は`review_gate.py session`の操作と引き継ぎを説明する。
既存の`receipt`は宣言の形式検査として残し、巡数を別の軸で扱う。

## 開始と実行

既存Issue/PRを開き、同じ作業の全巡数と承認を確認する。以下の`465`は例であり、
実際の対象Issue番号を使う。初回だけ、確認できた既実施巡数と確認元を渡す。
不明な巡数を0として初期化しない。既存の記録は`init`で上書きできない。

```bash
python3 scripts/review_gate.py session init --work 465 --prior-rounds 0 --history-source https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/465
python3 scripts/review_gate.py session status --work 465
```

記録は`git rev-parse --git-common-dir`配下の`deskcat-review/465.json`。
commit、rebase、差分更新で書き換えず、同じcloneのworktreeで共有する。
書き込みは排他lockを取り、開始巡の予約を保存してからreviewerへ進む。
途中停止でも予約を払い戻さない。lockが残った場合は書き手が停止したことを確認して復旧する。

CLI reviewerは次の経路で起動する。`--`以降はargvとして直接実行し、shellは解釈しない。
reviewerは下記の結果JSONだけを標準出力へ返す。成功時に自動保存し、失敗・不正JSONなら
巡を未完了のまま残して停止する。実際にレビューするcommandを指定し、no-opで代用しない。

```bash
python3 scripts/review_gate.py session run --work 465 -- reviewer-command argument
```

Claudeでは、開始前に`DESKCAT_REVIEW_WORK=465`を環境へ設定する。
登録済み`Agent|Task`の`fresh-context-reviewer` / `consistency-inspector`呼び出しはhookが
予約するため、親で`begin`を重ねない。親が結果を保存するまで次のreviewerは起動できない。
baseが既定の`origin/develop`と異なるなら`DESKCAT_REVIEW_BASE`も設定する。
hookの不足・異常終了は対象reviewerの起動拒否になる。

Codex等で会話内レビューを行う場合は各巡の前に次を実行し、**exit 2なら見直しを始めず
人へ返す**。この経路はCodexのtool自体へhookを登録していないので、手順による停止である。

```bash
python3 scripts/review_gate.py session begin --work 465
```

1回の起動を1巡とし、両Passを同じ巡で実施しても巡数は増やさない。
機械checkや実装をレビューと取り違えない。予約したdiffはbaseのmerge-baseからの差分に、
未commit変更とignoreされていない新規fileを含めたSHA-256。Pass中にdiffが変わった場合は
`interrupted`として記録し、新しいdiffで次の巡を予約する。修正は巡の結果保存後に行う。

## 結果と終了判定

結果JSONの最小例（承認や検証結果を表す実記録ではない）:

```json
{
  "passes": ["requirements-pass", "fresh-context-pass"],
  "findings": [
    {
      "kind": "optional",
      "origin": "prior-explanation",
      "decision": "decline",
      "reason": "同じ意味の注記を足すだけで、実行条件や判断は変わらない",
      "evidence": "対象fileの該当行"
    }
  ],
  "unresolved": [],
  "disposition": "continue"
}
```

`findings`は`defect` / `out-of-scope` / `optional`、出所は`diff` / `prior-explanation` /
`pre-existing`、採否は`fix` / `defer` / `decline`。重大な欠陥は`defect`として影響を理由へ書く。
`defect`を修正予定として採用しただけで解決済みにせず、残るものを`unresolved`に列挙する。
`passes`は実施したものだけを書く。中断時は`disposition: interrupted`でPassの証拠を残さない。
`capped`の場合は`cap_reason`へ人間の判断元、または直近2巡が任意改善だけだった根拠を足す。

```bash
python3 scripts/review_gate.py session finish --work 465 --record result.local.json
python3 scripts/review_gate.py session check --work 465
python3 scripts/review_gate.py gate --base origin/develop --head HEAD --review-work 465
```

`check`は現在のdiffに対して両Passと未解決欠陥を確認する。新規欠陥0件が2巡続けば
`converged`、それより前の明示的な打ち切りは条件が揃った場合だけ`capped`、それ以外は
`stopped`（exit 2）。5巡目の終わりに条件が揃っていれば完了判定はできるが、
揃わない場合は未完了である。`begin` / `run` / hookは承認なしの6巡目を拒否する。
`--review-work`付きのgateは、実行記録の終端状態と既存trailerが一致しなければ失敗する。
CIの通常のgateはローカル記録を持たず、形式検査のままである。

## 人間の承認と引き継ぎ

5巡後の続行は、判断の出所を既存Issue/PRへ残し、次の形で取り込む。
これは**書式の例であり承認ではない**。`actor`と`source`は実在する判断者と判断元にする。

```json
{
  "work": "465",
  "actor_kind": "human",
  "actor": "実際に判断した人間",
  "source": "人間の明示判断を保存したIssue/PRコメントのURL",
  "after_round": 5,
  "through_round": 7,
  "scope": "残存欠陥D1の修正確認"
}
```

```bash
python3 scripts/review_gate.py session approve --work 465 --record approval.local.json
python3 scripts/review_gate.py session begin --work 465 --scope 残存欠陥D1の修正確認
```

終了巡数は人間が決める。例の7を既定値として使わない。scopeの完全一致と終了巡数で
限定し、Claude hookでは`DESKCAT_REVIEW_SCOPE`に同じ範囲を設定する。
AI/PM、別作業、承認時巡数の不一致、終了巡数の欠落は拒否する。

各区切りで`session status`のJSON全体と未実行check・残件を既存Issue/PRに保存する。
新しいcloneでは保存済みJSONを`session init --work 465 --record handoff.local.json`で復元する。
元cloneの担当を止めてから引き継ぎ、2つのcloneで同じ作業の巡を並行消費しない。
ローカルに記録が無いだけでは新規作業と判断しない。

## 強制できる範囲

テストは子processの起動有無、登録hookのexit、状態の継続、diff変更によるPass無効化を確認する。
これで保証するのは**この記録を使ってこの起動経路を通った操作**である。
別種agent、直接CLI、MCP、会話内の読み直し、無効化したhookは自動捕捉しない。
状態fileを改ざんできる主体からの防御や、人間と同じアカウントを使うAIの本人性判定はしない。
`actor_kind: human`をAIが偽って入力すれば機械だけでは見破れない。
人間の承認元と申告の真偽の確認は残る。単なる文字列を認証済み承認・review実施の証拠とは呼ばない。
