#!/usr/bin/env python3
"""`gh`のIssue／Pull Request操作に、必須metadataが付いているかを検査する。

Claude CodeのPreToolUse hookとして、Bash tool呼び出しの前に走る。stdinからhookの
入力JSONを読み、`tool_input.command`だけを見る。実行はしない。

止める対象は6つである。

1. `gh issue create`／`gh pr create`に`--project`が無い。Projects v2 boardへの
   item追加は`CONTRIBUTING.md`の起票規約で必須だが、**#204／#205／#206は3件とも
   作成時に入っておらず、5分55秒〜18分29秒後に追加されている**（`added_to_project_v2`の
   event時刻と作成時刻の差）。文書に書いても実行されないため、作成の一部にする。
2. `gh pr create`に`--base`が無い。**`gh`は省略時にrepositoryのdefault branchを
   使い、このrepositoryのdefaultは`main`である。**2026-08-28に
   [PR #250](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/250)がbase `main`で
   作られ、baseの変更まで1時間17分かかった（timelineの`base_ref_changed`で実測）。
   **`gh issue create`に`--base`は存在しないため、そちらへは要求しない。**
3. `gh pr merge`のsquash messageから、`git`が`Change-Class`と`Self-Review`を
   trailerとして読めない。PR側のgateがSUCCESSでも、squash commitへtrailerが
   引き継がれないと次の昇格で落ちる（`18298ae`／`619c843`。どちらも
   `DECLARATION_EXEMPT`へ登録済み）。**「文字列としてあるか」では足りない。**
   `c171c52`は`Change-Class`・`Self-Review`・`Instruction-Change`を文字列として
   持っていたが、最後の段落にコロン無しの`Closes #304`が混じっていたため、
   `git interpret-trailers`は段落ごとtrailerと認めなかった。部分一致で見ていた
   当時のこの検査は素通りさせ、CIの`history`だけが検出した。
   **`--merge`（`main`昇格のmerge commit）にはこの要求を掛けない**
   （`_is_main_promotion_merge`）。`CONTRIBUTING.md`の`Merge方式`が`main`昇格へ
   これらのtrailerを要求しないと定めており、`_check_history`（`review_gate.py`）も
   `git rev-list --no-merges`でmerge commitを検査対象から外している。
   2026-09-16まで`--squash`と区別せず一律に要求しており（`#417`）、`#383`の
   main昇格でこの誤検知が`gh pr merge --merge`を2回denyし、ブラウザでの
   mergeを強いた結果、merge commit `afc86cf`のmessageがGitHub既定のまま入り、
   直前の昇格が持っていた検証経緯・実測値が失われた（main の履歴は書き換えない
   ため戻せない）。**この判定はPRのbaseを見ていない。**`--merge`の指定と
   `CONTRIBUTING.md`の規約（`develop`はsquash、`main`はmerge commit）に頼る
   heuristicであり、規約に反して`develop`へ`--merge`でmergeされた場合は
   誤って通す。`--rebase`は同節に定義が無く、trailerの扱いを定めていないため
   要求を掛けたままにする。戦略flagが1つも無い呼び出しも、`gh`がrepositoryの
   既定を使い hookからは何になるか分からないため、要求を掛けたままにする。
4. `gh issue create`／`gh pr create`に`--body`／`--body-file`が付いており、その
   本文が対応するtemplate（`.github/ISSUE_TEMPLATE/*.md`／
   `.github/pull_request_template.md`）の`## `節を欠いている。`--body`系は
   templateを丸ごと迂回できる唯一の経路であり、既存Issueの記載漏れの根本原因だった
   （[#348](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/348)）。
   **`--body`系が付いていない呼び出し（対話prompt／`-e`）は対象外である。**
   その経路は既にtemplateを経由するため、迂回にならない。
   **本文を特定できない場合（stdin `-`、読めないfile）は素通りさせない。**
   [#267](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/267)で
   `coderabbit_gate.py`が同じ理由でfail-openを廃止した前例に倣い、`_body_text`の
   「特定できない」は3の`_check_merge`と同じ扱い（`deny`）にする。

**4つとも`--help`／`-h`が付いた呼び出しを対象外にする。**helpの表示は何も作らず、
mergeもしない。metadataを要求しても誤検知しか生まない。**2026-08-28に
`gh issue create --help`／`gh pr create --help`／`gh pr merge --help`の3つとも
拒否されることを実測した。**`gh pr merge`はとりわけ効く。`--subject`と`--body-file`の
明示をCONTRIBUTINGが要求しているのに、**そのoption名を`--help`で確認できなかった。**

**判定は字句だけで行う。**意味は判定しない。`review_gate.py`と同じ方針である。
**`git interpret-trailers --parse`に読ませることは字句の判定である。**見ているのは
trailerとして解釈できる形かどうかだけで、値の正しさは見に行かない。

5. `gh pr merge`の対象Pull Requestが安全・protocol・firmwareの対象fileを含むのに、
   CodeRabbitのreviewが1度も完走していない（#688）。CONTRIBUTINGの「手動で依頼する前に
   状態を確認する」が「reviewが完走するまでmergeしない」と定めるが、2026-11-01〜02に
   #680と#684が`Review skipped`のままmergeされた。人が覚えていても守られなかったため、
   mergeの時点で止める。**この検査は`DESKCAT_SKIP_GH_GUARD`で無効化できない**
   （`_check_coderabbit_review`）。

6. Issueのcloseの操作（`gh issue close`、およびboardの`Status`を`Done`にする
   `gh project item-edit`）で、そのIssueの`受け入れ条件`の節に`- [ ]`が残っている（#693）。
   2026-11-02に受け入れ条件を照合せずに#679がcloseされた。走査するとclose済み193件のうち
   122件に未チェックが残っていた。人が覚えていても守られなかったため、closeの時点で止める。
   Issueはboardの`Done`で自動closeされるため、`item-edit`が主な経路である。
   **節が無い本文もdenyする**（CONTRIBUTINGの「Issueの範囲」が受け入れ条件を必須と
   しており、無ければ照合できない）。**この検査は`DESKCAT_SKIP_GH_GUARD`で無効化できない**
   （`_check_issue_close`／`_check_item_done`）。

`DESKCAT_SKIP_GH_GUARD=1`で5と6以外を丸ごと無効化できる。**誤検知で作業が止まったときの
逃げ道であり、常用するものではない。**使ったら理由をPull Request本文へ書く。
**ただしcommand行頭へ置く形では効かない**（`SKIP_ENV`の隣に実測を書いてある）。
"""

