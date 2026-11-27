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
3. `gh pr merge`に`--subject`と`--body`／`--body-file`が無い。messageを渡さないと
   GitHubが合成したmessageでmergeされ、検証の経緯や測定値がcommitに残らない
   （main昇格の`#383`／`afc86cf`で実際に失われた）。
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

**判定は字句だけで行う。**意味は判定しない。

5. `gh pr merge`の対象Pull Requestが慎重に扱うpath（`REVIEW_TARGET_PREFIXES`・
   `REVIEW_TARGET_FILES`。一覧はCONTRIBUTINGの「慎重に扱う path」と同じ）を含むのに、
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
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from concurrent.futures import TimeoutError as FuturesTimeout
from pathlib import Path

import command_line

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

# repositoryの正本template。**hookの起動位置に依存させない。**`__file__`から
# 辿るため、cwdがどこであっても同じfileを見る。
REPO_ROOT = Path(__file__).resolve().parent.parent.parent
ISSUE_TEMPLATE_DIR = REPO_ROOT / ".github" / "ISSUE_TEMPLATE"
PR_TEMPLATE_PATH = REPO_ROOT / ".github" / "pull_request_template.md"

SKIP_ENV = "DESKCAT_SKIP_GH_GUARD"

# CodeRabbitのreviewが完走するまでmergeさせない対象（#688）。CONTRIBUTINGの「慎重に扱う path」と
# 同じ一覧にする（`scripts/test_hooks.py`が両者の一致を検査する）。
REVIEW_TARGET_PREFIXES = (
    "firmware/esp32/src/",
    "crates/deskcat-protocol/src/",
    "crates/deskcat-servo/src/",
    ".github/workflows/",
    "scripts/hooks/",
    ".githooks/",
    "hardware/DeskCat.pretty/",
    "hardware/overview/",
    "hardware/breadboard/",
    "hardware/breadboard-rebuild/",
    "hardware/universal-board/",
)
REVIEW_TARGET_FILES = (
    ".claude/settings.json",
    "AGENTS.md",
    "CLAUDE.md",
    "CONTRIBUTING.md",
    "docs/protocol/esp32-pi-protocol.md",
    "docs/hardware/gpio-assignment.md",
    "docs/hardware/power-budget.md",
    "docs/hardware/servo-safety-limits.md",
    "docs/governance/ai-agent-policy.md",
    "docs/governance/development-workflow.md",
    "docs/governance/hardware-safety-policy.md",
    "hardware/deskcat.kicad_pcb",
    "hardware/deskcat.kicad_sch",
    "hardware/DeskCat.kicad_sym",
    "hardware/deskcat.kicad_pro",
    "hardware/fp-lib-table",
    "hardware/sym-lib-table",
    "scripts/check_wiring_figure_sync.py",
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
#
# **最初の1回は遅い（#760）。**しばらく照会しない後の最初の`gh`は、endpointによらず
# （`pr view`のgraphqlも`gh api`のRESTも）10.4〜10.7秒かかり、続く照会は0.4〜1秒で返る。
# DNS（`GODEBUG=netdns`の違い）にも依らなかった。**この遅れは照会の方法では避けられない。**
# 上限が10秒だと、この遅れだけで「確認できない」としてmergeを止める。そのため1回の上限は
# 遅れの約1.5倍に置く。**遅れが無ければ上限まで待たない。**照会できなければdenyする性質は同じ。
GH_TIMEOUT_SECONDS = 15
# **最初の`gh`照会の前に始め、残りを各照会へ渡す。**走っているworkerも同じ期限で
# 終わるため、denyを出した後にprocessが残って時間切れになることもない。
# hookの30秒に収まる値にする（最初の遅れの後に残りの照会も収まる）。
CHECK_DEADLINE_SECONDS = 18
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


def _repo_option(args, global_repo, value_options):
    """`-R`／`--repo`の値を返す。`-Rowner/repo`（値をつなげた短い形）も読む。

    `_option_value`は連結形を読まないため、`gh issue close 9 -Rowner/target`が
    別のrepositoryではなく作業中のcheckoutのIssueを検査していた（#751）。
    """
    value = _option_value(args, "--repo", "-R")
    if value is None:
        # 値を取るoptionの直後の語は値である（`--comment -Rx/y`のghは`-Rx/y`を値に取る）。
        index = 0
        while index < len(args):
            arg = args[index]
            if arg in value_options:
                index += 2
                continue
            if arg.startswith("-R") and len(arg) > 2:
                value = arg[2:]
                break
            index += 1
    return value or global_repo


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


def _check_merge(args):
    """`gh pr merge`に`--subject`と`--body`／`--body-file`があるかを見る。

    messageを渡さないとGitHubが合成したmessageでmergeされ、検証の経緯や測定値が
    commitに残らない（main昇格の`#383`／`afc86cf`）。strategyを問わず見る。
    helpの表示は何もmergeしないため対象外である。
    """
    if _is_help(args):
        return
    text, source = _body_text(args)
    if text is None:
        _deny(
            f"`gh pr merge`のmessageを確認できない（{source}）。"
            " messageを渡さない場合はGitHubが合成したmessageになり、"
            " 検証経緯や測定値が失われる（main昇格の`#383`／`afc86cf`で実際に起きた）。"
            " `--subject`と`--body-file`を明示する（CONTRIBUTINGの「Merge方式」）。"
        )
    if not _has_option(args, "--subject", "-t"):
        # 値は使わず、有無だけを見る。`-t`は結合形（`-ts`）でも値を渡せるため、
        # `_option_value`ではなく`_has_option`を使う。
        _deny(
            "`gh pr merge`に`--subject`が無い。"
            " `--subject`と`--body-file`の両方を明示する（CONTRIBUTINGの「Merge方式」）。"
            " 省略すると、GitHubがPull Request titleで補い、messageを呼び出し側が"
            "制御できなくなる。"
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


def _gh_json(command, deadline=None):
    """`gh`を実行して標準出力を返す。**失敗は例外のまま上げる。**

    `deadline`（`time.monotonic()`の値）を渡すと、残りの時間を超えて待たない。
    期限を過ぎていれば実行せず`TimeoutExpired`を上げる。
    """
    timeout = GH_TIMEOUT_SECONDS
    if deadline is not None:
        timeout = min(timeout, deadline - time.monotonic())
        if timeout <= 0:
            raise subprocess.TimeoutExpired(["gh", *command], 0)
    result = subprocess.run(
        ["gh", *command],
        capture_output=True, text=True, encoding="utf-8",
        timeout=timeout, check=True,
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


def _commit_review_state(repo, sha, deadline=None):
    """commit 1つのCodeRabbitの状態を`(完走か, 表示用の文字列)`で返す。"""
    runs = json.loads(_gh_json([
        "api", f"repos/{repo}/commits/{sha}/check-runs?per_page=100",
    ], deadline)).get("check_runs", [])
    runs = [run for run in runs if run.get("name") == CODERABBIT_CHECK_NAME]
    if runs:
        done = any(_run_review_completed(run) for run in runs)
        title = (runs[-1].get("output") or {}).get("title") or runs[-1].get("conclusion")
        return done, f"check-run `{title}`"
    statuses = json.loads(_gh_json([
        "api", f"repos/{repo}/commits/{sha}/statuses?per_page=100",
    ], deadline))
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
    base（`main`の昇格を含む）は区別しない。CONTRIBUTINGの昇格の節1も、範囲が
    慎重に扱うpathに触れるときに手動reviewの対象と定めている。
    """
    if _is_help(args):
        return
    deadline = time.monotonic() + CHECK_DEADLINE_SECONDS
    repo_option = _repo_option(args, global_repo, MERGE_VALUE_OPTIONS)
    selector = _merge_selector(args)
    try:
        view = ["pr", "view"] + ([selector] if selector else [])
        view += (["-R", repo_option] if repo_option else [])
        view += ["--json", "number,url"]
        info = json.loads(_gh_json(view, deadline))
        number = info["number"]
        match = re.match(r"https://[^/]+/([^/]+/[^/]+)/pull/\d+", info["url"])
        repo = match.group(1)
        files = _gh_json([
            "api", "--paginate", f"repos/{repo}/pulls/{number}/files?per_page=100",
            "--jq", ".[].filename",
        ], deadline).split()
        targets = _review_targets(files)
        if not targets:
            return
        shas = _gh_json([
            "api", "--paginate", f"repos/{repo}/pulls/{number}/commits?per_page=100",
            "--jq", ".[].sha",
        ], deadline).split()
        head_state = None
        # 新しいcommitから順に並列で見る。完走が1つでもあれば通す。
        pool = ThreadPoolExecutor(max_workers=COMMIT_CHECK_WORKERS)
        try:
            futures = {
                pool.submit(_commit_review_state, repo, sha, deadline): sha
                for sha in reversed(shas)
            }
            for future in as_completed(
                futures, timeout=max(0, deadline - time.monotonic())
            ):
                done, state = future.result()
                if done:
                    return
                if futures[future] == shas[-1]:
                    head_state = state
        finally:
            # **待つ。**走っているworkerは同じ期限で終わる。待たずに抜けると、
            # denyを出してもprocessが残り、hookの時間切れでdenyが捨てられる（#751）。
            pool.shutdown(wait=True, cancel_futures=True)
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
        " CONTRIBUTINGの「慎重に扱う path」に触れる変更は、reviewの完走までmergeしない。"
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
    repo_option = _repo_option(args, global_repo, ISSUE_CLOSE_VALUE_OPTIONS)
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
