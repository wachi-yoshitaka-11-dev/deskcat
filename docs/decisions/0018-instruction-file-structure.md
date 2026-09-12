# ADR-0018: 指示 file を、常時読み込みと path 限定に分ける

> 状態: Accepted
> 日付: 2026-09-09
> 起案: AIが起案した。**方針はユーザーが選択した。**
> **内容に対する人間の承認は、この決定を含むPull Requestのmergeで得る。**
> 記述の根拠は[Claude Code公式文書](https://code.claude.com/docs/en/memory)であり、
> 2026-09-09に取得した。出典は`## 出典`節が持つ。

## 背景

`CLAUDE.md`は`@AGENTS.md`をimportしており、**`AGENTS.md`の全文が毎sessionのcontextへ入る。**

**2026-09-09に測った。数え方は「見出し行から次の見出しの直前まで」である。**
**節に属さない冒頭ブロックを別行として立て、和が file 全体と一致することを確かめている。**

| 節 | 行数 |
|---|---|
| 検証 | 60 |
| 下位ディレクトリの追加指示 | 25 |
| 推測禁止 | 18 |
| 作業開始時に読むもの | 14 |
| 外部操作 | 14 |
| 完了報告 | 14 |
| 変更規則 | 12 |
| ハードウェア安全 | 11 |
| Git と公開 | 11 |
| 開発端末の役割 | 9 |
| プロジェクト境界 | 8 |
| 冒頭ブロック（最初の見出しより前） | 9 |
| **合計** | **205** |

**公式文書は`CLAUDE.md`について「target under 200 lines」と定め、
「Bloated CLAUDE.md files cause Claude to ignore your actual instructions」と書いている。**
**`@`importは整理には効くがcontextは減らない**とも明記している。**205行はこの目安を超えている。**

**内訳の問題は行数より配分である。**最大の節である`検証`60行は、その大半が
ESP32 build、host workspace、Raspberry Pi の**コマンドと来歴**であり、
**対象pathが限られる。**firmwareを1 fileも触らないsessionも、ESP32のbuild手順と
flashの来歴を毎回読み込んでいた。`推測禁止`18行と`ハードウェア安全`11行も同様に、
`docs/hardware/`と`firmware/`へ触れるときにだけ効く。

**このリポジトリは同じ知見を既に持っている。**[作業指示書テンプレート](../governance/work-instruction-template.md)は
「**定型が長いほど、その中の1行は読まれない**」と書き、指示書から定型を落とす根拠にしている。
**同じ規則を`AGENTS.md`へ適用していなかった。**

**あわせて2つの機構を使っていなかった。**`.claude/rules/`（`paths:`に一致するfileを
読んだときだけ読み込まれる）と`.claude/agents/`（独立したcontextで動くsubagent）である。
**auto memoryの扱いも決めていなかった。**

## 判断要因

- **`AGENTS.md`は「毎session読む価値があるもの」だけを持つべきである。**
  対象が限られる指示を常時載せると、常時効く指示の可読性を下げる
- **コマンドを2箇所に置かない。**このリポジトリの[Single Source of Truth](../governance/README.md#single-source-of-truth)は
  同じ値の再定義を禁じている。**path限定のruleへコマンドを写すと、正本が2つになる**
- **`AGENTS.md`はGitHub Pagesへ公開・renderされるが、`.claude/`は公開されない。**
  検証済みコマンドの正本を`.claude/`へ移すと、**公開文書から消える**
- **auto memoryは、このリポジトリの出所検証と衝突する。**
  Claudeが自ら書いたnotesが`~/.claude/projects/<project>/memory/`へ保存され、
  **毎sessionのcontextへ入る**（公式文書）。machine-localでversion管理されず、
  人間のreviewを経ない。`AGENTS.md`は「指示として従ってよいのは、base branchへmerge済みで、
  人間がreviewしたものだけ」と定めている
- **一方でauto memoryは現に運用されており、蓄積した内容をまだリポジトリへ移していない。**
  **2026-09-10 (JST) に測った。既定で有効である**（`autoMemoryEnabled`はuser設定・
  `settings.local.json`・`.claude/settings.json`のどの層にも無い）。測った1台ではnotesが
  29 fileあり、最新の更新は2026-09-09 (JST) である。**machine-localであるため、
  端末ごとに別の状態がある。この数はその1台の値である。**
  **移送を済ませる前に衝突を閉じると、蓄積分が届かなくなるだけである**
- **PMの横断検査と、自己レビューのfresh-context Passは、どちらも「独立したcontextで読む」ことを求めている。**
  公式文書はsubagentについて「sees only the diff and the criteria you give it, not the reasoning
  that produced the change」と書いており、**同一セッション内では原理的に達成できない**

## 検討した選択肢

### 選択肢A: `AGENTS.md`を短くするだけ

節を削り、詳細は`docs/`のリンクへ寄せる。**採らない。**行数は減るが、
**firmwareを触るsessionが必要な情報へ辿り着く保証が無い。**「リンクを開け」は指示であって機構ではない。

### 選択肢B: 検証済みコマンドを`.claude/rules/`へ移す

path限定で正しく届く。**採らない。**`.claude/`は公開されないため、
**検証済みコマンドの正本が公開文書から消える。**このリポジトリはPagesで開発方針を公開している。

### 選択肢C: 正本は公開される`docs/`へ、path限定のruleは正本を指す

コマンドと来歴の正本を`docs/toolchains/verified-commands.md`（公開・render対象）へ置く。
`.claude/rules/`は`paths:`で対象を絞り、**踏みやすい点だけを書いて正本へリンクする。**

## 決定

**選択肢Cを採る。**

1. **`docs/toolchains/verified-commands.md`を新設し、検証済みコマンドと来歴の正本とする。**
   `AGENTS.md`の`検証`節が持っていた内容を移す。**公開・render対象である**
2. **`.claude/rules/`を新設する。**`paths:`で対象を絞り、**コマンドを写さず正本へリンクする。**
   - `esp32-firmware.md`（`firmware/esp32/**`、`crates/deskcat-protocol/**`）
   - `host-and-pi-build.md`（`crates/**`、`apps/**`、`simulator/**`、`Cargo.toml`、`Cargo.lock`）
   - `hardware-values.md`（`docs/hardware/**`、`docs/protocol/**`、`hardware/**`、`tests/hil/**`）
3. **`AGENTS.md`を205行から152行にする。**同じ数え方で`検証`60→10、`推測禁止`18→10、
   `ハードウェア安全`11→8。**`セッションの役割`と`.claude/rules/`の1行を新たに足したうえでの152行である。**
   **削った内容は正本へ移すか、既に正本にあるものを消す。新しく失った規則は無い**
4. **`.claude/agents/`を新設し、read-onlyの検査subagentを2つ置く。**
   - `consistency-inspector`（正本との突き合わせ。PMが横断検査として行ってきたこと）
   - `fresh-context-reviewer`（diffだけを読む。意図を持たない読み手）
   **どちらも`model: opus`とする**（PM相当の作業であるため）。`tools`は`Read, Grep, Glob, Bash`である。
   **read-onlyは機構で保証していない。**`git diff`のために`Bash`を与えており、**`Bash`は書き込める。**
   **担保しているのは各 agent の本文の指示だけである**
   （**この欠点へ[ADR-0020](0020-inspector-readonly-by-hook.md)が機構の門を足した。ただし閉じてはいない**（同 ADR は guard を「保証ではなく best effort」と位置づけている）。agent frontmatterの
   `hooks.PreToolUse`でallowlist判定を掛ける。`tools`と`model`はこの決定のままである）
5. **auto memoryは無効にしない**（**2026-09-10にユーザーが選択した**）。
   `.claude/settings.json`へ`"autoMemoryEnabled"`を置かず、
   **既定の有効なままとする。**判断要因に挙げた出所検証との衝突は残るため、
   **欠点として引き受ける**（下の`欠点`）。**先に要るのは移送である。**蓄積したnotesのうち
   残す価値のあるものを`AGENTS.md`か`docs/`へ移し、**それを終えてから無効化を別に判断する。**
   **この決定が定めるのは順序だけである。移送は未着手であり、引き受け先も期限も決めていない**
6. **セッションの役割は`AGENTS.md`の4行で表す。**独立した統治文書を作らない。
   PMも作業セッションもAIであり、**PMの出力は人間の承認を代替しない**
7. **command を重複して持っていた file を、正本へのリンクへ置き換える。**
   root `README.md`、`firmware/esp32/README.md`、`crates/deskcat-serial/README.md`、
   `crates/deskcat-protocol/README.md`、および`CONTRIBUTING.md`の所在記述である。
   **`README.md`には、同節が古くなっていたという事実を記録として残す**（下の`重複が実際に食い違っていた`）。
8. **消さずに残す複製を2つだけ認め、追随義務を明記する。**
   `.github/workflows/README.md`は**workflowが実際に実行するものの記述**であり、
   `docs/runbooks/esp32-development-machine-setup.md`は**toolchain導入後の動作確認**である。
   **どちらも正本ではない。**各箇所へ「正本が変わったらここを合わせる」と書く
9. **`docs/toolchains/verified-commands.md`と`docs/toolchains/machine-profiles.md`を
   `review_gate.py`の`INSTRUCTION_SOURCES`へ足す。**
   移送先が分類の対象外だったため、**この決定は review 経路を弱めていた**（下の`移送が分類を弱めていた`）。
   **足すのはこの1 fileであり、`docs/toolchains/`全体ではない。**
   理由は2つある。**(a) この決定が正本と定めたのはこの1 fileである。**
   **(b) ディレクトリ単位で足すと、過去の免除commitが遡って落ちる**（下の節で実測した）。
   **「同ディレクトリの他のfileは規則を持たないから」ではない。**
   `machine-profiles.md`も`状態: Accepted policy`であり、`AGENTS.md`の`開発端末の役割`節が
   作業開始時に読むよう指示している。**同 file が入っていなかったのはこの決定が作った穴ではないが、
   同じ性質であるため合わせて足した。**残る4 file（`README.md`、`esp32-rust-toolchain.md`、
   `raspberry-pi-rust-toolchain.md`、`version-record-template.md`）と`version-records/`は
   **調査記録・検証記録・templateであり、この決定では足さない。**
   **列挙の拡大であり、縮小ではない。**[ADR-0010](0010-change-class-and-review-declaration.md)の
   見直し条件が禁じているのは縮小であり、同ADRは「`INSTRUCTION_SOURCES`に載っていないfileが
   将来規則を持ち始めた場合、列挙の更新が要る」と既に書いている。**その更新に当たる**

## 移送が分類を弱めていた

**この決定の作業中に実測した。**`docs/toolchains/`は`review_gate.py`の`INSTRUCTION_SOURCES`に
入っていなかった（追加前は12項目）。**`AGENTS.md`にあった間は、`検証`節を触る変更が必ず
`review-required`になっていた。**移送すると、`LINE_DENY`のどれにも当たらない散文行だけの変更が
軽微経路へ入りうる。

`verified-commands.md`の`実機試験が必要な変更を、PCテストだけで完了扱いにしない。`は
`LINE_DENY`の8規則（数字・inline code・link・autolink・`\|`・見出し・checkbox・HTML comment）の
いずれにも当たらない。**この1行だけを書き換えたcommitを作り、`classify --base <親> --head HEAD`で測った。**

| | `classify`の出力 |
|---|---|
| `docs/toolchains/`を足す前 | **`CLASS=minor`** |
| 足した後 | **`CLASS=review-required`**（`reason: docs/toolchains/verified-commands.md: instruction source`） |

**`minor`はIssueとPull Requestなしで`develop`へ入れてよい区分である。**
移送した内容は「build-onlyであり、flashと実機起動は主張しない」のような**主張範囲の宣言**であり、
そこが緩む。**決定の9で閉じた。**

**ディレクトリ単位では足さない。**`docs/toolchains/`全体を足すと、
`docs/toolchains/version-records/`しか触っていない過去の免除commitが
**遡って「指示sourceを触る」ことになり、`history`が次の`main`昇格で落ちる。**
`scripts/test_review_gate.py`の免除列挙の照合testが実際にこれを検出した。

**実測で落ちたのは`619c843`である**（`docs/toolchains/version-records/`の2 fileを触る）。
**`1a5dda8`も同じ性質だと`DECLARATION_EXEMPT_ENTRIES`が記録しているが、
測った checkout に同 commit が無く、こちらは追検証していない。**

**Version Recordは記録であって規則ではない。**

**同じ穴が`AGENTS.md`の出所検証の側にも残っている。**適用範囲は「作業開始時に読むもの」に
挙がる文書と、明示列挙した`docs/hardware/`・`docs/protocol/`等であり、
**`docs/toolchains/`はそのどちらにも入らない。**`INSTRUCTION_SOURCES`は機械の分類、
出所検証は読み手の行動規則であり、**別の経路である。この決定では閉じていない。**

## 重複が実際に食い違っていた

**この決定の作業中に測った。**`AGENTS.md`と root `README.md`が同じcommandを重複して持ち、
**内容が食い違っていた。**

| 項目 | root `README.md` | 実際 |
|---|---|---|
| ESP32の現行treeに対する最新の検証 | 2026-08-10 | **2026-08-15**（[Version Record](../toolchains/version-records/2026-08-15-esp32-build-native-linux.md)が存在する） |
| ESP32のflash・serial monitor | 「まだ検証済みcommandが無い」 | **2026-08-20に検証済み**（[Version Record](../toolchains/version-records/2026-08-20-esp32-flash-boot-native.md)） |
| Raspberry Piのbuildと実行 | 「まだ検証済みcommandが無い」 | **2026-08-26にbuild・lint・testを検証済み**（[Version Record](../toolchains/version-records/2026-08-17-pi-direct-build-native.md)の再検証節） |

**`AGENTS.md`側は追随していたが、`README.md`側は追随していなかった。**
**同じ性質のずれが`CONTRIBUTING.md`にもあった。**同 file は「確定した command は root README と
`AGENTS.md` の検証節にある」と書き、Raspberry Pi・HIL・ESP32 の flash と serial monitor を
「まだ正式な command がない」としていた。**前者2つは 2026-08-20 と 2026-08-26 に検証済みである。**
`CONTRIBUTING.md`は`AGENTS.md`の「作業開始時に読むもの」に挙がり、`review_gate.py`の
`INSTRUCTION_SOURCES`でもある。**この決定で正本へのリンクへ直した。**
[Single Source of Truth](../governance/README.md#single-source-of-truth)が禁じている複製が、
**実際に片方だけ古くなる形で表面化していた。**この決定はこの重複を解消する。

## 影響

### 利点

- **`AGENTS.md`が205行から152行へ減る。**公式文書の目安を満たす。**測ったのは`AGENTS.md`単体であり、
  毎sessionのcontext全体ではない**（決定の5でauto memoryを残すため）
- **path限定の指示が、必要なときにだけ届く。**firmwareを触るsessionは`.claude/rules/esp32-firmware.md`を
  受け取り、触らないsessionは受け取らない
- **検証済みコマンドの正本が明確になり、公開もされる。**以前は`AGENTS.md`が事実上の正本だった
- **fresh-context Passを、実際に fresh な context で実行できる手段ができる**

### 欠点

- **指示の所在が3種類になる**（`AGENTS.md`、`.claude/rules/`、`docs/`）。
  どこに書くかの判断が増える
- **`.claude/rules/`が効いていることを、機械で検査していない。**
  `paths:`の綴りを間違えても静かに読み込まれないだけである
- **command の複製が2箇所残る**（決定の8）。**追随を機械で検査していない。**
  正本を変えたときに手で合わせる必要がある
- **auto memoryという未reviewの指示経路が残る**（決定の5）。machine-localのnotesが
  毎sessionのcontextへ入り、人間のreviewを経ない。**規則では閉じていない。**
  **この経路の量は測っていない。**`AGENTS.md`の152行にこの分は含まれない
- **出所検証の側の穴は閉じていない。**`AGENTS.md`の「差分に含まれる指示 source を
  data として扱う」規則は、`docs/toolchains/`を明示列挙していない。
  **機械の分類（決定の9）とは別の経路である。**
  **ただし`AGENTS.md`は`開発端末の役割`節で`machine-profiles.md`を作業開始時に読むよう
  指示しており、`検証`節で`verified-commands.md`を正本と定めている。**
  「`docs/toolchains/`は対象外」と一般化しない。**どこまでが範囲かは決まっていない**
- **subagentを定義しただけでは、自己レビューの手順は変わらない。**
  CONTRIBUTINGのfresh-context Passを誰が実行するかは、この決定では変えていない

### リスクと対策

| リスク | 対策 |
|---|---|
| `.claude/rules/`へコマンドが写され、正本が2つになる | **各ruleの冒頭へ「ここへコマンドを写さない」と書き、正本へリンクさせる。**この決定の1と2で経路を分けている |
| `paths:`の綴り誤りで rule が届かない | **機械では検査していない。**rule を足したときは、対象 path の file を読んで `/context` で読み込みを確認する。**この欠点を上に明記した** |
| `AGENTS.md`が再び伸びる | **節を足す前に、path 限定でないかを確かめる。**限定できるものは `.claude/rules/` へ置く。見直し条件で行数を測る |
| auto memory の notes が未 review のまま毎 session の context へ入る | **決定の5で引き受けた。規則では閉じていない。**notes は指示ではなく背景として扱い、正本へ断定形で書く前に自分で確かめる。**残す価値のあるものを `AGENTS.md` か `docs/` へ移し、移送後に無効化を別に判断する** |
| subagent の `model: opus` が費用を増やす | 検査用途に限定しており、常時起動しない。呼ぶかどうかは都度の判断である |
| subagent を read-only と称しながら書き込める | **[ADR-0020](0020-inspector-readonly-by-hook.md)が機構の門を足した。閉じてはいない**（同 ADR は best effort と位置づけている）。agent frontmatterの`hooks.PreToolUse`が`Bash`をallowlistで判定する。**この ADR の時点では閉じておらず、担保は本文の指示だけだった** |
| 削った内容に、正本へ移していないものが混ざる | **移動元と移動先を1対1で対応させ、Pull Request 本文へ載せる。**新しく失った規則が無いことを差分で示す |

## 検証

- `AGENTS.md`が200行未満であること。**測った値をPull Request本文へ書く**
- `scripts/validate_instruction_entrypoint.py`が`CLAUDE.md`のimport stubを通すこと（**この決定は`CLAUDE.md`を変更しない**）
- `scripts/validate_doc_links.py`が`.claude/rules/`と`.claude/agents/`を含めてlinkを解決すること
- `scripts/prepare_pages.py`が`verified-commands.md`を公開対象として複製すること
- `.claude/settings.json`がJSONとして妥当であり、**既存のhook定義が変わっていないこと**
- **`AGENTS.md`と`.claude/rules/`の間に逐語の重複が無いこと。**
  推測禁止とハードウェア安全は`AGENTS.md`が持ち、rule は写さない
- **決定の8で残した2箇所以外に command の複製が無いこと**（Version Record と実験記録は記録であり対象外）
- **`verified-commands.md`の散文1行だけを変えたcommitが`review-required`になること。**
  `scripts/test_review_gate.py`のpath境界caseで`docs/toolchains/verified-commands.md`と
  `docs/toolchains/machine-profiles.md`が`True`、`docs/toolchains/README.md`と
  `docs/toolchains/version-records/README.md`が`False`であることを固定した
- **免除commitの列挙と実測が一致すること**（`test_the_comment_lists_every_exempt_commit_touching_instruction_sources`）。
  **ディレクトリ単位で足すとここが落ちる**
- 見直し条件: **`AGENTS.md`が再び200行を超えた場合は、この決定を改訂するのではなく、
  超えた分をpath限定へ切り出せるかを先に見る。**切り出せないものだけが行数増の候補である

## 出典

2026-09-09に取得した。**版を固定していない。食い違いが出たら公式文書を正とする。**

| 出典 | 使った箇所 |
|---|---|
| [How Claude remembers your project](https://code.claude.com/docs/en/memory) | 200行の目安、`@`importがcontextを減らさないこと、`.claude/rules/`の`paths:`、auto memory |
| [Subagents](https://code.claude.com/docs/en/sub-agents) | subagentのcontext isolation、`tools`と`model`のfrontmatter |
| [Best practices for Claude Code](https://code.claude.com/docs/en/best-practices) | contextが埋まると性能が落ちること、adversarial review step |

## 置き換える決定

なし。**[ADR-0010](0010-change-class-and-review-declaration.md)を置き換えない。**
`.claude/`と`AGENTS.md`は引き続き`review_gate.py`の`INSTRUCTION_SOURCES`であり、
**軽微経路へ入らない。**この決定はその分類を変更していない。