import json
import os
import re
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from concurrent.futures import TimeoutError as FuturesTimeout
from pathlib import Path

import command_line

# trailerの名前と、messageからtrailerを読む判定は`scripts/review_gate.py`が正本で
# ある。**hook側へ複製しない。**名前がずれると、gateが要求するものとhookが見るものが
# 食い違い、hookが素通りする。**判定も同じである。**名前だけを共有して存在の判定を
# 部分一致で別実装していたことが`c171c52`の穴だった。
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import review_gate  # noqa: E402

# 検査する`gh`のsubcommand。`gh`の呼び出しに続く2語がこの組のときだけ見る。
# 呼び出しの切り出しは`command_line.invocations`が行う。
PR_CREATE_SUBCOMMAND = ("pr", "create")
CREATE_SUBCOMMANDS = (("issue", "create"), PR_CREATE_SUBCOMMAND)
MERGE_SUBCOMMAND = ("pr", "merge")
ISSUE_CLOSE_SUBCOMMAND = ("issue", "close")
ITEM_EDIT_SUBCOMMAND = ("project", "item-edit")

# boardの`Status`fieldの`Done`のoption id（2026-11-02に`gh project field-list 5`で実測した値）。
# **boardを作り直すと変わり、検査は素通りになる。**`Done`への変更だけを見る。
STATUS_DONE_OPTION_ID = "98236657"
# 見出しの書き方は揃っていない（`## この Issue の受け入れ条件・成果物`＝#546、
# `### 測定可能な受け入れ条件`＝#236・#237）。「受け入れ条件」を含む見出しを節の始まりにする。
HEADING = re.compile(r"^(#{1,6})\s+(.*?)\s*$")
ACCEPTANCE_WORD = "受け入れ条件"
NOT_PLANNED_REASON = "not planned"
UNCHECKED_ITEM = re.compile(r"^\s*(?:[-*+]|\d+\.)\s+\[ \]\s*(.*)$")
ISSUE_CLOSE_VALUE_OPTIONS = ("--comment", "-c", "--reason", "-r", "--duplicate-of", "--repo", "-R")
ITEM_QUERY = (
    "query($id: ID!) { node(id: $id) { ... on ProjectV2Item { content {"
    " __typename ... on Issue { number state body } } } } }"
)

# squash messageが持たなければならないtrailerの名前。**値は見ない。**
REQUIRED_MERGE_TRAILERS = (review_gate.TRAILER_CLASS, review_gate.TRAILER_REVIEW)

# `gh pr merge`の戦略flag。**`CONTRIBUTING.md`の`Merge方式`が定義するのは
# squash（`develop`向け）とmerge commit（`main`昇格）の2つだけである。**
# `--rebase`は同節に定義が無く、trailerの扱いも定めていない。
MERGE_STRATEGY_FLAGS = ("--merge", "-m")
SQUASH_STRATEGY_FLAGS = ("--squash", "-s")
REBASE_STRATEGY_FLAGS = ("--rebase", "-r")

# repositoryの正本template。**hookの起動位置に依存させない。**`__file__`から
# 辿るため、cwdがどこであっても同じfileを見る（`GIT_ROOT`とは別の理由で位置に
# 依存させている。こちらはgitを呼ばない純粋なfile読み出しである）。
REPO_ROOT = Path(__file__).resolve().parent.parent.parent
ISSUE_TEMPLATE_DIR = REPO_ROOT / ".github" / "ISSUE_TEMPLATE"
PR_TEMPLATE_PATH = REPO_ROOT / ".github" / "pull_request_template.md"

# gitへ渡す作業ディレクトリ。**`interpret-trailers --parse`はrepositoryの外でも
# 動く**（2026-09-02に`/tmp`で実測。EXIT=0）。見ているのはstdinのmessageだけで、
# repositoryの状態もrevisionも読まない。hookの起動位置に依存させない。
GIT_ROOT = "."

# gitを待つ上限（秒）。**hookはtool呼び出しの前に走るため、返らないと作業が止まる。**
# 超えたら「確認できなかった」として扱う（`_merge_trailers`）。
GIT_TIMEOUT_SECONDS = 10

# 本文の前に置く1行。`gh pr merge`が作るcommit messageは`--subject`と本文を空行で
# 連結した形になるため、**本文だけをgitへ渡すと本文の先頭行がsubjectとして扱われる。**
# 本文がtrailerだけで構成されている場合、実際のcommitでは有効なtrailer blockなのに、
# 本文単体では段落が1つしか無い形になりtrailer 0件と読まれる（2026-09-02に実測）。
# **subjectの中身は解釈に効かない**（効くのは「空行の前に1行あること」だけ）ため、
# `--subject`の値は読まずにこの1行を前へ付ける。**値を読むと、渡されなかった場合に
# GitHubがPull Request titleで補う分を推測することになる。**
SUBJECT_PLACEHOLDER = "subject"

SKIP_ENV = "DESKCAT_SKIP_GH_GUARD"

# CodeRabbitのreviewが完走するまでmergeさせない対象（#688）。**範囲を広げない。**
# CodeRabbitにはクールタイム（1時間）があり、何でも通すと作業が止まる。
# 対象にしない: 実験記録、BOM、図、ルールの文書、検査のscript、Pi側だけのアプリ。
REVIEW_TARGET_PREFIXES = ("firmware/esp32/src/",) + tuple(
    f"crates/deskcat-{name}/src/"
    for name in ("protocol", "servo", "face", "tap", "touch", "accel", "env")
)
REVIEW_TARGET_FILES = (
    "docs/protocol/esp32-pi-protocol.md",
    "docs/hardware/gpio-assignment.md",
    "docs/hardware/power-budget.md",
    "docs/hardware/servo-safety-limits.md",
)

