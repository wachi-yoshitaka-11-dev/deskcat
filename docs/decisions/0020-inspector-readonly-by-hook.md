# ADR-0020: 検査 subagent の read-only を、sandbox ではなく agent 単位の hook で絞る

> 状態: Accepted
> 日付: 2026-09-10（**2026-09-11 に改訂。**題、決定3、選択肢A の再検討、選択肢E・F の追加、
> 利点・欠点・リスク表・検証、置き換える決定。`検証`を参照）

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

**2026-09-11 に再検討した。**[公式文書](https://code.claude.com/docs/en/sandboxing)と
[subagent の公式文書](https://code.claude.com/docs/en/sub-agents)を開いて確認した結果、
**却下の理由は今も成立している。**sandbox の設定先は `settings.json`（user／project／managed）だけで、
**subagent の frontmatter に sandbox を絞る field は無い。**
**制限に関わる field は `tools`／`disallowedTools`／`permissionMode`／`mcpServers`／`hooks`／`isolation` である**
（`name`／`description`／`model` のような、制限に関わらない field は挙げていない）。
**「全 field を見た」とは書かない。**公式文書に載っている範囲で、制限に使えるものを探した結果である。

**このうち評価したのは4つである。**`tools`（選択肢C・E）、`disallowedTools`（C）、`hooks`（B）、`isolation`（F）。
**`mcpServers` は MCP server の範囲を決めるもので、filesystem を制限しない**（[公式文書](https://code.claude.com/docs/en/sub-agents)の説明による。実行していない）。
**`permissionMode` は評価していない。**取りうる値（`default`／`acceptEdits`／`auto`／`dontAsk`／
`bypassPermissions`／`plan`）は承認の振る舞いを変えるものであり、
**`plan` が書き込みをどこまで止めるかは確かめていない。**
**残る候補として記録する。**

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

### 選択肢E: `Bash`を外し、差分を file で渡す（2026-09-11 に追加・却下）

**上の選択肢C は「差分を`git`で取るため`Bash`を外せない」として却下した。その理由は、
agent 自身が取りに行く場合にだけ成立する。**呼び出す側が `git diff` の出力を file へ書いて渡せば、
`tools: Read, Grep, Glob` で足りる。**command を1つも実行できないため、字句判定が要らない。**

**却下する。根拠は1つである。検査 agent が `git` を失う。**
差分の取得・範囲の取り直し・履歴の参照が、すべて呼び出す側への依存になり、
**「意図を知らない読み手」が自分で確かめる余地が消える。**これは性質からの判断である。

**試行も行った。**`fresh-context-reviewer` から `Bash` と hook を外し、`50671d9` として merge される前の
未 commit 差分を file で渡して回した。その agent は `python3`・`check()` の直接呼び出し・
使い捨て repository の3つを「確かめられなかったこと」として挙げた。
**ただしこの3つは選択肢B でも実行できない**（`python3`・`mktemp` は`ALLOWED_PROGRAMS`に無く、
`git init`は`GIT_READONLY_SUBCOMMANDS`に無い）。**B と E を分ける点ではないため、却下の根拠にしていない。**
**試行の出力は残していない。**指摘件数で品質を比べる形の測定も行っていない。

### 選択肢F: `isolation: worktree`（2026-09-11 に追加・却下）

subagent を使い捨ての git worktree で動かす `isolation` field がある
（[subagent の公式文書](https://code.claude.com/docs/en/sub-agents)に記載。**文書で確認した。実行していない**）。
**却下する。**`git worktree` は commit 済みの ref から作業ツリーを作るため、
**未 commit の変更は入らない。実測した**（使い捨て repository で working tree の file を書き換えてから
`git worktree add wt HEAD` を実行し、**元の file は変更後、worktree 側の file は変更前**だった）。
**ただし、subagent の `isolation` が内部で `git worktree add` を使うと公式文書が書いているわけではない。**
`isolation` の実装は確かめていない。**この却下は、`git worktree` の挙動からの推論である。**
**自己レビューは未 commit の最終 diff に対して行う**（[CONTRIBUTING の「2つのPass」](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/CONTRIBUTING.md#2つのpass)が
「最終diffに対して行う」と定め、commit 前に宣言 trailer を書く運用になっている）。
**そのため、この用途では成立しない。**

## 脅威モデル

**この guard が防ぐのは「誤って書き込むこと」である。「書き込もうとする agent を止めること」ではない。**

- **防ぐ**: 検査 subagent が、read-only のつもりで書き込む command を実行してしまう。
  `git status`（index を書く）、`git log --show-signature`（`gpg` が `~/.gnupg` を作る）のように、
  **読み取りのつもりの command が書き込む**形が実際にあった（`検証`）。
  **ただし、この型のすべてを防げるわけではない。**
  **同じ書き込みは `log.showSignature`（git config）でも起きる。**真なら
  **option を1つも付けない `git log` が `gpg` を起動し、`~/.gnupg` を作る。**
  **argv に現れないため、この guard は止めない。**
  **誤操作の側に立っていて、かつ防げない例である。**
  `git status` の index 書き戻しも、測ったのは `status` だけで、他の subcommand は測っていない。
- **防がない**: subagent が意図して境界を越えようとする場合。
  **ただし「意図せず踏むが防げていない」ものもある。**pathname expansion（`git grep -n -[N-P]sha1sum`）は、
  cwd の file 名次第で展開される。**file 名は検査対象 branch が持つもので、subagent の統制下に無い。**
  **定義上は「防ぐ」側だが、防げていない。**欠点として記録する。
  **字句判定で bash の表面を覆い切ることはできない。**
  [#384](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/384)の自己レビューでは、
  塞ぐたびに別の切れ目が出た（`{`／`!`／`\r`／option の短縮綴り／short option の束ね）。
  **14 件塞いだが、網羅は主張しない。**
  **14 件を「防ぐ側」「防がない側」へ割り振らない。**両方にまたがる。
  `git grep --open-files-in-pag=sha1sum` や `rg <pattern> ! cat --pre sha1sum` は誤操作で書ける形ではないが、
  `git <subcommand> --help` は読み取りのつもりで打つ形であり、`\r` は CRLF 混じりの文字列を貼れば意図せず入る。
  **見つかった以上、塞ぐのが安いから塞いだ。それだけである。**
  **塞いだことをもって「意図的な越境を止められる」とは主張しない。**

## 保証が要るときにどうするか

**いま利用できる機構は無い。**選択肢を探した結果である。

- **sandbox（`sandbox.filesystem.denyWrite`）は subagent 単位に絞れない。**
  設定先は `settings.json`（user／project／managed）だけで、subagent の frontmatter に
  sandbox を絞る field は無い（[公式文書](https://code.claude.com/docs/en/sandboxing)と
  [subagent の公式文書](https://code.claude.com/docs/en/sub-agents)で 2026-09-11 に確認）。
  入れると**通常の作業 session の書き込みまで止まる**
- **`Bash` は外せない。**両 agent は `git show`／`git diff` で検査対象へ到達する
  （[#373](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/373)で確認）。
  外して差分を file で渡す形（`選択肢E`）も検討したが、**検査 agent が `git` を失うため
  review の品質が落ちる**（性質からの判断である。試行も行ったが却下の根拠にしていない。`選択肢E`）
- **`isolation: worktree` は未 commit の変更を含まない**（実測）。自己レビューは
  未 commit の最終 diff に対して行うため成立しない

**ただし `permissionMode` は評価していない。**`plan` が書き込みをどこまで止めるかを確かめていない
（`選択肢A` の節）。**これは外部の変化を待つ話ではなく、いま調べれば分かることである。**
**残る候補はこれ1つである。**

**それ以外は、前提が変わるのを待つことになる。**見直す契機は下の`検証`の見直し条件が持つ。

**この区別を決めたのは 2026-09-11 である。**それ以前は「read-only を機構で保証する」と書いていた。
**保証は達成していない。**

## 決定

**選択肢Bを採る。sandbox は採らない。**
**この決定が与えるのは best effort の門であって、read-only の保証ではない**（上の`脅威モデル`）。

1. **`sandbox`設定を導入しない。**理由は「subagent 単位に絞れず、通常の作業 session を止めるため」である。
   **sandbox 自体を否定する決定ではない。**別の目的（外部 command の network 隔離など）で採る余地は残す。
2. **`scripts/hooks/inspector_readonly_guard.py`を新設する。**agent frontmatter の
   `hooks.PreToolUse`（`matcher: Bash`）から呼ぶ。**`.claude/settings.json`へは置かない。**
3. **判定は allowlist とする。**
   - command位置の program は`ALLOWED_PROGRAMS`のみ。**option を含めても外部 command を
     起動せず file を書かないものだけを入れる**（`sort -o`、`uniq out`、`sed -i`、`tee`、
     interpreter を除外した）。**当初は「単体で file を書けない」を基準にしていたが、
     それでは足りなかった**（[#384](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/384)。下の`欠点`）
   - **program は名前で照合する。**`/`を含む語を拒否する（`./git`、`/tmp/cat`）
   - `git`の subcommand は`GIT_READONLY_SUBCOMMANDS`のみ
     （`branch`／`tag`／`config`は flag 次第で壊せるため入れない）。
     **ただし「flag に関係なく安全」ではない。**`diff`／`show`／`log`／`blame`は
     `--no-ext-diff --no-textconv`の両方を、`status`は global の`--no-optional-locks`を要求する
   - **許可した program 自身の危険な option を個別に拒否する。**
     git の`-c`／`--config-env`／`--exec-path`／`-O`／`--output`／`--ext-diff`／
     `--textconv`／`--filters`、`rg`の`--pre`／`--hostname-bin`／`-z`／`--search-zip`
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
   - **`git` の global option も allowlist とする**（2026-09-11 に追加）。許すのは
     `-C`／`--git-dir`／`--work-tree`／`--namespace`（値を取る）と `--no-pager`／
     `--no-optional-locks`／`--no-replace-objects`／`--bare`／`--literal-pathspecs`だけである。
     **列挙に無いものは拒否する。**値を取るかどうかが分からず、読み飛ばし方を決められない。
     **許した理由は「読み飛ばし方が決まる」ことであって、「安全と判断した」ことではない。**
     `-C`／`--git-dir`／`--work-tree`／`--namespace`は**別の repository を指させる。**
     **指した先の config も差し替わる。**この guard は config 経由の経路を
     `-c`／`--config-env`の拒否と、helper 系4 subcommand の必須 flag で塞いでいるが、
     **それ以外の config 項目（`core.fsmonitor`など、外部 program を起動しうるもの）は見ていない。**
     **別 repository を指した場合の挙動は測っていない。**欠点として記録する。
     `git --super-prefix rev-parse submodule--helper x`では、
     **guard が `rev-parse` を、git が `submodule--helper` を subcommand として読む**
   - **必須 flag は subcommand の直後2語に要求する**（2026-09-11 に追加。順序は問わない）。
     「語として在るか」だけを見る形は、`--`より後ろ（pathspec）と、
     `-S`のように次の語を値として飲む option の後ろで抜けられる。
     **この規則が担うのは「前に何も置けない」ことだけである。**
     後ろへ`--ext-diff`／`--textconv`を置いて打ち消す形は、
     **拒否 option の一覧（短縮綴りを含む）が止めている。**片方だけでは閉じない
   - **必須 global option は、値として消費される位置では満たされない**（2026-09-11 に追加）。
     `git --namespace --no-optional-locks status`は、`--no-optional-locks`が namespace の値になる
   - **`--help` を位置に依らず拒否する**（2026-09-11 に追加）。
     `git <subcommand> --help`は`git help <subcommand>`へ書き換わり、外部 viewer（`man`）を起動する。
     **短縮綴りは対象にしていない。**`git help`への書き換えは完全一致でしか起きず、
     `git version --hel`／`git grep --hel` は git 自身が`unknown option`で落とす（実測）
   - **拒否する long option は短縮綴りも拒否する**（2026-09-11 に追加）。
     parse-options を通す subcommand は短縮を受理する。対象は`--textconv`／`--ext-diff`／
     `--open-files-in-pager`／`--output`／`--show-signature`（全 subcommand）と、
     `--filters`（`cat-file`のみ）、`--verbose`／`-v`（`status`のみ）。
     **短縮が実際に実行されることを実測したのは、`--textconv`（`cat-file`／`grep`）、
     `--filters`（`cat-file`）、`--open-files-in-pager`（`grep`）、`--verbose`（`status`）である。**
     `git status --verb`／`--ver`／`--v` はいずれも git が受理する（実測）。`--ext-diff`と`--output`は diff 系にしか無く、
     diff 系の parser は短縮を受け付けない（実測）。**この2つは予防として同じ扱いにしている。**
     **`--filters`の短縮拒否を限定するのは、`--filter`が`rev-list`の正当な option だからである**
     （`git log --filter=...`は git 2.34.1 では`unrecognized argument`になる。実測）。
     **`--text`が`--textconv`の前方一致で落ちる巻き添えを1つ引き受けている**
   - **単独の区切り語も拒否する**（2026-09-11 に変更）。初版は`command_line.SEPARATORS`として
     単独で現れた語を metacharacter 検査から除外していた。**`shlex`は引用符を剥いだ後の語を返すため、
     bash が literal な引数として渡す語と区別が付かない。**
     `rg <pattern> { cat --pre <任意の command> <file>`では、`command_line.invocations`が`{`で
     invocation を切り、**その先が無検査になった。実測で`/usr/bin/uname`が起動した。**
     **metacharacter 検査だけでは足りなかった。**`!`は`SEPARATORS`にあるが
     `SHELL_METACHARACTERS`の文字を1つも含まず、`rg <pattern> ! cat --pre sha1sum <file>`で
     **`sha1sum`が実際に起動した**（実測）。**`command_line.SEPARATORS`の語を直接拒否する形にした。**
     共有 module へ区切りが増えても穴にならない。
     **pipe が使えなくなる。**判定が`command_line`の語り分けに依存しなくなることを採った
   - **署名検証を拒否する**（2026-09-11 に追加）。`--show-signature`（短縮綴りを含む）と、
     **pretty format の `%G*` を含む語**を拒否する。どちらも `gpg.program`（既定 `gpg`）を起動する。
     **実測では、検査 subagent の中で `gpg` が `~/.gnupg` に directory と keybox file を作った。**
     `--show-signature` だけでは閉じない（`--format=%GK` でも走る）
   - **`git status` の verbose を拒否する**（2026-09-11 に追加）。`git status -vv` は
     working tree の patch を出す過程で **textconv を走らせる**（実測）。
     **`status` は `--no-ext-diff --no-textconv` を受理しない**（実測）ため、打ち消す形が無い。
     **verbose そのものを拒否する。**`-v`／`-vv`／`--verbose`／その短縮／`-sv` のような束ねが対象である
   - **short option は束ねを見る**（2026-09-11 に追加）。`git grep -nOzzz`は`-n -O zzz`であり、
     `token.startswith("-O")`では見えない。**cluster のどこかに文字があれば拒否する。**
     **過剰に拒否する。**`git log -GFOO`のように値へ同じ文字が入る形も落ちる
   - **`version` を`GIT_READONLY_SUBCOMMANDS`へ足した**（2026-09-11）。
     この判定はすべて git の version 依存の観測であり、**検査 session 側で前提を記録できるようにする。**
     `git --version`は**許可していない global option として拒否される**（`--version`は allowlist に無い）
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

- **書き込み経路に機構の門が付いた。**（**「閉じた」とは書かない。**下の`欠点`のとおり best effort である）`Edit`／`Write`を持たない agent にとって、`Bash`が唯一の経路であり、そこに allowlist が掛かる。
- **通常の作業 session に影響しない。**hook は agent frontmatter にあり、`.claude/settings.json`には無い。**test でそれを固定した。**
- **allowlist の中身が test で固定された。**`sort`や`tee`を後から足すと test が落ちる。足すならこの ADR を更新することになる。
- **ADR-0018 の欠点へ、機構の門を1つ足した**（**「閉じた」とは書かない**）。同 ADR の決定4とリスク表も、この修正で「閉じた」から改めた。

### 欠点

- **閉じたと言えるのは1つの型だけである。**
  [#384](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/384)の自己レビュー 17〜20巡で出た
  **「guard が command を切る位置と、bash が切る位置がずれる」型**は、根元で閉じた
  （**区切り語**と **metacharacter を含む語**を拒否し、**CR を tokenize 前に拒否**して、
  **1 行 1 command に固定した**）。
  **`command_line` への依存が消えたわけではない。**program allowlist は `command_starts`、
  git と `rg` の option 検査は `invocations` を通る。
  **それらが呼び出しを拾い損ねれば、option 検査は一度も走らない。**
  閉じたのは「**区切りが増えても穴にならない**」という範囲だけである。
  **他の型が無いことは示していない。**穴が出なくなったことを根拠にしていない。
- **この guard は保証ではない。best effort である**（2026-09-11 に位置づけを改めた。
  **題も「保証する」から「絞る」へ改めた**）。
  **`read-only を機構で保証する`とは書けない。**[#384](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/384)と
  その自己レビューで、**guard を通るのに保護が git へ届かない形が繰り返し見つかった。**
  塞ぐたびに別の形が出た。字句判定で `git` の option 文法を追い続ける構造であり、
  **網羅は主張できない。**
  **件数の境界**: [#384](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/384)の
  起票分 5 件は `50671d9`（[PR #386](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/386)）で
  merge 済みである。**下の 14 件はそこに含まれない。**`50671d9` を開いても見つからない。**その後の自己レビューで 14 件出た**（下の`検証`。**測定表は 16 行で穴は 12 件、13 件目（short option の束ね）と 14 件目（`\r`）は表の後ろに書いてある。**行と件数の対応は同節に書いた）。
- **それでも選択肢Bを維持する。**選択肢A（sandbox）は subagent 単位に絞れず通常セッションを止める。
  選択肢E（`Bash`を外す）は**検査の品質を落とす**（上の`検討した選択肢`）。
  **調べた範囲（A・E・F と、公式文書に載る subagent frontmatter の制限系 field）では、
  「品質を保ったまま read-only を per-subagent で強制する」方法は見つからなかった。**
  **ただし F の却下は`git worktree`の挙動からの推論であり、`isolation`の実装は確かめていない。
  `permissionMode` は評価していない**（上の`選択肢A`の節）。
  **網羅は主張しない。**E の「品質が落ちる」は、**検査 agent が`git`を失うという性質から
  導いた判断**であり、指摘件数で測ったものではない（上の`選択肢E`）。
  トレードオフを引き受け、**保証ではないと明記したうえで**使う。
- **OS の強制ではない。**字句判定であり、`command_line.py`が取れないもの（alias、shell function、変数展開、`sh -c`の内側）は取れない。
  **ただし`sh`・`bash`・`xargs`・interpreter は allowlist に無く、command位置に現れれば拒否される。**
- **hook が実際に掛かることを、この決定では実測していない。**agent frontmatter の`hooks`は
  [公式文書](https://code.claude.com/docs/en/sub-agents)に記載があるが、**subagent を起動して
  書き込みが実際に拒否されるところまでは測っていない。**測ったのは hook script 単体の判定である
  （`check()`への 78 例（`ALLOWED` 19 と `DENIED` 59）に加え、個別 test の例、wrapper の 4 条件、hook として起動したときの`deny` payload）。
- **config 経由の経路を全数は見ていない。**塞いだのは`-c`／`--config-env`／`--exec-path`と、
  `diff.external`／textconv／filter driver である。**`core.fsmonitor`のように、
  index の refresh 時に外部 program を起動しうる config 項目は見ていない。**
  **`gpg.program`は 2026-09-11 に、option と format の側だけ塞いだ**（`--show-signature`と`%G*`）。
  **config で暗黙に走る側は塞いでいない。**`log.showSignature`が真なら、
  `git log`は option 無しで署名検証を走らせる（git 2.34.1 の binary に key の文字列が実在することを確認。
  **config を設定して起動させるところは測っていない**）。
  **`rg`も同じである。**`RIPGREP_CONFIG_PATH`が指す file に`--pre=<command>`があれば、
  `rg <pattern> <file>`だけで前処理 command が走る。
  **どちらも argv に現れないため、この guard では原理的に見えない。**
  **暗黙に走るものと、option／format で明示的に起動するものの両方がある。**
  `-C`／`--git-dir`／`--work-tree`／`--namespace`は別 repository を指させ、**その config も差し替わる。**
  **別 repository を指した場合の挙動は測っていない。**
- **誤検知がある。**`grep "=>"`のように、metacharacter を含む正当な引数を拒否する。
  **2026-09-11 に2種類増えた。**(1) `--text`は`--textconv`の前方一致で落ちる
  （`git diff`／`grep`／`log` のいずれでも）。(2) **short option の束ね判定**により、
  `git log -GFOO`のように値へ`O`が入る pickaxe 検索が落ちる。
  **どちらも代替がある**ため引き受けた。(1) は binary を text として見る必要が無い。
  (2) は**値を別の語にすれば通る**（`git log -GFOO` は落ちるが `git log ... -G FOO` は通る。実測）。
  `rg` 側は `Grep` tool を使う。
  **代替がある**（両 agent は`Grep` tool を持つ）ため引き受けた。
- **`ALLOWED_PROGRAMS`は、載せた program が option を含めても安全だという判断に依存する。**
  判断が誤っていれば穴になる。`sort -o`と`uniq out`は実際に見落としやすく、
  **除外した理由を script の comment へ書いた。**
- **当初の基準「単体では file を書けない」は誤りだった。**[#384](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/384)で
  **5 箇所の穴**が見つかった（[PR #383](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/383)の手動 review）。
  いずれも「program 名は allowlist にあるが、option または path が実体を変える」形である。
  **基準を「option を含めても外部 command を起動せず file を書かない」へ引き上げ、
  option 側の拒否と対で使う形にした。**穴の内訳は次のとおりである。

  | 穴 | 実体 |
  |---|---|
  | `git diff --ext-diff`／`show --textconv`／`grep --textconv`／`cat-file --filters` | git config の外部 helper を起動する |
  | **`git diff`／`show`／`log`／`blame` は flag 無しでも helper が走る** | `--no-ext-diff --no-textconv` の両方を要求する。**`--no-ext-diff` は textconv を止めない**（実測） |
  | `rg --pre COMMAND`／`rg --hostname-bin COMMAND` | `--pre`は検索対象ごとに、`--hostname-bin`は hostname を得るために `COMMAND` を起動する |
  | `git status` | 既定で index を refresh し `.git/index` を書く |
  | `./git`／`/tmp/cat` | **basename で照合していた。**名前が一致するだけの別の実行 file が通った |

- **program 名の allowlist だけでは read-only を保証できない。**上の 5 件はすべて
  「名前は allowlist にある」ものだった。**allowlist へ program を足すときは、
  その program の option を全数見る必要がある。**この作業は機械化していない。
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
| 字句判定を抜ける形が見つかる | **初版は「allowlist であり、抜けるには許可した program 自身の書き込み経路を使うことになる」と書いていた。2026-09-11 にこの説明が実例と合わなくなった。**同日に塞いだ 14 件のうち **11 件は program の書き込み経路ではなく、guard の parse と git の parse がずれる形**である（**内訳は`検証`が持つ。ここへ写さない**）。**残る 3 件（`status -vv`の textconv、`--show-signature`、pretty format の`%G*`）は読みのずれではない。**guard も git も subcommand を同じに読んでおり、**その option が helper を起動することを拒否一覧へ入れていなかった**という、旧来の型である。**`%G*`は option ですらなく、format 文字列の中身である。****手順は3つに分かれる。**program 自身の書き込み経路なら`ALLOWED_PROGRAMS`から外す。**parse のずれなら、ずれる読み方そのものを直す。**helper を起動する option の見落としなら、拒否一覧へ足す（`status -vv`がこれ）。[#384](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/384)起票分の 5 件は option 側の拒否で塞ぎ、**`rg`は allowlist に残したまま`DENIED_RG_OPTIONS`で扱っている**（外していない）。**`rg --help` の option 一覧を見て拾った**（`--pre`／`--hostname-bin`／`-z`／`--search-zip`）。**`--pre`／`--hostname-bin` は help の表記（`=COMMAND`）が command 実行を含意する。`-z`／`--search-zip` は含意しない。外部 process の起動は測っておらず、ripgrep の文書化された挙動に依る。****`--pre-glob` は `--pre` が無ければ効かない。**全数を目で見ただけであり、機械で照合していない |
| 許可 program の危険な option を見落とす | **`DENIED_GIT_GLOBAL_OPTIONS`／`DENIED_GIT_SUBCOMMAND_OPTIONS`／`DENIED_GIT_OPTIONS_WITH_ABBREVIATION`／`DENIED_GIT_OPTIONS_BY_SUBCOMMAND`／`DENIED_GIT_HELP_OPTIONS`／`DENIED_RG_OPTIONS`／`REQUIRED_GIT_HELPER_OPTIONS`／`GIT_ALLOWED_GLOBAL_OPTIONS_WITHOUT_VALUE`／`GIT_GLOBAL_OPTIONS_WITH_VALUE`が持つ。**当たり方は`_matches_option`（完全一致・`=`付き・short optionの束ね）、`_matches_option_or_abbreviation`（前方一致）、`_global_option_names`（値消費）が決める。****網羅は保証していない。**program を足すときに option を全数見る手順は機械化していない |
| 要求 option（`--no-ext-diff --no-textconv`等）を agent が知らず、検査が止まる | **両 agent 本文へ`Bash の制約`節を足した。**拒否理由にも正しい形を書いてある |
| `.claude/settings.json`へ間違って置かれる | `test_the_guard_is_not_wired_globally`が固定する |
| 片方の agent にだけ書き忘れる | `test_both_inspector_agents_wire_the_guard`が固定する |

## 検証

- `python3 scripts/test_hooks.py` — **176件 OK**（`InspectorReadonlyGuardTests` 35件を含む）。**#384 の merge 後（`50671d9`）は 162件・21件だった**
- allowlist 側 19 例が通り、拒否側 59 例が落ちることを`check()`で確認した
- **[#384](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/384)の 5 件を、使い捨て repository で実測してから塞いだ。**
  helper script に `echo ... > PWNED` を書かせ、**file が実際に作られたことで判定した。**
  `diff.external` は `git diff` だけが flag 無しで起動し、textconv は
  `diff`／`show`／`log`／`blame` の 4 つが flag 無しで起動する。
  4 つとも `--no-ext-diff --no-textconv` を受け付けることも確認した
- **wrapper を 4 条件で実測した。**guard が無い／`python3`が無い → **exit 2**、
  読み取り command → exit 0 かつ stdout 空、書き込み command → exit 0 かつ`deny` payload
- hook として起動し、`rm -rf /`に対し`permissionDecision: deny`が出ることを確認した
- **`git show --no-ext-diff --no-textconv HEAD`に対し stdout が空（素通り）であることを確認した。**
  **flag を欠いた`git show HEAD`は拒否される**（[#384](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/384)の対処後）。
  **この行は当初「`git show HEAD`が素通りする」と書いていた。対処後は誤りである。**
- **2026-09-11 の測定**（git `2.34.1`、ripgrep `14.1.1`。使い捨て repository は `mktemp -d` + `git init`）。
  **大半は通常の作業 session で実行した。****ただし表中で「検査 subagent の中で実行した」と書いた 2 行**（`rg <pattern> { cat --pre /usr/bin/uname`／`git log --show-signature`）**だけは、guard が掛かった検査 subagent の中で起きたものである。****guard を通ったうえで起動した**という意味で、条件が違う。
  **判定方法は形ごとに違う。表の「判定」列に書く。**

  | 形 | 結果 | 判定 |
  |---|---|---|
  | `git <subcommand> --help` | `git help <subcommand>` へ書き換わり、**man が出力された** | 出力に man ページが出たこと |
  | `git cat-file --te`／`--textc`／`--textcon` | **`--textconv` として実行された** | textconv driver に `echo TEXTCONV_RAN` する script を設定し、**3形とも出力が `TEXTCONV_RAN`、対照の `-p` は file の中身（`secret`）になったこと** |
  | `git cat-file --filt` | **`--filters` として実行された** | filter driver（小文字を大文字にする script）を設定し、**`--filt` の出力が `--filters` と同じ `A`、`-p` は `a` になったこと** |
  | `git grep --textc` | **`--textconv` として実行された** | 同じ textconv driver で、**`--textc` が `--textconv` と同じく `f:TEXTCONV_RAN` を出し、flag 無しでは出力が無かったこと** |
  | `git grep --open-files-in-pag=sha1sum` | **`sha1sum` が実行された**（`ALLOWED_PROGRAMS` に無い） | 出力が `sha1sum` の出力（hash 値＋file 名）になったこと |
  | `git --namespace --no-optional-locks status` | `--no-optional-locks` が namespace の値になり、status へ届かない | **`git --namespace --no-optional-locks status` が正常終了し、値を欠いた `git --namespace status` はエラーになったこと**（＝次の語を値として飲んでいる）。**git 側の index 書き戻しは測っていない** |
  | `git log -S --no-ext-diff --no-textconv -p` | `-S` が `--no-ext-diff` を検索文字列として飲む | 出力が 0 行になったこと（同じ範囲で flag を先頭へ置くと commit が出る） |
  | `git diff -- f --no-ext-diff --no-textconv` | `--` より後ろは pathspec | **helper script が `PWNED` file を実際に作ったこと** |
  | `git --super-prefix rev-parse submodule--helper x` | guard は `rev-parse`、git は `submodule--helper` を読む | git のエラー文が `'x' is not a valid submodule--helper subcommand` になったこと |
  | `git diff --ext-dif`／`--ext`／`--e` | **git 自身が拒否した**（diff 系 parser は短縮を受けない） | git が `unknown option` を返したこと |
  | `rg <pattern> ! cat --pre sha1sum <file>` | **`sha1sum` が起動した** | 出力が README の本文ではなく `sha1sum` の hash 値になったこと。**`!` は metacharacter を1つも含まないため、`{` を塞いだ後も残っていた** |
  | `rg <pattern> { cat --pre /usr/bin/uname <file>` | **`/usr/bin/uname` が起動した**（`ALLOWED_PROGRAMS` に無い） | ripgrep が `preprocessor command failed: '"/usr/bin/uname" ...'` を出したこと。**検査 subagent の中で実行した** |
  | `git log --show-signature` | **`gpg` が起動し、`~/.gnupg` に directory と keybox file を作った** | `gpg: directory '<HOME>/.gnupg' created` の出力と、**検査 subagent の中で実行したこと** |
  | `git log -1 --format=%GK` | **署名検証が走った** | 鍵 ID（`B5690EEE…`）が出力されたこと。**`--show-signature` を拒否するだけでは閉じない** |
  | `git --no-optional-locks status -vv` | **textconv が走った** | helper script が `PWNED` file を作ったこと。`-v` では作られず、`diff.external` は `-vv` でも作られなかった |
  | `git --no-optional-locks status -v --no-ext-diff --no-textconv` | **git が拒否した** | `status` が両 flag を受理しないこと |

  **`--no-optional-locks` を `status` にだけ要求している理由を書いておく。**
  `.git/index` の書き戻しを測ったのは `status` だけである。
  `diff`／`log`／`grep`／`ls-files`／`blame`／`show`／`rev-parse`／`merge-base`／`ls-tree` でも
  mtime が変わらないことは [#384](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/384) の
  時点で測ったが、**stat-dirty な状態を作って測り直してはいない。**
  **測っていない subcommand が他にもある**（`cat-file`／`rev-list`／`describe`／`shortlog`／
  `show-ref`／`name-rev`／`version` の 7 個。許可している 17 個のうち測ったのは 10 個である）。
  **「status 以外は書かない」と言い切れる根拠は持っていない。**

  **表は 16 行ある。穴は 12 件である。**対応は次のとおり。
  `--help` 1 件、**`cat-file` の短縮綴り 1 件**（`--te`／`--textc`／`--textcon` と `--filt` の 2 行）、
  **`grep` の短縮綴り 1 件**（`--textc` と `--open-files-in-pag=` の 2 行）、
  `--namespace` の値食い 1 件、`-S` の値食い 1 件、`--` 以降 1 件、`--super-prefix` 1 件、
  **`status -vv` の textconv 1 件**、**署名検証 2 件**（`--show-signature` と `%G*`）、**区切り語に見える引数 2 件**（`{` と `!`）。
  **`--ext-dif` と `status -v --no-ext-diff --no-textconv` の 2 行は穴ではなく、
  それぞれ短縮が効かないこと・`status` が flag を受理しないことの確認である**
  （16 行 − 2 行の束ね − 2 行の確認 = 12 件）。
  **この表の後に、13 件目として short option の束ね（`git grep -nOzzz`）を見つけて塞いだ。**
  `-O`を`token.startswith("-O")`だけで見ていたため、`-nOzzz`が素通りし、
  **git が pager として `zzz` を exec しようとした**（`error: cannot run zzz...` で判定）。
  **14 件目は `\r` である。**`LINE_SPLIT_RE` を `[\r\n]+` から `\n+` へ変え、
  tokenize 前に CR を拒否するようにした。**表に行が無いのは、実測していないためである**（`Bash` tool へ生の CR を送ると transport が LF へ正規化する）。
- 見直し条件: **subagent を実際に起動して書き込みが拒否されることを測れたとき**、
  この ADR の「実測していない」を実測結果へ差し替える。
- **sandbox の再検討条件を書き換えた**（2026-09-11）。**上の bullet の「実測できたら差し替える」は有効である。
  書き換えたのは、そこに続いていた「選択肢A を再検討する条件」の1文だけである。**
  **書き換え前の条件は「測れないまま allowlist を広げる変更を重ねる場合は、選択肢A を再検討する」だった。**
  **この修正は「測れないまま広げる」に当たる。**`version`を`GIT_READONLY_SUBCOMMANDS`へ足しており、
  「hook が実際に掛かることを実測していない」も解消していない。
  **「重ねる」に達したかどうかは判断していない。**広げたのは 1 件である。
  **それでも再検討したのは、穴が繰り返し出たためである。**
  **条件どおり再検討し、A・E・F をすべて却下した**（上の`検討した選択肢`）。
  **もう「穴が出たこと」は見直しの引き金にならない。**guard は best effort であり、
  穴が見つかるのは想定内である。**次に見直すのは、前提が変わったときである。**
  - subagent の frontmatter に、sandbox または filesystem 制限を絞る field が追加されたとき
  - `isolation` が未 commit の変更を含められるようになったとき。
    **現状は`git worktree`の挙動からの推論であり、観測手段が無い。**
    確認するなら、`isolation: worktree` の subagent を起動して、
    **未 commit の変更が見えるかを実際に測る**（この ADR では行っていない）
  - 検査 agent が `git` を必要としなくなったとき

  **どれも外部の変化であり、自分の commit からは観測できない。**
  **契機を1つ決めておく。**この guard を触る変更（`ALLOWED_PROGRAMS`／`GIT_READONLY_SUBCOMMANDS`／
  拒否 option の一覧を変える変更）を行うとき、**上の3条件を1つずつ確認してから着手する。**
  確認した結果はこの ADR の`検証`へ追記する。

## 置き換える決定

なし。[ADR-0018](0018-instruction-file-structure.md)の決定4が残した欠点へ機構の門を足す。**閉じてはいない**（`欠点`を参照）。同決定の`tools`と`model`は変えない。
