# GitHub Wiki入口の保守

> 状態: Verified — 2026-07-28に初回更新と公開結果を確認済み
> 対象Issue: [#27](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/27)
> 方針: [ADR-0003](../decisions/0003-public-documentation-publishing.md)

## 目的

GitHub Wikiを、DeskCatの公開文書へ案内する日本語の入口ページとして保守する。
技術情報はmain repositoryでreviewし、Wikiを独立した仕様保管場所にしない。

## 管理境界

- 技術情報の正本はmain repositoryのroot Markdownと`docs/`である。
- GitHub Pagesは正本から生成する閲覧用siteである。
- Wikiは別のGit repositoryであり、`Home.md` 1件だけを案内用に保持する。
- 技術仕様、設計値、ADR、runbook、進捗、Issue checklist、release noteをWikiへ複製しない。
- Wikiへ長文が必要になった場合は、先にmain repositoryの正本へ追加し、Wikiからlinkする。

| 対象 | 値 |
|---|---|
| Wiki URL | `https://github.com/wachi-yoshitaka-11-dev/deskcat/wiki` |
| Git remote | `https://github.com/wachi-yoshitaka-11-dev/deskcat.wiki.git` |
| Branch | `master` |
| 公開page | `Home.md` |

## 更新手順

1. Wiki変更用Issueを用意し、一つの目的に限定する。
2. main repositoryとWiki repositoryの未commit変更、remote、branchを確認する。
3. Wiki repositoryを一時directoryへcloneする。
4. cloneした作業copyへ、公開用のcommit identityを設定する（下記「Commit identity」を参照）。
5. `Home.md`だけを編集し、仕様本文やlive statusを追加していないことをreviewする。
6. `Home.md`だけをstageし、staged blobを検査用fileへ固定する。
7. **そのstaged blob**に含まれるlinkを下記allowlistと照合し、許可したURLだけが最終的にHTTP 200を返すことを確認する。
8. **同じstaged blob**にSecret様pattern、資格情報、個人path、local専用資料がないことを確認する。
9. 検査後にindexのblobが変わっていないこと、`user.name`／`user.email`が公開用identityであることを確認してcommitする。実際の`HEAD:Home.md`、author、committerはcommit後・push前に再検証する。
10. ユーザーからpushの明示的な指示を受けた後に、Wikiの`master`へpushする。
11. Rendered Wikiとraw Markdownをread-backし、日本語本文、正本方針、linkを確認する。
12. Wikiのlocal／remote SHAが一致することを確認し、自分が作成した一時cloneと状態保存directoryの
    絶対pathを再確認してから安全に削除する。検査が失敗した場合も、`Home.staged.md`を残さない。

Wikiはmain repositoryのbranch protectionやPages workflowとは別に更新される。
そのため、push前のdiff確認とpush後のread-backを省略しない。

## Link destination allowlist

[ADR-0003](../decisions/0003-public-documentation-publishing.md)が定めるWikiの責務を、
次のURL allowlistとして適用する。host名の部分一致や文字列prefixだけで判定しない。
URI parserでscheme、userinfo、port、正規化後のhostとpathを個別に比較する。

| 公開先 | 許可する最終URL |
|---|---|
| Wiki | `https://github.com/wachi-yoshitaka-11-dev/deskcat/wiki`、`https://github.com/wachi-yoshitaka-11-dev/deskcat/wiki/Home` |
| Pages／文書index | `https://wachi-yoshitaka-11-dev.github.io/deskcat/`およびその配下 |
| Repository README | `https://github.com/wachi-yoshitaka-11-dev/deskcat`、`https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/README.md` |
| Issues | `https://github.com/wachi-yoshitaka-11-dev/deskcat/issues`および`/issues/<正の10進整数>` |

検査は次の順で行う。

1. Inline link、reference linkとその定義、autolink、image、HTMLの`href`／`src`をすべて
   抽出する。code span／code fence内は対象外とする。解釈できないlink構文を黙って
   skipせず検査失敗にする。相対targetはWiki HomeのURLに対して解決する。fragmentは
   request URIから分離して保持し、allowlist判定用にはfragmentを除いたURIをparseする。
2. `https`以外、userinfoを持つURI、明示portまたはqueryを持つURI、allowlist外host、
   正規化後にallowlist外となるpath、percent-encoded separatorまたはpath traversalを拒否する。
3. redirectを自動追従しないrequestを送る。3xxの場合は`Location`を現在のURLに対して
   解決し、**次のrequestを送る前に**手順2を再適用する。allowlist外へのredirectは
   requestせず失敗とする。redirectは5回までとし、循環も失敗とする。
4. 最終URLがallowlist内であることをもう一度確認し、最終responseがHTTP 200であることを
   確認する。redirect先がfragmentを明示した場合はその値へ更新し、省略した場合は元の
   fragmentを保持する。
5. fragmentがある場合はpercent-encodingを1回decodeし、不正encodingを拒否する。最終responseを
   content typeに対応するparserで読み、HTMLでは実際の`id`属性、Markdownではheadingから
   生成されるanchorに完全一致することを確認する。対象が無い場合、本文をparseできない場合、
   または検査対象外のcontent typeの場合は失敗とする。

HTTP 200だけでは成功にしない。未承認hostが200を返す場合や、許可URLから未承認hostへ
redirectする場合も公開境界違反である。検査対象は後述の`Home.staged.md`とし、作業treeの
`Home.md`を別に検査して済ませない。

2026-08-02のread-only確認では、現行`Home.md`にallowlist外の`CONTRIBUTING.md`への
直接linkが1件残っている。このrunbook変更はWikiへの書き込みを許可しないため、外部状態は
変更していない。次の承認済みWiki更新では、このlinkを削除するか、PagesまたはREADMEの
許可済み導線へ置き換え、allowlist検査が成功するまでpushしない。

## Commit identity

Wikiは独立したrepositoryであり、main repositoryの`--local`設定を引き継がない。
一時cloneはglobal設定のまま commit されるため、意図しないaddressが公開履歴へ残る。

clone直後に必ず設定する。以下のblockでは、一時root、clone先、状態保存先を環境変数で渡す。
各blockを実行するshellで3つすべてを設定し、同じ値を使う。未設定なら`${parameter:?word}`で
処理を停止する。山括弧の置換用文字列をshell commandへ直接書かない。

```bash
set -euo pipefail

wiki_tmp_root=$(mktemp -d "${TMPDIR:-/tmp}/deskcat-wiki-update.XXXXXX")
chmod 700 "$wiki_tmp_root"
printf '%s\n' 'deskcat-wiki-update-root-v1' > "$wiki_tmp_root/.deskcat-generated-root"
chmod 600 "$wiki_tmp_root/.deskcat-generated-root"
export DESKCAT_WIKI_TMP_ROOT="$wiki_tmp_root"
export DESKCAT_WIKI_CLONE_DIR="$wiki_tmp_root/wiki-clone"
export DESKCAT_WIKI_STATE_DIR="$wiki_tmp_root/state"
mkdir -- "$DESKCAT_WIKI_STATE_DIR"
git clone --branch master --single-branch \
  https://github.com/wachi-yoshitaka-11-dev/deskcat.wiki.git \
  "$DESKCAT_WIKI_CLONE_DIR"
```

`mktemp`が返したrootとその直下の2 pathを記録する。repository、home directory、既存の
作業directoryへ置き換えない。shellを分ける場合は、この3 pathを公開ログへ出さず安全に引き継ぐ。

```bash
set -euo pipefail

clone_dir=${DESKCAT_WIKI_CLONE_DIR:?Set DESKCAT_WIKI_CLONE_DIR}
git -C "$clone_dir" config user.name "wachi-yoshitaka-11-dev"
git -C "$clone_dir" config user.email "219948400+wachi-yoshitaka-11-dev@users.noreply.github.com"
```

確認は2段階で行う。`git log`は既存のcommitしか見られないため、
commit前の確認だけでは、これから作るcommitのidentityを保証できない。

`git log -1`はcommitが失敗しても既存のHEADを返す。古いcommitのidentityを
新規commitの確認結果として読み違えないよう、SHAが変わったことを先に確かめる。

commit前とcommit後は**別の実行段階**である。1 blockにまとめて実行すると、
その間にcommitが無いためSHAが変わらず必ず失敗する。SHAは`$state_dir`へ保存し、
shellが変わっても引き継げるようにする。

`$state_dir`は状態保存用の空directoryであり、`$clone_dir`とは別のpathにする。
同じpathを使うと、`git clone`が非空directoryを拒否して失敗する。

**`$state_dir`は最初に作る。**作らずに進むと、SHAを保存する`>`の
redirectがdirectory不在で失敗し、後続の比較対象が残らない。

```bash
set -euo pipefail

tmp_root=${DESKCAT_WIKI_TMP_ROOT:?Set DESKCAT_WIKI_TMP_ROOT}
clone_dir=${DESKCAT_WIKI_CLONE_DIR:?Set DESKCAT_WIKI_CLONE_DIR}
state_dir=${DESKCAT_WIKI_STATE_DIR:?Set DESKCAT_WIKI_STATE_DIR}
case "$tmp_root" in /*) ;; *) exit 1 ;; esac
case "$clone_dir" in /*) ;; *) exit 1 ;; esac
case "$state_dir" in /*) ;; *) exit 1 ;; esac
test "$clone_dir" = "$tmp_root/wiki-clone"
test "$state_dir" = "$tmp_root/state"
test "$clone_dir" != "$state_dir"
# `mktemp`で作ったrootと、その直下へ通常directoryとして作った2 pathだけを使う。
# `mkdir -p`は既存symlinkもdirectoryとして受理するため、ここでは再作成しない。
test ! -L "$tmp_root"
test ! -L "$clone_dir"
test ! -L "$state_dir"
test -d "$tmp_root"
test -d "$clone_dir"
test -d "$state_dir"
tmp_real=$(cd "$tmp_root" && pwd -P)
clone_real=$(cd "$clone_dir" && pwd -P)
state_real=$(cd "$state_dir" && pwd -P)
test "$clone_real" = "$tmp_real/wiki-clone"
test "$state_real" = "$tmp_real/state"
test "$clone_real" != "$state_real"
# 一方を他方の配下に置かない。単なるpath不一致だけでは、後のcleanup対象が重なりうる
case "$state_real/" in "$clone_real/"*) exit 1 ;; esac
case "$clone_real/" in "$state_real/"*) exit 1 ;; esac
# command substitutionを`test -z`へ直接渡さない。find自体の失敗を空directoryと
# 取り違えず、成功した出力だけを空判定する。
state_first_entry=
if ! state_first_entry=$(find "$state_dir" -mindepth 1 -maxdepth 1 -print -quit); then
  exit 1
fi
test -z "$state_first_entry"
```

commit前:

```bash
set -euo pipefail

clone_dir=${DESKCAT_WIKI_CLONE_DIR:?Set DESKCAT_WIKI_CLONE_DIR}
state_dir=${DESKCAT_WIKI_STATE_DIR:?Set DESKCAT_WIKI_STATE_DIR}
expected='wachi-yoshitaka-11-dev'
expected_mail='219948400+wachi-yoshitaka-11-dev@users.noreply.github.com'

# 表示ではなく完全一致で判定する。未設定なら`--get`が非0で終了する
test "$(git -C "$clone_dir" config --local --get user.name)"  = "$expected"
test "$(git -C "$clone_dir" config --local --get user.email)" = "$expected_mail"

before=$(git -C "$clone_dir" rev-parse HEAD)   # 失敗すれば非0で止まる
test -n "$before"
printf '%s\n' "$before" > "$state_dir/before-sha"
```

`Home.md`をstageし、commit前にstagedの内容を検証する。
`Home.md`以外が混ざったままcommitすると、そのfileも公開対象になる。

```bash
set -euo pipefail

clone_dir=${DESKCAT_WIKI_CLONE_DIR:?Set DESKCAT_WIKI_CLONE_DIR}
state_dir=${DESKCAT_WIKI_STATE_DIR:?Set DESKCAT_WIKI_STATE_DIR}

git -C "$clone_dir" add Home.md

# commandの結果を先に変数へ取る。pipeline内で直接判定すると、
# `git`が失敗して出力が空になった場合も`test`が通る
# `--name-status`はpathだけでなく変更種別も返す。`--no-renames`でrenameと
# copyの検出を無効にし、R/Cへまとめず個別のA／Dとして現れるようにする
staged=$(git -C "$clone_dir" diff --cached --name-status --no-renames)

# stagedは「Home.md 1件だけ」であること。fieldはtabで区切られる
# 既存Wikiの更新はM、初回作成は実体がまだ無いためAになる。両方を許可する
# **判定は必ずこの1 blockに収める。**`set -e`があるため、別blockへ分けると
# 最初のtestが非0になった時点で終了し、後続の判定へ到達しない
test "$staged" = "$(printf 'M\tHome.md')" || test "$staged" = "$(printf 'A\tHome.md')"

# symlink（120000）、gitlink（160000）、実行可能file（100755）を含む、意図しない
# Git modeを拒否する。出力が0件または複数件でも完全一致にならない。
staged_mode=$(git -C "$clone_dir" ls-files --format='%(objectmode)' -- Home.md)
test "$staged_mode" = 100644

# link・公開禁止情報の検査対象を、通常fileと確認したindex blobから固定する
git -C "$clone_dir" show ':Home.md' > "$state_dir/Home.staged.md"
candidate_blob=$(git -C "$clone_dir" rev-parse ':Home.md')
artifact_blob=$(git -C "$clone_dir" hash-object "$state_dir/Home.staged.md")
test -n "$candidate_blob"
test "$candidate_blob" = "$artifact_blob"
printf '%s\n' "$candidate_blob" > "$state_dir/candidate-home-blob"
```

許可するのは`M`（既存の更新）と`A`（初回作成）、Git mode `100644`だけである。
`D`削除、`R`rename、`C`copy、`T`typechangeはいずれも不一致になり、ここで止まる。
初回作成を許可しない運用にするなら、`|| test ...`の側を削って`M`だけにする。

`--name-only`とは違い、`--name-status`はpathに加えて変更種別を検証する。
pathだけを見ると、`Home.md`への**rename、copy、typechange**が検査を通過する。
`--diff-filter=D`は削除しか検出しないため、これらを塞げない。
Wikiはpublicであり、意図しない種別の変更をそのまま公開すると、
別fileの内容が`Home.md`として公開されうる。

無効化には`--no-renames`を使い、`-M0`／`-C0`は使わない。`-M0`は
`--find-renames=0`と同義で、**検出を閾値0で有効にする**指定であって無効化ではない。
`R100<TAB>old.md<TAB>Home.md`のような3 field出力になる。上の完全一致では
不一致になって止まるため結果は安全側だが、意図と逆の指定を残さない。

`set -euo pipefail`があるため、`git`自体が非0で終了すればここで止まる。
変数へ取るのは、`git`が0で終了しつつ出力が空になる場合を`test`が
「条件を満たした」と解釈しないようにするためである。
上の完全一致は、staged件数が0件でも2件以上でも不一致になる。

ここで`$state_dir/Home.staged.md`に対し、上記allowlistを用いたlink検査と、公開禁止情報の
検査を実行する。抽出件数を0件の成功として扱わず、想定したlinkが一つ以上抽出されたことも
assertする。両方が成功したあと、検査中にindexが変わっていないことを次で確認する。

```bash
set -euo pipefail

clone_dir=${DESKCAT_WIKI_CLONE_DIR:?Set DESKCAT_WIKI_CLONE_DIR}
state_dir=${DESKCAT_WIKI_STATE_DIR:?Set DESKCAT_WIKI_STATE_DIR}

candidate_blob=$(cat "$state_dir/candidate-home-blob")
artifact_blob=$(git -C "$clone_dir" hash-object "$state_dir/Home.staged.md")
current_index_blob=$(git -C "$clone_dir" rev-parse ':Home.md')
test -n "$candidate_blob"
test "$candidate_blob" = "$artifact_blob"
test "$candidate_blob" = "$current_index_blob"
printf '%s\n' "$current_index_blob" > "$state_dir/validated-home-blob"
```

ここで`git -C "$clone_dir" commit`を実行する。

commit後、push前:

```bash
set -euo pipefail

clone_dir=${DESKCAT_WIKI_CLONE_DIR:?Set DESKCAT_WIKI_CLONE_DIR}
state_dir=${DESKCAT_WIKI_STATE_DIR:?Set DESKCAT_WIKI_STATE_DIR}
expected='wachi-yoshitaka-11-dev <219948400+wachi-yoshitaka-11-dev@users.noreply.github.com>'

# 比較する値を先に取り、空でないことを確かめる
before=$(cat "$state_dir/before-sha")
after=$(git -C "$clone_dir" rev-parse HEAD)
test -n "$before"
test -n "$after"

# 新しいcommitが作られたこと
test "$before" != "$after"

# commit hookやcommit直前のindex変更を含む、実際のHEADを再検査する。
# 変更種別とpathはHome.mdのMまたはAだけ、HEAD treeもHome.mdだけであること。
committed=$(git -C "$clone_dir" diff-tree --root --no-commit-id --name-status --no-renames -r HEAD)
test "$committed" = "$(printf 'M\tHome.md')" || test "$committed" = "$(printf 'A\tHome.md')"
test "$(git -C "$clone_dir" ls-tree -r --name-only HEAD)" = Home.md
head_mode=$(git -C "$clone_dir" ls-tree --format='%(objectmode)' HEAD -- Home.md)
test "$head_mode" = 100644

# 事前に検査した通常fileのstaged blobと、commit hook等を通った実際のHEAD本文が同一であること
validated_blob=$(cat "$state_dir/validated-home-blob")
head_blob=$(git -C "$clone_dir" rev-parse 'HEAD:Home.md')
test -n "$validated_blob"
test "$validated_blob" = "$head_blob"

# 検査済みblobとの一致確認後は、公開候補本文を状態保存先へ残さない
rm -- "$state_dir/Home.staged.md"
test ! -e "$state_dir/Home.staged.md"

# authorとcommitterが公開用identityと完全一致すること
test "$(git -C "$clone_dir" log -1 --format='%an <%ae>')" = "$expected"
test "$(git -C "$clone_dir" log -1 --format='%cn <%ce>')" = "$expected"

git -C "$clone_dir" rev-parse HEAD > "$state_dir/expected-push-sha"
```

**上の各blockに`set -euo pipefail`が要る。**無いと、`test`が非0を返しても後続が実行され、
最後のcommandが成功すればblock全体は0で終わる。SHAが変わらないまま古いcommitを
`expected-push-sha`として保存し、pushへ進めてしまう。

identityは表示ではなく**完全一致の比較**で判定し、不一致ならpushへ進まない。
authorとcommitterの両方を見る。片方だけでは、意図しないcommitter addressが検査を通過する。
Wikiはpublicであり、commit authorのaddressは誰でも取得できる。
設定の失敗やcommit時の上書きを、push後のread-backまで持ち越さない。

## 初回公開の検証記録

2026-07-28に、既定の英語Homeを日本語の案内ページへ置き換えた。

| 確認対象 | 結果 |
|---|---|
| Wiki commit（当時） | `8402a8e8e2622f27af0d7707709aa66b6d3cd0e1` |
| Files | `Home.md` 1件 |
| Rendered Wiki | HTTP 200、日本語の入口と正本方針を確認 |
| Raw Markdown | HTTP 200、日本語の入口と正本方針を確認 |
| Link | 一意なURL 11件を確認し、失敗0 |
| 公開禁止情報 | Secret様pattern 0、個人path 0 |
| 二重管理 | 技術仕様、設計値、live status、Issue checklistなし |
| Git同期 | Wikiのlocal／remote SHA一致 |

上記commitは2026-07-29のcommit identity是正で書き換えたため、
**現在の`master`履歴からは到達できない。**objectそのものはGitHub側に残っており、
SHAを直接指定すれば当面は取得できる（下記「[gitで確認できないもの](#gitで確認できないもの)」を参照）。
「存在しない」とは区別する。監査と復旧の手順で意味が変わる。

現行のSHAは下記「gitによる検証」を参照する。

## gitによる検証

Wikiは`https://github.com/wachi-yoshitaka-11-dev/deskcat.wiki.git`としてcloneできる。
GitHub UIを開かずに、次を機械的に確認できる。

この検証の範囲は、`git ls-files`では**現在のWiki `master`のHEAD tree**、`git log`では
**そのHEADから到達可能なcommit履歴**である。履歴書き換えで到達不能になったcommit、
過去commitで削除されたblob、GitHub側に残る到達不能objectまでは検査しない。
下記の記録も「repositoryの全履歴」ではなく、この範囲に対する結果として読む。

`git clone`は既定branchに従うが、本runbookが記録する対象branchは`master`である。
既定branchが変われば、`ls-files`と`log`は別branchを検証してしまう。
branchを明示し、checkout先を確認してから内容を読む。

期待値は目視ではなく比較で判定し、不一致なら非0で終了させる。
表示するだけの手順にすると、見落としがそのまま「確認済み」として記録される。

`expected`には、pushしたcommitのSHAを入れる。取得元は状況で選ぶ。
**どちらの経路でも以降の判定は同じである。**

- push直後に検証する場合: `expected=$(cat "$DESKCAT_WIKI_STATE_DIR/expected-push-sha")`
- 作業copyも状態保存directoryも手元に無い場合: 下の記録表の`Head commit`をそのまま代入する
  （例: `expected=9ec03b743bbab0b70cdeece179706007b4523a3d`）

独立clone先は、この手順が`mktemp`で作る専用rootの直下へ生成する。
`DESKCAT_WIKI_READBACK_DIR`は生成したpathを後続blockへ渡す出力であり、外部から指定しない。
生成先は`DESKCAT_WIKI_STATE_DIR`や更新用cloneと同じpathまたはその配下にしない。

```bash
set -euo pipefail

readback_tmp_root=$(mktemp -d "${TMPDIR:-/tmp}/deskcat-wiki-readback.XXXXXX")
chmod 700 "$readback_tmp_root"
printf '%s\n' 'deskcat-wiki-readback-root-v1' > "$readback_tmp_root/.deskcat-generated-root"
chmod 600 "$readback_tmp_root/.deskcat-generated-root"
export DESKCAT_WIKI_READBACK_TMP_ROOT="$readback_tmp_root"
readback_dir="$readback_tmp_root/wiki-clone"
export DESKCAT_WIKI_READBACK_DIR="$readback_dir"
case "$readback_dir" in /*) ;; *) exit 1 ;; esac
test ! -L "$readback_tmp_root"
test -d "$readback_tmp_root"
test ! -e "$readback_dir"
readback_parent=$(cd "$(dirname -- "$readback_dir")" && pwd -P)
readback_real="$readback_parent/$(basename -- "$readback_dir")"

# push直後は状態保存directoryから取得する。環境変数だけが残り実体が無い場合は、
# 更新用cloneも参照対象から外して両環境変数をunsetし、記録表の値へ明示的にfallbackする。
recorded_expected=9ec03b743bbab0b70cdeece179706007b4523a3d
if test -n "${DESKCAT_WIKI_STATE_DIR:-}"; then
    state_dir=$DESKCAT_WIKI_STATE_DIR
    test ! -L "$state_dir"
    if test -e "$state_dir"; then
        test -d "$state_dir"
        state_real=$(cd "$state_dir" && pwd -P)
        test "$readback_real" != "$state_real"
        case "$readback_real/" in "$state_real/"*) exit 1 ;; esac
        case "$state_real/" in "$readback_real/"*) exit 1 ;; esac
        expected=$(cat "$state_dir/expected-push-sha")
    else
        unset DESKCAT_WIKI_STATE_DIR DESKCAT_WIKI_CLONE_DIR
        state_dir=
        expected=$recorded_expected
    fi
else
    unset DESKCAT_WIKI_STATE_DIR DESKCAT_WIKI_CLONE_DIR
    state_dir=
    expected=$recorded_expected
fi
test -n "$expected"

if test -n "${DESKCAT_WIKI_CLONE_DIR:-}"; then
    clone_dir=$DESKCAT_WIKI_CLONE_DIR
    test ! -L "$clone_dir"
    test -d "$clone_dir"
    clone_real=$(cd "$clone_dir" && pwd -P)
    test "$readback_real" != "$clone_real"
    case "$readback_real/" in "$clone_real/"*) exit 1 ;; esac
    case "$clone_real/" in "$readback_real/"*) exit 1 ;; esac
fi

git clone --branch master --single-branch https://github.com/wachi-yoshitaka-11-dev/deskcat.wiki.git "$readback_dir"

# push済みのSHAと、独立cloneのremote HEADが一致すること
test "$expected" = "$(git -C "$readback_dir" rev-parse HEAD)"

test "$(git -C "$readback_dir" rev-parse --abbrev-ref HEAD)" = master
test "$(git -C "$readback_dir" ls-files)" = Home.md
identity=$(git -C "$readback_dir" log --format='%an <%ae>|%cn <%ce>')   # 失敗すれば非0で止まる
test -n "$identity"
# grepの終了codeを明示的に扱う。1（一致なし）だけを成功とし、
# 2以上（grep自体のerror）は失敗にする。command substitutionの中で潰さない。
mismatch=$(printf '%s\n' "$identity" | grep -vFx 'wachi-yoshitaka-11-dev <219948400+wachi-yoshitaka-11-dev@users.noreply.github.com>|wachi-yoshitaka-11-dev <219948400+wachi-yoshitaka-11-dev@users.noreply.github.com>') || rc=$?
rc=${rc:-0}
test "$rc" -le 1
test -z "$mismatch"

# SHA、author、committer、署名状態を記録用に出力する
git -C "$readback_dir" log --format='%H %an <%ae> | %cn <%ce> %G?'
```

先頭の`set -euo pipefail`が無いと、`test`が非0を返しても後続commandが実行され、
最後のcommandが成功すればshell全体の終了codeは0になる。失敗した検査を
「確認済み」として記録してしまうため、blockごとfail-fastにする。

local SHAとremote HEADの比較を最初に行う。独立cloneのHEADだけを読むと、
別commitがpushされていた場合や、pushの対象を誤った場合も同期済みと記録できる。

最後の`git log`は下の表へ転記するための出力であり、判定はその前で完了している。

`git log`の出力は先に変数へ取り、空でないことを確認してからfilterする。
pipelineの中で直接filterすると、`git log`が失敗して出力が空になったとき、
`grep`の終了code 1と`test -z ""`の組み合わせで**検証が成功したように見える**。

identityは**完全一致**で比較する。`users.noreply.github.com`を含むかどうかだけでは、
別のGitHub accountのnoreply addressが通る。nameとaddressの両方を、
上の`git config`で設定した値と`grep -vFx`で突き合わせる。

### 一時directoryのcleanup

成功時と失敗時のどちらも、必要な証跡を保存したあとに次の共通blockを実行する。
`mktemp`で生成したmarker付きrootと、その直下の既知のdirectoryだけを対象にする。
root作成時からcleanup完了まで`TMPDIR`を変更しない。変更されていればcleanupは削除を拒否する。
検証が一つでも失敗した場合は再帰削除せず、pathと内容を人間が確認する。

```bash
set -euo pipefail

validate_generated_root() {
    local root kind parent_path parent_real root_real root_name temporary_parent_real
    local marker marker_value children entry entry_name child_name child child_real
    root=$1
    kind=$2
    validated_root=
    test -n "$root" || return 1
    case "$root" in /*) ;; *) return 1 ;; esac
    test ! -L "$root" || return 1
    if test ! -e "$root"; then
        return 0
    fi
    test -d "$root" || return 1

    parent_path=$(dirname -- "$root") || return 1
    parent_real=$(cd "$parent_path" && pwd -P) || return 1
    root_real=$(cd "$root" && pwd -P) || return 1
    root_name=$(basename -- "$root") || return 1
    test "$root_real" = "$parent_real/$root_name" || return 1
    temporary_parent_real=$(cd "${TMPDIR:-/tmp}" && pwd -P) || return 1
    test "$parent_real" = "$temporary_parent_real" || return 1

    marker="$root/.deskcat-generated-root"
    test ! -L "$marker" || return 1
    test -f "$marker" || return 1
    marker_value=$(cat -- "$marker") || return 1
    case "$kind" in
        update)
            [[ $root_name =~ ^deskcat-wiki-update\.[[:alnum:]]{6}$ ]] || return 1
            test "$marker_value" = 'deskcat-wiki-update-root-v1' || return 1
            children='wiki-clone state'
            ;;
        readback)
            [[ $root_name =~ ^deskcat-wiki-readback\.[[:alnum:]]{6}$ ]] || return 1
            test "$marker_value" = 'deskcat-wiki-readback-root-v1' || return 1
            children='wiki-clone'
            ;;
        *) return 1 ;;
    esac

    # 通常entry、隠しentry、dangling symlinkを列挙し、既知の直下entryだけを許可する。
    for entry in "$root"/* "$root"/.[!.]* "$root"/..?*; do
        if test ! -e "$entry" && test ! -L "$entry"; then
            continue
        fi
        entry_name=${entry##*/}
        case "$kind:$entry_name" in
            update:.deskcat-generated-root|update:wiki-clone|update:state) ;;
            readback:.deskcat-generated-root|readback:wiki-clone) ;;
            *) return 1 ;;
        esac
    done

    for child_name in $children; do
        child="$root/$child_name"
        test ! -L "$child" || return 1
        if test -e "$child"; then
            test -d "$child" || return 1
            child_real=$(cd "$child" && pwd -P) || return 1
            test "$child_real" = "$root_real/$child_name" || return 1
        fi
    done
    validated_root=$root_real
}

update_real=
readback_root_real=
if test -n "${DESKCAT_WIKI_TMP_ROOT:-}"; then
    validate_generated_root "$DESKCAT_WIKI_TMP_ROOT" update
    update_real=$validated_root
fi
if test -n "${DESKCAT_WIKI_READBACK_TMP_ROOT:-}"; then
    validate_generated_root "$DESKCAT_WIKI_READBACK_TMP_ROOT" readback
    readback_root_real=$validated_root
fi

# 片方のrootを消すことで他方まで巻き込まないことを、削除直前に再確認する。
if test -n "$update_real" && test -n "$readback_root_real"; then
    test "$update_real" != "$readback_root_real"
    case "$update_real/" in "$readback_root_real/"*) exit 1 ;; esac
    case "$readback_root_real/" in "$update_real/"*) exit 1 ;; esac
fi

if test -n "$readback_root_real"; then
    rm -rf -- "$readback_root_real"
    test ! -e "$readback_root_real"
fi
if test -n "$update_real"; then
    rm -rf -- "$update_real"
    test ! -e "$update_real"
fi
unset DESKCAT_WIKI_READBACK_DIR DESKCAT_WIKI_READBACK_TMP_ROOT
unset DESKCAT_WIKI_CLONE_DIR DESKCAT_WIKI_STATE_DIR DESKCAT_WIKI_TMP_ROOT
```

2026-07-29のread-back結果（identity是正後に取得し直した独立clone、**現行link gate導入前の履歴**）:

この記録はHTTP 200と当時の抽出方法による結果であり、現在必須の全link形式、redirectごとの
allowlist、最終fragment／anchor検査を実施していない。現行gateの成功証跡として扱わない。
2026-08-02の別のread-only確認ではallowlist外link 1件を確認したが、broken anchor数と
全link形式の結果は未計測であるため、現行`Home.md`を合格扱いにしない。

| 確認対象 | 結果 |
|---|---|
| Files（現在の`master` HEAD tree） | `Home.md` 1件 |
| Head commit | `9ec03b743bbab0b70cdeece179706007b4523a3d` |
| Parent commit | `b18cc4420e699857412df02482c8ddab8aeb4cbd` |
| Commit author（現在のHEAD以下、`%an <%ae>`） | 到達可能な全commitが`wachi-yoshitaka-11-dev <219948400+wachi-yoshitaka-11-dev@users.noreply.github.com>` |
| Commit committer（現在のHEAD以下、`%cn <%ce>`） | 到達可能な全commitが`wachi-yoshitaka-11-dev <219948400+wachi-yoshitaka-11-dev@users.noreply.github.com>` |
| 署名状態（現在のHEAD以下、`%G?`） | 到達可能な全commitが`N`（署名なし）。Wikiのcommit署名は要求していない |
| Link件数の数え方 | `Home.md`のMarkdown link出現回数を数える。Issues URLが2箇所から参照されるため、延べ12件／一意11件になる。初回記録の11件は一意数である |
| Link | 延べ12件（一意11件）すべてHTTP 200 |
| 現行link gate | 未実施（allowlist外1件は別途確認。broken anchorと全link形式は未計測） |
| Rendered Wiki／Raw Markdown | HTTP 200 |
| Secret様pattern／個人path（現在のHEAD treeの`Home.md`） | 0件 |
| 二重管理 | 技術仕様、設計値、live status、Issue checklistなし |

### 実行環境（sanitized）

| 項目 | 値 |
|---|---|
| 実施日 | 2026-07-29 |
| 用途 | Wikiの独立clone取得、内容・identity・linkのread-back |
| 端末profile | Docs / Review（[Machine Profiles](../toolchains/machine-profiles.md)） |
| 実際に成功したtool | `git` 2.44.0、`gh` 2.76.0、PowerShell 7.6.3、`curl` 8.6.0（link確認） |
| 対象 | `https://github.com/wachi-yoshitaka-11-dev/deskcat.wiki.git` |

上記は**実際に実行して成功したversion**であり、採用候補や未検証のversionを含まない。
端末名、個人path、token、shell履歴は記録しない。
一時cloneはread-back後に削除しており、作業copyは残していない。

## Commit identityの是正記録

2026-07-29に、既存2 commitのauthor／committerを公開用identityへ書き換えた。

| 項目 | 内容 |
|---|---|
| 経緯 | Wikiは別repositoryのため、main repositoryの`--local`設定を引き継がない。runbookが一時cloneを指示していたため、global設定のaddressで記録されていた |
| 対象 | `eaddf75` → `b18cc44`、`8402a8e` → `9ec03b7` |
| 変更内容 | author／committerのnameとaddressのみ。日時とcommit messageは保存 |
| 内容の同一性 | tree `fe3054182d…`、`Home.md` blob `ac094c9c77…` が書き換え前後で一致 |
| 実施方法 | `git filter-branch --env-filter`後にforce push（下記の警告を参照） |
| 残存 | 旧commitはGitHub上で到達不能だが、SHAを直接指定すれば当面取得できる。完全消去はGitHub Supportへの依頼が必要 |

上記「Commit identity」の手順により、以後の更新では再発しない。

> **この履歴書き換えは、通常の保守手順ではない。**
> [AGENTS.md](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/AGENTS.md)は
> 履歴書き換えとforce pushを禁止している。今回は、既に公開されたcommitへ意図しない
> addressが記録されていたことへの一回限りの是正として、ユーザーの明示的な指示のもとで実施した。
>
> 今後Wikiの内容や記録を訂正する場合は、**履歴を書き換えず新しいcommitで修正する**。
> 同種の是正が再び必要になった場合も、この記録を先例として自動的に実施してはならない。
> 影響範囲を提示し、ユーザーの指示を個別に得る。

### gitで確認できないもの

Wikiの「Restrict editing to collaborators only」は、clone内容にもAPIにも現れない。
REST APIは`has_wiki`、GraphQLは`hasWikiEnabled`のみを返し、いずれも機能の
有効／無効であって編集権限ではない。

この設定だけはGitHubのSettings → Features → Wikisで目視確認し、結果を
[Repository設定計画](https://github.com/wachi-yoshitaka-11-dev/deskcat/blob/main/.github/REPOSITORY_SETTINGS.md)へ記録する。

2026-07-29の目視結果: 有効（collaboratorだけが編集可能）。

publicのWikiは、この設定が無効だと任意のGitHubユーザーが編集できる。
READMEとPagesの両方からWikiへ導線があるため、改竄・誘導の経路になりうる。
Wikiの設定を変更した場合は、この節と記録側を同じ変更で更新する。

`.github/`はPagesへ複製しないため、公開文書からは相対linkではなく絶対URLで参照する。

## 失敗時

- Link切れや公開禁止情報を認めた場合は完了扱いにしない。
- `Home.staged.md`作成後に検査が失敗した場合は、上記の絶対path、symlink、実体path、相互包含の
  各assertionだけを再実行する（`$state_dir`の空directory assertionは、この時点では`before-sha`や
  `Home.staged.md`が既に存在するため対象外とする。再実行に含めると`set -e`で必ず止まり、
  削除step自体に到達できない）。そのうえで`rm -- "$state_dir/Home.staged.md"`でその1 fileだけを
  削除し、`test ! -e`でread-backする。検査失敗を理由にcloneやstate directoryを未確認のまま
  再帰削除しない。
- 原因調査と必要な証跡保存が終わった後は、[一時directoryのcleanup](#一時directoryのcleanup)を実行する。
  cleanup側のmarker、実体path、包含関係、想定外entryの検査が失敗したrootは削除しない。
- Wikiへ仕様を直接追記して修正を急がず、main repositoryの正本を先に更新する。
- Remote、branch、対象fileが想定と異なる場合はpushしない。
- 意図しない公開があった場合は、影響範囲を記録し、安全な内容への修正を優先する。