# CodeRabbitの結果を持つcheck-runの名前。**2026-11時点の結果はcheck-runに残り、
# commit statusには出ない**（#678の全commitでstatus 0件。docs/runbooks/
# coderabbit-review-observations.mdの「check-runの表示」）。旧い記録（#397・#399）は
# commit statusの`Review completed`で残るため、両方を見る。
CODERABBIT_CHECK_NAME = "CodeRabbit"
REVIEW_COMPLETED_TEXT = "Review completed"

# ghを待つ上限（秒）。超えたら確認できなかったものとして扱いdenyする。
# **hook全体の上限は`.claude/settings.json`の30秒である。**超えるとhookが落ち、
# 通る側に倒れるおそれがあるため、commitごとの確認は並列で行い（昇格は100件を超える）、
# 全体にも期限を置く。
GH_TIMEOUT_SECONDS = 10
COMMIT_CHECK_DEADLINE_SECONDS = 15
COMMIT_CHECK_WORKERS = 16

# `gh pr merge`の値を取るoption。selector（PR番号・URL・branch）を探すときに値を飛ばす。
MERGE_VALUE_OPTIONS = (
    "--subject", "-t", "--body", "-b", "--body-file", "-F",
    "--author-email", "-A", "--match-head-commit", "--repo", "-R",
)

# **この逃げ道は`DESKCAT_SKIP_GH_GUARD=1 gh ...`という形では効かない。**hookは対象
# commandとは別のprocessとして起動されるため、command行頭の代入はhookのenvironmentへ
# 届かない（2026-08-28に`gh issue create --help`で実測。拒否された）。効かせるには
# hookのprocessが継ぐenvironmentへ入れる必要がある。**診断文はCONTRIBUTINGの記述に
# 合わせてあり、記述側の扱いは別途判断する。**


def _deny(reason):
    """PreToolUse hookの拒否を出力して終了する。"""
    json.dump(
        {
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": reason,
            }
        },
        sys.stdout,
    )
    sys.stdout.write("\n")
    raise SystemExit(0)


# `--project`の短縮形。`gh issue create`と`gh pr create`はどちらも`-p, --project`を持つ
# （`gh <sub> create --help`で確認した）。**長い方だけを見ると、`-p deskcat`を
# 誤って拒否する。**誤検知はhookごと無効化される側の失敗である。
PROJECT_OPTIONS = ("--project", "-p")

# `--base`の短縮形。**`gh pr create`は`-B, --base`を持ち、大文字である**
# （`gh help pr create`で確認した。小文字の`-h`はhelp、`-H`は`--head`）。
# `gh issue create`の option 一覧に`--base`は無い（同じく確認した。出現0件）。
BASE_OPTIONS = ("--base", "-B")

# 本文を渡すoption。**存在するかどうかだけをまず見る。**この組が1つも無い呼び出しは
# 対話prompt（またはeditor）経由であり、templateを迂回していない。対象外にする。
BODY_OPTIONS = ("--body", "-b", "--body-file", "-F")

# labelを渡すoption。**繰り返し指定でき、値はcomma区切りでも複数持てる**
# （`gh label`系の一般的な仕様。`gh issue create --help`で確認した）。
LABEL_OPTIONS = ("--label", "-l")

# `gh issue create`のtemplate選択option。値はfile名ではなくtemplateの`name:`
# frontmatter（表示名）である（`gh issue create --template "Bug Report"`の例が
# `gh issue create --help`に載っている）。
TEMPLATE_OPTIONS = ("--template", "-T")

# helpの表示だけを求める呼び出し。**何も作らないため、metadataを要求しない。**
# **4つの検査すべてに効かせる**（`_check_create`と`_check_merge`の先頭。前者は
# `_check_body_sections`も呼ぶ）。**同 file の docstring が「4つとも」と書いている方が正しい。**
#
# `gh <sub> <cmd> --help`は option 一覧の確認に使う。**ここを拒否すると、hookが
# 要求しているoption名を調べる手段そのものが塞がる。**2026-08-28に
# `gh issue create --help`が拒否され、`gh help issue create`へ回避した。
# **規則を守るために要る情報を、規則が隠している状態だった。**
# `gh pr merge`では`--subject`と`--body-file`の名前がそこにある。
#
# **完全一致だけを見る。**`_has_option`の連結判定を当てると`-hello`がhelpになる。
# 代わりに`--title -h`のように値としての`-h`を拾い、**その呼び出しは検査を抜ける。**
# 字句だけで区別する手立てが無い。CONTRIBUTINGの「取り切れていないもの」に載せる。
HELP_OPTIONS = ("--help", "-h")


def _has_option(args, *names):
    """`--name value`／`--name=value`／短縮形の連結（`-pvalue`）を拾う。"""
    for arg in args:
        for name in names:
            if arg == name or arg.startswith(name + "="):
                return True
            # 短縮形は値を連結して書ける（`-pdeskcat`）。長い形には当てない。
            if (
                not name.startswith("--")
                and not arg.startswith("--")
                and arg.startswith(name)
                and len(arg) > len(name)
            ):
                return True
    return False


def _is_help(args):
    """helpの表示だけを求める呼び出しかを返す。**完全一致だけを見る。**"""
    return any(arg in HELP_OPTIONS for arg in args)


def _option_value(args, *names):
    """`--name value`／`--name=value`の値を返す。無ければ`None`。"""
    for index, arg in enumerate(args):
        for name in names:
            if arg == name:
                if index + 1 < len(args):
                    return args[index + 1]
                return None
            if arg.startswith(name + "="):
                return arg[len(name) + 1:]
    return None


def _option_values(args, *names):
    """`--name value`／`--name=value`の値を、繰り返し分すべて返す。

    `_option_value`は最初の1つしか返さない。`--label`のような繰り返し可能な
    optionは、2回目以降を見落とすと存在するlabelを見落とす。
    """
    values = []
    index = 0
    while index < len(args):
        arg = args[index]
        for name in names:
            if arg == name:
                if index + 1 < len(args):
                    values.append(args[index + 1])
                index += 1
                break
            if arg.startswith(name + "="):
                values.append(arg[len(name) + 1:])
                break
        index += 1
    return values


