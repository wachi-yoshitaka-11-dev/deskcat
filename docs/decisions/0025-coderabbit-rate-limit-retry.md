# ADR-0025: CodeRabbitの事前残数確認を外し、枠切れ後の再依頼を定める

> 状態: Accepted
> 日付: 2026-10-06

## 背景

[ADR-0013](0013-manual-only-coderabbit-review.md)の決定2・3は、依頼前の残数確認と
人による依頼判断を前提にしていた。今はPMが依頼を判断し、PR担当のセッションが投稿する。
残数確認を外すと、`Review rate limited`の後に再依頼できる条件が必要になる（[#533](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/533)）。

## 決定

依頼前の`@coderabbitai rate limit`を必須としない。PMが依頼を判断・調整し、
PR担当のセッションが投稿する。枠は全PRで共有するため、投稿前に全PRの最新依頼時刻を確認する。
`Review rate limited`でreviewが始まらなかった場合は、条件を満たしてから再依頼できる。
**投稿・再依頼・最大1回の扱いは[CONTRIBUTINGの手順](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#手動で依頼する前に状態を確認する)を正本とする。**

## 影響

事前の残数確認を待たずに依頼できる。枠切れ後の再依頼は、他のPRの投稿と競合しないよう
時刻を確認する必要がある。CodeRabbitの返答時間に上限がないため、結果が未確定の間は
再依頼の可否も確定しない。

## 検証

文書間の矛盾とリンクを検査する。実機確認は不要。

## 置き換える決定

[ADR-0013](0013-manual-only-coderabbit-review.md)の決定2の判断主体と「最大1回」の数え方、
決定3の依頼前の残数確認・再依頼条件を置き換える。手動依頼の対象、自己レビュー後に依頼すること、
自動reviewを行わないことは維持する。
