# ADR-0020: 検査 subagent の read-only を、sandbox ではなく agent 単位の hook で保証する

> 状態: Accepted
> 日付: 2026-09-10

## 背景

[ADR-0018](0018-instruction-file-structure.md)の決定4で`.claude/agents/`へ検査 subagent を2つ置いた。

- `consistency-inspector` — 変更範囲を正本文書と突き合わせる
- `fresh-context-reviewer` — 最終 diff だけを読む

**どちらも`tools: Read, Grep, Glob, Bash`であり、read-only は機構で保証されていない。**
`Bash`は書き込める。担保しているのは各 agent 本文の「read-only である。file を変更しない」という指示だけである。

**`Bash`を外す案は採れない。**両 agent は`git show`／`git diff`で差分を取得する。
`Read`／`Grep`／`Glob`はgitを実行できず、外すと検査対象へ到達できない（#373 で確認済み）。

#373 の CodeRabbit review が同じ点を`CWE-250`として指摘し、**write-protected sandbox を代替として挙げた。**
[#376](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/376)はその可否を判断するための Issue である。

## 判断要因

**一次資料で確かめた**（[Configure the sandboxed Bash tool](https://code.claude.com/docs/en/sandboxing)、
[Subagents](https://code.claude.com/docs/en/sub-agents)。いずれも 2026-09-10 に参照。**版は固定されていない**）。

| 確かめたこと | 結果 |
|---|---|
| 書き込みを禁じる設定 key は実在するか | **実在する。**`sandbox.filesystem.denyWrite`。`allowWrite`／`denyRead`／`allowRead`／`disabled`も同じ階層にある |
| 既定の書き込み範囲 | working directory、session の temp directory、`permissions.additionalDirectories`で足した directory |
| 対応 platform | macOS、Linux、WSL2。**native Windows は非対応** |
| **適用範囲** | **`.claude/settings.json`等の settings scope である。**subagent 単位で切り替える key は無い |
| **sandbox が覆う tool** | **`Bash`だけである。**文書は「`Read`、`Edit`、`Write`は sandbox ではなく permission system を直接使う」と明記する |
| subagent frontmatter に sandbox 用の field はあるか | **無い。**`tools`／`disallowedTools`／`permissionMode`／`hooks`等はあるが、sandbox 設定も Bash allowlist の field も無い |
| subagent 単位で hook を掛けられるか | **掛けられる。**frontmatter の`hooks`が「lifecycle hooks scoped to this subagent」と定義されている。文書は`PreToolUse`で Bash を検証する形を、「read-only な query だけを許す」ような条件付き規則の**機構**として挙げている |

ここから3つが出る。

1. **sandbox は subagent 単位に絞れない。**`.claude/settings.json`へ書けば、
   **この repository のすべての session に効く。**通常の作業 session が書けなくなれば開発が止まる。
   除外を設計するとしても、除外の単位は path であって agent ではない。**agent を分ける手段が無い。**
2. **このリポジトリには既に agent 単位でない hook が5本ある**（`.claude/settings.json`）。
   **hook を agent frontmatter へ置く形は、既存の書き方をそのまま使える。**
   `command_line.py`（command位置の判定を1箇所に寄せる module）も既にある。
3. **sandbox が`Bash`しか覆わないことは、この用途では欠点にならない。**
   検査 agent の`tools`は`Read, Grep, Glob, Bash`であり、**`Edit`と`Write`を持たない。**
   書き込み経路は`Bash`だけである。**sandbox でも hook でも、塞ぐ対象は同じ1つである。**

## 検討した選択肢

### 選択肢A: `.claude/settings.json`へ`sandbox.filesystem.denyWrite`を置く

利点: OS が強制する。hook のような字句判定より強い。settings は既に repository で管理している。

コスト: **repository 全体に効く。**検査 agent 2件のために、通常の作業 session の書き込みまで止まる。
除外を path 単位で書くことになるが、**通常の作業が書きたい path は「repository 全体」であり、
検査 agent が書いてはいけない path も「repository 全体」である。同じ集合を agent で分けられない。**
platform 依存もある（native Windows 非対応）。**採れない。**

### 選択肢B: agent frontmatter の`hooks.PreToolUse`で`Bash`を allowlist 判定する

`.claude/agents/*.md`の frontmatter へ`hooks`を書き、`scripts/hooks/inspector_readonly_guard.py`を呼ぶ。
**その agent の`Bash`呼び出しにだけ掛かる。**

利点: **agent 単位で閉じる。**通常の作業 session に一切影響しない。
**allowlist にできる。**通ってよい program と git subcommand だけを列挙し、それ以外を拒否する。
既存の hook 5本と同じ書き方であり、`command_line.py`を再利用できる。**unit test で固定できる。**

コスト: **OS の強制ではない。字句判定である。**`command_line.py`の docstring が挙げる取り漏らし
（alias、shell function、変数展開、`sh -c`の内側）はここでも残る。
**allowlist で塞ぐが、塞ぎ方は「解釈できないものを拒否する」であって「実行を封じる」ではない。**

### 選択肢C: `disallowedTools`で`Bash`を外し、検査を諦める

利点: 完全に閉じる。

コスト: **検査が成立しない。**両 agent は差分を`git`で取る。#373 で確認済みである。

### 選択肢D: 何もせず、指示だけで運用を続ける

利点: 変更が無い。

コスト: **#373 の指摘がそのまま残る。**「read-only」と名乗る agent が書き込める状態が続く。

## 決定

**選択肢Bを採る。sandbox は採らない。**

1. **`sandbox`設定を導入しない。**理由は「subagent 単位に絞れず、通常の作業 session を止めるため」である。
   **sandbox 自体を否定する決定ではない。**別の目的（外部 command の network 隔離など）で採る余地は残す。
2. **`scripts/hooks/inspector_readonly_guard.py`を新設する。**agent frontmatter の
   `hooks.PreToolUse`（`matcher: Bash`）から呼ぶ。**`.claude/settings.json`へは置かない。**
3. **判定は allowlist とする。**
   - command位置の program は`ALLOWED_PROGRAMS`のみ。**単体で file を書けるものを入れない**
     （`sort -o`、`uniq out`、`sed -i`、`tee`、interpreter を除外した）
   - `git`の subcommand は`GIT_READONLY_SUBCOMMANDS`のみ。**flag に関係なく書かないものだけ**
     （`branch`／`tag`／`config`は flag 次第で壊せるため入れない）
   - **shell metacharacter（`>|;&()`` ` ``$<{}`）を含む語を拒否する。**
     `shlex`は空白でしか語を切らないため、`cat a>b`も`cat a;rm -rf /`も1語になり、
     redirect も次の command も command 位置として見えない
   - **判定は行ごとに行う。**`shlex`は改行を空白として扱うため、
     `git show HEAD`と`rm -rf /`を改行で並べると1つの語列に潰れる
   - **語へ分けられない command は拒否する（fail closed）。**他の hook は素通りさせるが、
     **こちらは境界であり、解釈できない入力を通すことは境界を開けることと同じである**
   - **subcommand が読み取り専用でも、`git` の option が抜け道になる。**次を拒否する。
     global 位置の `-c`／`--config-env`（`git -c core.pager='rm -rf /' log`で任意 command を実行できる）、
     `--exec-path`、`git grep` の `-O`／`--open-files-in-pager`（pager として任意 command を起動する）、
     `--output`（**`git diff --output=<file>` は file を書く**）。
     **subcommand の後ろの `-c` は merge の combined diff であり、拒否しない**
   - **command 位置の環境変数代入を拒否する。**`command_line` は後続 command へ透過させるが、
     `GIT_EXTERNAL_DIFF=rm git show` のように**環境変数だけで任意 command を起動できる。**
     **判定は command 位置の語だけに当てる。**全語へ当てると `grep FOO=bar file` や
     `git diff -- file=1` まで落ちる
   - **command 位置の前置語を拒否する。**`command_line.TRANSPARENT_PREFIXES`
     （`env`／`sudo`／`nohup`／`time`／`command`／`exec`）は後続 command へ透過するため、
     **`sudo cat /etc/shadow` は `cat` だけを見れば allowlist を通る。**
     `sudo` は実行の権限を変え、他も allowlist の外を呼ぶ足場になりうる。**1 つも許さない。**
     `env` も許さない（代入自体を拒否しているため使い道が無い）
4. **`tools`から`Bash`を外さない。`model: opus`も変えない。**ADR-0018 の決定4のままとする。
5. **agent 本文の「read-only である」は残す。**機構と指示の両方を置く。
   **hook が掛からない環境（対応していない version）では、指示だけが残る。**
6. **hook の wrapper を fail closed にする。**repository root、guard file、`python3` の
   いずれかが無い場合と、guard が異常終了した場合は **`exit 2` で `Bash` を拒否する。**
   **`exit 2` だけが tool 呼び出しを止める**（[公式文書](https://code.claude.com/docs/en/hooks)。
   他の非 0 は「block しない error」として扱われ、動作は続行する）。
   `.claude/settings.json` の既存 hook 5 本は `|| exit 0` で fail open だが、
   **あちらは「書き忘れを指摘する」層であり、素通りは指摘漏れで済む。こちらは境界である。**

## 影響

### 利点

- **書き込み経路が機構で閉じた。**`Edit`／`Write`を持たない agent にとって、`Bash`が唯一の経路であり、そこに allowlist が掛かる。
- **通常の作業 session に影響しない。**hook は agent frontmatter にあり、`.claude/settings.json`には無い。**test でそれを固定した。**
- **allowlist の中身が test で固定された。**`sort`や`tee`を後から足すと test が落ちる。足すならこの ADR を更新することになる。
- **ADR-0018 の欠点を1つ閉じた。**同 ADR の決定4とリスク表を更新した。

### 欠点

- **OS の強制ではない。**字句判定であり、`command_line.py`が取れないもの（alias、shell function、変数展開、`sh -c`の内側）は取れない。
  **ただし`sh`・`bash`・`xargs`・interpreter は allowlist に無く、command位置に現れれば拒否される。**
- **hook が実際に掛かることを、この決定では実測していない。**agent frontmatter の`hooks`は
  [公式文書](https://code.claude.com/docs/en/sub-agents)に記載があるが、**subagent を起動して
  書き込みが実際に拒否されるところまでは測っていない。**測ったのは hook script 単体の判定である
  （`check()`への 55 例、wrapper の 4 条件、hook として起動したときの`deny` payload）。
- **誤検知がある。**`grep "=>"`のように、metacharacter を含む正当な引数を拒否する。
  **代替がある**（両 agent は`Grep` tool を持つ）ため引き受けた。
- **`ALLOWED_PROGRAMS`は「単体で書けない」という判断に依存する。**判断が誤っていれば穴になる。
  `sort -o`と`uniq out`は実際に見落としやすく、**除外した理由を script の comment へ書いた。**
- **入力が JSON でない場合は素通りする。**`command_from`が`None`を返す場合と同じ扱いである。
  **境界としては弱い。**hook 側の事故を対象 command の問題として扱わないほうを優先した。
- **参照した公式文書の版を固定していない。**ADR-0018 と同じ欠点である。
- **allowlist の設計は、`command_line` が透過させるものを打ち消す形になっている。**
  同 module が`TRANSPARENT_PREFIXES`を増やすと、この guard は自動では追随しない。
  `test_transparent_prefixes_do_not_smuggle_an_allowed_program`が全件を走査して固定するが、
  **「増やしたときに落ちる」形であって、「増やしたら正しくなる」形ではない。**

### リスクと対策

| リスク | 対策 |
|---|---|
| hook が掛からない version で、read-only が指示だけに戻る | **agent 本文の指示を残した**（決定5）。**掛かっていないことを検出する手段は無い。引き受ける** |
| allowlist が狭すぎて検査が止まる | 拒否理由に「許可しているのは何か」と「広げるなら ADR-0020 を更新する」を書いた。**黙って諦めさせない** |
| allowlist を後から安易に広げる | `test_hooks.py`が、書き込めるprogramと状態を変えるgit subcommandが入っていないことを固定する。**広げる変更はtestを落とす** |
| 字句判定を抜ける形が見つかる | **allowlistであり、抜けるには許可した program 自身の書き込み経路を使うことになる。**見つかったら`ALLOWED_PROGRAMS`から外す |
| `.claude/settings.json`へ間違って置かれる | `test_the_guard_is_not_wired_globally`が固定する |
| 片方の agent にだけ書き忘れる | `test_both_inspector_agents_wire_the_guard`が固定する |

## 検証

- `python3 scripts/test_hooks.py` — **158件 OK**（`InspectorReadonlyGuardTests` 17件を含む）
- allowlist 側 16 例が通り、拒否側 39 例が落ちることを`check()`で確認した
- **wrapper を 4 条件で実測した。**guard が無い／`python3`が無い → **exit 2**、
  読み取り command → exit 0 かつ stdout 空、書き込み command → exit 0 かつ`deny` payload
- hook として起動し、`rm -rf /`に対し`permissionDecision: deny`が出ることを確認した
- `git show HEAD`に対し stdout が空（素通り）であることを確認した
- 見直し条件: **subagent を実際に起動して書き込みが拒否されることを測れたとき**、
  この ADR の「実測していない」を実測結果へ差し替える。
  **測れないまま allowlist を広げる変更を重ねる場合は、選択肢A（sandbox）を再検討する。**

## 置き換える決定

なし。[ADR-0018](0018-instruction-file-structure.md)の決定4が残した欠点を閉じる。同決定の`tools`と`model`は変えない。