def _body_text(args):
    """`--body`系optionで渡された本文を返す。

    戻りは`(text, source)`。`text`が`None`のときは本文を特定できなかったことを表し、
    `source`がその理由を持つ。**特定できないことを「入っている」と扱わない。**
    `gh pr merge`のsquash messageと`gh issue create`／`gh pr create`の本文は、
    同じoption（`--body`／`-b`／`--body-file`／`-F`）で渡される。判定を複製しない
    ため、subcommandを問わずここで一本化する。
    """
    path = _option_value(args, "--body-file", "-F")
    if path is not None:
        if path == "-":
            return None, "本文がstdin（`-`）から渡されており、hookからは読めない"
        try:
            with open(path, encoding="utf-8") as handle:
                return handle.read(), f"--body-file {path}"
        except OSError as error:
            return None, f"`--body-file`の読み出しに失敗した: {error}"
    body = _option_value(args, "--body", "-b")
    if body is not None:
        return body, "--body"
    return None, "本文が渡されていない"


def _section_headings(text):
    """`## `見出しの並びを返す。字句だけで見る。Markdownの意味解釈はしない。"""
    return [heading.strip() for heading in re.findall(r"(?m)^## (.+)$", text)]


def _frontmatter_field(text, field):
    """templateの frontmatter（先頭の`---`〜`---`）から`field:`の値を返す。"""
    match = re.match(r"^---\n(.*?\n)---\n", text, re.DOTALL)
    if not match:
        return None
    field_match = re.search(
        rf"^{re.escape(field)}:\s*(.+)$", match.group(1), re.MULTILINE
    )
    if not field_match:
        return None
    return field_match.group(1).strip().strip('"')


def _issue_template_catalog():
    """`.github/ISSUE_TEMPLATE/*.md`から`{表示名: path}`と`{type:*label: path}`を作る。

    値は起票時に実際に使われる2つの手がかり、`--template`の表示名（frontmatterの
    `name:`）と`type:*` label（frontmatterの`labels:`）である。**filename には
    紐付けない。**`gh issue create --template`が受け取るのは表示名であり、
    filenameではない。
    """
    by_name = {}
    by_label = {}
    if not ISSUE_TEMPLATE_DIR.is_dir():
        return by_name, by_label
    for path in sorted(ISSUE_TEMPLATE_DIR.glob("*.md")):
        try:
            text = path.read_text(encoding="utf-8")
        except OSError:
            continue
        name = _frontmatter_field(text, "name")
        if name:
            by_name[name] = path
        labels = _frontmatter_field(text, "labels")
        if labels:
            for label in labels.split(","):
                label = label.strip()
                if label.startswith("type:"):
                    by_label[label] = path
    return by_name, by_label


def _expected_template(subcommand, args):
    """検査に使うtemplateの`(見出しの並び, path)`を返す。決められなければ`(None, 理由)`。

    `gh pr create`は`.github/pull_request_template.md`の1つしか無く、曖昧さが無い。
    `gh issue create`は5つある。**`--template`の指定があればそれを最優先する**
    （明示された意図と食い違うfallbackをしない）。無ければ`--label`のうち
    `type:*`のものを見る。**該当が0件でも複数件でも決められない。**PMの判断で、
    決められない場合は実装者ではなくこの呼び出し自体を止める
    （[#348](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/348)受け入れ条件）。
    """
    if subcommand == PR_CREATE_SUBCOMMAND:
        try:
            text = PR_TEMPLATE_PATH.read_text(encoding="utf-8")
        except OSError as error:
            return None, f"pull request templateを読めない（{error}）"
        return _section_headings(text), None

    by_name, by_label = _issue_template_catalog()
    template_name = _option_value(args, *TEMPLATE_OPTIONS)
    if template_name is not None:
        path = by_name.get(template_name)
        if path is None:
            return None, (
                f"`--template {template_name}`が`.github/ISSUE_TEMPLATE/`のどのtemplateの"
                f"`name:`とも一致しない（候補: {'／'.join(sorted(by_name)) or 'なし'}）"
            )
        return _section_headings(path.read_text(encoding="utf-8")), None

    type_labels = [
        label.strip()
        for value in _option_values(args, *LABEL_OPTIONS)
        for label in value.split(",")
        if label.strip().startswith("type:")
    ]
    matches = {label: by_label[label] for label in type_labels if label in by_label}
    if len(matches) == 1:
        path = next(iter(matches.values()))
        return _section_headings(path.read_text(encoding="utf-8")), None
    if not matches:
        return None, (
            "どのIssue templateと突き合わせるか決められない"
            "（`--template`も、templateに対応する`type:*` labelも無い）。"
            " `--template <name>`（`.github/ISSUE_TEMPLATE/*.md`の`name:`と一致する表示名）"
            " か、`type:*` labelを1つ指定する。"
        )
    return None, (
        f"どのIssue templateと突き合わせるか決められない"
        f"（`type:*` labelが複数の候補に一致した: {'／'.join(sorted(matches))}）。"
        " `--template <name>`で明示する。"
    )


def _check_body_sections(subcommand, args):
    """`--body`系で渡された本文が、templateの`## `節を欠いていないかを見る。

    **`--body`系が付いていない呼び出しは対象外である。**対話prompt／`-e`は
    templateを経由しており、迂回にならない（module docstringの4を参照）。
    """
    if not _has_option(args, *BODY_OPTIONS):
        return
    text, source = _body_text(args)
    if text is None:
        _deny(
            f"`gh {' '.join(subcommand)}`の本文（{source}）を確認できない。"
            " templateの節が揃っているか判定できないため、確認できる形で本文を渡す。"
            " `--body-file`にfileのpathを渡すと判定できる。"
            f" 意図して確認させない場合は{SKIP_ENV}=1を付けて実行し、理由を残す。"
        )
    expected, error = _expected_template(subcommand, args)
    if expected is None:
        _deny(error)
        return
    present = set(_section_headings(text))
    missing = [heading for heading in expected if heading not in present]
    if missing:
        _deny(
            f"`gh {' '.join(subcommand)}`の本文（{source}）が、templateの節を"
            f"{len(missing)}個欠いている: {'／'.join(missing)}。"
            " `--body`はtemplateを丸ごと迂回できる経路であり、欠けた節は記載漏れとして"
            " 残る。欠けている節を本文へ足すか、templateを埋めたfileを用意して"
            " `--body-file`で渡す。"
            f" 意図して外す場合は{SKIP_ENV}=1を付けて実行し、理由を残す。"
        )


