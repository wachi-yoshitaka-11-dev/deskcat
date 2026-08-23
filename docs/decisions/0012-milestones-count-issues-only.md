# ADR-0012: milestoneはIssueだけに設定する

> 状態: Accepted
> 日付: 2026-08-22

## 背景

`CONTRIBUTING.md`は、Pull Request作成時に「対応するIssueと同じassignee・label・milestone」を
設定するよう定めていた。GitHubのmilestoneはIssueとPull Requestの両方を数えるため、
**同じ仕事が2回数えられる。**

2026-08-22時点の実測である。

| milestone | Issue | Pull Request |
|---|---|---|
| M0 Development Foundation | 46 | 84 |
| M1 ESP32 Bring-up | 7 | 6 |
| M2 ESP32–Pi Protocol | 6 | 5 |
| M3以降 | 13 | 0 |

**M0の進捗率は、上のIssue件数とPull Request件数の合計を分母にしている。**
1つのIssueを何本のPull Requestに割ったかで分母が動くため、進捗率が仕事の量を表していない。
実際に[Issue #154](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/154)は
1件のIssueから複数のPull Requestを出しており、その本数だけM0が増えた。

Projects v2 boardでも同じ二重計上が起きる。ただしboardのPull Request itemには役割がある。
`Pull request merged` workflowがPull Request itemの`Status`を`Done`にし、それを見て
Issue itemを`Done`にすると`Auto-close issue`がIssueをcloseする。**この連鎖はPull Request
itemを必要とする。**

## 判断要因

- 進捗率の分母を、仕事の量で決まる値にする
- boardのworkflow連鎖を壊さない
- 手作業を増やさない。むしろ作成時の設定を1つ減らす
- 履歴を失う変更は、失う内容が別の場所から辿れることを確認してから行う

## 検討した選択肢

### 選択肢A: Pull Requestをboardに入れず、milestoneも付けない

利点: 二重計上が両方から消える。

コスト: **採らない。**`Pull request merged` workflowが働かず、merge済みかどうかをboardで
追えなくなる。`CONTRIBUTING.md`が「boardが進行管理の正本」と定めている前提が崩れる。

### 選択肢B: Pull Requestはboardに入れるが、milestoneは付けない

利点: milestoneの分母がIssueだけになり、進捗率が節目までに出すものの数を表す。
boardのworkflow連鎖はそのまま働く。作成時の設定が1つ減る。

コスト: boardのitem一覧にはPull Requestが並び続ける。boardは`Status`で絞るviewを持つため、
一覧の見え方は運用で調整できる。

### 選択肢C: 現状維持

利点: 何も変えない。履歴も動かさない。

コスト: **採らない。**進捗率を見ない運用になる。見ない数字を表示し続けるのは、
誤読の余地を残すだけである。

## 決定

**選択肢Bを採る。**

1. **milestoneはIssueだけに設定する。Pull Requestには設定しない**
2. Pull Requestは引き続きboardへitemとして追加し、`Status`・`Start date`・`Target date`を
   設定する。**boardのworkflow連鎖は変えない**
3. `gh pr create`の例から`--milestone`を外す
4. **既存のPull Requestからmilestoneを外す。**外さなければ分母は元に戻らない
5. 昇格Pull Request（base `main`）も同じ扱いとする。**milestoneを付けない**

## 影響

### 利点

- milestoneの進捗率が、節目までに出すIssueの数と完了数で決まる
- Pull Requestを何本に割っても分母が動かない
- Pull Request作成時の設定が1つ減る

### 欠点

- **既存のPull Requestからmilestoneを外すと、「そのPull Requestがどの節目に属したか」の
  記録がPull Request自身から消える。**対応Issueの`Refs #N`から辿る形になる
- boardのitem一覧にはPull Requestが並び続ける。二重計上そのものはboardからは消えない

### リスクと対策

| リスク | 対策 |
|---|---|
| 過去のPull Requestとmilestoneの対応が分からなくなる | 各Pull Requestは本文で`Refs #N`または`Closes #N`により対応Issueを指しており、Issue側にmilestoneが残る。**辿る経路は失われない** |
| 運用が戻り、Pull Requestにmilestoneが付き始める | `CONTRIBUTING.md`の作成手順と`gh pr create`の例から`--milestone`を外し、Pull Request側に付けないことを明記した。**機械的な強制は無い** |
| boardの一覧がPull Requestで埋まる | boardの`Status`による絞り込みで運用する。本ADRはboardのitem構成を変えない |

## 検証

- milestoneのPull Request件数が0になること。**適用後に`gh api`で読み出して確認する**
- boardの`Pull request merged`→`Auto-close issue`の連鎖が引き続き働くこと。
  **本ADRはboardのitemを変更しないため、動作は変わらない。**次のmergeが実測になる
- 見直し条件: 「どのPull Requestがどの節目のものか」をmilestone以外から辿れない場面が
  実際に生じた場合は、Pull Request本文の`Refs`を必須にする方向を先に検討する。
  **milestoneを戻さない**

## 置き換える決定

なし。boardの役割と`Pull request merged` workflowの扱いは
[ADR-0004](0004-main-develop-branch-strategy.md)および`CONTRIBUTING.md`のままである。
本ADRが変えるのはmilestoneの付与先だけである。
