# ADR-0021: 宣言 trailer の検出を、hook ではなく共有 branch への push に置く

> 状態: Accepted
> 日付: 2026-09-10

## 背景

squash commit が`Change-Class`・`Self-Review`・`Instruction-Change`を失う事故が、これまでに**8 件**起きている
（`scripts/review_gate.py`の`DECLARATION_EXEMPT_ENTRIES`。2026-09-10 に数えた）。
**うち直近の`6bcd7b9`は、既存 7 件と原因が違う。**

`scripts/hooks/gh_metadata_guard.py`は Claude Code の**Bash tool の PreToolUse hook** であり、
`tool_input.command`だけを見る（同 file の docstring）。`gh pr merge`のsquash messageを検査し、
`git interpret-trailers --parse`が宣言を返さなければ`deny`する。

**`6bcd7b9`は GitHub MCP の`merge_pull_request` toolで merge された。**Bash を通らないため、
**hook は 1 度も呼ばれていない。**GitHub の web UI からの merge も同じである。

`b71c7ef`の記録は、hook が有効な状態で同じ型が起きたことについて
「別の経路を通ったと考えられるが、**経路は特定していない。推測で書かない**」と書いている。
**今回それが特定できた。**

**hook の判定粒度（`c171c52`の穴）は既に閉じている。**`_check_merge`は`review_gate.trailers_from_message`を
呼んでおり、文字列の部分一致ではない。**残っているのは経路だけである。**

## 判断要因

- **hook をどれだけ賢くしても、hook を通らない経路は塞げない。**Bash 以外の merge 経路は
  少なくとも 2 つある（GitHub MCP tool、web UI）。**列挙して塞ぐ形は、次の経路で破れる。**
- **経路に依らない位置は、commit が共有 branch へ入った後である。**そこには経路の区別が無い。
- **失われた宣言は後から付けられない。**共有 branch の履歴を書き換えないため
  （`AGENTS.md`）、手当ては`DECLARATION_EXEMPT`への登録＝Pull Request 1 本である。
  **早く気付くほど、その 1 本が軽い。**
- **現状、検出は`main`昇格の`history`だけである。**`6bcd7b9`は 2026-09-10 に`develop`へ入り、
  **昇格まで誰も見ない状態だった。**気付いたのは merge 直後に自分で`history`を回したからであり、
  **仕組みではない。**
- **判定を複製しない。**`review_gate.py`が正本であり、`gh_metadata_guard.py`は
  名前と判定の両方を同 module から受け取っている。**新しい検出も同じにする。**

## 検討した選択肢

### 選択肢A: `push_gate.py`（push 側の hook）を広げる

`b93b309`（`fixup`による直接 push）の再発防止として既にある。

**採らない。**merge commit は GitHub 側で作られ、**push を経由しない。**この経路に効かない。

### 選択肢B: `permissions.deny`で MCP の merge tool を止める