def _option_names(names):
    """診断文に出す option 名の並び。定数から作り、文面へ値を複製しない。"""
    return "／".join(f"`{name}`" for name in names)


def _check_project(subcommand, args):
    if _has_option(args, *PROJECT_OPTIONS):
        return
    _deny(
        f"`gh {' '.join(subcommand)}`にprojectの指定が無い"
        f"（{_option_names(PROJECT_OPTIONS)}のいずれかが要る）。"
        " Projects v2 boardへのitem追加は起票・作成時に必要である"
        "（CONTRIBUTINGの「起票時に設定する項目」）。"
        " #204／#205／#206は3件とも作成時に入っておらず後から追加している。"
        " 注意ではなく、作成の一部として強制している。"
        f" 意図してboardへ入れない場合は{SKIP_ENV}=1を付けて実行し、理由を残す。"
    )


def _check_base(args):
    """`gh pr create`にbaseの指定があるかを見る。**値の正しさは見ない。**

    存在するかだけを見る。`develop`と書くべきか`main`と書くべきかは、その
    Pull Requestの目的で決まり、字句からは読めない。**明示させることだけを強制する。**
    """
    if _has_option(args, *BASE_OPTIONS):
        return
    _deny(
        "`gh pr create`にbaseの指定が無い"
        f"（{_option_names(BASE_OPTIONS)}のいずれかが要る）。"
        " 省略すると`gh`はrepositoryのdefault branchを使い、"
        "このrepositoryのdefaultは`main`である。"
        " 2026-08-28にPR #250がbase `main`で作られ、baseの変更まで1時間17分かかった。"
        " 日常のPull Requestのbaseは`develop`であり、`main`は昇格でだけ使う"
        "（CONTRIBUTINGの「全体の流れ」）。"
        " **どちらが正しいかはhookは判定しない。明示することだけを求めている。**"
        f" 意図して既定へ委ねる場合は{SKIP_ENV}=1を付けて実行し、理由を残す。"
    )


def _check_create(subcommand, args):
    if _is_help(args):
        # helpの表示だけを求める呼び出しは何も作らない。**metadataを要求しない。**
        return
    _check_project(subcommand, args)
    # `--base`は`gh pr create`だけが持つ。`gh issue create`へ要求しない。
    if subcommand == PR_CREATE_SUBCOMMAND:
        _check_base(args)
    _check_body_sections(subcommand, args)


def _merge_trailers(text, source):
    """squash messageから、`git`がtrailerとして読む組を返す。

    **読めなかった場合は`deny`する。**gitを実行できない、返らない、想定外の入力で
    落ちる、のいずれでも「trailerがある」とは言えない。この検査が守っているのは
    「宣言がsquash commitへ入るか」であり、**入らなかったときの手当ては
    `DECLARATION_EXEMPT`への登録＝Pull Request 1本である**（`c171c52`が実例）。
    確認できないまま通すほうが高くつく。**止まったときの逃げ道は`DESKCAT_SKIP_GH_GUARD`
    であり、診断文にも書く。**
    """
    message = f"{SUBJECT_PLACEHOLDER}\n\n{text}"
    try:
        return review_gate.trailers_from_message(
            GIT_ROOT, message, timeout=GIT_TIMEOUT_SECONDS
        )
    except (SystemExit, OSError, subprocess.SubprocessError) as error:
        _deny(
            f"`gh pr merge`のsquash message（{source}）を`git`で解釈できなかった"
            f"（{type(error).__name__}: {error}）。"
            " **確認できないまま通さない。**trailerがsquash commitへ入らなかった"
            "場合の手当ては`DECLARATION_EXEMPT`への登録＝Pull Request 1本であり"
            "（`c171c52`が実例）、通すほうが高くつく。"
            f" gitを実行できない環境で作業する場合は{SKIP_ENV}=1で無効化し、"
            " 理由をPull Request本文へ書く。"
        )


def _is_main_promotion_merge(args):
    """`--merge`（merge commit戦略）が指定されているかを返す。

    **これはPRのbaseを見ていない。**`CONTRIBUTING.md`の`Merge方式`が
    `develop`はsquash、`main`はmerge commitと定めていることに基づく
    heuristicであり、**その規約が守られている前提のもとでのみ正しい。**
    baseを実際に問い合わせて判定する形はここでは採らない
    （`gh pr view --json baseRefName`が必要になり、hookからの追加のnetwork
    呼び出しを増やす。字句だけで判定する既存方針（module docstring）に合わせ、
    flagの有無だけで判定する）。

    **`--squash`／`--rebase`が同時に指定されている場合は主昇格として扱わない。**
    `gh`自身が複数戦略の同時指定を拒否するため通常は起きないが、字句だけで
    見ている以上、単独で`--merge`が立っている場合だけに限定する。
    **`--rebase`単独の場合は、この関数はFalseを返す**（`CONTRIBUTING.md`が
    `--rebase`の扱いを定めていないため、trailerを要求する既存の扱いのままにする）。
    **戦略flagが1つも無い場合もFalseを返す。**`gh`はrepositoryの既定を使うが、
    hookはcommand文字列しか見ないため既定が何かを知りようが無い。
    `_body_text`が「特定できない場合はdeny」としているのと同じ理由で、
    分からないものは従来どおりtrailerを要求する側へ倒す。
    """
    if any(flag in SQUASH_STRATEGY_FLAGS for flag in args):
        return False
    if any(flag in REBASE_STRATEGY_FLAGS for flag in args):
        return False
    return any(flag in MERGE_STRATEGY_FLAGS for flag in args)


