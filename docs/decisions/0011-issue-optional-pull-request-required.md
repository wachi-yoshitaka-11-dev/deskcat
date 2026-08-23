# ADR-0011: Issueの要否とPull Requestの要否を分ける

> 状態: Accepted
> 日付: 2026-08-22

## 背景

[ADR-0010](0010-change-class-and-review-declaration.md)の決定3で、「規約の言い回し」を軽微経路から
外した。規則を持つfileの散文では、言い回しの修正と意味の変更を機械的に区別できないためである。

その結果、**`AGENTS.md`のtypoを1文字直すのにIssueの起票が要る状態**になった。
[ADR-0007](0007-review-scope-and-self-review.md)が解こうとしたのは「保守Issueが製品作業を
追い越す」ことであり、この状態はその問題を部分的に呼び戻している。

ADR-0007より前に同じ失敗modeが起きている。
[#84](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/84)は規約の言い回しを直すために
起票し、直後にcloseした。**これはADR-0010の結果ではなく、ADR-0007が「Issueを立てずに直接
反映してよい範囲」を定めた理由である。**その範囲がADR-0010で狭まったため、同じ状態へ戻った。

原因は判定の厳しさではなく、**`CONTRIBUTING.md`の表が2つの問いを1つの段にまとめていたこと**である。

- Issueを起票する必要があるか
- Pull Requestを経る必要があるか

軽微経路はこの両方を同時に免除していた。だから「Pull Requestは通したいが、Issueまでは要らない」
変更の置き場所が無かった。

## 判断要因

- 安全側を緩めない。差分がreviewを経ずに`develop`へ入る条件は広げない
- 起票の手間だけを減らす。reviewの手間は減らさない
- **機械的に判定できないものを、判定できるふりで扱わない**
- 表の段数を増やす代わりに、各段の境界がどこで守られるかを書く

## 検討した選択肢

### 選択肢A: Issueの要否とPull Requestの要否を別の列にする

軽微経路（Issue不要かつPull Request不要）は`review_gate.py`の`CLASS=minor`に限る。
そのうえで、**Issue不要だがPull Requestは必要**な段を新設し、意味を変えない記述修正を置く。

利点: 起票の手間が消える。差分はPull Requestを通るため、Review gateと自己レビューが必ず見る。
判定できない境界（言い回しか意味か）を誤っても、**黙って入ることがない。**

コスト: 表が3段から5段になる。境界の一つが人の判断のままである。

### 選択肢B: ADR-0010の決定3を戻し、「規約の言い回し」を軽微経路へ返す

利点: 表が増えない。起票もPull Requestも要らなくなる。

コスト: **採らない。**[Issue #154](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/154)の
受け入れ条件「規約の意味変更は軽微経路へ入らない」に反する。言い回しか意味かを機械的に区別できない
以上、軽微経路へ返せばreviewを経ない規約変更を許すことになる。

### 選択肢C: 現状維持

利点: 何もしない。

コスト: **採らない。**規約文書のtypoが放置されるか、typo 1件ごとにIssueが立つ。
ADR-0007が測った「保守Issueが製品作業を追い越す」状態へ戻る。

## 決定

**選択肢Aを採る。**

1. `CONTRIBUTING.md`の表を、Issueの要否とPull Requestの要否の2列に分ける
2. **Pull Requestを省けるのは`CLASS=minor`のときだけである。**判定は`review_gate.py`が行う
3. **意味を変えない記述修正は、Issue不要でPull Requestは必要とする。**
   「言い回しか意味か」は人の判断であり、**scriptは判定しない**
4. 迷ったらIssue必須の側にする。ADR-0007とADR-0010のこの規則は変えない
5. `review_gate.py`は変更しない。**この決定はIssueの要否だけを動かし、`CLASS`の定義には触れない**

## 影響

### 利点

- 規約文書のtypo修正が、Issueの起票を伴わなくなる
- 差分がreviewを経ずに`develop`へ入る条件は広がらない。`CLASS=minor`のままである
- 判定できない境界での誤りが、Pull Requestの差分として見える位置に留まる

### 欠点

- 表の段数が増える
- 3段目と4段目の境界が人の判断のままである。**機械化しない**
- 「Issue不要」を「review不要」と読み違える余地が残る。表と本文で明示するほかに手段が無い

### リスクと対策

| リスク | 対策 |
|---|---|
| 意味の変更が3段目として扱われ、Issueなしで入る | Pull Requestを通るため、Review gateと自己レビューが差分を見る。**黙って入る経路にはならない。**迷ったら4段目という規則も維持する |
| 「Issue不要」が「Pull Request不要」と読まれる | 表を2列に分け、どちらが不要かを段ごとに書く。Pull Requestを省ける段は`CLASS=minor`だけである |
| 段が増えて判断が重くなる | 判断は「scriptが`minor`と言ったか」と「意味を変えるか」の2問だけである。前者は実行できるcommandである |

## 検証

- **`CLASS=minor`でない変更がPull Requestなしで`develop`へ入ることを、機械的に止める仕組みは
  無い。**Review gateはPull Requestで起動するため、直接pushした変更は見ない。
  `develop`のbranch protectionには必須reviewも必須status checkも設定していない
  （ADR-0007の決定4。2026-08-22にAPIで読み出して確認した）。
  **Pull Requestを省ける段の規則は、人が守るものである。**GitHub設定の変更は
  [Issue #154](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/154)の対象外である
- `scripts/review_gate.py`とその回帰testに差分が無いこと。**この決定はcodeを変えない。**
  `CLASS`の定義を動かしていないことが、差分そのもので確認できる
- 見直し条件: 3段目を使った変更で、意味を変える修正がreviewを通ってしまった事例が出た場合は、
  3段目を廃止して選択肢Cへ戻す。**段を細分化して救わない**

## 置き換える決定

なし。[ADR-0007](0007-review-scope-and-self-review.md)と
[ADR-0010](0010-change-class-and-review-declaration.md)を置き換えない。
ADR-0010の決定3（「規約の言い回し」を軽微経路から外す）はそのままである。
本ADRが動かすのはIssueの要否だけである。
