# ADR-0024: 用語の索引を、既存の Single Source of Truth 表へ追加する形で置く

> 状態: Accepted
> 日付: 2026-09-21
>
> 起案: AIが起案した（2026-09-21）。**[#325](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/325)の候補4として、人間が対象を指定している。**
> **選択肢と決定の文面はAIが起案した。人間の承認はPull Requestのmerge承認として得る**
> （[ADR-0017](0017-what-counts-as-a-primary-source.md)と同じ扱いである）。
>
> **この決定を起票した時点（2026-09-21）では、番号がADR-0022の次にADR-0024へ飛んでいた。**
> ADR-0023（[PR #435](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/435)、
> #325の候補3・サーボ安全制限と通信断処理の状態機械へのモデル検査の採否）が
> 未mergeのまま`docs/decisions/0023-no-model-checking-for-now.md`という番号を
> 使っていたためである。**この決定の branch を`origin/develop`へrebaseした時点で
> PR #435は既にmergeされており、ADR-0023は`docs/decisions/README.md`に実在する。**
> 欠番は解消し、ADR-0022・0023・0024が連番で並ぶ。**この決定（候補4・用語索引）が
> 選択肢Cを採る判断そのもの（判断要因1〜4、選択肢の比較、決定1〜5）は、ADR-0023／
> PR #435の中身を根拠にしていない。**背景節の実測表は、rebase後の`origin/develop`が
> ADR-0023を含むことで出現file数が一部動いた事実を記録しているが、**これは
> 用語がなお閾値を満たすことの追加確認であり、判断の根拠を差し替えるものではない。**

## 背景

[#325](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/325)の候補4は、
用語の定義が ADR、CONTRIBUTING、`docs/governance/*`、`docs/protocol/esp32-pi-protocol.md`
に分散しており、用語集は無い、という観測から始まる。**用語集を新設すると定義が2箇所になる**
と自ら釘を刺しており、選択肢を (a) 何もしない／(b) 定義の場所だけを指す索引を置く／
(c) 用語集を正本として作る の3つに絞っている。

**この ADR は誌面（Interface 2026年9月号）を根拠にしない。**#325 本文が「候補として挙げるだけ
であり、採否は DeskCat 側の根拠に基づいて決める」と定めている
（[ADR-0017](0017-what-counts-as-a-primary-source.md)の丙）。

### 実測: 用語ごとの正本は既に1箇所に定まっている

2026-09-21に、候補として挙がりうる用語の一部を選び、`git grep -clE '<pattern>' origin/develop -- '*.md' | wc -l`
でこの決定を入れる前の`develop`を対象に出現fileを数えた（`origin/develop`は`baae66a`。
**このADRの起票時点の値である。**後述のとおり、この決定のbranchをrebaseした時点の
`origin/develop`（`4649e6a`。#325候補3のADR-0023を含む）で数え直した値も併記する）。

| 用語 | 検索pattern | 出現file数（`baae66a`時点） | 出現file数（`4649e6a`時点） |
|---|---|---|---|
| 安全要件5項目 | `安全要件の?5項目` | 14 | 15 |
| 一次資料 | `一次資料` | 25 | 26 |
| TBD台帳／`tbd-register.md` | `tbd-register\.md\|TBD 台帳\|TBD台帳` | 27 | 28 |
| 承認済みのリポジトリポリシー | `承認済みのリポジトリポリシー` | 3 | 3 |
| 後始末（`fixup`）の範囲 | `fixup.{0,6}範囲\|後始末.{0,10}範囲` | 5 | 5 |

**3つの用語（安全要件5項目・一次資料・TBD台帳）で件数が1ずつ増えている。**
`comm -13`で`baae66a`と`4649e6a`の出現file一覧を突き合わせたところ（2026-09-21実行）、
増分の出所は3つとも`docs/decisions/0023-no-model-checking-for-now.md`（ADR-0023）の
1 fileだけだった。同ADRが本文中でこの3語をそれぞれ使っているためである。
**この3語（安全要件5項目・一次資料・TBD台帳）自体は、依然としてADR番号に紐づかない用語であり、
判断要因3・決定4の対象から外れない。**動いたのは出現file数という計測値であって、
用語の性質ではない。**ADR-0023という個別の概念**（サーボ安全制限と通信断処理の状態機械への
モデル検査の採否）は、それ自体`docs/decisions/README.md`の一覧で既に索引されているため、
同ADRの本文がこの3語を使ったこと自体を新たに索引する必要はない。この決定が言っているのは
「ADR-0023の本文が既存の3語を使った」という事実であって、「この3語がADR-0023に
紐づく新しい概念になった」ではない。両時点とも閾値（3箇所以上）を満たし、結論は変わらない。

**表中の`\|`は正規表現の選言（alternation）であり、markdownの列区切りではない。**
実行したcommandは`git grep -clE '<pattern>' origin/develop -- '*.md' | wc -l`である
（2026-09-21実行）。

**いずれも閾値（3箇所以上）を満たす。**この出現file数には、定義そのものを持つfile自身を含む
（例: `承認済みのリポジトリポリシー`の3件には`ai-agent-policy.md`自身が入るため、
**それを他fileが参照している件数は2件である。**「3箇所以上」は「定義fileを含む延べ出現数」で
判定しており、「定義fileを除いた参照元の数」ではない）。「独立した定義を持つfileが1つだけである」ことは、
出現fileの全て（TBD台帳は`baae66a`時点で27、`4649e6a`時点で28）を開いて確認してはいない。
**確認したのは、各用語について定義そのものを
持つと見出しから分かるfile（[Hardware Safety Policy](../governance/hardware-safety-policy.md)の
`### 安全要件の5項目`と`#### 一次資料に当たるもの`、[AI Agent Policy](../governance/ai-agent-policy.md)の
`### 「承認済みのリポジトリポリシー」の範囲`、[CONTRIBUTING](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#後始末fixupの範囲)の
`### 後始末（fixup）の範囲`、および`docs/hardware/tbd-register.md`自体）と、
残りのfileのうち数件を無作為に開き、**定義を書き直さずその用語をそのまま使っている
ことを確認した。**残りの全fileを1件ずつ照合してはいない。**「1つだけ」は、この確認の範囲での
結論であり、未確認の少数fileに二重定義が紛れている可能性を排除しない。**
`一次資料`は典型例である。[ADR-0017](0017-what-counts-as-a-primary-source.md)自身が
「用語が定義なしに繰り返し使われていた」ことを起点に、上の1節へ定義を集約している。
**候補4が問題にしている状態を、この repository は既に1件自分で解消している。**

用語集らしき file（`glossary`／`term`／`用語`／`index`を名前に含む file）は`docs/`に無い
（2026-09-21、`find docs -iname '*glossary*' -o -iname '*term*' -o -iname '*用語*' -o -iname '*index*'`
で確認、0件）。

一方で、`docs/governance/README.md`の**`## Single Source of Truth`節**は、
「GPIO割り当て」「電源構成と電流予算」のような**情報のまとまり単位**で
「情報 → 正本」の表を既に持っている。**これは候補4が言う「定義の場所だけを指す索引」と
同じ形である。**候補4が指しているのは、この表がまだ個別の**用語**（情報のまとまりより
細かい単位）までは拾っていない、という隙間である。

ADR単位の概念（`docs/decisions/README.md`の一覧表）は、**既に番号とリンクで索引化されている。**
候補4が新たに埋めるべき隙間ではない。**ただし上の5用語のうち`安全要件5項目`と`一次資料`は、
その概念を導入したADR（[ADR-0014](0014-safety-requirements-and-general-values.md)、
[ADR-0016](0016-evidence-bar-inside-the-safety-requirements.md)、
[ADR-0017](0017-what-counts-as-a-primary-source.md)）自体を経由しても見つかる。**
それでも下の表へ足す理由は、`docs/decisions/README.md`の一覧が「ADRが何を決めたか」の
索引であり、「この用語の現在の運用定義がどこにあるか」を1手で示す索引ではないためである
（現に上のADR群自身が「定義そのものはHardware Safety Policyへ持たせ、ここへ再掲しない」
と書いている）。**判断要因3が言う「ADR番号に紐づかない」とは、この2つの索引が
指す先が違う、という意味である。**

**したがって、この ADR が扱う「隙間」は、定義が複数箇所にある状態ではなく、
単数の定義がどこにあるか探しにくい状態である。**

## 判断要因

1. **実害が既に起きているか。**`git grep -clE '用語.{0,15}(食い違|混乱|誤って|齟齬)' origin/develop -- '*.md'`
   は0件だった（2026-09-21実行）。**この検索はrepository内のtracked Markdownに限られ、
   closeしたIssueやPRのcomment、外部でのやり取りは対象にしていない。**その範囲では、
   [ADR-0018](0018-instruction-file-structure.md)が記録した`README.md`とAGENTS.mdの
   command食い違いのような、**実際に古くなった実例はこの候補には見つからなかった。**
2. **既存の仕組みと重複しないか。**`docs/governance/README.md`のSingle Source of Truth表は
   「情報 → 正本」を1行1件で持ち、「同じ値を複数の文書で再定義しない。他の文書からは
   正本の定義へリンクする」という運用規則も既に明記している。**新しい索引機構を別に作ると、
   索引そのものが2つになる。**
3. **保守costがどこへ乗るか。**この判断のために、起票時点（`baae66a`、ADR-0022まで）の
   `docs/decisions/`配下のADR22件と
   `docs/governance/*`・`CONTRIBUTING.md`を目視で洗い出したところ、
   **概算で20〜30程度**の用語・概念があった（**この数は精密な集計ではなく、
   目視での見積もりである。**厳密な一致基準を機械で数えたものではない）。
   その大半はADR番号に紐づき、`docs/decisions/README.md`が既に索引済みである。**残るのは、
   ADR番号を持たない少数の用語**（`安全要件5項目`、`一次資料`、`TBD台帳`、
   `後始末（fixup）の範囲`、`承認済みのリポジトリポリシーの範囲`）だけであり、
   これらを既存表へ数行足す程度なら、新しい file や運用を増やさずに済む。
   **新規に用語集 file を作ると、file の存在自体を維持する責務が増える**
   （どの用語を載せるかの基準、載せ忘れの検知、正本が動いたときの追随）。
4. **安全要件5項目に効くか。**効かない。これは文書の見つけやすさの改善であり、
   安全境界にも判定基準にも触れない。

## 検討した選択肢

### 選択肢A: 何もしない

**採らない。**判断要因3のとおり、既存のSingle Source of Truth表へ数行足すだけで隙間を
埋められる状況で、**それすら行わない理由が無い。**実害が無いことは「緊急に着手する理由が
無い」ことは示すが、「足すコストがゼロに近い改善を見送る」根拠にはならない。

### 選択肢B: 用語集を正本として新設する（候補4の(c)）

新しい `docs/glossary.md` のような file を作り、そこへ用語の定義そのものを書く。

**採らない。**候補4本文が指摘するとおり、**定義が2箇所になる。**背景節の実測のとおり
既に単一の正本がある用語について、二つ目の定義を作れば、
[Single Source of Truth](../governance/README.md#single-source-of-truth)の
「同じ値を複数の文書で再定義しない」に反する。**この ADR が実測した「隙間は索引の欠如で
あって定義の欠如ではない」という事実（背景節）と最も相性が悪い選択肢である。**

### 選択肢C: 定義の場所だけを指す索引を置く（候補4の(b)）。既存表を使う

**採る。**新しい索引 file を作らず、`docs/governance/README.md`の
Single Source of Truth表へ、ADR番号を持たない用語の行を数行足す。
**索引の仕組みを1つに保ったまま隙間を埋める。**

### 選択肢C': 定義の場所だけを指す索引を置く。新しい file として

選択肢Cと同じ内容を、`docs/terminology-index.md`のような新規 file に書く案も検討した。
**採らない。**索引という機能そのものは同じだが、**索引機構が2つ（既存のSSoT表と新規file）
になり、どちらへ足すかの判断が今後増える。**判断要因2に反する。

## 決定

**選択肢Cを採る。**

1. **`docs/governance/README.md`の`## Single Source of Truth`表へ、次の用語の行を足す。**
   定義は複製せず、既存の正本 file・節を指すリンクだけを置く。

   | 情報 | 正本 |
   |---|---|
   | 安全要件5項目とその根拠の水準 | [Hardware Safety Policy](../governance/hardware-safety-policy.md#安全要件の5項目) |
   | 一次資料に当たるもの | [Hardware Safety Policy](../governance/hardware-safety-policy.md#一次資料に当たるもの) |
   | 「承認済みのリポジトリポリシー」の範囲 | [AI Agent Policy](../governance/ai-agent-policy.md)の「「承認済みのリポジトリポリシー」の範囲」節 |
   | ハードウェアTBDの状態と解決手順 | [Hardware TBD Register](../hardware/tbd-register.md) |
   | Issueを立てずに直接反映してよい範囲（後始末／`fixup`を含む） | [CONTRIBUTING](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#後始末fixupの範囲)の「後始末（`fixup`）の範囲」節 |

   **「承認済みのリポジトリポリシー」の行にanchorを付けていない。**該当見出し
   （`docs/governance/ai-agent-policy.md`の`### 「承認済みのリポジトリポリシー」の範囲`）は
   `「」`を含む。**2026-09-21に、この行を一時的にanchor付き
   （`ai-agent-policy.md#「承認済みのリポジトリポリシー」の範囲`）へ書き換えて
   `validate_doc_links.py`を実行し、確認後に元へ戻した。**結果は次のとおりである。

   ```text
   Unverified anchor in docs/governance/README.md: ai-agent-policy.md#「承認済みのリポジトリポリシー」の範囲
   (the heading contains 「」; kramdown's handling is not confirmed. Link to the document
   without a fragment, or confirm with a Jekyll build first)
   ```

   **`scripts/validate_doc_links.py`は、anchorの算出自体はできているが、`「」`を含む見出しに
   ついてkramdown（Jekyll側のmarkdown処理系）が生成site上で同じanchorを作るかを確認できない
   ため、anchor付きlinkを拒否する。**`AGENTS.md`（53行目、2026-09-21時点）も同じ見出しを、
   `AI Agent Policy`へのanchor無しlinkと「「承認済みのリポジトリポリシー」の範囲」という
   地の文の組み合わせで参照しており、この決定は同じ書き方に揃えている。
   **CONTRIBUTINGの行を絶対URLにしている。**`CONTRIBUTING.md`はGitHub Pagesでrenderされず、
   同validatorが相対linkを拒否するためである（[自己レビュー](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#自己レビュー)の
   「公開されない路への相対linkを張っていない」）。**上の表のlink先・link文言は
   `docs/governance/README.md`の実際の記述と対応させてある。**ただし相対pathの表記は
   `../governance/`の有無がfileごとに異なる（この ADR は`docs/decisions/`、
   `docs/governance/README.md`は`docs/governance/`が起点であるため）。**解決先は同じであり、
   文字列としての一致ではない。**

2. **ADR単位の概念には触れない。**`docs/decisions/README.md`の一覧表が既に索引であり、
   この決定はそこへ重複する行を足さない。
3. **新しい file を作らない。**`docs/glossary.md`のような用語集は置かない。
4. **行を足す基準を明記する。**「3箇所以上の file で説明なしに使われ、かつADR番号に
   紐づかない用語」だけを対象とする。**「説明なしに」は、背景節の実測で行った
   spot check（定義を持つ file 以外を数件開き、定義を書き直さずそのまま使っていることを
   確認する）と同じやり方で判定する。**「3箇所以上」は`git grep`の出現file数で機械的に
   数えられるが、「説明なしに」は数だけでは判定できず、目視の確認を要る。
   **基準を緩めて用語を機械的に列挙しない。**
   表が肥大化すると、Single Source of Truthとしての一覧性そのものが失われる
   （[ADR-0018](0018-instruction-file-structure.md)が`AGENTS.md`の行数について
   指摘したのと同じ性質の懸念である）。
5. **テスト方針とドメイン・モデル文書は対象外とする。**#325 候補4が明示的に対象外と
   定めており、この決定も触れない。

## 影響

### 利点

- 索引の仕組みを1つ（Single Source of Truth表）に保ったまま、見つけにくかった5用語が
  探せるようになる
- 新しい file・新しい運用規則を増やさない。**維持責務が増えない**
- 定義そのものを複製しないため、正本が変わったときに直す箇所は1箇所のまま

### 欠点

- **今回足す5用語は、判断要因3で目視概算した20〜30用語のうち一部である。**残りはADR索引で足りると
  判断したが、この判断が誤っていた場合、別の用語でも同じ「見つけにくさ」が起きうる
- 行を足す基準（決定4の2条件: 3箇所以上の file で説明なしに使われる／ADR番号に紐づかない）を
  機械で検査していない。**基準を外れた行が紛れ込んでも、機械では検出しない**

### リスクと対策

| リスク | 対策 |
|---|---|
| 表が用語で肥大化し、Single Source of Truthとしての一覧性を失う | 決定の4で基準を明記した。足すたびに基準へ照らす |
| 足した行の定義が、正本側の見出し変更でリンク切れになる | `scripts/validate_doc_links.py`がPull Requestごとにlinkを検証する（検証節） |
| 用語集（選択肢B）を求める提案が将来また出る | この ADR の判断要因1・2・3を、次の提案の検討材料として使う |

## 検証

- `scripts/validate_doc_links.py`が、足した5行のうち**相対link4件**（anchor付き2件、
  file単位2件）をすべて解決すること。**この決定のcommitをcheckoutしたworking treeで
  `python3 scripts/validate_doc_links.py`を実行し（2026-09-21）、
  `MARKDOWN=112 LINKS=1751 BROKEN=0 DIGEST=F5F6167A45EDE0C3`を得た（`origin/develop`へ
  rebase後の値。rebase前の値は`MARKDOWN=111 LINKS=1692 BROKEN=0 DIGEST=FBE07BCC763630A2`
  だった）。**`DIGEST`は`scripts/validate_doc_links.py`が検査したlink一覧から計算する値であり、
  この3 fileだけでなくrepository全体の追跡Markdownを対象にする（同scriptのcomment）。
  再現するには、この決定のcommitをcheckoutした状態で同scriptを実行すればよい。**
  CONTRIBUTINGの行は
  絶対URLであり、同scriptは外部schemeのlinkをlink切れ検査の対象にしない。**その1件は
  検証節の対象ではなく、`CONTRIBUTING.md`側に実在する見出し
  （2026-09-21時点の`CONTRIBUTING.md`109行目、`### 後始末（fixup）の範囲`）を目視で確認した。
  **「承認済みのリポジトリポリシー」の行にanchorを付けなかった判断も、実際にanchor付きへ
  書き換えて実行し、確かめている。**実行結果と経緯は決定1の直後に記載した
- `docs/governance/README.md`の表に、定義そのもの（節の本文相当の記述）を書いていないこと。
  **リンクと1行の見出しだけであること。**2026-09-21に、`docs/governance/README.md`の
  該当5行（安全要件5項目からfixupの範囲まで）を読み、各行がlinkと短い名称だけを持ち、
  定義の本文を含まないことを確認した
- 見直し条件: **同じ「用語が見つからない」という具体的な困りごとが、この5用語以外で
  Issue またはPull Requestのreviewとして観測されたとき。**そのとき、その用語がADR番号に
  紐づくかを先に確認し、紐づかなければ同じ形でこの表へ行を足す。**紐づくものは
  `docs/decisions/README.md`側の索引の精度を見直す（この表を肥大化させない）。**

## 置き換える決定

なし。