def _check_merge(args):
    """`gh pr merge`のmessageが指定されているか、squash messageなら宣言trailerも見る。

    **この関数は判定を持たない。**trailerの名前も、messageからtrailerを読む判定も
    `scripts/review_gate.py`が正本である（名前は`REQUIRED_MERGE_TRAILERS`経由で、
    判定は`trailers_from_message`で受け取る）。ここがするのは、その2つを突き合わせて
    欠けている名前を診断文にすることだけである。
    **hook側へ複製しない。**名前だけを共有して存在の判定を部分一致で別実装して
    いたことが`c171c52`の穴だった（module docstringの3を参照）。

    **messageの有無と、trailerの有無は別の要求であり、分けて見る。**
    `CONTRIBUTING.md`の`Merge方式`は「`--subject`と`--body-file`を省略しない」と
    定めており、これは`--squash`／`--merge`のどちらにも掛かる。**trailer
    （`Change-Class`／`Self-Review`）を要求しないのは`--merge`（`main`昇格）だけ**
    である。

    **2026-09-17に、この2つを分けずに書いた版が入りかけた。**`_is_main_promotion_merge`
    がtrueのとき`_body_text`より前でreturnしており、**`gh pr merge N --merge`を
    messageの指定なしで実行できた。**message省略はGitHubが既定messageを合成する
    経路であり、これは`afc86cf`（`#383`）で実際に起きた損失そのものである。
    **`#417`が防ごうとした事象を、この修正自体が再び開けるところだった。**
    push前のreviewで実行順の問題として発見し、通す前に直した。

    見る順は5つである。**helpの表示は何もmergeしないため対象外**、
    **messageを特定できない場合は`deny`**（`_body_text`。strategyを問わない）、
    **`--subject`の有無も見て、無ければ`deny`**（strategyを問わない。
    `_body_text`は`--body`／`--body-file`の有無しか見ておらず、`--subject`は
    見ていない。2026-09-17にCodeRabbitのreviewが`--merge`側の欠如を指摘し
    （`#417`）、その時点では`--squash`側の同じ穴を既存test
    `test_merge_message_trailers_are_checked`の前提を理由に対象外としたが、
    その穴は`#423`として別途報告され、この修正でstrategyを問わず塞いだ。
    `CONTRIBUTING.md`の`Merge方式`は`--subject`と`--body-file`の両方を
    明示すると定めている）、
    **`--merge`（`main`昇格）はここでtrailerを要求せず終える**
    （`_is_main_promotion_merge`。`CONTRIBUTING.md`の`Merge方式`が`main`昇格へ
    これらのtrailerを要求しないと明記しており、`scripts/review_gate.py`の
    `_check_history`も`git rev-list --no-merges`でmerge commitを検査対象から
    外している。このhookが`--squash`と区別せず一律に要求していたことが
    2026-09-16に`#417`として報告された。実際に`#383`のmain昇格でこの誤検知により
    `gh pr merge --merge`が2回denyされ、ブラウザでのmergeを強いた結果、
    merge commit `afc86cf`のmessageがGitHub既定のまま入り、直前の昇格が
    持っていた検証経緯・実測値が失われた。この記録は戻せない）、
    そのうえで（`--merge`でない場合）`git`がtrailerとして読むかを見る。
    """
    if _is_help(args):
        # helpの表示だけを求める呼び出しはmergeしない。**messageを要求しない。**
        return
    text, source = _body_text(args)
    if text is None:
        strategy_note = (
            "main昇格（`--merge`）でもmessageの指定を省略しない。"
            if _is_main_promotion_merge(args)
            else "trailerがsquash commitへ引き継がれたかを後から辿れない。"
        )
        _deny(
            f"`gh pr merge`のmessageを確認できない（{source}）。"
            f" 確認できないmessageでmergeすると、{strategy_note}"
            " messageを渡さない場合はGitHubが合成したmessageになり、"
            " 検証経緯や測定値が失われる"
            "（main昇格の`#383`／`afc86cf`で実際に起きた。squashでは"
            "`18298ae`／`619c843`でtrailerが入らなかった）。"
            " `--subject`と`--body-file`を明示する（CONTRIBUTINGの「Merge方式」）。"
        )
    if not _has_option(args, "--subject", "-t"):
        # `_body_text`は`--body`／`--body-file`の有無しか見ておらず、
        # `--subject`無しの呼び出しがここまで素通りしていた。`--merge`側は
        # 2026-09-17にCodeRabbitのreviewが指摘し（`#417`）、`--squash`側の
        # 同じ穴は`#423`として別途報告された。**strategyを問わず見る**ため、
        # 診断文もstrategy名を決め打ちしない（`--rebase`／flag無しでも
        # 同じ理由でdenyされる）。
        # **値は使わず、有無だけを見るため`_option_value`ではなく`_has_option`
        # を使う。**`gh`の`-t`はpflagの短縮string flagであり、`-ts`（`-t s`の
        # 結合形）でも値を渡せる。`_option_value`はこの結合形を読まず、
        # `-ts`を指定していても`--subject`が無いと誤判定していた
        # （2026-09-17のCodeRabbit reviewが指摘）。
        _deny(
            "`gh pr merge`に`--subject`が無い。"
            " `CONTRIBUTING.md`の`Merge方式`は`--subject`と`--body-file`の"
            "両方を明示すると定めており、`--body`／`--body-file`の"
            "有無だけでは足りない。"
            " `--subject`を省略すると、GitHubがPull Request titleを"
            "推測で補う可能性があり、messageの内容を呼び出し側が"
            "制御できなくなる。"
        )
    if _is_main_promotion_merge(args):
        # `main`昇格のmerge commitである。messageの指定（`--subject`を含む）は
        # 上で確認済みであり、**squash向けのtrailer要求だけをここで掛けない。**
        return
    found = _merge_trailers(text, source)
    missing = [name for name in REQUIRED_MERGE_TRAILERS if name not in found]
    if missing:
        _deny(
            f"`gh pr merge`のsquash message（{source}）から"
            f" {', '.join(missing)} を`git`がtrailerとして読めない。"
            " **文字列として書いてあるかではなく、`git interpret-trailers --parse`が"
            "trailerとして返すかを見ている。**messageの最後の段落にコロンの無い行が"
            "1行でも混じると、その段落はまるごとtrailerでなくなる"
            "（`c171c52`は`Closes #304`をtrailerと同じ段落へ置き、`Change-Class`・"
            "`Self-Review`・`Instruction-Change`・`Refs`をすべて失った）。"
            " 宣言はmessage末尾の段落へまとめ、その段落へ他の行を混ぜない。"
            " 引き継がれないと、そのmergeは通っても次の`main`昇格で"
            " `review_gate.py history`が落ちる。"
            " 値の正本は`scripts/review_gate.py`である。"
            f" 誤検知で止まった場合は{SKIP_ENV}=1で無効化し、理由を残す。"
        )


