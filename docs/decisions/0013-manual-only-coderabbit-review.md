# ADR-0013: CodeRabbitの自動reviewを廃止し、手動依頼だけにする

> 状態: Accepted
> 日付: 2026-08-22

## 背景

[ADR-0007](0007-review-scope-and-self-review.md)の決定1は、`.coderabbit.yaml`のlabel
allowlistで自動reviewを高リスク変更（firmware、protocol、Raspberry Pi、hardware）へ
限定した。rate limitで必要なreviewが返らない問題への対処である。

[Issue #154](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/154)の受け入れ条件は、
そこからさらに進んで次を求めている。

> CodeRabbitは自動起動せず、意味上criticalな変更でのみ自己レビュー後に最大1回使う

**現行設定はこれを満たさない。**allowlistのlabelを持つPull Requestは自動起動する設計である。

あわせて、allowlistが実際には機能していないことが分かっている。`CONTRIBUTING.md`の
「GitHubが強制しないもの」に記録した観測のうち、**allowlistのlabelを持つPull Requestが
`Review skipped`になった事象**（[#123](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/123)・
[#125](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/125)）がそれである。
**設定は「走る」と宣言し、実際には走らない状態が続いている。**
**その原因は未特定である**（`CONTRIBUTING.md`の「5行目の観測」が「未特定のまま残る」と
記録している）。**本ADRは原因を特定していない。**特定できないまま「走る」と宣言する設定を
残さない、という判断である。

## 判断要因

- 受け入れ条件を満たす
- **効かない設定を残さない。**設定が宣言する挙動と実際の挙動を一致させる
- rate limitの消費を、意味のある変更へ意図して割り当てる
- 自己レビューを弱めない。ADR-0007の決定2（checklistと2 round収束）は変えない
- 規則の理由が消えたら、規則も消す

## 検討した選択肢

### 選択肢A: 自動reviewを廃止し、手動依頼だけにする

`.coderabbit.yaml`から`labels`と`base_branches`を外し、`enabled: false`だけを残す。
reviewは意味上criticalな変更に対して、自己レビューの後で手動で依頼する。

利点: 受け入れ条件を満たす。設定が宣言する挙動と実際の挙動が一致する。
rate limitの枠を、投げると決めた変更へ割り当てられる。

コスト: 高リスク変更で依頼を忘れるとreviewが1件も無い状態でmergeできる。
**自動の網が無くなる。**

### 選択肢B: 記述だけを実態へ合わせ、設定は変えない

allowlistは残したまま、「実際には自動起動しない」と文書へ記録する。

利点: 設定を触らない。原因が解消したときに自動reviewが戻る。

コスト: **採らない。**受け入れ条件を満たさない。そして**設定が宣言する挙動と実際の挙動が
食い違ったまま**になる。次に読む人は「labelを付ければ走る」と読み、走らない理由を
文書の別の場所から探すことになる。

### 選択肢C: 現状維持

コスト: **採らない。**受け入れ条件を満たさず、上の食い違いも残る。

## 決定

**選択肢Aを採る。**

1. `.coderabbit.yaml`は`reviews.auto_review.enabled: false`だけを持つ。
   **`labels`と`base_branches`を外す。**発火させない設定で発火条件を残さない
2. reviewは**意味上criticalな変更**に対して、**自己レビューの後**で、**最大1回**手動で依頼する。
   判断は人が行う。**機械的な判定は置かない**
3. 依頼の手順は`CONTRIBUTING.md`の「手動で依頼する前に状態を確認する」のままとする。
   rate limitの確認、`review`と`full review`の使い分け、同じ状態で2回投げない規則は変えない
4. **`CONTRIBUTING.md`の「labelは作成時に付ける」規則を廃止する。**labelは引き続き必須だが、
   **「作成時でなければならない」理由が消えた**（下記）
5. ADR-0007の決定2（自己レビューのchecklistと回数）は変えない。**自己レビューが唯一のreviewで
   ある状態が、例外ではなく既定になる**

### labelの時期を縛る理由が消えた

`CONTRIBUTING.md`の「labelは作成時に付ける」は、**CodeRabbitが対象判定をPull Requestの
作成直後に行い、後からのlabel追加では再判定しない**ことだけを理由にしていた
（[#94](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/94#issuecomment-5242077436)での
CodeRabbit自身の回答。実測ではない）。

**対象判定そのものが無くなるため、この理由は成立しない。**labelは仕分けと
`review_gate.py`の分類のために必須のままだが、時期の縛りは外す。

同節が記録していた実測は、履歴としてここへ移す。いずれも`.coderabbit.yaml`が`develop`に
ある状態で作成したPull Requestである。

| 作成時のlabel | Pull Request | 結果 |
|---|---|---|
| allowlistに一致する（当時は`type:decision`） | [#90](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/90)・[#91](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/91) | 対象判定を通過した。#91 は作成の4分47秒後に自動reviewが提出され完走した |
| labelはあるがallowlistに一致しない | [#95](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/95)・[#96](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/96) | `Review skipped: excluded by label configuration` |
| allowlistに一致するが走らない | [#123](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/123)・[#125](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/125) | `Review skipped`。**設定は「走る」と宣言していた** |

廃止した設定が持っていた制約も、ここへ移す。**pathでauto reviewの発火は制御できない。**
`reviews.path_filters`はreviewの中で見るfileを絞るだけであり、reviewを走らせるかどうかは
決めない。発火を制御できるのは`base_branches`、`labels`、`ignore_title_keywords`、
`ignore_usernames`である。**選択的な自動reviewを将来復活させる場合、pathでは実現できない。**

**「後から付けたので発火しなかった」ことを実測した記録は無い。**
[#89](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/89)は判定の5秒後に
`.coderabbit.yaml`が`develop`へmergeされており、labelの未付与と設定の未反映が
切り分けられない。**この決定はその切り分けを不要にする。**

## 影響

### 利点

- 設定が宣言する挙動と実際の挙動が一致する
- rate limitの枠を、投げると決めた変更へ割り当てられる
- Pull Request作成時の制約が1つ減る（labelの時期）
- 「reviewが走ったか」を毎回checkの説明文で確かめる運用は変わらない

### 欠点

- **高リスク変更で依頼を忘れると、reviewが1件も無い状態でmergeできる。**自動の網が無い
- **skipの原因が解消しても自動reviewは戻らない。**戻すには本ADRを置き換える判断が要る
- 依頼するかどうかが人の判断になる。**「意味上critical」の境界は機械的に定めない**

### リスクと対策

| リスク | 対策 |
|---|---|
| 高リスク変更でreview依頼を忘れる | `AGENTS.md`と`CONTRIBUTING.md`が、安全・電気・protocol・firmwareに関わる変更を自己レビューで代替しないと定めている。**この規則は変えない。**依頼の判断はPull Request本文の「Review thread」節で明示する |
| 自己レビューが唯一のreviewである状態が既定になる | ADR-0007の決定2（checklist、2 round収束）とADR-0010・0012の宣言trailerがそのまま働く。**弱めていない** |
| 「意味上critical」の解釈が広がりも狭まりもする | 機械的に定めない。迷ったら依頼する側へ倒す。**枠の消費は`@coderabbitai rate limit`で事前に確認できる** |
| 設定を戻したくなる | 本ADRを置き換える。**`labels`を足すだけの変更も軽微経路へは入らない。**`.coderabbit.yaml`はMarkdownではないため、`review_gate.py`が内容を見る前に`review-required`へ落とす（`not Markdown`。2026-08-22に実測した）。Pull Requestとreviewを通る |

## 検証

- `.coderabbit.yaml`が`develop`へ入った後のPull Requestで、CodeRabbitのcheckが
  `Review skipped`のままであること。**本ADRを入れるPull Request自身が最初の観測になる。**
  ただし変更前も`area:docs`のPull Requestは同じ表示だったため、**表示だけでは
  「設定が効いた」ことの証拠にならない。**証拠になるのは、allowlistに一致するlabelを持つ
  Pull Request（`area:firmware`等）でも同じ表示になることである
- 手動依頼の手順が変わっていないこと。`CONTRIBUTING.md`の該当節を変更しない
- 見直し条件: 高リスク変更でreviewが1件も無いままmergeされた事例が出た場合は、
  **自動reviewを戻すのではなく、依頼を必須にする位置**（Pull Request templateの
  checkboxやGateの検査）を先に検討する

## 置き換える決定

**[ADR-0007](0007-review-scope-and-self-review.md)の決定1を置き換える。**
自動reviewの発火をlabelで制御するという方針だけを廃止する。

**本ADRが変えるのは決定1だけである。**決定2・決定4・決定5は同ADRのままである。
決定3は本ADRとは別に、[ADR-0010](0010-change-class-and-review-declaration.md)と
[ADR-0011](0011-issue-optional-pull-request-required.md)が狭めている。
