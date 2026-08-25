# CodeRabbitのreview状態の観測記録

> 状態: Active
> 適用範囲: CodeRabbitのcheck表示とskip文言の読み方

**これは履歴である。規約ではない。**merge前に必要な判断は
[CONTRIBUTINGの「GitHubが強制しないもの」](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#githubが強制しないもの)が持つ。
**そこを読めば足りる。**この文書を読むのは、**見た文言が既知かどうかを調べるとき**だけでよい。

自動reviewは行わない（[ADR-0013](../decisions/0013-manual-only-coderabbit-review.md)）。
そのため`Review skipped`は既定の状態である。allowlistに言及する記述は、廃止前の設定に対する観測である。

**新しい文言を見たら、推測せずここへ追記する。**

**`Review skipped`の説明文で、skipの原因を切り分けられる。**これまでに観測した文言は次のとおりである。
**文言はCodeRabbit側のものであり、将来変わりうる。**一致しない文言を見たら、推測せず実際の表示を記録する。

**説明文の出どころは2つあり、同じ事象でも文言が違う。**commit statusの`description`と、
CodeRabbitがPull Requestへ投稿するcommentの本文である。たとえば前者は
`Review skipped: automatic reviews are disabled`、後者は`Auto reviews are disabled on this repository.`と表示される。
**記録するときはどちらで見たかを併記する。**commentの本文はCodeRabbitが後から書き換えるため、
review完走後には最初の文言が残らない。**後から検証できるのはcommit statusの履歴だけである。**
GitHubのcheck欄は文脈ごとの最新1件しか表示しないため、履歴は次で読む。

```bash
gh api --paginate "repos/<owner>/<repo>/commits/<sha>/statuses?per_page=100" \
  --jq '.[] | select(.context=="CodeRabbit") | [.created_at,.state,.description] | @tsv'
```

| 説明文 | 読み方 | 観測した Pull Request |
|---|---|---|
| `excluded by label configuration` | **設定は効いている。**labelは付いていたが、allowlistに一致しなかった。**対象外として正しくskipされた** | [#95](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/95)・[#96](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/96)（`area:docs`＋`type:maintenance`） |
| comment: `Auto reviews are disabled on this repository.`<br>status: `Review skipped: automatic reviews are disabled` | **設定が定着する前の観測である。**[#89](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/89)の判定時点では`.coderabbit.yaml`がまだ`develop`に無かった。**同じ文言を設定の定着後に見たら5行目である** | #89（判定の5秒後に設定がmergeされた） |
| `reviews are disabled for this base branch` | baseが対象外と判定された | [#88](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/88)（`.coderabbit.yaml`を`develop`へmergeする前） |
| `manual review required for this OSS repository` | **labelの判定では説明できない。**allowlistのlabelが作成時から付いており、1行目には当たらない。#127では`@coderabbitai rate limit`が`Reviews are available now`を返したためrate limitでもなかった。**`@coderabbitai full review`を投げると実際にreviewが走った。****2026-08-16に原因が判明した。**[#135](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/135)でCodeRabbitが投稿したcommentが`Reviews should be triggered manually for repositories with fewer than 10 stars.`と述べている。**star数による条件であり、設定の誤りではない。**この文言を見たら設定を疑わず、手動で`full review`を投げる | [#127](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/127)（`area:firmware`＋`area:protocol`）・[#123](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/123)・[#125](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/125)（後の2件は5行目と同時に観測） |
| **2行目と同じ文言を、設定の定着後に観測した**<br>status: `Review skipped: automatic reviews are disabled` | **設定は読まれている。**2行目の読み方（設定が未反映）を当てはめない。**原因は未特定。**対応は4行目と同じで、`@coderabbitai full review`でreviewが走った。**なお4行目の原因（star数）が2026-08-16に判明したが、この文言との関係は確かめていない。**同一Pull Requestに両方出るため（#123・#125・#135）同じ原因である可能性はあるが、CodeRabbitはこの文言について何も述べていない。**推測で4行目へ畳まない** | [#123](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/123)・[#125](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/125)（いずれも2026-08-15、作成直後）・[#124](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/124)（push後。後述） |

**行ごとに意味が違う。**同じ`Review skipped`でも取るべき対応が違う。

- 1行目: `.coderabbit.yaml`の意図どおりのskipである。[自己レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#自己レビュー)で通す。
  ただし**変更の内容に対してlabelの付け方が誤っていないかは確認する。**安全・電気・protocol・
  firmwareに関わる変更が`area:docs`だけになっていれば、labelが誤っており1行目に該当しない
- 2行目・3行目: **allowlistの判定まで届いていない。**設定が`develop`にあるか、baseが
  `base_branches`に含まれるかを確認する。対象範囲の変更なら
  [手動で依頼する](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#手動で依頼する前に状態を確認する)。安全に関わる変更では自己レビューで代替しない
- 4行目・5行目: **labelもrate limitも原因ではない。**
  [手動で依頼する](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#手動で依頼する前に状態を確認する)と得られる。`review`ではなく
  **`full review`**を使う。自己レビューで代替しない。
  ただし**「自動reviewは二度と起動しない」と決めつけない。**#125ではこの2文言の約2分後に
  自動で`Review in progress`へ移っている（その回はrate limitで止まった）

**2行目・3行目は、いずれも`.coderabbit.yaml`が`develop`に無かった時期の観測である。**4行目・5行目は設定が定着した後の観測であり、原因が別である。
設定が定着した後にこの文言を見たら、**それは新しい事象である。**推測で1行目と同じ扱いにしない。
5行目はその事象であり、確認できた事実を[5行目の観測](#5行目の観測)にまとめてある。

**thread 0件は、reviewが終わったことを意味しない。**
[#76](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/76)では0件を確認した**28秒後**に
reviewが届き、actionable comment 2件がmerge済みPull Requestへ付いた。
GitHubはthreadが存在しないものをblockできない。**reviewの到着を待たずに0件を「解決済み」と読まない。**

**自動reviewは行わない**（[ADR-0013](../decisions/0013-manual-only-coderabbit-review.md)）。
既定では[自己レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#自己レビュー)が唯一のreviewである。

**手動で依頼するのは、意味上criticalな変更に対してだけ、自己レビューの後で、最大1回である。**
判断は人が行う。

**依頼したreviewの指摘に対応したcommitは、自己レビューで見る。ここで投げ直さない。**
投げ直すと1つのPull Requestでreviewを何度も消費する。
[#91](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/91)で実際にそうなり、2回目はrate limitで終わった。

依頼したreviewが得られなかったときは、次の表に従う。**依頼しなかった場合も、
安全・電気・protocol・firmwareに関わる変更では同じ扱いとする。**

| 変更の種類 | reviewが得られなかったとき |
|---|---|
| 安全、電気、protocol、firmware | **rate limitが解けるまで待つ。**自己レビューで代替しない |
| 上記以外 | **自己レビューで通してよい。**Pull Request本文へ機械reviewを通していない旨と、その判断の根拠を書く |

**`Review stopped after lock loss`もこの表の対象である。**`state`が`failure`でcheckは赤くなるため`Review rate limited`／`Review skipped`とは表示で見分けられるが、**reviewが完走していない点は同じ**である。したがって初回reviewが得られなかった場合として扱い、上の表に従う。**安全・電気・protocol・firmwareに関わる変更では、`Review completed`へ到達するか手動の`full review`が完走するまでmergeしない。**赤いcheckを「reviewは走ったが失敗しただけ」と読み替えない。観測例は[GitHubが強制しないもの](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#githubが強制しないもの)にある。

##### 5行目の観測

上の「設定が定着した後に2行目の文言を見たら新しい事象である」に**実際に当たった**。
**1件限りではない。**同日の[#123](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/123)と
[#125](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/125)で、いずれも作成直後に同じ並びが出ている。
まず#123について、確認できた事実だけを次に記録する。

| 確認したこと | 根拠 |
|---|---|
| **設定は読まれている** | reviewの`Run configuration`が`Configuration used: Path: .coderabbit.yaml`（`Review profile: CHILL`、`Plan: Pro Plus`）を示す |
| 設定は`main`・`develop`の両方にあり、内容も同一 | 両branchでblob sha `816e60d` |
| baseは`develop`で、`base_branches`に含まれる | Pull Requestのbase |
| allowlistのlabelが**作成時**に付いている | `13:19:16Z`に`area:firmware`・`area:hardware`（作成は`13:19:15Z`。`gh pr create --label`で指定） |
| それでもskipされた | commit statusの履歴（下記） |

commit statusは`Review queued`が6件出たあと、**4行目の文言が5件と5行目の文言が1件**という内訳になった。

| 時刻（UTC） | `description` |
|---|---|
| `13:19:23Z`〜`13:19:33Z` | `Review skipped: manual review required for this OSS repository`（5件） |
| `13:19:35Z` | `Review skipped: automatic reviews are disabled`（1件） |

**この2文言は排他ではない。**同じPull Requestの同じ時刻帯に両方出る。
**どちらか一方だけを見て切り分けたと判断しない。**

解決は4行目と同じく`@coderabbitai full review`だが、**1回目は空振りした。**

| 時刻（UTC） | 出来事 |
|---|---|
| `13:21:20Z` | `@coderabbitai rate limit` → `More reviews will be available in 26 minutes.` |
| `13:50:36Z` | `@coderabbitai full review` → `13:50:41Z`のreplyが`Review rate limited.`（`next included review will be available in 59 minutes`）。commit statusも`13:50:45Z`に`Review rate limited` |
| `14:53:54Z` | `@coderabbitai rate limit` → `Reviews are available now.` |
| `14:54:26Z` | `@coderabbitai full review` → `14:54:43Z`のreplyが`Full review finished.`、`14:58:39Z`にcommit statusが`Review completed`。review本文は`Actionable comments posted: 3` |

**`full review`自体もrate limitで空振りする。**空振りしても同じCodeRabbit checkが
`Review rate limited`という**別の説明文**で`pass`になるため、投げっぱなしにすると走ったように見える。
[手動で依頼する前に状態を確認する](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#手動で依頼する前に状態を確認する)の手順を省かない。
`26 minutes`の案内どおりに待っても枠が空いていなかった点にも注意する。**案内の時刻は保証ではない。**

**#125（作成`13:41:11Z`、allowlistのlabel`area:hardware`を`13:41:13Z`に付与）でも同じ並びが再現した。**
`13:41:17Z`〜`13:41:19Z`に`Review queued`が5件、`13:41:19Z`〜`13:41:27Z`に4行目の文言が4件、
`13:41:30Z`に5行目の文言が1件である。**#123と同じ形であり、単発の事故ではない。**

**ただし#125では、その後に自動reviewが自力で起動している。**同じcommit（`be52d9b`）に対し、
手動依頼も新しいpushも無いまま`13:43:30Z`に`Review in progress`へ移り、`13:43:33Z`に
`Review rate limited`で止まった。**同じことが`13:50:15Z`と`15:08:54Z`にも起きている。**
最終的に`Review completed`へ至ったのは手動の`full review`（`17:26:49Z`）である。

**したがって「5行目＝自動reviewは二度と動かない」ではない。**skipの後に自動で再試行されることがあり、
そのときrate limitに当たると`Review rate limited`へ変わる。**表示が変わったことを「解決した」と読まない。**

**#124でも5行目の文言を観測しているが、条件が違う。**作成直後ではなく、
commit日時`2026-08-16T00:12:31Z`のcommitに対する`00:13:58Z`の判定である。
**#124に現存するcommitはすべてPull Request作成より後のものであり、作成時点のcommitに
付いたstatusはもう辿れない。**作成直後の観測として数えられるのは#123と#125の2件である。

##### `enabled: false`と`labels`の関係（公式文書の確認）

**5行目を「設定の誤りだ」と読まないため、CodeRabbitの公式文書を確認した。**
設定schema（[`schema.v2.json`](https://coderabbit.ai/integrations/schema.v2.json)。
`.coderabbit.yaml`冒頭の`$schema`が指すもの）の`reviews.auto_review.labels`の説明は次を含む。

> When `enabled` is false, a positive label match (for example ['review-ready']) triggers a review;
> negative-only labels such as ['!wip'] remain exclusion filters and do not opt PRs in by themselves.

**`enabled: false`＋`labels`によるopt-inは、文書どおりの使い方である。**
`.coderabbit.yaml`は誤設定ではなく、**5行目を設定変更で解消する根拠は無い。**

実測もこれと矛盾しない。[#90](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/90)・
[#91](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/91)は
`area:docs`＋`type:decision`＋`priority:*`で作成され、**当時のallowlistには`type:decision`が入っていた**
（`.coderabbit.yaml`から外したのは[#95](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/95)）。
一致するlabelがあり、`enabled: false`のまま自動reviewが走っている。
**「#90・#91はallowlistに一致していないのに通った」ではない。**現在のlabel一覧だけを見て
過去の観測を読み替えない。

**したがって5行目の原因は未特定のまま残る。**CodeRabbit側の挙動か、設定以外の要因である。
**推測を書き足さない。**次に同じ文言を見たら、上と同じ形で観測を追記する。