def _merge_selector(args):
    """`gh pr merge`のselector（PR番号・URL・branch）を返す。無ければ`None`。"""
    index = 0
    while index < len(args):
        arg = args[index]
        if arg in MERGE_VALUE_OPTIONS:
            index += 2
            continue
        if not arg.startswith("-"):
            return arg
        index += 1
    return None


def _gh_json(command):
    """`gh`を実行して標準出力を返す。**失敗は例外のまま上げる。**"""
    result = subprocess.run(
        ["gh", *command],
        capture_output=True, text=True, encoding="utf-8",
        timeout=GH_TIMEOUT_SECONDS, check=True,
    )
    return result.stdout


def _review_targets(files):
    return [
        name for name in files
        if name in REVIEW_TARGET_FILES or name.startswith(REVIEW_TARGET_PREFIXES)
    ]


def _run_review_completed(run):
    """check-runが完走を示すかを返す。**`success`かつsummaryが`Review completed`。**"""
    output = run.get("output") or {}
    return (
        run.get("status") == "completed"
        and run.get("conclusion") == "success"
        and REVIEW_COMPLETED_TEXT in (output.get("summary") or "")
    )


def _commit_review_state(repo, sha):
    """commit 1つのCodeRabbitの状態を`(完走か, 表示用の文字列)`で返す。"""
    runs = json.loads(_gh_json([
        "api", f"repos/{repo}/commits/{sha}/check-runs?per_page=100",
    ])).get("check_runs", [])
    runs = [run for run in runs if run.get("name") == CODERABBIT_CHECK_NAME]
    if runs:
        done = any(_run_review_completed(run) for run in runs)
        title = (runs[-1].get("output") or {}).get("title") or runs[-1].get("conclusion")
        return done, f"check-run `{title}`"
    statuses = json.loads(_gh_json([
        "api", f"repos/{repo}/commits/{sha}/statuses?per_page=100",
    ]))
    statuses = [
        item for item in statuses if item.get("context") == CODERABBIT_CHECK_NAME
    ]
    if statuses:
        done = any(
            item.get("state") == "success"
            and (item.get("description") or "").startswith(REVIEW_COMPLETED_TEXT)
            for item in statuses
        )
        return done, f"status `{statuses[0].get('description')}`"
    return False, "CodeRabbitの記録なし"


def _check_coderabbit_review(args, global_repo):
    """対象fileを含むPull Requestは、CodeRabbitのreviewが完走するまでmergeさせない。

    **headのcommitだけを見ない。**CONTRIBUTINGは「依頼したreviewが走った後の対応commitは
    自己レビューで見て、投げ直さない」と定めており、review完走の後に指摘対応のcommitを
    足すとheadは`Review skipped`になる（#678の`0629fc7`）。**Pull Requestのどれかの
    commitで完走していればよい。**commitごとの確認は並列で行い、完走が見つかれば止める。

    **確認できない場合もdenyする。**`gh`が失敗する、応答を読めない、のいずれでも
    「完走している」とは言えない。**逃げ道は足さない**（`DESKCAT_SKIP_GH_GUARD`も効かない）。
    base（`main`の昇格を含む）は区別しない。昇格もCONTRIBUTINGの昇格の節1が
    手動reviewの対象と定めている。
    """
    if _is_help(args):
        return
    repo_option = _option_value(args, "--repo", "-R") or global_repo
    selector = _merge_selector(args)
    try:
        view = ["pr", "view"] + ([selector] if selector else [])
        view += (["-R", repo_option] if repo_option else [])
        view += ["--json", "number,url"]
        info = json.loads(_gh_json(view))
        number = info["number"]
        match = re.match(r"https://[^/]+/([^/]+/[^/]+)/pull/\d+", info["url"])
        repo = match.group(1)
        files = _gh_json([
            "api", "--paginate", f"repos/{repo}/pulls/{number}/files?per_page=100",
            "--jq", ".[].filename",
        ]).split()
        targets = _review_targets(files)
        if not targets:
            return
        shas = _gh_json([
            "api", "--paginate", f"repos/{repo}/pulls/{number}/commits?per_page=100",
            "--jq", ".[].sha",
        ]).split()
        head_state = None
        # 新しいcommitから順に並列で見る。完走が1つでもあれば通す。
        pool = ThreadPoolExecutor(max_workers=COMMIT_CHECK_WORKERS)
        try:
            futures = {
                pool.submit(_commit_review_state, repo, sha): sha
                for sha in reversed(shas)
            }
            for future in as_completed(futures, timeout=COMMIT_CHECK_DEADLINE_SECONDS):
                done, state = future.result()
                if done:
                    return
                if futures[future] == shas[-1]:
                    head_state = state
        finally:
            pool.shutdown(wait=False, cancel_futures=True)
    except (OSError, subprocess.SubprocessError, ValueError, KeyError,
            AttributeError, TypeError, FuturesTimeout) as error:
        _deny(
            "`gh pr merge`の対象が、CodeRabbitのreviewを要する変更かを確認できなかった"
            f"（{type(error).__name__}: {error}）。**確認できないまま通さない。**"
            " この検査に解除の手段は無い（#688）。"
        )
    shown = ", ".join(f"`{name}`" for name in targets[:8])
    more = f"（ほか{len(targets) - 8}件）" if len(targets) > 8 else ""
    _deny(
        f"PR #{number}は、CodeRabbitのreviewが完走するまでmergeしない変更を含むが、"
        "どのcommitでもreviewが完走していない。"
        f" 対象file: {shown}{more}。"
        f" CodeRabbitの状態（head）: {head_state or 'commitを取得できない'}。"
        " CONTRIBUTINGの「手動で依頼する前に状態を確認する」は、安全・電気・protocol・"
        "firmwareに関わる変更をreviewの完走までmergeしないと定める。"
        " `@coderabbitai full review`の依頼は手順に従い、完走を待つ。"
        " この検査に解除の手段は無い（#688）。"
    )