一次資料で確かめた（[Configure permissions](https://code.claude.com/docs/en/permissions)。2026-09-10 参照）。
**構文は実在する。**`permissions`の`deny`は`mcp__<server>__<tool>`の形を受け、
`mcp__github__merge_pull_request`のように 1 tool を名指しできる。

**採らない。**理由は 3 つある。

1. **経路を列挙して塞ぐ形である。**web UI からの merge には効かない。**次の経路でまた破れる。**
2. **server 名に依存する。**規則が当たるのは MCP server を`github`という名前で設定した環境だけである。
   **repository の設定として書いても、環境によって当たらない。**
3. **ユーザーが実際に使った経路を塞ぐことになる。**2026-09-10 に、ユーザーは
   [#373](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/373)の merge をこの経路で実行するよう指示している。
   **運用を否定する変更を、検出の副作用として入れない。**

**構文が使えることは記録として残す。**将来ユーザーがこの経路を使わないと決めたなら、
**多層の 1 枚として足す余地はある。**この決定はそれを禁じない。

### 選択肢C: `develop`／`main`への push で`history`を走らせる

`.github/workflows/declaration-audit.yml`を新設し、pushされた範囲を`review_gate.py history`で検証する。

利点: **経路に依らない。**MCP でも web UI でも `gh pr merge` でも、commit が入れば見る。
判定は`review_gate.py`が持ち、**`git interpret-trailers --parse`で読む。**複製しない。

コスト: **阻止ではなく事後検出である。**push は既に完了している。

### 選択肢D: 何もせず、`main`昇格の`history`だけに任せる

利点: 変更が無い。

コスト: **昇格の直前まで気付かない。**`6bcd7b9`はその状態だった。
昇格 Pull Request を作った時点で赤くなり、**そこから登録の Pull Request を挟むことになる。**

## 決定

**選択肢Cを採る。**

1. **`.github/workflows/declaration-audit.yml`を新設する。**`main`と`develop`への**push**で起動し、
   `before..after`の各 commit を`scripts/review_gate.py history`で検証する。
2. **判定を workflow へ複製しない。**`review_gate.py`を呼ぶだけとする。
   **`git interpret-trailers --parse`で読むことは、同 script が保証する。**
3. **`before`が解決できない場合（branch 作成、force push 後）は、`main`へ入っていない範囲を
   すべて見る。****直前の 1 commit へ縮めない。**縮めると、force push が複数の commit を
   持ち込んだときに**中間の commit を 1 つも見ない**（下の`検証`で実測した）。
   `main`到達済みの commit は昇格時に検査済みであり、そこから先を全部見れば取りこぼしが無い。
   既に通っている commit を再検査するだけなので、余分に落ちることもない。
   `main`自身への push では merge-base が head と一致するため、**そこだけ直前の 1 commit を見る。**
   親を持たない commit のときだけ、対象無しとして終了する。
4. **`cancel-in-progress`を有効にしない。**push ごとに範囲が違うため、
   後の push で前の範囲の検査を打ち切ると、**その範囲を誰も見なくなる。**
5. **`permissions.deny`による MCP tool の遮断は採らない。**理由は選択肢Bに書いた。
   **構文が使えることは記録として残す。**
6. **`gh_metadata_guard.py`を変更しない。**判定粒度の穴（`c171c52`）は既に閉じており、
   **hook は Bash 経路の「押す前に止める」層として残す。**多層のうちの 1 枚である。

## 影響

### 利点

- **経路に依らずに検出する。**hook を通らない merge でも、commit が共有 branch へ入れば見る。
- **気付く時点が、`main`昇格から merge の直後へ移る。**手当ての Pull Request が軽くなる。
- **判定が 1 箇所のままである。**`review_gate.py`を呼ぶだけであり、workflow は範囲を解決するだけである。
- **`main`への直接 push も見る。**昇格 Pull Request を通らない経路が入っても検出する。

### 欠点

- **阻止ではない。**push は完了しており、**赤い check が出るだけである。**
  失われた宣言は後から付けられないため、**手当ては`DECLARATION_EXEMPT`への登録のままである。**
- **`DECLARATION_EXEMPT`へ登録した commit は、その後の push でも素通りする。**
  免除は`history`の仕様であり、この workflow はそれを変えない。
- **範囲の解決を shell で書いている。**`review_gate.py`のような unit test を持たない。
  **手で実測して確かめた**（下の`検証`）。
- **hook を通らない経路そのものは塞いでいない。**多層の下側を足しただけである。
- **参照した公式文書の版を固定していない。**ADR-0018・ADR-0020 と同じ欠点である。

### リスクと対策

| リスク | 対策 |
|---|---|
| workflow が赤いまま放置され、慣れる | **`main`昇格で必ず落ちる。**放置しても最後に止まる。この workflow はその時点を早めるだけである |
| 範囲の解決が誤り、検査対象が空になる | `before`が解決できないときは`main`へ入っていない範囲へ落とす。**素通りさせない。**4 通り（正常・zero・複数 commit・親なし）を実測した |
| force push で範囲が飛ぶ | `before`は force push 前の head を指す。履歴に無ければ fallback が効く。**共有 branch への force push は`AGENTS.md`が禁じている** |
| 判定が workflow 側へ漏れ出す | workflow は範囲を解決して`review_gate.py`を呼ぶだけである。**trailer の名前も読み方も持たない** |
| MCP 経路が塞がれないままである | **引き受ける。**選択肢Bの 3 つの理由による。構文は記録した |

## 検証

- **`6bcd7b9`と同じ踏み方の commit を作り、検出されることを実測した。**
  worktree を切って`docs/toolchains/verified-commands.md`を 1 行変え、
  `Refs: #379`と`Co-Authored-By:`の間へ空行を入れた commit を作った。
  `interpret-trailers --parse`は`Co-Authored-By`だけを返し、
  workflow と同じ手順で`history`を回すと**exit 1** で 3 件の problem を出した。

  ```text
  30fc04e must carry exactly one valid Change-Class trailer, found []
  30fc04e carries no Self-Review trailer
  the range changes 1 instruction source path(s) but 30fc04e... does not carry
  Instruction-Change: reviewed-as-data.
  ```

- **範囲の解決を実測した。**正常な`before`で exit 1、`before`が all-zero のとき
  fallback が効いて exit 1、宣言が揃っている範囲では exit 0。

- **fallback を「直前の 1 commit」にしていた形が、実際に取りこぼすことを実測した。**
  worktree で 2 commit を作り、**中間は宣言なし・head は宣言あり**とした。

  | fallback | 結果 |
  |---|---|
  | `${PUSH_AFTER}^`（当初） | **exit 0。中間 commit を見逃す** |
  | `merge-base` with `origin/main`（採用） | **exit 1。中間 commit を検出** |

- **merge commit が抜け道にならないことを実測した。**`history`は`rev-list --no-merges`を
  使うため merge commit 自体は検査しない。**しかし merge commit が持ち込む commit は検査する。**
  宣言を持たない commit を feature branch へ置き、`--no-ff`で merge して測った。

  ```text
  HISTORY_CHECKED=1 MERGES_SKIPPED=1 EXEMPT=0
  6267c31 must carry exactly one valid Change-Class trailer, found []
  ```

  **`main`昇格の merge commit が宣言を持たないのは設計どおりである**（[ADR-0010](0010-change-class-and-review-declaration.md)）。
  宣言は squash commit が持ち、`history`はそれを見る。**merge commit を検査対象にすると、
  昇格そのものが落ちる。**
- 見直し条件: **この workflow が検出した事故が、それでも`main`昇格まで放置された場合。**
  そのときは検出ではなく阻止が要る。選択肢Bと、branch protection 側の手段を再検討する。

## 置き換える決定

なし。[ADR-0010](0010-change-class-and-review-declaration.md)が定めた宣言の検査に、
**経路に依らない検出点を 1 つ足す。**同 ADR の分類も宣言も変えていない。