def _acceptance_denial(number, body):
    """Issue本文の`受け入れ条件`の節を見て、止める理由を返す。通してよければ`None`。

    節は「受け入れ条件」を含む見出し（`##`〜`######`）から、次の同じかそれより上の階層の
    見出しまでである。字句だけで見る。
    """
    open_items = []
    found = False
    section_level = None
    for line in (body or "").splitlines():
        heading = HEADING.match(line)
        if heading:
            level = len(heading.group(1))
            if section_level is not None and level <= section_level:
                section_level = None
            if section_level is None and ACCEPTANCE_WORD in heading.group(2) and level >= 2:
                section_level = level
                found = True
            continue
        item = UNCHECKED_ITEM.match(line) if section_level is not None else None
        if item:
            open_items.append(item.group(1).strip() or "（空の項目）")
    if not found:
        return (
            f"Issue #{number}の本文に`受け入れ条件`の節が無く、照合できない。"
            " CONTRIBUTINGの「Issueの範囲」は測定可能な受け入れ条件をIssueに含めると定める。"
            " 本文へ「受け入れ条件」を含む`##`見出しの節を足し、満たしたものをチェックしてからcloseする。"
            " この検査に解除の手段は無い（#693）。"
        )
    if not open_items:
        return None
    shown = "／".join(f"`{item}`" for item in open_items[:8])
    more = f"（ほか{len(open_items) - 8}件）" if len(open_items) > 8 else ""
    return (
        f"Issue #{number}の`受け入れ条件`に未チェックの項目が{len(open_items)}件残っている: "
        f"{shown}{more}。"
        " 満たしたものをチェックし、満たせないものはIssueをopenのままにして残りを書く"
        "（development-workflowの「未実行のcheckが受け入れ条件ならIssueをopenのままにする」）。"
        " この検査に解除の手段は無い（#693）。"
    )


def _unverifiable_close(error):
    _deny(
        "Issueのcloseの対象が、受け入れ条件を満たしているかを確認できなかった"
        f"（{type(error).__name__}: {error}）。**確認できないまま通さない。**"
        " この検査に解除の手段は無い（#693）。"
    )


def _check_issue_close(args, global_repo):
    """`gh issue close`の対象Issueの受け入れ条件に、未チェックが残っていないかを見る。

    **確認できない場合もdenyする。**逃げ道は足さない（`DESKCAT_SKIP_GH_GUARD`も効かない）。
    """
    if _is_help(args):
        return
    if (
        _option_value(args, "--reason", "-r") == NOT_PLANNED_REASON
        and (_option_value(args, "--comment", "-c") or "").strip()
    ):
        # 方針として閉じるIssue（close済みのうち3件がnot planned）。完了扱いではないため、
        # 理由のコメントが付いているときだけ受け入れ条件を見ない。boardの`Done`は完了扱いで、
        # `_check_item_done`が見る。
        return
    repo_option = _option_value(args, "--repo", "-R") or global_repo
    selector = None
    index = 0
    while index < len(args):
        if args[index] in ISSUE_CLOSE_VALUE_OPTIONS:
            index += 2
            continue
        if not args[index].startswith("-"):
            selector = args[index]
            break
        index += 1
    try:
        if selector is None:
            raise ValueError("Issueの番号またはURLが指定されていない")
        view = ["issue", "view", selector] + (["-R", repo_option] if repo_option else [])
        info = json.loads(_gh_json(view + ["--json", "number,body"]))
        reason = _acceptance_denial(info["number"], info["body"])
    except (OSError, subprocess.SubprocessError, ValueError, KeyError, TypeError) as error:
        _unverifiable_close(error)
        return
    if reason:
        _deny(reason)


def _check_item_done(args):
    """boardの`Status`を`Done`にする`gh project item-edit`の対象を見る。

    `Done`はIssueを自動closeする。Pull Request itemとDraft issueは対象外である。
    既にCLOSEDのIssueは、`Done`が閉じるものが無いため止めない。
    """
    if _is_help(args):
        return
    if _option_value(args, "--single-select-option-id") != STATUS_DONE_OPTION_ID:
        return
    try:
        item_id = _option_value(args, "--id")
        if item_id is None:
            raise ValueError("`--id`（item id）が指定されていない")
        node = json.loads(_gh_json([
            "api", "graphql", "-f", f"query={ITEM_QUERY}", "-f", f"id={item_id}",
        ]))["data"]["node"]
        content = node["content"]
        if content["__typename"] != "Issue":
            return
        if content["state"] == "CLOSED":
            return
        reason = _acceptance_denial(content["number"], content["body"])
    except (OSError, subprocess.SubprocessError, ValueError, KeyError, TypeError) as error:
        _unverifiable_close(error)
        return
    if reason:
        _deny(reason)


def main():
    skip = os.environ.get(SKIP_ENV) == "1"
    try:
        payload = json.load(sys.stdin)
    except (json.JSONDecodeError, UnicodeDecodeError):
        # hookの入力を読めないことを、対象commandの問題として扱わない。
        return 0
    command = command_line.command_from(payload)
    if command is None or "gh" not in command:
        # 全Bash呼び出しでこのhookが走る。`if`条件でBash(gh *)へ絞ると
        # `cd x && gh pr create`が素通りするため、絞らずここで安く抜ける。
        return 0
    for args in command_line.invocations(command, "gh"):
        # **subcommandより前のglobal optionを外す。**外さないと`gh --repo o/r pr create`の
        # ように1語足すだけで検査が抜ける（#325で実測）。
        global_repo = (
            command_line.global_option_value(args, "gh", "--repo")
            or command_line.global_option_value(args, "gh", "-R")
        )
        args = command_line.skip_global_options(args, "gh")
        head = tuple(args[:2])
        if head in CREATE_SUBCOMMANDS:
            if not skip:
                _check_create(head, args[2:])
        elif head == MERGE_SUBCOMMAND:
            if not skip:
                _check_merge(args[2:])
            # **skipの指定に関わらず走らせる。**解除の手段を足さない（#688）。
            _check_coderabbit_review(args[2:], global_repo)
        elif head == ISSUE_CLOSE_SUBCOMMAND:
            # **skipの指定に関わらず走らせる。**解除の手段を足さない（#693）。
            _check_issue_close(args[2:], global_repo)
        elif head == ITEM_EDIT_SUBCOMMAND:
            _check_item_done(args[2:])
    return 0


if __name__ == "__main__":
    sys.exit(main())
