#!/usr/bin/env python3
"""`hooks/`配下の回帰test。

hookを子processとして起動し、stdinへhookの入力JSONを渡して、**拒否したか通したか**と
診断文を検査する。基点の検査はfixture repositoryを作って実際に`git fetch`させる。

**主に見るのは、素通りしないことである。**guardが黙って通るようになると、
止めているはずの失敗（boardへのitem追加漏れ、mergeのmessageの指定漏れ、古い基点）が
また起きる。**そしてhookは失敗しても静かなため、壊れたことに気付けない。**
"""

import atexit
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
import warnings
from pathlib import Path

SCRIPTS_ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS_ROOT / "lib"))

import publish_guards as guards  # noqa: E402

sys.path.insert(0, str(SCRIPTS_ROOT))


sys.path.insert(0, str(SCRIPTS_ROOT / "hooks"))

import command_line  # noqa: E402

GH_GUARD = str(SCRIPTS_ROOT / "hooks" / "gh_metadata_guard.py")
BASE_GUARD = str(SCRIPTS_ROOT / "hooks" / "branch_base_guard.py")
CODERABBIT_GATE = str(SCRIPTS_ROOT / "hooks" / "coderabbit_gate.py")
REPO_ROOT_FOR_TEMPLATES = SCRIPTS_ROOT.parent
ISSUE_TEMPLATE_DIR = REPO_ROOT_FOR_TEMPLATES / ".github" / "ISSUE_TEMPLATE"
PR_TEMPLATE_PATH = REPO_ROOT_FOR_TEMPLATES / ".github" / "pull_request_template.md"

sys.path.insert(0, str(SCRIPTS_ROOT / "hooks"))

import gh_metadata_guard as guard  # noqa: E402



# fixtureのcommitに使うidentity。実行者の設定に依存させない。
GIT_IDENTITY = (
    "-c", "user.name=test",
    "-c", "user.email=test@example.invalid",
    "-c", "commit.gpgsign=false",
)


def _template_sections(path):
    """正本templateから`## `見出しの並びを読む。hook側の抽出方法を複製しない。

    見出し名はhookの実装（`_section_headings`）と同じ正規表現で取る。
    testがhookのロジックと違う方法で見出しを数えると、hookが見ていないずれを
    testが見落とす。
    """
    text = Path(path).read_text(encoding="utf-8")
    return [heading.strip() for heading in re.findall(r"(?m)^## (.+)$", text)]


def _body_with_sections(headings):
    """指定した見出しだけを持つ本文を作る。中身は節が空でなければ何でもよい。"""
    return "\n".join(f"## {heading}\nx\n" for heading in headings)


def _git(root, *arguments):
    result = subprocess.run(
        ["git", "-C", root, *GIT_IDENTITY, *arguments],
        capture_output=True, text=True, encoding="utf-8", errors="replace",
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"git {' '.join(arguments)} failed: {result.stdout}{result.stderr}"
        )
    return result.stdout

# `gh`を差し替えるshim（#688）。`gh pr merge`の前にhookが呼ぶ`gh pr view`／`gh api`へ、
# 環境変数`DESKCAT_TEST_GH`のJSONから答える。**hookの解除手段ではなく、testの入力である。**
# 未設定なら対象fileを含まない1件のPull Requestとして答える。
GH_SHIM = f"""#!{sys.executable}
import json, os, sys, time
fixture = json.loads(os.environ.get("DESKCAT_TEST_GH") or "{{}}")
if fixture.get("log"):
    with open(fixture["log"], "a", encoding="utf-8") as log:
        log.write(json.dumps(sys.argv[1:]) + "\\n")
time.sleep(fixture.get("sleep", 0))
if fixture.get("cold_marker") and not os.path.exists(fixture["cold_marker"]):
    # 最初の1回だけ遅い（#760。しばらく照会しない後の最初のgh）。
    open(fixture["cold_marker"], "w").close()
    time.sleep(fixture.get("cold_sleep", 0))
if fixture.get("fail"):
    sys.stderr.write("gh: boom")
    sys.exit(1)
args = sys.argv[1:]
if args[:2] == ["pr", "view"]:
    # base と head を指定しない fixture では、どちらも作業ツリーの HEAD にする。
    # merge-base と head が同じなので、レビューの記録の検査（#814 の作業リストの9）は
    # 範囲を要さずに通る。記録の検査そのものは ReviewCoverageMergeTests が見る。
    import subprocess
    here = subprocess.run(["git", "rev-parse", "HEAD"], capture_output=True,
                          text=True).stdout.strip()
    print(json.dumps({{"number": 7, "url": "https://github.com/o/r/pull/7",
                      "baseRefOid": fixture.get("base", here),
                      "headRefOid": fixture.get("head", here),
                      "body": fixture.get("body", "")}}))
elif args[:2] == ["issue", "view"]:
    print(json.dumps(fixture["issue"]))
elif args[:2] == ["api", "graphql"]:
    print(json.dumps({{"data": {{"node": fixture["item"]}}}}))
elif args[0] == "api":
    path = [a for a in args[1:] if a.startswith("repos/")][0]
    if path.startswith("repos/o/r/pulls/7/files"):
        print("\\n".join(fixture.get("files", ["README.md"])))
    elif path.startswith("repos/o/r/issues/7/comments"):
        # 指定が無ければ、head の SHA を書いた承認のコメントが1件ある扱いにする。
        import subprocess
        here = subprocess.run(["git", "rev-parse", "HEAD"], capture_output=True,
                              text=True).stdout.strip()
        default = ["承認 " + fixture.get("head", here)]
        print(json.dumps([[{{"user": {{"type": "User"}}, "body": body}}
                           for body in fixture.get("comments", default)]]))
    elif path.startswith("repos/o/r/pulls/7/commits"):
        print("\\n".join(fixture.get("commits", {{}}).keys()))
    elif "/check-runs" in path:
        time.sleep(fixture.get("sleep_commits", 0))
        sha = path.split("/commits/")[1].split("/")[0]
        print(json.dumps({{"check_runs": fixture["commits"][sha].get("check_runs", [])}}))
    elif "/statuses" in path:
        time.sleep(fixture.get("sleep_commits", 0))
        sha = path.split("/commits/")[1].split("/")[0]
        print(json.dumps(fixture["commits"][sha].get("statuses", [])))
    else:
        sys.exit(2)
else:
    sys.exit(2)
"""
_GH_SHIM_DIR = tempfile.mkdtemp(prefix="deskcat-gh-shim-")
atexit.register(shutil.rmtree, _GH_SHIM_DIR, ignore_errors=True)
Path(_GH_SHIM_DIR, "gh").write_text(GH_SHIM, encoding="utf-8")
Path(_GH_SHIM_DIR, "gh").chmod(0o755)


def _invoke(script, command, cwd=None, environment=None):
    """hookへcommandを渡し`(exit code, 解釈した出力)`を返す。

    出力が空のときは`None`を返す。**空は「通した」を意味する。**
    """
    payload = json.dumps({"tool_name": "Bash", "tool_input": {"command": command}})
    env = dict(os.environ)
    # 呼び出し側の環境に無効化用の変数が残っていても、testの前提を壊さない。
    env.pop("DESKCAT_SKIP_GH_GUARD", None)
    env.pop("DESKCAT_SKIP_BASE_GUARD", None)
    env.pop("DESKCAT_SKIP_WORKTREE_GUARD", None)
    env.pop("DESKCAT_TEST_GH", None)
    if script in (GH_GUARD, CODERABBIT_GATE):
        env["PATH"] = f"{_GH_SHIM_DIR}{os.pathsep}{env['PATH']}"
    if script == GH_GUARD and cwd is None:
        # `gh pr merge` の検査は、手元の git で base と head を照らす（#814 の作業リストの9）。
        # CI は test を repository の外（`runner.temp`）で走らせるので、repository の中で走らせる。
        cwd = str(REPO_ROOT_FOR_TEMPLATES)
    if environment:
        env.update(environment)
    result = subprocess.run(
        [sys.executable, script],
        input=payload, capture_output=True, text=True,
        encoding="utf-8", errors="replace", cwd=cwd, timeout=120, env=env,
    )
    text = result.stdout.strip()
    return result.returncode, (json.loads(text) if text else None)


def _reason(output):
    return output["hookSpecificOutput"]["permissionDecisionReason"]


class GhMetadataGuardTests(unittest.TestCase):
    """`gh`の必須metadataを見るhookのtest。"""

    def assertDenied(self, command, *, contains=None):
        code, output = _invoke(GH_GUARD, command)
        self.assertEqual(code, 0, command)
        self.assertIsNotNone(output, f"通してしまった: {command}")
        self.assertEqual(
            output["hookSpecificOutput"]["permissionDecision"], "deny", command
        )
        if contains:
            self.assertIn(contains, _reason(output))

    def assertAllowed(self, command):
        code, output = _invoke(GH_GUARD, command)
        self.assertEqual(code, 0, command)
        self.assertIsNone(output, f"止めてしまった: {command}")

    def test_global_options_do_not_hide_the_subcommand(self):
        """**subcommandより前のglobal optionで検査が抜けない**（#325で実測した穴）。

        `args[:2]`で位置から読んでいた版では、`gh --repo o/r pr create`と
        `gh -R o/r pr merge`が**denyを素通りしていた。**`--project`の要求も
        mergeのmessageの要求も、**global optionを1語足すだけで抜けた。**
        """
        for command in (
            "gh --repo owner/repo pr create --title x --body y",
            "gh -R owner/repo pr create --title x --body y",
            "gh --repo owner/repo issue create --title x --body y",
        ):
            with self.subTest(command=command):
                self.assertDenied(command)

    def test_global_options_do_not_hide_the_merge_subcommand(self):
        """merge側も同じである。messageの検査が素通りしていた。"""
        self.assertDenied(
            'gh -R owner/repo pr merge 1 --squash --body "y"',
            contains="`--subject`が無い",
        )

    def test_create_without_project_is_denied(self):
        """`--project`が無いIssue／Pull Request作成を止める。"""
        for command in (
            "gh pr create --title t --body b",
            "gh issue create --title t",
        ):
            with self.subTest(command=command):
                self.assertDenied(command, contains="--project")

    def test_calls_on_a_later_line_are_denied(self):
        """複数行commandの2行目以降の`gh`を見る（#389）。"""
        for command in (
            "cat x\ngh pr merge 1 --squash",
            "git status\ngh pr create --title t --body b",
            "cd /tmp\n\n  gh issue create --title t",
        ):
            with self.subTest(command=command):
                self.assertDenied(command)

    def test_calls_written_in_a_heredoc_body_are_allowed(self):
        """heredocのbodyの`gh`呼び出しでは止めない（#389）。**bashは実行しない。**

        本文へ`gh pr merge`と書く操作は、このリポジトリでは日常である。
        """
        self.assertAllowed("cat > body.md <<'EOF'\ngh pr merge 1 --squash\nEOF\n")

    def test_create_with_project_is_allowed(self):
        """`--project value`と`--project=value`のどちらも通す。

        `gh pr create`側の`--base`は、この検査の前提を満たすために足しているだけである。
        **見ている対象は`--project`のままである。**
        """
        for command in (
            "gh pr create --title t --project deskcat --base develop",
            "gh pr create --title t --project=deskcat --base develop",
            "gh issue create --title t --project deskcat",
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_short_project_option_is_accepted(self):
        """`--project`の短縮形`-p`を拾う。

        `gh issue create`と`gh pr create`はどちらも`-p, --project`を持つ。
        **長い形だけを見ると`-p deskcat`を誤って拒否する。**誤検知は、hookそのものを
        無効化される側の失敗である。
        """
        for command in (
            "gh pr create --title t -p deskcat --base develop",
            "gh issue create --title t -p deskcat",
            "gh pr create --title t -pdeskcat --base develop",
            "gh pr create --title t -p=deskcat --base develop",
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_long_options_are_not_matched_by_the_short_form(self):
        """`-p`の連結判定を、`--p`で始まる長い option へ当てない。"""
        for command in (
            "gh pr create --title t --paginate",
            "gh pr create --title t --draft",
        ):
            with self.subTest(command=command):
                self.assertDenied(command)

    def test_pr_create_without_base_is_denied(self):
        """`--base`が無いPull Request作成を止める。

        **`gh`は省略時にrepositoryのdefault branchを使い、このrepositoryのdefaultは
        `main`である。**2026-08-28にPR #250がbase `main`で作られ、baseの変更まで
        1時間17分かかった。**機構は何も止めなかった。**
        """
        for command in (
            "gh pr create --title t --project deskcat",
            "gh pr create --title t -p deskcat --draft",
        ):
            with self.subTest(command=command):
                self.assertDenied(command, contains="--base")

    def test_pr_create_with_base_is_allowed(self):
        """`--base value`／`--base=value`／短縮形`-B`のいずれも通す。

        **短縮形は大文字の`-B`である**（`gh help pr create`で確認した）。
        """
        for command in (
            "gh pr create --title t -p deskcat --base develop",
            "gh pr create --title t -p deskcat --base=develop",
            "gh pr create --title t -p deskcat -B develop",
            "gh pr create --title t -p deskcat -Bdevelop",
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_issue_create_does_not_require_base(self):
        """`gh issue create`へ`--base`を要求しない。

        **`gh issue create`の option 一覧に`--base`は存在しない**
        （`gh help issue create`で確認した。出現0件）。要求すると、満たしようのない
        条件で起票が止まる。
        """
        self.assertAllowed("gh issue create --title t --project deskcat")

    def test_base_short_form_is_case_sensitive(self):
        """`-b`（`--body`）を`-B`（`--base`）と読まない。

        `gh pr create`は両方を持ち、意味が違う。**大小を区別しないと、本文を渡した
        だけのcommandがbase指定として通る。**
        """
        for command in (
            "gh pr create --title t -p deskcat -b 本文",
            "gh pr create --title t -p deskcat --body 本文",
        ):
            with self.subTest(command=command):
                self.assertDenied(command, contains="--base")

    def test_base_value_is_not_judged(self):
        """**`--base`の値の正しさを見ない。**存在するかだけを見る。

        `develop`と書くべきか`main`と書くべきかは、そのPull Requestの目的で決まり、
        字句からは読めない。昇格Pull Requestのbaseは`main`が正しい。
        """
        for command in (
            "gh pr create --title t -p deskcat --base main",
            "gh pr create --title t -p deskcat --base 実在しないbranch",
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_help_requires_no_metadata(self):
        """helpの表示だけを求める呼び出しへ、`--project`も`--base`も要求しない。

        **何も作らない呼び出しである。**boardへのitem追加漏れは起きようがない。
        **ここを拒否すると、hookが要求しているoption名を`--help`で調べる手段そのものが
        塞がる。**2026-08-28に`gh issue create --help`が実際に拒否され、
        `gh help issue create`へ回避してoption一覧を確認した。
        **規則を守るために要る情報を、規則が隠している状態だった。**
        """
        for command in (
            "gh pr create --help",
            "gh issue create --help",
            "gh pr create -h",
            "gh issue create -h",
            "gh pr create -p deskcat --help",
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_help_needs_no_merge_message(self):
        """`gh pr merge --help`へsquash messageを要求しない。

        **helpの表示はmergeしない。**この検査でとりわけ効く。`--subject`と
        `--body-file`の明示をCONTRIBUTINGが要求しているのに、**そのoption名を
        `--help`で確認できなかった**（2026-08-28に実測。拒否された）。
        """
        for command in (
            "gh pr merge --help",
            "gh pr merge -h",
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_help_is_matched_only_as_a_whole_word(self):
        """`-h`の連結判定をしない。`-help`や`-hello`をhelpと読まない。

        `_has_option`の連結判定を当てると、`-h`で始まる語がすべてhelpになる。
        **boolean flagに連結形は無い。**
        """
        for command in (
            "gh pr create --title t -hello",
            "gh issue create --title t -help",
        ):
            with self.subTest(command=command):
                self.assertDenied(command, contains="--project")
        self.assertDenied("gh pr merge 1 --squash -hello", contains="本文")

    def test_compound_command_is_inspected(self):
        """`cd x && gh pr create`を見落とさない。

        `if`条件で`Bash(gh *)`へ絞ると、この形が素通りする。絞らない理由である。
        """
        self.assertDenied("cd /tmp && gh pr create --title t")

    def test_absolute_path_invocation_is_inspected(self):
        """`/usr/local/bin/gh`のような絶対pathでの呼び出しも見る。"""
        self.assertDenied("/usr/local/bin/gh pr create --title t")

    def test_option_after_separator_is_not_credited(self):
        """区切りより後ろの語を、直前の`gh`の引数として数えない。"""
        self.assertAllowed("gh pr view 1 && echo --project")

    def test_merge_without_message_is_denied(self):
        """messageを渡さないmergeを止める。GitHubが合成したmessageでは経緯が残らない。"""
        self.assertDenied("gh pr merge 1 --squash", contains="afc86cf")

    def test_merge_with_unreadable_message_is_denied(self):
        """本文を読めない経路を「入っている」と扱わない。"""
        self.assertDenied("gh pr merge 1 --squash -F -", contains="stdin")
        self.assertDenied(
            "gh pr merge 1 --squash --body-file /nonexistent/body.txt",
            contains="読み出しに失敗",
        )

    def test_merge_message_without_trailers_is_allowed(self):
        """messageの中身は見ない。trailerが無くても、`--subject`と本文があれば通す。"""
        self.assertAllowed('gh pr merge 1 --squash --subject s --body "本文だけ"')
        self.assertAllowed('gh pr merge 1 --subject s --body "本文だけ"')
        self.assertAllowed('gh pr merge 1 --rebase --subject s --body "本文だけ"')

    def test_merge_strategy_allows_main_promotion(self):
        """`--merge`（`main`昇格）も、`--subject`と本文があれば通す。"""
        self.assertAllowed(
            'gh pr merge 383 --merge --subject "s" --body "develop を main へ昇格する"'
        )
        self.assertAllowed('gh pr merge 383 -m --subject s --body "本文"')

    def test_merge_strategy_still_requires_a_message(self):
        """**`--merge`でも、messageの指定そのものは省略できない。**

        省略するとGitHubが既定messageを合成する。`afc86cf`（`#383`）で実際に起きた損失である。
        """
        self.assertDenied(
            "gh pr merge 383 --merge", contains="messageを確認できない"
        )
        self.assertDenied(
            "gh pr merge 383 -m --subject s", contains="messageを確認できない"
        )

    def test_merge_strategy_requires_subject_not_just_body(self):
        """**`--merge`は`--body`だけでは足りない。`--subject`も要る。**

        `_body_text`は`--body`／`--body-file`の有無しか見ておらず、
        `gh pr merge N --merge --body "本文"`（`--subject`無し）が
        素通りしていた。`CONTRIBUTING.md`の
        `Merge方式`は`--subject`と`--body-file`の両方を明示すると定めている。
        2026-09-17にCodeRabbitのreviewが指摘し、push前に直した。

        **本文自体は読める形にする。**bodyが特定できない場合のdeny理由にも
        たまたま`--subject`という文字列が含まれるため、それだけでは
        「`--subject`欠如を専用に検出しているか」を確かめたことにならない。
        """
        with tempfile.TemporaryDirectory() as directory:
            body = Path(directory) / "body.txt"
            body.write_text("develop を main へ昇格する", encoding="utf-8")
            self.assertDenied(
                f"gh pr merge 383 --merge --body-file {body}",
                contains="`--subject`が無い",
            )
            self.assertDenied(
                f"gh pr merge 383 -m --body-file {body}",
                contains="`--subject`が無い",
            )
            self.assertAllowed(
                f'gh pr merge 383 --merge --subject "s" --body-file {body}'
            )
            self.assertAllowed(f"gh pr merge 383 -m -t s -F {body}")

    def test_squash_strategy_requires_subject_too(self):
        """**`--squash`も`--subject`が無ければ拒否する（`#423`）。**

        strategyを問わず`--subject`を見る。
        """
        self.assertDenied(
            'gh pr merge 1 --squash --body '
            '"x\n\n本文"',
            contains="`--subject`が無い",
        )
        with tempfile.TemporaryDirectory() as directory:
            body = Path(directory) / "body.txt"
            body.write_text(
                "x\n\n本文",
                encoding="utf-8",
            )
            # 短縮形（`-s`／`-t`）でも同じ判定になることを確かめる。
            self.assertDenied(
                f"gh pr merge 1 -s --body-file {body}",
                contains="`--subject`が無い",
            )
            self.assertAllowed(f"gh pr merge 1 -s -t s --body-file {body}")

    def test_subject_short_form_with_attached_value_is_accepted(self):
        """**`-ts`（`-t`＋値の結合形）でも`--subject`ありと判定する。**

        `gh`の`-t`はpflagの短縮string flagであり、`-ts`は`-t s`と同じ意味に
        なる。`_option_value`はこの結合形を読まず、`-ts`を指定していても
        `--subject`が無いと誤判定していた（2026-09-17のCodeRabbit reviewが
        `#425`で指摘）。**値は使わず有無だけを見るため、`_option_value`ではなく
        `_has_option`で判定する。**
        """
        with tempfile.TemporaryDirectory() as directory:
            body = Path(directory) / "body.txt"
            body.write_text(
                "x\n\n本文",
                encoding="utf-8",
            )
            self.assertAllowed(f"gh pr merge 1 --squash -ts --body-file {body}")
            self.assertAllowed(
                f"gh pr merge 1 --merge -ts --body-file {body}"
            )

    def test_unrelated_commands_are_allowed(self):
        """`gh`以外と、`gh`の他のsubcommandは通す。"""
        for command in ("git status", "ls -la", "gh pr view 1 --json state"):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_skip_environment_disables_the_guard(self):
        """逃げ道が効く。**効かない逃げ道は、hookごと無効化される。**"""
        code, output = _invoke(
            GH_GUARD, "gh pr create --title t",
            environment={"DESKCAT_SKIP_GH_GUARD": "1"},
        )
        self.assertEqual(code, 0)
        self.assertIsNone(output)

    def test_skip_environment_disables_the_body_section_check(self):
        """決定3の節検査も`DESKCAT_SKIP_GH_GUARD`で無効化される。

        `main()`の先頭で全検査を一括して止める既存の仕組みに乗っているだけだが、
        新設した検査が実際にその経路を通ることは別途確認する必要がある。
        （通常なら節が欠けてdenyされる本文で確認する。）
        """
        code, output = _invoke(
            GH_GUARD,
            'gh issue create --title t --project deskcat '
            '--label type:maintenance --body "## 背景\nx\n"',
            environment={"DESKCAT_SKIP_GH_GUARD": "1"},
        )
        self.assertEqual(code, 0)
        self.assertIsNone(output)

    def test_unreadable_input_blocks_and_broken_command_does_not(self):
        """読めない入力は exit 2 で止める（前は通していた。#814 の作業リストの9）。"""
        result = subprocess.run(
            [sys.executable, GH_GUARD], input="{ではないJSON",
            capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=60,
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("gh_metadata_guard", result.stderr)
        # 既定の文字コードが cp932 でも、UTF-8 の日本語の command を読んで検査する。
        payload = json.dumps({"tool_name": "Bash", "tool_input": {
            "command": 'gh pr merge 7 --squash --subject "日本語の件名 ✓"'}}, ensure_ascii=False)
        env = {k: v for k, v in os.environ.items() if not k.startswith("DESKCAT_")}
        env.update({"PYTHONIOENCODING": "cp932", "PYTHONUTF8": "0",
                    "PATH": f"{_GH_SHIM_DIR}{os.pathsep}{env['PATH']}"})
        result = subprocess.run(
            [sys.executable, GH_GUARD], input=payload.encode("utf-8"), capture_output=True,
            timeout=120, env=env, cwd=str(REPO_ROOT_FOR_TEMPLATES),
        )
        # 本文が無い merge なので、入力を読めていれば deny になる（前は読めずに何も出さずに通した）。
        self.assertEqual(result.returncode, 0)
        decision = json.loads(result.stdout.decode("utf-8"))
        self.assertEqual(decision["hookSpecificOutput"]["permissionDecision"], "deny")
        # 引用符が閉じていないcommandは、shell自身が落とす。ここで二重に報告しない。
        self.assertAllowed('gh pr create --title "閉じていない')

    # --- 決定3（#348）: `--body`系の本文がtemplateの節を欠いていないかの検査 ---

    def test_body_option_absent_is_not_checked(self):
        """`--body`系が無い呼び出しは節検査の対象外である。

        対話prompt（またはeditor）経由はtemplateを経由しており、迂回にならない。
        `--template`も`type:*` labelも無いのに通ることが、検査が本文の有無で
        分岐していることを示す。
        """
        self.assertAllowed("gh issue create --title t --project deskcat")

    def test_issue_body_with_all_sections_is_allowed_via_template_name(self):
        """`--template`（表示名）で指定したtemplateの全節が揃った本文を通す。"""
        sections = _template_sections(ISSUE_TEMPLATE_DIR / "maintenance_task.md")
        body = _body_with_sections(sections)
        self.assertAllowed(
            "gh issue create --title t --project deskcat "
            f'--template 保守作業 --body "{body}"'
        )

    def test_issue_body_with_all_sections_is_allowed_via_type_label(self):
        """`--template`が無くても、`type:*` labelから一意にtemplateを決めて通す。"""
        sections = _template_sections(ISSUE_TEMPLATE_DIR / "maintenance_task.md")
        body = _body_with_sections(sections)
        self.assertAllowed(
            "gh issue create --title t --project deskcat "
            f'--label type:maintenance --body "{body}"'
        )

    def test_issue_body_missing_sections_is_denied(self):
        """節が欠けたIssue本文を止め、欠けている節名を名指しする。"""
        sections = _template_sections(ISSUE_TEMPLATE_DIR / "maintenance_task.md")
        body = _body_with_sections(sections[:2])  # 先頭2節だけにする
        self.assertDenied(
            "gh issue create --title t --project deskcat "
            f'--label type:maintenance --body "{body}"',
            contains=sections[-1],
        )

    def test_pr_body_with_all_sections_is_allowed(self):
        """`gh pr create`はtemplateが1つだけなので、`--template`無しでも判定できる。"""
        sections = _template_sections(PR_TEMPLATE_PATH)
        body = _body_with_sections(sections)
        self.assertAllowed(
            "gh pr create --title t --project deskcat --base develop "
            f'--body "{body}"'
        )

    def test_pr_body_missing_sections_is_denied(self):
        """節が欠けたPull Request本文を止める。"""
        sections = _template_sections(PR_TEMPLATE_PATH)
        body = _body_with_sections(sections[:1])
        self.assertDenied(
            "gh pr create --title t --project deskcat --base develop "
            f'--body "{body}"',
            contains=sections[-1],
        )

    def test_undeterminable_template_is_denied(self):
        """`--template`も対応する`type:*` labelも無いIssue本文は、素通りさせない。

        **fail-openを選ばない。**存在検査ではなく内容検査へ変えた理由そのものが、
        「決められないなら安全側で止める」である。素通りは選択肢に無い。
        """
        self.assertDenied(
            'gh issue create --title t --project deskcat --body "## 背景\nx\n"',
            contains="決められない",
        )

    def test_ambiguous_type_labels_are_denied(self):
        """複数の`type:*` labelが異なるtemplateへ一致する場合も決められないとする。"""
        self.assertDenied(
            "gh issue create --title t --project deskcat "
            '--label type:bug --label type:maintenance --body "## 背景\nx\n"',
            contains="決められない",
        )

    def test_unknown_template_name_is_denied(self):
        """`--template`の値がどのtemplateの`name:`とも一致しない場合を通さない。"""
        self.assertDenied(
            "gh issue create --title t --project deskcat "
            '--template 存在しない名前 --body "## 背景\nx\n"',
            contains="--template",
        )

    def test_unreadable_create_body_is_denied(self):
        """`gh issue create`／`gh pr create`でも、本文を読めない経路を素通りさせない。

        [#267](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/267)で
        `coderabbit_gate.py`が同じ理由でfail-openをやめた前例と同じ扱いにする。
        """
        self.assertDenied(
            "gh issue create --title t --project deskcat -F -", contains="stdin"
        )
        self.assertDenied(
            "gh pr create --title t --project deskcat --base develop "
            "--body-file /nonexistent/body.txt",
            contains="読み出しに失敗",
        )

    def test_help_skips_body_check(self):
        """`--help`は`--body`が付いていても節検査をしない。何も作らないためである。"""
        self.assertAllowed(
            'gh issue create --title t --body "## 背景\nx\n" --help'
        )


class CodeRabbitReviewMergeGateTests(unittest.TestCase):
    """対象fileを含むPull Requestは、CodeRabbitのreviewが完走するまでmergeさせない（#688）。"""

    MERGE = "gh pr merge 7 --squash --subject s --body-file {body}"
    DONE = {"name": "CodeRabbit", "status": "completed", "conclusion": "success",
            "output": {"title": "Finished", "summary": "Result: Success - Review completed"}}
    SKIPPED = {"name": "CodeRabbit", "status": "completed", "conclusion": "skipped",
               "output": {"title": "Review skipped", "summary": "Result: Skipped - Review skipped"}}
    LIMITED = {"name": "CodeRabbit", "status": "completed", "conclusion": "success",
               "output": {"title": "Review rate limited", "summary": "Rate limited"}}
    PENDING = {"name": "CodeRabbit", "status": "in_progress", "conclusion": None,
               "output": {"title": "Review in progress", "summary": ""}}

    def _run(self, fixture, command=None, environment=None):
        with tempfile.TemporaryDirectory() as directory:
            body = Path(directory) / "body.txt"
            body.write_text(
                "本文\n",
                encoding="utf-8",
            )
            env = {"DESKCAT_TEST_GH": json.dumps(fixture)}
            env.update(environment or {})
            return _invoke(
                GH_GUARD, (command or self.MERGE).format(body=body), environment=env
            )

    def assertDenied(self, fixture, *contains, **kwargs):
        code, output = self._run(fixture, **kwargs)
        self.assertEqual(code, 0)
        self.assertIsNotNone(output, "通してしまった")
        self.assertEqual(output["hookSpecificOutput"]["permissionDecision"], "deny")
        for text in contains:
            self.assertIn(text, _reason(output))

    def assertAllowed(self, fixture, **kwargs):
        code, output = self._run(fixture, **kwargs)
        self.assertEqual(code, 0)
        self.assertIsNone(output, f"止めてしまった: {output}")

    def _pr(self, files, **commits):
        return {"files": files, "commits": {
            sha: {"check_runs": runs} for sha, runs in commits.items()}}

    def test_every_target_path_is_checked(self):
        """対象のどれか1つに当たればdenyし、理由に当たったfileと状態を出す。"""
        targets = [
            "firmware/esp32/src/main.rs",
            "crates/deskcat-protocol/src/lib.rs",
            "crates/deskcat-servo/src/lib.rs",
            "docs/protocol/esp32-pi-protocol.md",
            "docs/hardware/gpio-assignment.md",
            "docs/hardware/power-budget.md",
            "docs/hardware/servo-safety-limits.md",
            "docs/governance/hardware-safety-policy.md",
            "docs/governance/ai-agent-policy.md",
            "docs/governance/development-workflow.md",
            ".github/workflows/pages.yml",
            "scripts/hooks/gh_metadata_guard.py",
            ".githooks/pre-commit",
            ".claude/settings.json",
            "AGENTS.md",
            "CLAUDE.md",
            "CONTRIBUTING.md",
            "hardware/deskcat.kicad_pcb",
            "hardware/deskcat.kicad_sch",
            "hardware/DeskCat.kicad_sym",
            "hardware/DeskCat.pretty/R_Vertical_P5.08.kicad_mod",
            "hardware/overview/deskcat-netlist.xml",
            "hardware/breadboard/layout.svg",
            "hardware/breadboard-rebuild/generate.py",
            "hardware/universal-board/check_star_point.py",
            "hardware/deskcat.kicad_pro",
            "hardware/fp-lib-table",
            "hardware/sym-lib-table",
            "scripts/check_wiring_figure_sync.py",
        ]
        for name in targets:
            with self.subTest(name=name):
                self.assertDenied(
                    self._pr(["README.md", name], a=[self.SKIPPED]),
                    f"`{name}`", "Review skipped",
                )

    def test_files_outside_the_targets_are_allowed(self):
        """対象外は、CodeRabbitの状態によらず通す（広げない範囲）。"""
        outside = [
            "docs/hardware/bom.md", "docs/experiments/exp-1.md",
            "scripts/README.md", "scripts/test_hooks.py", "docs/CONTRIBUTING.md",
            "crates/deskcat-pi/src/lib.rs", "crates/deskcat-protocol/Cargo.toml",
            "crates/deskcat-face/src/lib.rs", "crates/deskcat-env/src/lib.rs",
            "firmware/esp32/Cargo.toml", "firmware/esp32/srcx/a.rs",
            "hardware/README.md", "hardware/measurement/test_adc_stream_rate.py",
            "hardware/wokwi/esp32-boot-pulls/diagram.json",
            "hardware/bench-18/layout.svg",
        ]
        for name in outside:
            with self.subTest(name=name):
                self.assertAllowed(self._pr([name], a=[self.SKIPPED]))

    def test_not_completed_states_are_denied(self):
        """完走でない状態は、すべてdenyする。"""
        for label, run in (
            ("skipped", self.SKIPPED), ("rate limited", self.LIMITED),
            ("in progress", self.PENDING),
        ):
            with self.subTest(label=label):
                self.assertDenied(
                    self._pr(["firmware/esp32/src/a.rs"], a=[run]),
                    run["output"]["title"],
                )

    def test_no_coderabbit_record_is_denied(self):
        self.assertDenied(
            self._pr(["firmware/esp32/src/a.rs"], a=[]), "CodeRabbitの記録なし"
        )

    def test_completed_review_allows_the_merge(self):
        self.assertAllowed(self._pr(["firmware/esp32/src/a.rs"], a=[self.DONE]))

    def test_commit_added_after_the_review_is_allowed(self):
        """**review完走の後に対応commitを足し、headが`Review skipped`でも通す。**

        CONTRIBUTINGは対応commitで投げ直さないと定める（#678の`431ca0e`→`0629fc7`）。
        """
        self.assertAllowed(
            self._pr(["firmware/esp32/src/a.rs"], a=[self.SKIPPED],
                     b=[self.DONE], c=[self.SKIPPED])
        )

    def test_legacy_commit_status_counts_as_completed(self):
        fixture = {"files": ["docs/protocol/esp32-pi-protocol.md"], "commits": {
            "a": {"statuses": [{"context": "CodeRabbit", "state": "success",
                                "description": "Review completed"}]}}}
        self.assertAllowed(fixture)

    def test_legacy_skipped_status_is_denied(self):
        fixture = {"files": ["docs/protocol/esp32-pi-protocol.md"], "commits": {
            "a": {"statuses": [{"context": "CodeRabbit", "state": "success",
                                "description": "Review skipped: automatic reviews are disabled"}]}}}
        self.assertDenied(fixture, "Review skipped")

    def test_promotion_is_checked_like_any_other(self):
        """base `main`の昇格（`--merge`）にも同じ検査を掛ける。"""
        command = "gh pr merge 7 --merge --subject s --body-file {body}"
        self.assertDenied(
            self._pr(["firmware/esp32/src/a.rs"], a=[self.SKIPPED]),
            "firmware/esp32/src/a.rs", command=command,
        )
        self.assertAllowed(
            self._pr(["firmware/esp32/src/a.rs"], a=[self.DONE]), command=command
        )

    def test_many_commits_are_checked(self):
        """昇格は100件を超えるcommitを持つ。完走が最古にあっても見つける。"""
        commits = {f"c{i:03d}": [self.SKIPPED] for i in range(120)}
        commits["c000"] = [self.DONE]
        self.assertAllowed(self._pr(["firmware/esp32/src/a.rs"], **commits))
        del commits["c000"]
        self.assertDenied(self._pr(["firmware/esp32/src/a.rs"], **commits), "Review skipped")

    def test_gh_failure_is_denied(self):
        self.assertDenied({"fail": True}, "確認できないまま通さない")

    def test_skip_environment_does_not_disable_this_check(self):
        """解除の手段を足さない。`DESKCAT_SKIP_GH_GUARD=1`でも止まる。"""
        self.assertDenied(
            self._pr(["firmware/esp32/src/a.rs"], a=[self.SKIPPED]),
            "Review skipped", environment={"DESKCAT_SKIP_GH_GUARD": "1"},
        )

    def test_help_is_not_checked(self):
        self.assertAllowed({"fail": True}, command="gh pr merge --help")


class GhHookRepositoryAndDeadlineTests(unittest.TestCase):
    """`-Rowner/repo`の読み取りと、hook全体の期限（#751）。"""

    DONE = CodeRabbitReviewMergeGateTests.DONE
    TARGET = "firmware/esp32/src/a.rs"
    DRIVER = (
        "import sys; sys.path.insert(0, {hooks!r}); import gh_metadata_guard as g; "
        "{overrides}sys.exit(g.main())"
    )

    def _logged(self, command, fixture):
        with tempfile.TemporaryDirectory() as directory:
            log = Path(directory) / "gh.log"
            body = Path(directory) / "body.txt"
            body.write_text(
                "本文\n",
                encoding="utf-8",
            )
            _invoke(GH_GUARD, command.format(body=body), environment={
                "DESKCAT_TEST_GH": json.dumps({**fixture, "log": str(log)})})
            return [json.loads(line) for line in log.read_text().splitlines()]

    def test_attached_repo_option_selects_the_repository(self):
        """`-Rowner/repo`でも、照会へ`-R owner/repo`を渡す。"""
        issue = {"issue": {"number": 9, "body": "## 受け入れ条件\n- [ ] a\n"}}
        pr = {"files": [self.TARGET], "commits": {"a": {"check_runs": [self.DONE]}}}
        for command, fixture, view in (
            ("gh issue close 9 -Rowner/target", issue, ["issue", "view"]),
            ("gh issue close 9 -R owner/target", issue, ["issue", "view"]),
            ("gh pr merge 7 --squash -Rowner/target --subject s --body-file {body}", pr, ["pr", "view"]),
            ("gh pr merge 7 --squash -R owner/target --subject s --body-file {body}", pr, ["pr", "view"]),
        ):
            with self.subTest(command=command):
                # 回数は数えない。merge の前は、検査ごとに `pr view` を呼ぶ（#814 の作業リストの9）。
                # どの照会も同じ repository を見ることを確かめる。
                calls = [c for c in self._logged(command, fixture) if c[:2] == view]
                self.assertTrue(calls)
                for call in calls:
                    self.assertIn("-R", call)
                    self.assertEqual(call[call.index("-R") + 1], "owner/target")

    def test_attached_repo_text_in_a_value_position_is_not_a_repository(self):
        """`--comment -Rother/repo`の`-R...`は値であり、repositoryではない。"""
        issue = {"issue": {"number": 9, "body": "## 受け入れ条件\n- [ ] a\n"}}
        pr = {"files": [self.TARGET], "commits": {"a": {"check_runs": [self.DONE]}}}
        for command, fixture, view in (
            ("gh issue close 9 --comment -Rother/repo", issue, ["issue", "view"]),
            ("gh pr merge 7 --squash --subject -Rother/repo --body-file {body}", pr, ["pr", "view"]),
        ):
            with self.subTest(command=command):
                calls = [c for c in self._logged(command, fixture) if c[:2] == view]
                self.assertTrue(calls)
                for call in calls:
                    self.assertNotIn("-R", call)

    def test_option_name_used_as_a_value_does_not_hide_a_real_repo_option(self):
        """`--comment -c`の`-c`は値。続く`-Rowner/target`は本物の`-R`である。"""
        issue = {"issue": {"number": 9, "body": "## 受け入れ条件\n- [ ] a\n"}}
        command = "gh issue close 9 --comment -c -Rowner/target"
        calls = [c for c in self._logged(command, issue) if c[:2] == ["issue", "view"]]
        self.assertEqual(len(calls), 1, calls)
        self.assertEqual(calls[0][calls[0].index("-R") + 1], "owner/target")

    def test_short_options_of_one_subcommand_do_not_hide_the_repository_of_another(self):
        """`pr merge`の`-r`は`--rebase`（値なし）。`issue close`の`-r`（`--reason`）と混ぜない。"""
        pr = {"files": [self.TARGET], "commits": {"a": {"check_runs": [self.DONE]}}}
        command = "gh pr merge 7 -r -Rowner/target --subject s --body-file {body}"
        calls = [c for c in self._logged(command, pr) if c[:2] == ["pr", "view"]]
        self.assertTrue(calls)
        for call in calls:
            self.assertEqual(call[call.index("-R") + 1], "owner/target")

    def _timed(self, fixture, deadline=None, timeout=10):
        """`deadline`が`None`なら、hookの本物の上限（定数）のまま実行する。"""
        overrides = "" if deadline is None else (
            f"g.GH_TIMEOUT_SECONDS = {timeout}; g.CHECK_DEADLINE_SECONDS = {deadline}; ")
        driver = self.DRIVER.format(
            hooks=str(SCRIPTS_ROOT / "hooks"), overrides=overrides)
        env = dict(os.environ, DESKCAT_TEST_GH=json.dumps(fixture))
        env["PATH"] = f"{_GH_SHIM_DIR}{os.pathsep}{env['PATH']}"
        with tempfile.TemporaryDirectory() as directory:
            body = Path(directory) / "body.txt"
            body.write_text(
                "本文\n",
                encoding="utf-8",
            )
            payload = json.dumps({"tool_name": "Bash", "tool_input": {
                "command": f"gh pr merge 7 --squash --subject s --body-file {body}"}})
            start = time.monotonic()
            # `_invoke` と同じく repository の中で走らせる。CI は test を repository の外
            # （`runner.temp`）で走らせるので、そのままでは shim が HEAD を読めず、merge の前の
            # レビューの記録の検査が base と head を空として止める（#814 の作業リストの9）。
            result = subprocess.run([sys.executable, "-c", driver], input=payload,
                                    capture_output=True, text=True, env=env, timeout=60,
                                    cwd=str(REPO_ROOT_FOR_TEMPLATES))
        text = result.stdout.strip()
        return time.monotonic() - start, (json.loads(text) if text else None)

    def _fixture(self, **extra):
        return {"files": [self.TARGET], "commits": {"a": {"check_runs": [self.DONE]}}, **extra}

    def test_deadline_starts_before_the_first_lookup(self):
        """3回の事前照会が各1.2秒でも、全体の期限（3秒）を超えたらdenyする。"""
        elapsed, output = self._timed(self._fixture(sleep=1.2), deadline=3)
        self.assertIsNotNone(output, "期限を超えたのに通した")
        self.assertEqual(output["hookSpecificOutput"]["permissionDecision"], "deny")
        self.assertLess(elapsed, 5.5)

    def test_cold_first_lookup_over_ten_seconds_still_passes(self):
        """しばらく照会しない後の最初のghが11秒かかっても、確認が通れば通す（#760）。

        本物の定数で走らせる。実測した遅れは10.4〜10.7秒で、上限が10秒だと
        これだけで確認できないとしてdenyしていた。
        """
        with tempfile.TemporaryDirectory() as directory:
            fixture = self._fixture(
                cold_marker=str(Path(directory) / "cold"), cold_sleep=11)
            elapsed, output = self._timed(fixture)
        self.assertIsNone(output, f"最初の遅れだけで止めた: {output}")
        self.assertGreaterEqual(elapsed, 11)
        self.assertLess(elapsed, guard.CHECK_DEADLINE_SECONDS)

    def test_hook_limits_fit_inside_the_hook_timeout(self):
        """1回の上限は最初の遅れ（約10.7秒）を超える。全体の期限は、hookの30秒に収まる。"""
        self.assertGreater(guard.GH_TIMEOUT_SECONDS, 10.7)
        self.assertLessEqual(guard.GH_TIMEOUT_SECONDS, guard.CHECK_DEADLINE_SECONDS)
        self.assertLess(guard.CHECK_DEADLINE_SECONDS, 30)

    def test_running_workers_end_by_the_deadline(self):
        """commit照会が期限を超えて走り続けても、processは期限の直後に終わる。"""
        elapsed, output = self._timed(self._fixture(sleep_commits=6), deadline=2)
        self.assertIsNotNone(output)
        self.assertEqual(output["hookSpecificOutput"]["permissionDecision"], "deny")
        self.assertLess(elapsed, 4.5)


class AcceptanceCriteriaCloseGateTests(unittest.TestCase):
    """受け入れ条件に未チェックが残るIssueをcloseさせない（#693）。"""

    CLOSE = "gh issue close 9"
    DONE_EDIT = ("gh project item-edit --id PVTI_x --project-id P --field-id F"
                 " --single-select-option-id 98236657")
    OPEN_BODY = "## 背景\nb\n\n## 受け入れ条件\n\n- [x] 済み\n- [ ] 残り1\n- [ ]\n\n## 検証\n- [ ] 範囲外\n"
    DONE_BODY = "## 受け入れ条件\n\n- [x] 済み\n- [X] これも\n\n## 検証\n- [ ] 範囲外\n"

    def _run(self, fixture, command, environment=None):
        return _invoke(
            GH_GUARD, command,
            environment={"DESKCAT_TEST_GH": json.dumps(fixture), **(environment or {})},
        )

    def assertDenied(self, fixture, command, *contains, **kwargs):
        code, output = self._run(fixture, command, **kwargs)
        self.assertEqual(code, 0)
        self.assertIsNotNone(output, "通してしまった")
        self.assertEqual(output["hookSpecificOutput"]["permissionDecision"], "deny")
        for text in contains:
            self.assertIn(text, _reason(output))

    def assertAllowed(self, fixture, command):
        code, output = self._run(fixture, command)
        self.assertEqual(code, 0)
        self.assertIsNone(output, f"止めてしまった: {output}")

    def _issue(self, body, number=9):
        return {"issue": {"number": number, "body": body}}

    def _item(self, typename="Issue", body="", state="OPEN", number=9):
        content = {"__typename": typename}
        if typename == "Issue":
            content.update(number=number, body=body, state=state)
        return {"item": {"content": content}}

    def test_close_with_unchecked_items_is_denied_and_lists_them(self):
        self.assertDenied(
            self._issue(self.OPEN_BODY), self.CLOSE,
            "#9", "2件", "`残り1`", "`（空の項目）`",
        )

    def test_unchecked_items_outside_the_section_are_not_counted(self):
        code, output = self._run(self._issue(self.OPEN_BODY), self.CLOSE)
        self.assertNotIn("範囲外", _reason(output))

    def test_close_with_all_checked_is_allowed(self):
        self.assertAllowed(self._issue(self.DONE_BODY), self.CLOSE)

    def test_issue_without_the_section_is_denied(self):
        self.assertDenied(
            self._issue("## 背景\n- [x] a\n"), self.CLOSE, "受け入れ条件`の節が無く",
        )

    def test_section_ends_at_the_next_level2_heading_only(self):
        body = "## 受け入れ条件\n- [x] a\n### 補足\n- [ ] 下位見出しの中\n## 検証\n"
        self.assertDenied(self._issue(body), self.CLOSE, "下位見出しの中")

    def test_heading_variants_start_the_section(self):
        """実例: `## この Issue の受け入れ条件・成果物`（#546）、`### 測定可能な受け入れ条件`（#236）。"""
        for heading in ("## この Issue の受け入れ条件・成果物", "### 測定可能な受け入れ条件"):
            body = f"## 背景\n- [ ] 範囲外\n\n{heading}\n- [x] 済み\n- [ ] 残り\n"
            with self.subTest(heading=heading):
                self.assertDenied(self._issue(body), self.CLOSE, "`残り`")
                self.assertAllowed(
                    self._issue(body.replace("- [ ] 残り", "- [x] 残り")), self.CLOSE
                )

    def test_level3_section_ends_at_a_level2_heading(self):
        body = "### 測定可能な受け入れ条件\n- [x] a\n## 検証\n- [ ] 範囲外\n"
        self.assertAllowed(self._issue(body), self.CLOSE)

    def test_not_planned_with_a_comment_is_allowed(self):
        for command in (
            "gh issue close 9 --reason 'not planned' --comment 方針",
            "gh issue close 9 -r 'not planned' -c 方針",
        ):
            with self.subTest(command=command):
                self.assertAllowed(self._issue(self.OPEN_BODY), command)

    def test_not_planned_without_a_comment_is_denied(self):
        for command in (
            "gh issue close 9 --reason 'not planned'",
            "gh issue close 9 -r 'not planned' -c ''",
            "gh issue close 9 --reason completed -c 方針",
            "gh issue close 9 -c 方針",
        ):
            with self.subTest(command=command):
                self.assertDenied(self._issue(self.OPEN_BODY), command, "未チェック")

    def test_done_is_not_exempted_by_not_planned(self):
        self.assertDenied(self._item(body=self.OPEN_BODY), self.DONE_EDIT, "未チェック")

    def test_options_and_repo_do_not_hide_the_issue(self):
        for command in (
            "gh issue close 9 --reason completed -c done",
            "gh --repo o/r issue close 9",
            "gh issue close -R o/r https://github.com/o/r/issues/9",
        ):
            with self.subTest(command=command):
                self.assertDenied(self._issue(self.OPEN_BODY), command, "未チェック")

    def test_done_on_an_issue_item_with_unchecked_items_is_denied(self):
        self.assertDenied(
            self._item(body=self.OPEN_BODY), self.DONE_EDIT, "#9", "`残り1`",
        )

    def test_done_on_an_issue_item_with_all_checked_is_allowed(self):
        self.assertAllowed(self._item(body=self.DONE_BODY), self.DONE_EDIT)

    def test_done_on_an_issue_item_without_the_section_is_denied(self):
        self.assertDenied(self._item(body="本文"), self.DONE_EDIT, "節が無く")

    def test_done_on_a_pull_request_item_is_allowed(self):
        self.assertAllowed(self._item("PullRequest"), self.DONE_EDIT)

    def test_done_on_a_draft_issue_item_is_allowed(self):
        self.assertAllowed(self._item("DraftIssue"), self.DONE_EDIT)

    def test_done_on_an_already_closed_issue_is_allowed(self):
        """`Done`が閉じるものが無い。boardの同期を止めない。"""
        self.assertAllowed(self._item(body=self.OPEN_BODY, state="CLOSED"), self.DONE_EDIT)

    def test_other_status_changes_are_allowed(self):
        """`Done`以外のStatusと、Status以外のfieldは、gh失敗でも見ない。"""
        for command in (
            "gh project item-edit --id I --project-id P --field-id F"
            " --single-select-option-id 47fc9ee4",
            "gh project item-edit --id I --project-id P --field-id F --date 2026-11-02",
        ):
            with self.subTest(command=command):
                self.assertAllowed({"fail": True}, command)

    def test_gh_failure_is_denied(self):
        for command in (self.CLOSE, self.DONE_EDIT):
            with self.subTest(command=command):
                self.assertDenied({"fail": True}, command, "確認できないまま通さない")

    def test_unreadable_responses_are_denied(self):
        self.assertDenied({"issue": {"number": 9}}, self.CLOSE, "確認できないまま通さない")
        self.assertDenied({"item": None}, self.DONE_EDIT, "確認できないまま通さない")
        self.assertDenied(
            {"item": {"content": None}}, self.DONE_EDIT, "確認できないまま通さない"
        )

    def test_missing_selector_or_item_id_is_denied(self):
        self.assertDenied(self._issue(self.DONE_BODY), "gh issue close", "確認できない")
        self.assertDenied(
            self._item(body=self.DONE_BODY),
            "gh project item-edit --project-id P --single-select-option-id 98236657",
            "確認できない",
        )

    def test_skip_environment_does_not_disable_this_check(self):
        for command, fixture in (
            (self.CLOSE, self._issue(self.OPEN_BODY)),
            (self.DONE_EDIT, self._item(body=self.OPEN_BODY)),
        ):
            with self.subTest(command=command):
                self.assertDenied(
                    fixture, command, "未チェック",
                    environment={"DESKCAT_SKIP_GH_GUARD": "1"},
                )

    def test_help_is_not_checked(self):
        self.assertAllowed({"fail": True}, "gh issue close --help")
        self.assertAllowed({"fail": True}, self.DONE_EDIT + " --help")


class BranchBaseGuardTests(unittest.TestCase):
    """branchの基点を見るhookのtest。fixture repositoryへ実際にfetchさせる。"""

    def setUp(self):
        self.directory = tempfile.mkdtemp()
        self.addCleanup(guards.remove_tree, Path(self.directory))
        root = self.directory
        _git(root, "init", "--quiet", ".")
        (Path(root) / "note.md").write_text("最初\n", encoding="utf-8")
        _git(root, "add", "note.md")
        _git(root, "commit", "--quiet", "-m", "first")
        self.old = _git(root, "rev-parse", "HEAD").strip()
        (Path(root) / "note.md").write_text("最初\n進んだ\n", encoding="utf-8")
        _git(root, "commit", "--quiet", "-am", "second")
        # 自分自身をoriginにする。networkへ出ずに`git fetch origin`を成立させる。
        _git(root, "branch", "--force", "trunk", "HEAD")
        _git(root, "remote", "add", "origin", root)
        _git(
            root, "config", "remote.origin.fetch",
            "+refs/heads/trunk:refs/remotes/origin/develop",
        )
        _git(root, "fetch", "--quiet", "origin")

    def _at_old_base(self):
        _git(self.directory, "checkout", "--quiet", "-B", "work", self.old)

    def _at_trunk(self):
        _git(self.directory, "checkout", "--quiet", "-B", "work", "trunk")

    def assertDenied(self, command):
        code, output = _invoke(BASE_GUARD, command, cwd=self.directory)
        self.assertEqual(code, 0, command)
        self.assertIsNotNone(output, f"通してしまった: {command}")
        self.assertIn("遅れている", _reason(output))

    def assertAllowed(self, command):
        code, output = _invoke(BASE_GUARD, command, cwd=self.directory)
        self.assertEqual(code, 0, command)
        self.assertIsNone(output, f"止めてしまった: {command}")

    def test_stale_base_is_denied(self):
        """遅れた基点からのbranch作成を止める。`switch -c`と絶対pathも同じに扱う。"""
        self._at_old_base()
        for command in (
            "git checkout -b chore/1-x",
            "git switch -c chore/1-x",
            "git switch --create chore/1-x",
            "/usr/bin/git checkout -b chore/1-x",
        ):
            with self.subTest(command=command):
                self.assertDenied(command)

    def test_separator_does_not_look_like_a_start_point(self):
        """`git checkout -b x && echo done`の`echo`を基点と読まない。

        基点を明示したものと誤って読むと、検査を飛ばしてしまう。
        """
        self._at_old_base()
        self.assertDenied("git checkout -b chore/1-x && echo done")

    def test_creation_on_a_later_line_is_denied(self):
        """複数行commandの2行目のbranch作成を止める（#389）。"""
        self._at_old_base()
        for command in (
            "git fetch origin\ngit checkout -b chore/1-x",
            "git status\ngit switch -c chore/1-x",
            "cat note.md\n\n  git checkout -b chore/1-x",
        ):
            with self.subTest(command=command):
                self.assertDenied(command)

    def test_creation_written_in_a_heredoc_body_is_allowed(self):
        """heredocのbodyの`git checkout -b`では止めない（#389）。**実行されない。**"""
        self._at_old_base()
        self.assertAllowed(
            "cat > note.md <<'EOF'\ngit checkout -b chore/1-x\nEOF\n"
        )

    def test_global_options_do_not_hide_the_creation(self):
        """**global optionでbranch作成の検査が抜けない**（#325）。

        `args[:2]`で位置から読んでいた版では、`git -C . checkout -b`と
        `git --no-pager checkout -b`が検査を素通りしていた。
        """
        self._at_old_base()
        for command in (
            "git -C . checkout -b chore/1-x",
            "git --no-pager checkout -b chore/1-x",
            "git -c user.name=x switch -c chore/1-x",
        ):
            with self.subTest(command=command):
                self.assertDenied(command)

    def test_current_base_is_allowed(self):
        """基点が`origin/develop`と一致していれば通す。"""
        self._at_trunk()
        self.assertAllowed("git checkout -b chore/1-x")

    def test_explicit_start_point_is_respected(self):
        """基点を明示しているなら、作成者が選んだものとして通す。"""
        self._at_old_base()
        self.assertAllowed("git checkout -b chore/1-x origin/develop")

    def test_hotfix_branch_is_out_of_scope(self):
        """`hotfix/`は`main`から作るのが正しい（ADR-0004）。"""
        self._at_old_base()
        self.assertAllowed("git checkout -b hotfix/1-x")

    def test_unrelated_git_commands_are_allowed(self):
        """branchを作らない操作は見ない。`git branch`もcheckoutを伴わない。"""
        self._at_old_base()
        for command in ("git status", "git branch chore/1-x", "git checkout trunk"):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_skip_environment_disables_the_guard(self):
        """逃げ道が効く。"""
        self._at_old_base()
        code, output = _invoke(
            BASE_GUARD, "git checkout -b chore/1-x", cwd=self.directory,
            environment={"DESKCAT_SKIP_BASE_GUARD": "1"},
        )
        self.assertEqual(code, 0)
        self.assertIsNone(output)


WORKTREE_GUARD = str(SCRIPTS_ROOT / "hooks" / "worktree_guard.py")


class WorktreeGuardTests(unittest.TestCase):
    """未commitの変更を失うgit操作を見るhookのtest（#325の候補1）。

    **fixture repositoryを実際に汚してから判定させる。**このhookは字句だけでは
    決まらず、`git status`の結果で向きが変わる。**汚れていないtreeで止めないことも、
    汚れたtreeで止めることと同じだけ見る必要がある**（止めすぎるhookは無効化される）。
    """

    def setUp(self):
        self.directory = tempfile.mkdtemp()
        self.addCleanup(guards.remove_tree, Path(self.directory))
        root = Path(self.directory)
        _git(self.directory, "init", "--quiet", ".")
        (root / "note.md").write_text("最初\n", encoding="utf-8")
        (root / "pages").mkdir()
        (root / "pages" / "assets-manifest.json").write_text("{}\n", encoding="utf-8")
        _git(self.directory, "add", "note.md", "pages/assets-manifest.json")
        _git(self.directory, "commit", "--quiet", "-m", "first")
        _git(self.directory, "branch", "pages")

    def _dirty_note(self):
        (Path(self.directory) / "note.md").write_text("書き換えた\n", encoding="utf-8")

    def _dirty_manifest(self):
        (Path(self.directory) / "pages" / "assets-manifest.json").write_text(
            '{"Assets": []}\n', encoding="utf-8"
        )

    def assertAsked(self, command, *, cwd=None, contains=None):
        code, output = _invoke(WORKTREE_GUARD, command, cwd=cwd or self.directory)
        self.assertEqual(code, 0, command)
        self.assertIsNotNone(output, f"通してしまった: {command}")
        self.assertEqual(
            output["hookSpecificOutput"]["permissionDecision"], "ask", command
        )
        if contains:
            self.assertIn(contains, _reason(output))

    def assertAllowed(self, command, *, cwd=None):
        code, output = _invoke(WORKTREE_GUARD, command, cwd=cwd or self.directory)
        self.assertEqual(code, 0, command)
        self.assertIsNone(output, f"止めてしまった: {command}")

    def test_reset_hard_with_a_dirty_tree_is_asked(self):
        """`git reset --hard`は作業tree全体を捨てる。"""
        self._dirty_note()
        for command in (
            "git reset --hard",
            "git reset --hard HEAD",
            "/usr/bin/git reset --hard",
            "git -C . reset --hard",
        ):
            with self.subTest(command=command):
                self.assertAsked(command)

    def test_global_options_do_not_hide_the_subcommand(self):
        """**subcommandの前のglobal optionで素通りしない**（#325で実際に開いていた穴）。

        `command_line.skip_global_options`が`-C`しか知らなかった版では、
        `git --no-pager reset --hard`と`git -c key=value reset --hard`は
        `remaining[0]`がoptionになり、**検査ごと素通りしていた**
        （2026-09-16実測）。
        """
        self._dirty_note()
        for command in (
            "git --no-pager reset --hard",
            "git -c user.name=x reset --hard",
            "git -c user.name=x -c user.email=y@z reset --hard",
            "git --literal-pathspecs clean -fd",
            "git --no-pager checkout -- note.md",
        ):
            with self.subTest(command=command):
                self.assertAsked(command)

    def test_a_dirty_tree_in_another_repository_is_seen(self):
        """**`-C`で指した先のtreeを見る。**hookのcwdではない。

        `prefix`を`git status`へ引き継がない実装に戻ると、cwd側のcleanなtreeを
        見て通してしまう。**`git -C .`だけのcaseではその退行を検出できない。**
        """
        other = tempfile.mkdtemp()
        self.addCleanup(guards.remove_tree, Path(other))
        _git(other, "init", "--quiet", ".")
        (Path(other) / "other.md").write_text("最初\n", encoding="utf-8")
        _git(other, "add", "other.md")
        _git(other, "commit", "--quiet", "-m", "first")
        (Path(other) / "other.md").write_text("汚した\n", encoding="utf-8")
        # 自分のtreeはcleanのまま。汚れているのは`-C`の先だけである。
        self.assertAsked(f"git -C {other} reset --hard")

    def test_value_options_select_the_tree_and_do_not_hide_the_subcommand(self):
        """`--git-dir X --work-tree Y reset --hard`は`Y`のtreeを見る（#472）。

        **`--git-dir`等を1語として飛ばしていた版では、値`X`を subcommand と読み、
        検査ごと素通りしていた。**読んだ値は`git status`へ引き継ぐため、
        `-C`と同じく指した先のtreeを見る。
        """
        other = tempfile.mkdtemp()
        self.addCleanup(guards.remove_tree, Path(other))
        _git(other, "init", "--quiet", ".")
        (Path(other) / "other.md").write_text("最初\n", encoding="utf-8")
        _git(other, "add", "other.md")
        _git(other, "commit", "--quiet", "-m", "first")
        (Path(other) / "other.md").write_text("汚した\n", encoding="utf-8")
        # 自分のtreeはcleanのまま。汚れているのは指した先だけである。
        for command in (
            f"git --git-dir {other}/.git --work-tree {other} reset --hard",
            f"git --git-dir={other}/.git --work-tree={other} reset --hard",
        ):
            with self.subTest(command=command):
                self.assertAsked(command)

    def test_reset_hard_with_a_clean_tree_is_allowed(self):
        """**汚れていなければ失うものが無い。**

        **このtestが確かめているのは「cleanなtreeなら通す」だけである。**
        refを引数に取る形も同じに通ることは見ているが、hookはrefを一切見ないため、
        **未pushのcommitの扱いを検証してはいない**（対象にしていないという決定は
        hookのdocstringが持つ）。
        """
        self.assertAllowed("git reset --hard")
        self.assertAllowed("git reset --hard HEAD~1")

    def test_other_reset_modes_are_allowed(self):
        """`--soft`／`--mixed`／`--keep`は作業treeのfileを捨てない。"""
        self._dirty_note()
        for command in (
            "git reset --soft HEAD~1",
            "git reset --mixed HEAD~1",
            "git reset --keep HEAD~1",
            "git reset note.md",
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_forced_checkout_and_switch_are_asked(self):
        """強制optionは作業tree全体を上書きする。"""
        self._dirty_note()
        for command in (
            "git checkout -f main",
            "git checkout --force main",
            "git switch -f main",
            "git switch --discard-changes main",
        ):
            with self.subTest(command=command):
                self.assertAsked(command)

    def test_checkout_of_a_dirty_path_is_asked(self):
        """pathを指したcheckout／restoreは、そのpathの変更を捨てる。"""
        self._dirty_note()
        for command in (
            "git checkout -- note.md",
            "git checkout note.md",
            "git checkout HEAD -- note.md",
            "git checkout .",
            "git restore note.md",
            "git restore --source=HEAD note.md",
            "git restore --staged --worktree note.md",
        ):
            with self.subTest(command=command):
                self.assertAsked(command)

    def test_checkout_of_a_clean_path_is_allowed(self):
        """同じ形でも、そのpathが汚れていなければ失うものが無い。"""
        self._dirty_note()
        self.assertAllowed("git restore pages")
        self.assertAllowed("git checkout -- pages")

    def test_switching_branches_is_allowed(self):
        """branchの移動はこのhookの対象ではない。

        **gitは未commitの変更を捨てない。**持ち越せるなら持ち越し、衝突するなら
        拒否する。**どちらでも失われない**ため、強制optionの無い移動は見ない。
        """
        self._dirty_note()
        for command in (
            "git checkout main",
            "git switch main",
            "git checkout -b chore/325-x",
            "git switch -c chore/325-x",
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_a_branch_named_like_a_dirty_directory_is_allowed(self):
        """**`git checkout -b pages`の`pages`をpathspecと読まない。**

        読むと、`pages/`が汚れている限りbranch作成が毎回askになる。
        `git checkout`では`VALUE_OPTIONS`がoptionの値を落とすことで避けている。
        `git switch`はそもそもpathspecを取らないため、強制optionが無ければ見ない。
        """
        self._dirty_manifest()
        for command in (
            "git checkout -b pages",
            "git checkout -B pages",
            "git switch -c pages",
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_staged_only_restore_is_allowed(self):
        """`git restore --staged`はindexだけを戻す。**作業treeのfileは残る。**

        **短縮形`-S`も同じに扱う。**長い形だけを見ていた版では、同じ意味の
        `git restore -S <path>`がaskになっていた（2026-09-17実測）。
        """
        self._dirty_note()
        _git(self.directory, "add", "note.md")
        self.assertAllowed("git restore --staged note.md")
        self.assertAllowed("git restore -S note.md")

    def test_worktree_restore_is_asked_even_with_staged(self):
        """`--worktree`／`-W`が付けば作業treeを書き換える。**短縮形も見る。**

        `-s`（`--source`。値を取る）は`-S`とは別のoptionである。**取り違えない。**
        """
        self._dirty_note()
        _git(self.directory, "add", "note.md")
        for command in (
            "git restore --staged --worktree note.md",
            "git restore -W note.md",
            "git restore -SW note.md",
            "git restore -s HEAD note.md",
        ):
            with self.subTest(command=command):
                self.assertAsked(command)

    def test_exhausted_budget_is_not_read_as_clean(self):
        """**1呼び出し全体の予算を使い切ったら、通さない。**

        1行に対象commandが複数あると`git status`もその数だけ走る。予算を使い切った
        状態で`_git`が成功を返すと、**確認していないtreeを「汚れていない」と読む。**
        """
        sys.path.insert(0, str(SCRIPTS_ROOT / "hooks"))
        import worktree_guard

        original = worktree_guard._started
        worktree_guard._started = time.monotonic() - worktree_guard.TOTAL_BUDGET - 1
        try:
            self.assertLessEqual(worktree_guard._remaining_budget(), 0)
            ok, output = worktree_guard._git(["status", "--porcelain"])
            self.assertFalse(ok, "予算切れで成功を返した")
            self.assertEqual(output, "")
        finally:
            worktree_guard._started = original

    def test_staged_change_matching_the_worktree_is_still_asked(self):
        """**過剰に止める側の挙動を、そうと分かる形で固定する。**

        staged済みで作業treeがindexと一致する変更は、`git checkout -- <path>`では
        失われない（indexから戻るため。2026-09-16実測）。**それでもaskする。**
        `git status --porcelain -uno`の出力側で分けると、同じ変更を実際に失う
        `git reset --hard`を取り落とす。理由はhookのdocstringが持つ。
        """
        self._dirty_note()
        _git(self.directory, "add", "note.md")
        self.assertAsked("git checkout -- note.md")

    def test_patch_mode_is_asked(self):
        """hunk単位で捨てる形も対象である。pathが無ければ作業tree全体が対象になる。"""
        self._dirty_note()
        self.assertAsked("git checkout -p")
        self.assertAsked("git restore -p")

    def test_generated_manifest_restore_is_allowed(self):
        """**`prepare_pages.py`の後の`git restore`を止めない。**

        `deskcat-preflight` skillがpush前の手順として指示している経路である。
        """
        self._dirty_manifest()
        for command in (
            "git restore pages/assets-manifest.json",
            "git restore ./pages/assets-manifest.json",
            "git checkout -- pages/assets-manifest.json",
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_generated_manifest_does_not_cover_other_paths(self):
        """除外は指定したpathだけである。**同じ呼び出しの他のpathは見る。**"""
        self._dirty_note()
        self._dirty_manifest()
        self.assertAsked("git restore pages/assets-manifest.json note.md")

    def test_clean_is_asked_even_with_a_clean_tree(self):
        """**`git clean`だけは作業treeの状態を見ずにaskする。**

        `-x`が消す無視fileは`git status --porcelain`に出ない。
        **確かめられない対象で、確かめた顔をしない。**
        """
        for command in ("git clean -fd", "git clean -f", "git clean -fdx", "git clean"):
            with self.subTest(command=command):
                self.assertAsked(command)

    def test_clean_dry_run_is_allowed(self):
        """`-n`／`--dry-run`は何も消さない。**確認そのものを止めない。**"""
        for command in ("git clean -n", "git clean -nd", "git clean --dry-run"):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_unrelated_git_commands_are_allowed(self):
        """捨てない操作は見ない。"""
        self._dirty_note()
        for command in (
            "git status",
            "git add note.md",
            "git commit -m x",
            "git stash",
            "git diff -- note.md",
            "git log --oneline",
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_help_is_out_of_scope(self):
        """helpの表示は何も捨てない。既存hookと同じ扱いにする。"""
        self._dirty_note()
        self.assertAllowed("git clean --help")
        self.assertAllowed("git reset -h")

    def test_discard_on_a_later_line_is_asked(self):
        """複数行commandの2行目も検査する（#389と同じ範囲）。"""
        self._dirty_note()
        self.assertAsked("git fetch origin\ngit reset --hard origin/develop")

    def test_discard_written_in_a_heredoc_body_is_allowed(self):
        """heredocのbodyは実行されない。**検査しない。**

        **treeを先に汚す。**汚していないと、bodyの`git reset --hard`を拾って
        しまってもcleanなtreeとして通り、**検出が壊れてもこのtestは通る。**
        """
        self._dirty_note()
        self.assertAllowed(
            "cat > note.md <<'EOF'\ngit reset --hard\nEOF\n"
        )

    def test_discard_as_an_argument_is_allowed(self):
        """command位置にない語は呼び出しではない。

        **これもtreeを汚してから見る。**理由は上と同じである。
        """
        self._dirty_note()
        self.assertAllowed("echo git reset --hard")

    def test_unknown_tree_state_is_asked(self):
        """**状態を確認できないときは通さない。**

        repositoryの外で`git status`は失敗する（2026-09-16実測、exit 128）。
        **確認できなかったことを、汚れていないことと読まない。**
        """
        outside = tempfile.mkdtemp()
        self.addCleanup(guards.remove_tree, Path(outside))
        # **temp領域がrepositoryの内側にある環境でも前提を保つ。**
        # ceilingを置かないと、上位のrepositoryが見つかってcleanと読まれ、
        # このtestは黙って逆の意味になる。
        code, output = _invoke(
            WORKTREE_GUARD, "git reset --hard", cwd=outside,
            environment={"GIT_CEILING_DIRECTORIES": str(Path(outside).parent)},
        )
        self.assertEqual(code, 0)
        self.assertIsNotNone(output, "通してしまった")
        self.assertIn("確認できなかった", _reason(output))

    def test_reason_names_the_count(self):
        """診断文が、数えた件数をそのまま書く。"""
        self._dirty_note()
        self.assertAsked("git reset --hard", contains="1件")

    def test_skip_environment_disables_the_guard(self):
        """逃げ道が効く。"""
        self._dirty_note()
        code, output = _invoke(
            WORKTREE_GUARD, "git reset --hard", cwd=self.directory,
            environment={"DESKCAT_SKIP_WORKTREE_GUARD": "1"},
        )
        self.assertEqual(code, 0)
        self.assertIsNone(output)

    def test_broken_input_does_not_block(self):
        """hookの入力が壊れていることを、対象commandの問題として扱わない。"""
        result = subprocess.run(
            [sys.executable, WORKTREE_GUARD], input="{ではないJSON",
            capture_output=True, text=True, encoding="utf-8", timeout=60,
            cwd=self.directory,
        )
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.strip(), "")


STOP_CLAIM_GUARD = str(SCRIPTS_ROOT / "hooks" / "stop_claim_guard.py")

# transcript行の最小形。**PMの決定5により、判定の対象となる形をfixtureとして
# 固定する。**Claude Codeのtranscript形式が変わればこのtestが落ちる。
TRANSCRIPT_HUMAN_STRING = {"type": "user", "message": {"role": "user", "content": "何か送って"}}
TRANSCRIPT_TOOL_RESULT_ONLY = {
    "type": "user",
    "message": {"role": "user", "content": [{"type": "tool_result", "content": "ok"}]},
}
TRANSCRIPT_HUMAN_WITH_TEXT_BLOCK = {
    "type": "user",
    "message": {"role": "user", "content": [{"type": "text", "text": "続けて"}]},
}


def _assistant_text(text):
    return {"type": "assistant", "message": {"role": "assistant", "content": [{"type": "text", "text": text}]}}


def _assistant_tool_use(name, command=None):
    tool_input = {"command": command} if command is not None else {}
    return {
        "type": "assistant",
        "message": {"role": "assistant", "content": [{"type": "tool_use", "name": name, "input": tool_input}]},
    }


class StopClaimGuardTests(unittest.TestCase):
    """完了の主張とtool呼び出しを突き合わせるhookのtest（決定1、#350）。"""

    def setUp(self):
        self._tmpdir = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmpdir.cleanup)

    def test_global_options_do_not_hide_the_evidence(self):
        """**global optionで「やった証拠」を見失わない**（#325）。

        **このhookだけは向きが逆である。**他のhookは止める側なので、外さないと
        止め損なう。こちらは証拠として読む側であり、**外さないと実際に実行した
        pushやmergeを数えず、本当にやった後の主張を止める側へ倒れる。**

        `--dry-run`のような失格flagの判定は、global optionを外しても変わらない。
        """
        sys.path.insert(0, str(SCRIPTS_ROOT / "hooks"))
        import stop_claim_guard

        for command in (
            "git push origin HEAD",
            "git -C /tmp/repo push origin HEAD",
            "git --no-pager push origin HEAD",
            "git -c user.name=x push origin HEAD",
        ):
            with self.subTest(command=command):
                self.assertTrue(
                    stop_claim_guard._git_invocation_matches(
                        command, "push", ("--dry-run", "-n")
                    ),
                    f"証拠として数えなかった: {command}",
                )
        for command in (
            "git push --dry-run origin HEAD",
            "git -C /tmp/repo push --dry-run origin HEAD",
        ):
            with self.subTest(command=command):
                self.assertFalse(
                    stop_claim_guard._git_invocation_matches(
                        command, "push", ("--dry-run", "-n")
                    ),
                    f"実際には送信しない呼び出しを証拠にした: {command}",
                )
        for command in (
            "gh pr merge 413 --squash",
            "gh --repo owner/repo pr merge 413 --squash",
        ):
            with self.subTest(command=command):
                self.assertTrue(
                    stop_claim_guard._gh_invocation_matches(
                        command, ("pr", "merge"), ("--auto",)
                    ),
                    f"証拠として数えなかった: {command}",
                )

    def _write_transcript(self, entries):
        path = Path(self._tmpdir.name) / "transcript.jsonl"
        with open(path, "w", encoding="utf-8") as handle:
            for entry in entries:
                handle.write(json.dumps(entry, ensure_ascii=False) + "\n")
        return str(path)

    def _scratchpad(self, name="scratchpad"):
        path = Path(self._tmpdir.name) / name
        path.mkdir(exist_ok=True)
        return str(path)

    def _invoke(self, message, transcript_entries, *, scratchpad=None):
        payload = {
            "last_assistant_message": message,
            "transcript_path": self._write_transcript(transcript_entries),
            "scratchpad_dir": scratchpad or self._scratchpad(),
        }
        result = subprocess.run(
            [sys.executable, STOP_CLAIM_GUARD],
            input=json.dumps(payload, ensure_ascii=False),
            capture_output=True, text=True, encoding="utf-8", timeout=60,
        )
        text = result.stdout.strip()
        return result.returncode, (json.loads(text) if text else None)

    def assertBlocked(self, message, transcript_entries, *, contains=None, scratchpad=None):
        code, output = self._invoke(message, transcript_entries, scratchpad=scratchpad)
        self.assertEqual(code, 0, message)
        self.assertIsNotNone(output, f"通してしまった: {message}")
        self.assertEqual(output["decision"], "block", message)
        if contains:
            self.assertIn(contains, output["reason"])

    def assertAllowed(self, message, transcript_entries, *, scratchpad=None):
        code, output = self._invoke(message, transcript_entries, scratchpad=scratchpad)
        self.assertEqual(code, 0, message)
        self.assertIsNone(output, f"止めてしまった: {message}")

    def test_send_claim_without_evidence_is_blocked(self):
        """証拠の無い送信主張をblockし、主張した語を名指しする。"""
        self.assertBlocked(
            "送りました", [TRANSCRIPT_HUMAN_STRING], contains="送りました"
        )

    def test_send_claim_with_evidence_is_allowed(self):
        """`mcp__ccd_session_mgmt__send_message`の呼び出しがあれば通す。"""
        self.assertAllowed(
            "送りました",
            [
                TRANSCRIPT_HUMAN_STRING,
                _assistant_tool_use("mcp__ccd_session_mgmt__send_message"),
                TRANSCRIPT_TOOL_RESULT_ONLY,
            ],
        )

    def test_evidence_across_multiple_assistant_blocks_is_found(self):
        """複数回のtool往復（複数の`requestId`相当）を跨いでも証拠を見つける。

        1つの人間turnの中で、thinking→tool_use→tool_result→…→最後はtextだけ、
        という繰り返しが実際に起きる（担当セッションが自身のtranscriptで観測）。
        直前の1回分だけを見ると、途中のtool呼び出しを見落とす。
        """
        self.assertAllowed(
            "起票しました",
            [
                TRANSCRIPT_HUMAN_STRING,
                {"type": "assistant", "message": {"role": "assistant", "content": [{"type": "thinking", "text": "..."}]}},
                _assistant_tool_use("Bash", "gh issue create --title t --body-file f"),
                TRANSCRIPT_TOOL_RESULT_ONLY,
                _assistant_text("起票しました"),
            ],
        )

    def test_issue_create_evidence_for_filing_claim(self):
        """`起票しました`は`gh issue create`のBash呼び出しで満たす。"""
        self.assertAllowed(
            "起票しました",
            [TRANSCRIPT_HUMAN_STRING, _assistant_tool_use("Bash", "gh issue create --title t")],
        )

    def test_merge_evidence_for_merge_claim(self):
        """`mergeしました`は`gh pr merge`のBash呼び出しで満たす。"""
        self.assertAllowed(
            "mergeしました",
            [TRANSCRIPT_HUMAN_STRING, _assistant_tool_use("Bash", "gh pr merge 1 --squash")],
        )

    def test_push_evidence_for_push_claim(self):
        """`pushしました`は`git push`のBash呼び出しで満たす。"""
        self.assertAllowed(
            "pushしました",
            [TRANSCRIPT_HUMAN_STRING, _assistant_tool_use("Bash", "git push origin main")],
        )

    def test_unrelated_bash_command_is_not_evidence(self):
        """関係ないBash呼び出しは証拠にならない。"""
        self.assertBlocked(
            "pushしました",
            [TRANSCRIPT_HUMAN_STRING, _assistant_tool_use("Bash", "git status")],
        )

    def test_dry_run_push_is_not_evidence(self):
        """`git push --dry-run`は完了の証拠にならない（`#355`のreview指摘）。

        `--dry-run`は実際には何も送信しない。subcommandが一致するだけで
        証拠として数えると、送信していないことを「pushしました」と主張できてしまう。
        """
        self.assertBlocked(
            "pushしました",
            [TRANSCRIPT_HUMAN_STRING, _assistant_tool_use("Bash", "git push --dry-run origin main")],
        )

    def test_auto_merge_is_not_evidence(self):
        """`gh pr merge --auto`は完了の証拠にならない（`#355`のreview指摘）。

        `--auto`は要件が揃うまでの予約であり、その場でmergeするわけではない。
        """
        self.assertBlocked(
            "mergeしました",
            [TRANSCRIPT_HUMAN_STRING, _assistant_tool_use("Bash", "gh pr merge 1 --auto")],
        )

    def test_push_on_a_later_line_is_evidence(self):
        """複数行commandの2行目の`git push`も証拠として数える（#389）。

        **`command_line.invocations`を共有しているため、範囲がここへも及ぶ。**
        こちら側で変わるのは、**実際に押しているのにblockしていた形**（偽陽性）が
        通るようになる向きである。
        """
        self.assertAllowed(
            "pushしました",
            [
                TRANSCRIPT_HUMAN_STRING,
                _assistant_tool_use("Bash", "git status\ngit push origin main"),
            ],
        )

    def test_push_in_a_branch_that_did_not_run_is_counted_as_evidence(self):
        """**走らなかった`git push`も証拠として数える**（#389）。

        `command_line.invocations`は字句だけで見るため、条件が偽でも呼び出しとして返す。
        **止める門では安全側だが、この hook では逆である。**押していないのに
        「pushしました」を通す。**過検出の向きは呼び出し側で変わる。**
        取り落としではなく、**#389で増えた側の乖離である。**ここへ書いて残す。
        """
        self.assertAllowed(
            "pushしました",
            [
                TRANSCRIPT_HUMAN_STRING,
                _assistant_tool_use(
                    "Bash", "if false; then\ngit push origin main\nfi"
                ),
            ],
        )

    def test_push_written_in_a_heredoc_body_is_not_evidence(self):
        """heredocのbodyに書いた`git push`は証拠にならない（#389）。

        **bashは実行していない。**数えると、押していないことを「pushしました」と
        主張できてしまう。
        """
        self.assertBlocked(
            "pushしました",
            [
                TRANSCRIPT_HUMAN_STRING,
                _assistant_tool_use(
                    "Bash", "cat > note.md <<'EOF'\ngit push origin main\nEOF\n"
                ),
            ],
        )

    def test_actual_push_is_still_evidence(self):
        """`--dry-run`を伴わない`git push`は引き続き証拠になる。"""
        self.assertAllowed(
            "pushしました",
            [TRANSCRIPT_HUMAN_STRING, _assistant_tool_use("Bash", "git push origin main")],
        )

    def test_actual_merge_is_still_evidence(self):
        """`--auto`を伴わない`gh pr merge`は引き続き証拠になる。"""
        self.assertAllowed(
            "mergeしました",
            [TRANSCRIPT_HUMAN_STRING, _assistant_tool_use("Bash", "gh pr merge 1 --squash")],
        )

    def test_quoted_claim_is_not_counted(self):
        """引用（`「」`）の中の語は主張として数えない。"""
        self.assertAllowed("彼は「送りました」と言っていた", [TRANSCRIPT_HUMAN_STRING])

    def test_negated_claim_is_not_counted(self):
        """否定形は主張として数えない。"""
        for message in ("まだ送っていません", "送りませんでした"):
            with self.subTest(message=message):
                self.assertAllowed(message, [TRANSCRIPT_HUMAN_STRING])

    def test_future_claim_is_not_counted(self):
        """未来形・意志形は主張として数えない。"""
        for message in ("これから送ります", "送信します"):
            with self.subTest(message=message):
                self.assertAllowed(message, [TRANSCRIPT_HUMAN_STRING])

    def test_no_claim_is_allowed(self):
        """完了を主張する語が無い応答は何も見ない。"""
        self.assertAllowed("承知しました。作業を進めます。", [TRANSCRIPT_HUMAN_STRING])

    def test_same_category_is_blocked_once_then_allowed(self):
        """同じ主張分類に対してblockするのは1回だけとし、2回目は通す。

        **`stop_hook_active`には依存しない**（担当セッションの実測で、
        block後の再実行でもfalseのままだったため）。状態は`scratchpad_dir`
        （セッション内で完結する場所）に持つ。
        """
        scratchpad = self._scratchpad()
        self.assertBlocked("送りました", [TRANSCRIPT_HUMAN_STRING], scratchpad=scratchpad)
        self.assertAllowed("送りました", [TRANSCRIPT_HUMAN_STRING], scratchpad=scratchpad)

    def test_different_session_state_does_not_leak(self):
        """`scratchpad_dir`が違えば、blockの記録は引き継がれない。

        **セッションをまたいで残る場所へ書くと、前のセッションのblockが
        次のセッションを素通りさせる。**`scratchpad_dir`はsession_idを含む
        pathであり、別sessionなら別の場所になる。
        """
        self.assertBlocked(
            "送りました", [TRANSCRIPT_HUMAN_STRING], scratchpad=self._scratchpad("session_a")
        )
        self.assertBlocked(
            "送りました", [TRANSCRIPT_HUMAN_STRING], scratchpad=self._scratchpad("session_b")
        )

    def test_human_turn_boundary_uses_tool_result_only_rule(self):
        """境界判定は「文字列か」ではなく「tool_resultだけで構成されていないか」で行う。

        **PMの決定1（2026-09-06）。**画像添付等、人間の入力でも`content`が
        listになる形（`tool_result`以外のblockを含むlist）を、境界として正しく
        扱えることを確かめる。
        """
        self.assertAllowed(
            "送りました",
            [
                TRANSCRIPT_TOOL_RESULT_ONLY,  # これより前の(無い)tool_useは境界外
                TRANSCRIPT_HUMAN_WITH_TEXT_BLOCK,  # tool_result以外を含むlist→人間の入力
                _assistant_tool_use("mcp__ccd_session_mgmt__send_message"),
            ],
        )

    def test_boundary_not_found_uses_whole_file(self):
        """人間の入力entryが1つも無い場合は、file全体を対象にする。

        **tool_useを多く集める方向であり、blockを減らす安全側である。**
        """
        self.assertAllowed(
            "送りました",
            [_assistant_tool_use("mcp__ccd_session_mgmt__send_message")],
        )

    def test_unreadable_transcript_is_allowed(self):
        """transcriptを読めない場合はblockしない。判定できないことを理由にしない。"""
        payload = {
            "last_assistant_message": "送りました",
            "transcript_path": "/nonexistent/path.jsonl",
            "scratchpad_dir": self._scratchpad(),
        }
        result = subprocess.run(
            [sys.executable, STOP_CLAIM_GUARD],
            input=json.dumps(payload, ensure_ascii=False),
            capture_output=True, text=True, encoding="utf-8", timeout=60,
        )
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.strip(), "")

    def test_non_string_transcript_path_is_allowed(self):
        """`transcript_path`が文字列以外のtruthy値でも`TypeError`を出さず通す。

        `#355`のreview指摘。非空listや数値は真偽値としてtruthyだが、`open()`へ
        渡すと`TypeError`になる。
        """
        for transcript_path in ([1, 2], {"a": 1}, 123):
            with self.subTest(transcript_path=transcript_path):
                payload = {
                    "last_assistant_message": "送りました",
                    "transcript_path": transcript_path,
                    "scratchpad_dir": self._scratchpad(),
                }
                result = subprocess.run(
                    [sys.executable, STOP_CLAIM_GUARD],
                    input=json.dumps(payload, ensure_ascii=False),
                    capture_output=True, text=True, encoding="utf-8", timeout=60,
                )
                self.assertEqual(result.returncode, 0)
                self.assertEqual(result.stderr, "")
                self.assertEqual(result.stdout.strip(), "")

    def test_broken_input_does_not_block(self):
        """hookの入力が壊れていることを、対象応答の問題として扱わない。"""
        result = subprocess.run(
            [sys.executable, STOP_CLAIM_GUARD], input="{ではないJSON",
            capture_output=True, text=True, encoding="utf-8", timeout=60,
        )
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.strip(), "")

    def test_no_last_assistant_message_is_allowed(self):
        """`last_assistant_message`が無い（または空）payloadは何も見ない。"""
        for payload in ({}, {"last_assistant_message": ""}, {"last_assistant_message": None}):
            with self.subTest(payload=payload):
                result = subprocess.run(
                    [sys.executable, STOP_CLAIM_GUARD],
                    input=json.dumps(payload), capture_output=True, text=True,
                    encoding="utf-8", timeout=60,
                )
                self.assertEqual(result.returncode, 0)
                self.assertEqual(result.stdout.strip(), "")


class ReviewTargetListTests(unittest.TestCase):
    """CONTRIBUTINGの「慎重に扱う path」と、hookがCodeRabbitの完走を求めるpathの一致。"""

    HEADING = "### 慎重に扱う path"

    def _documented_paths(self):
        text = (REPO_ROOT_FOR_TEMPLATES / "CONTRIBUTING.md").read_text(encoding="utf-8")
        self.assertEqual(text.count(self.HEADING + "\n"), 1, "見出しが1つではない")
        section = text.split(self.HEADING + "\n", 1)[1]
        section = re.split(r"^#{1,3} ", section, maxsplit=1, flags=re.MULTILINE)[0]
        paths = []
        bullet = False
        for line in section.splitlines():
            if line.startswith("- "):
                bullet = True
            elif not line.startswith("  "):
                bullet = False
            if bullet:
                # 項目名と「：」の後ろにあるpathだけを読む。
                body = line.split("：", 1)[1] if "：" in line else line
                paths.extend(re.findall(r"`([^`]+)`", body))
        return paths

    def test_documented_list_matches_the_hook(self):
        documented = self._documented_paths()
        self.assertEqual(len(documented), len(set(documented)), "一覧に重複がある")
        hook = list(guard.REVIEW_TARGET_PREFIXES) + list(guard.REVIEW_TARGET_FILES)
        self.assertEqual(sorted(documented), sorted(hook))

    def test_every_hardware_file_is_classified(self):
        """`hardware/`の追跡fileは、対象か、CONTRIBUTINGが挙げた対象外のどちらかに入る。"""
        text = (REPO_ROOT_FOR_TEMPLATES / "CONTRIBUTING.md").read_text(encoding="utf-8")
        line = [x for x in text.splitlines() if x.startswith("`hardware/`のうち対象外は次だけである")]
        self.assertEqual(len(line), 1, "対象外の一覧の行が1つではない")
        outside = re.findall(r"`([^`]+)`", line[0].split("：", 1)[1])
        result = subprocess.run(
            ["git", "ls-files", "-z", "hardware"],
            cwd=REPO_ROOT_FOR_TEMPLATES, capture_output=True, check=True,
        )
        names = [n for n in result.stdout.decode("utf-8").split("\0") if n]
        self.assertTrue(names)
        for name in names:
            with self.subTest(name=name):
                inside = bool(guard._review_targets([name]))
                excluded = any(
                    name == item or (item.endswith("/") and name.startswith(item))
                    for item in outside
                )
                self.assertTrue(inside != excluded, "対象と対象外のどちらか一方だけに入らない")

    def test_prefixes_end_with_a_slash_and_files_do_not(self):
        for prefix in guard.REVIEW_TARGET_PREFIXES:
            with self.subTest(prefix=prefix):
                self.assertTrue(prefix.endswith("/"))
        for name in guard.REVIEW_TARGET_FILES:
            with self.subTest(name=name):
                self.assertFalse(name.endswith("/"))


class CodeRabbitGateTests(unittest.TestCase):
    """CodeRabbitのreviewを起動するcommandを見るhookのtest。

    **人間の判断が要るものは`ask`、規則に反すると決まるものは`deny`を返す**（#781）。
    `ask`は`assertAsked`、`deny`は`assertDecision`で検査する。
    **止めたいのはAIの独断であって、人間の依頼ではない。**
    """

    def assertAsked(self, command, *, contains=None):
        code, output = _invoke(CODERABBIT_GATE, command)
        self.assertEqual(code, 0, command)
        self.assertIsNotNone(output, f"通してしまった: {command}")
        self.assertEqual(
            output["hookSpecificOutput"]["permissionDecision"], "ask", command
        )
        if contains:
            self.assertIn(contains, _reason(output))

    def assertAllowed(self, command):
        code, output = _invoke(CODERABBIT_GATE, command)
        self.assertEqual(code, 0, command)
        self.assertIsNone(output, f"止めてしまった: {command}")

    def test_review_triggers_are_asked(self):
        """reviewを起動する語を止める。**`full review`は`ask`で止める。**"""
        for command in (
            'gh pr comment 239 --body "@coderabbitai full review"',
            'gh issue comment 240 --body "@coderabbitai full review"',
            'gh pr comment 239 -b "@coderabbitai full review"',
            'gh pr comment 239 --body="@coderabbitai full review"',
        ):
            with self.subTest(command=command):
                self.assertAsked(command)

    # ---- #781: `full`の付かない`review`と、走った後の`full review`は`deny` ----

    DONE = CodeRabbitReviewMergeGateTests.DONE
    SKIPPED = CodeRabbitReviewMergeGateTests.SKIPPED
    LIMITED = CodeRabbitReviewMergeGateTests.LIMITED
    PENDING = CodeRabbitReviewMergeGateTests.PENDING
    LOCK_LOSS = {"name": "CodeRabbit", "status": "completed", "conclusion": "failure",
                 "output": {"title": "Review stopped after lock loss", "summary": ""}}
    FULL = 'gh pr comment 7 --body "@coderabbitai full review"'

    def _state(self, command, fixture=None):
        code, output = _invoke(
            CODERABBIT_GATE, command,
            environment={"DESKCAT_TEST_GH": json.dumps(fixture or {})},
        )
        self.assertEqual(code, 0, command)
        return output

    def assertDecision(self, expected, command, fixture=None, contains=None):
        output = self._state(command, fixture)
        if expected is None:
            self.assertIsNone(output, f"止めてしまった: {output}")
            return
        self.assertIsNotNone(output, f"通してしまった: {command}")
        self.assertEqual(
            output["hookSpecificOutput"]["permissionDecision"], expected, command
        )
        if contains:
            self.assertIn(contains, _reason(output))

    def test_bare_review_is_denied_on_every_route(self):
        """`full`の付かない`review`は、どの経路でも`deny`で止める。理由はCONTRIBUTINGの節を指す。"""
        with tempfile.TemporaryDirectory() as directory:
            body = Path(directory) / "b.md"
            body.write_text("@coderabbitai review\n", encoding="utf-8")
            payload = Path(directory) / "p.json"
            payload.write_text('{"body": "@coderabbitai review"}', encoding="utf-8")
            commands = (
                'gh pr comment 239 --body "@coderabbitai review"',
                'gh pr comment 239 -b "@coderabbitai review"',
                'gh pr comment 239 --body="@coderabbitai review"',
                'gh issue comment 240 --body "@coderabbitai review"',
                'gh pr review 239 --comment --body "@coderabbitai review"',
                'gh api repos/o/r/issues/1/comments -f body="@coderabbitai review"',
                'gh api repos/o/r/issues/1/comments -f body="@coderabbitai review"',
                'gh api repos/o/r/issues/1/comments --raw-field body="@coderabbitai review"',
                f"gh pr comment 239 --body-file {body}",
                f"gh pr comment 239 -F {body}",
                f"gh issue comment 240 -F {body}",
                f"gh pr review 239 --comment -F {body}",
                f"gh issue comment 240 --body-file {body}",
                f"gh api repos/o/r/issues/1/comments -F body=@{body}",
                f"gh api repos/o/r/issues/1/comments --input {payload}",
                'gh --repo o/r pr comment 239 --body "@coderabbitai review"',
                'cd /tmp && gh pr comment 239 --body "@CodeRabbitAI  REVIEW"',
                'gh pr comment 239 --body "@coderabbitai rate limit and then review"',
                'gh pr comment 239 --body "@coderabbitai full review\n@coderabbitai review"',
            )
            for command in commands:
                with self.subTest(command=command):
                    self.assertDecision(
                        "deny", command, contains="手動で依頼する前に状態を確認する"
                    )

    def test_full_review_with_a_bare_review_in_the_same_line_is_denied(self):
        self.assertDecision(
            "deny", 'gh pr comment 7 --body "@coderabbitai full review and review"'
        )

    def test_full_review_after_a_completed_review_is_denied(self):
        """どれかのcommitで完走していれば、headが`Review skipped`でも`deny`。"""
        fixture = {"commits": {"a": {"check_runs": [self.DONE]},
                               "b": {"check_runs": [self.SKIPPED]}}}
        for command in (
            self.FULL,
            'gh issue comment 7 --body "@coderabbitai full review"',
            'gh pr review 7 --comment --body "@coderabbitai full review"',
            'gh api repos/o/r/issues/7/comments -f body="@coderabbitai full review"',
            'gh api /repos/o/r/issues/7/comments -f body="@coderabbitai full review"',
            'gh api https://api.github.com/repos/o/r/issues/7/comments -f body="@coderabbitai full review"',
            'gh api repos/o/r/pulls/7/comments -f body="@coderabbitai full review"',
            'gh --repo o/r pr comment 7 --body "@coderabbitai full review"',
        ):
            with self.subTest(command=command):
                self.assertDecision("deny", command, fixture, contains="投げ直さない")

    def test_full_review_while_a_review_is_in_progress_is_denied(self):
        fixture = {"commits": {"a": {"check_runs": [self.PENDING]}}}
        self.assertDecision("deny", self.FULL, fixture)

    def test_a_legacy_commit_status_counts_as_completed(self):
        fixture = {"commits": {"a": {"check_runs": [], "statuses": [
            {"context": "CodeRabbit", "state": "success",
             "description": "Review completed"}]}}}
        self.assertDecision("deny", self.FULL, fixture)

    def test_full_review_after_rate_limit_or_lock_loss_is_asked_not_denied(self):
        """例外。走っていない原因が枠切れ・lock lossなら、今までどおり`ask`。"""
        for run in (self.LIMITED, self.LOCK_LOSS):
            with self.subTest(title=run["output"]["title"]):
                self.assertDecision(
                    "ask", self.FULL, {"commits": {"a": {"check_runs": [run]}}}
                )

    def test_full_review_on_a_pr_never_reviewed_is_asked(self):
        """初回の`full review`は今までどおり`ask`。"""
        for fixture in (
            {"commits": {"a": {"check_runs": [self.SKIPPED]}}},
            {"commits": {"a": {"check_runs": []}}},
            {"commits": {}},
        ):
            with self.subTest(fixture=fixture):
                self.assertDecision("ask", self.FULL, fixture)

    def test_full_review_is_asked_when_the_state_cannot_be_determined(self):
        """判定できないときは素通りさせず`ask`。`deny`でもない。"""
        completed = {"commits": {"a": {"check_runs": [self.DONE]}}}
        self.assertDecision(
            "ask", self.FULL, {**completed, "fail": True}, contains="判定できない"
        )
        # 宛先を読めない`gh api`（graphql）。
        self.assertDecision(
            "ask",
            'gh api graphql -f query="mutation { x }" -f body="@coderabbitai full review"',
            completed, contains="判定できない",
        )
        # 本文を読めない経路は従来どおり`ask`。
        self.assertDecision("ask", "gh pr comment 7 --body-file -", completed)

    def test_text_without_review_still_passes_with_a_completed_review(self):
        completed = {"commits": {"a": {"check_runs": [self.DONE]}}}
        for command in (
            'gh pr comment 7 --body "@coderabbitai rate limit"',
            'gh pr comment 7 --body "@coderabbitai resolve"',
            'gh pr comment 7 --body "reviewで出た指摘を反映しました"',
        ):
            with self.subTest(command=command):
                self.assertDecision(None, command, completed)

    def test_short_body_file_option_is_read_on_comment_commands(self):
        """`gh pr comment -F -`（stdin）は読めない本文として`ask`にする。"""
        self.assertDecision("ask", "gh pr comment 7 -F -", contains="読めない")

    def test_a_later_deny_wins_over_an_earlier_ask_in_one_command(self):
        """同じcommandの先の呼び出しが`ask`でも、後ろの`deny`は`ask`に下がらない。"""
        first = 'gh pr comment 7 --body "@coderabbitai full review"'
        for second in (
            'gh pr comment 7 --body "@coderabbitai review"',
        ):
            self.assertDecision("deny", f"{first} && {second}", {})
        self.assertDecision(
            "deny", 'gh pr comment 7 --body-file - && gh pr comment 8 --body "@coderabbitai review"', {}
        )
        # `deny`が無ければ`ask`のまま。
        self.assertDecision("ask", f"{first} && gh pr comment 7 --body ok", {})

    def test_several_full_reviews_share_one_deadline(self):
        """`full review`が複数あっても、照会の合計がhookの30秒を超えない（期限は1つ）。"""
        import time as _time
        first = 'gh pr comment 7 --body "@coderabbitai full review"'
        command = f"{first} && {first} && {first}"
        started = _time.monotonic()
        # 各`gh`が遅く、期限（18秒）を超える。呼び出しごとに期限を持つと30秒を超える。
        output = self._state(command, {"sleep": 8, "commits": {}})
        self.assertLess(_time.monotonic() - started, 30)
        self.assertEqual(output["hookSpecificOutput"]["permissionDecision"], "ask")

    def test_an_unreadable_body_does_not_downgrade_a_deny(self):
        """読めない経路を足しても、走った後の`full review`は`deny`のまま。"""
        completed = {"commits": {"a": {"check_runs": [self.DONE]}}}
        self.assertDecision(
            "deny",
            'gh api repos/o/r/issues/7/comments -f body="@coderabbitai full review"'
            " --input -",
            completed,
        )

    def test_only_the_latest_status_counts_as_in_progress(self):
        """statusの履歴に残る古い`pending`で、枠切れ後の再依頼を`deny`にしない。"""
        statuses = [
            {"context": "CodeRabbit", "state": "success",
             "description": "Review rate limited"},
            {"context": "CodeRabbit", "state": "pending",
             "description": "Review in progress"},
        ]
        self.assertDecision(
            "ask", self.FULL, {"commits": {"a": {"check_runs": [], "statuses": statuses}}}
        )
        self.assertDecision(
            "deny", self.FULL,
            {"commits": {"a": {"check_runs": [], "statuses": statuses[::-1]}}},
        )

    def test_gh_failure_is_not_a_pass_even_for_a_completed_review(self):
        """`gh`の失敗を「走っていない」と読まない（`ask`であって素通りではない）。"""
        self.assertDecision("ask", self.FULL, {"fail": True})

    def test_global_options_do_not_hide_the_subcommand(self):
        """**global optionで検査が抜けない**（#325）。`args[:2]`で位置から読んでいた
        版では、`gh --repo o/r pr comment`が素通りしていた。"""
        for command in (
            'gh --repo owner/repo pr comment 239 --body "@coderabbitai full review"',
            'gh -R owner/repo issue comment 240 --body "@coderabbitai full review"',
        ):
            with self.subTest(command=command):
                self.assertAsked(command)

    def test_trigger_on_a_later_line_is_asked(self):
        """複数行commandの2行目のreview依頼も止める（#389）。

        **`command_line.invocations`を他のhookと共有している。**改行対応で
        このhookの範囲も広がる。**広がる向きは、これまで素通りしていた形を
        `ask`へ入れる側である。**
        """
        self.assertAsked('cat x\ngh pr comment 239 --body "@coderabbitai full review"')

    def test_trigger_written_in_a_heredoc_body_is_allowed(self):
        """heredocのbodyに書いた依頼文では止めない（#389）。**bashは投稿しない。**"""
        self.assertAllowed(
            "cat > note.md <<'EOF'\n"
            'gh pr comment 239 --body "@coderabbitai full review"\n'
            "EOF\n"
        )

    def test_trigger_inside_a_quoted_heredoc_substitution_is_asked(self):
        """`--body "$(cat <<'EOF' … EOF)"`の本文も見る（#389）。

        **引用の中へ入ったheredocのbodyは落とさない。**引用ごと1語として残るため、
        本文の語を見るこの判定はそこでも効く。**この形はこのリポジトリの常用形であり、
        落とすと外向きの操作を止める門が抜ける。**
        """
        self.assertAsked(
            'gh pr comment 239 --body "$(cat <<\'EOF\'\n'
            '@coderabbitai full review\n'
            'EOF\n'
            ')"'
        )

    def test_line_continuation_inside_the_body_is_still_missed(self):
        """**引用の中の`\\`改行は今も素通りする。**#389では直していない。

        bashは`\\`と改行を消して1行へ繋ぐため、**この形は依頼として投稿される。**
        `segments`は引用の中を素通しし、`_review_kinds`は行ごとに見るため、
        `@coderabbitai`と`review`が別の行になる。
        **#389より前から同じである**（2026-09-14に実測）。
        CONTRIBUTINGの「取り切れていないもの」に書いた形であり、
        **直したらこのtestが落ちる。**そのときはCONTRIBUTINGも直す。
        """
        self.assertAllowed('gh pr comment 239 --body "@coderabbitai \\\nfull review"')

    def test_body_piped_from_a_heredoc_is_still_asked(self):
        """`--body-file -`へheredocで流す形でも止める（#389）。

        **bodyを落とすとhookから本文が読めなくなるが、素通りはしない。**
        `--body-file -`はstdinであり、このhookは読めない本文をaskへ倒す
        （`_read_body`／`_check`の`unreadable`）。**#389の前後で判定は変わらない**
        （2026-09-14に実測）。
        **「bashが実行しないから落としてよい」は、本文を読む判定には直接効かない。**
        ここが素通りに変わらないことを固定する。
        """
        self.assertAsked(
            "gh pr comment 239 --body-file - <<'EOF'\n"
            "@coderabbitai full review\n"
            "EOF\n"
        )

    def test_passthrough_words_are_allowed(self):
        """枠を消費しない語は通す。**残数確認とthreadの後始末を塞がない。**"""
        for command in (
            'gh pr comment 239 --body "@coderabbitai rate limit"',
            'gh pr comment 239 --body "@coderabbitai resolve"',
            'gh pr comment 239 --body "@coderabbitai help"',
            'gh pr comment 239 --body "@coderabbitai configuration"',
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_spelling_variants_are_asked(self):
        """表記の揺れで抜けない。**語の並びを列挙していた版は`full-review`を通した。**"""
        for command in (
            'gh pr comment 239 --body "@coderabbitai full-review"',
            'gh pr comment 239 --body "@CodeRabbitAI FULL REVIEW"',
            'gh pr comment 239 --body "  @coderabbitai   full review  "',
        ):
            with self.subTest(command=command):
                self.assertAsked(command)

    def test_passthrough_word_does_not_grant_immunity(self):
        """`rate limit`で始めて後ろに`review`を置く形を止める。

        **免除listを先に見る版はこれを通した。**実測して直した。
        """
        self.assertAsked(
            'gh pr comment 239 --body "@coderabbitai rate limit and then full review"'
        )

    def test_review_on_another_line_is_allowed(self):
        """mentionと同じ行に`review`が無ければ通す。

        指摘への返信は別の行で`review`に触れる。**行単位で見る理由である。**
        """
        self.assertAllowed(
            'gh pr comment 239 --body "確認しました\nreviewで出た指摘を反映\n@coderabbitai resolve"'
        )

    def test_mention_without_command_is_allowed(self):
        """`@coderabbitai`だけではreviewが始まらない。止めない。"""
        self.assertAllowed('gh pr comment 239 --body "@coderabbitai"')

    def test_mention_elsewhere_in_body_is_allowed(self):
        """`@coderabbitai`の直後以外にある`review`で止めない。

        指摘への返信は「reviewで出た指摘を反映した」のような文を含む。
        **返信を塞ぐと、threadの後始末ができなくなる。**
        """
        for command in (
            'gh pr comment 239 --body "reviewで出た指摘を反映しました"',
            'gh pr comment 239 --body "full review の結果を記録します"',
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_compound_command_is_inspected(self):
        """`cd x && gh pr comment ...`の形を素通りさせない。

        `if`条件で`Bash(gh *)`へ絞るとこの形が抜ける。絞らない理由である。
        """
        self.assertAsked(
            'cd /tmp && gh pr comment 239 --body "@coderabbitai full review"'
        )

    def test_quoted_command_is_not_inspected(self):
        """`echo`の引数として書かれた形は実行ではない。止めない。"""
        self.assertAllowed(
            'echo gh pr comment 239 --body "@coderabbitai full review"'
        )

    def test_api_field_is_inspected(self):
        """`gh api`の`-f body=...`でも止める。**commentはこの経路でも投げられる。**"""
        self.assertAsked(
            'gh api repos/o/r/issues/1/comments -f body="@coderabbitai full review"'
        )

    def test_body_file_is_read(self):
        """`--body-file`の中身を読んで判定する。"""
        with tempfile.TemporaryDirectory() as work:
            path = Path(work) / "body.md"
            path.write_text("@coderabbitai full review\n", encoding="utf-8")
            self.assertAsked(f'gh pr comment 239 --body-file {path}')

    def test_an_unreadable_body_is_asked_not_allowed(self):
        """本文を読めない呼び出しを素通りさせない。**askする。**

        以前は素通りさせ、docstringも「この方向の取りこぼしは意図である」としていた。
        **しかし`-F body=@-`は意図して選べる経路であり、外したい側が自分で選べる穴だった。**
        **判定は`deny`ではなく`ask`である。**誤検知の代償は人への確認1回で、
        見逃しの代償はこのhookが在る理由そのものである。
        """
        for command in (
            "gh pr comment 1 --body-file /nonexistent/body.md",
            "gh api repos/o/r/issues/1/comments --input /nonexistent/body.json",
            "gh api -X POST repos/o/r/issues/1/comments -F body=@/nonexistent/b.txt",
            "gh pr comment 1 --body-file -",
            "gh api -X POST repos/o/r/issues/1/comments -F body=@-",
        ):
            with self.subTest(command=command):
                self.assertAsked(command, contains="読めない")

    def test_a_readable_body_without_review_is_still_allowed(self):
        """**読めた本文にreviewが無ければ素通りする。**askを一律にはしない。"""
        with tempfile.TemporaryDirectory() as directory:
            body = Path(directory) / "b.md"
            body.write_text("@coderabbitai rate limit", encoding="utf-8")
            self.assertAllowed(f"gh pr comment 1 --body-file {body}")
            self.assertAllowed(
                f"gh api repos/o/r/issues/1/comments --input {body}"
            )

    def test_unrelated_commands_are_allowed(self):
        """CodeRabbitに関係しない`gh`と読み取りを通す。"""
        for command in (
            "gh pr view 239 --json body",
            'gh pr comment 239 --body "通常のコメント"',
            "gh pr merge 239 --squash --subject s --body-file /tmp/x",
            "git status",
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_broken_input_does_not_block(self):
        """hookの入力が壊れていても止めない。**既存hookと同じ扱いである。**"""
        result = subprocess.run(
            [sys.executable, CODERABBIT_GATE],
            input="{ not json", capture_output=True, text=True,
            encoding="utf-8", errors="replace", timeout=60,
        )
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.strip(), "")
        # **握りつぶさない。**素通りの動作は変えずに、理由をstderrへ残す
        # （PR #241のreview指摘）。
        self.assertIn("coderabbit_gate:", result.stderr)

    def test_invalid_utf8_input_is_recorded(self):
        """不正なUTF-8の入力も分類して記録する。**素通りの動作は変えない。**"""
        result = subprocess.run(
            [sys.executable, CODERABBIT_GATE],
            input=b"\xff\xfe not utf-8", capture_output=True, timeout=60,
        )
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.decode("utf-8", "replace").strip(), "")
        self.assertIn("coderabbit_gate:", result.stderr.decode("utf-8", "replace"))

    def test_non_mapping_input_does_not_raise(self):
        """妥当なJSONでもmappingでない入力で落ちない。

        `payload.get`／`tool_input.get`は`AttributeError`を出す。
        **hookが例外で落ちると、止めているはずの判定が走らない。**
        PR #241のreview指摘で見つけた。
        """
        for payload in ("[]", '"text"', "null", '{"tool_input": ["x"]}',
                        '{"tool_input": "x"}'):
            with self.subTest(payload=payload):
                result = subprocess.run(
                    [sys.executable, CODERABBIT_GATE],
                    input=payload, capture_output=True, text=True,
                    encoding="utf-8", errors="replace", timeout=60,
                )
                self.assertEqual(result.returncode, 0, payload)
                self.assertEqual(result.stdout.strip(), "", payload)

    def test_read_only_api_method_is_allowed(self):
        """読み取りと明示した`gh api`は見ない。**commentを投げられない。**

        `-f`を付けると`gh`は既定でPOSTになるが、`-X GET`を明示すると読み取りである。
        PR #241のreview指摘で見つけた。
        """
        for command in (
            'gh api -X GET repos/o/r/issues -f body="@coderabbitai full review"',
            'gh api --method HEAD repos/o/r -f body="@coderabbitai full review"',
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_write_api_method_is_still_inspected(self):
        """書き込みのmethodと、method未指定は引き続き見る。"""
        for command in (
            'gh api -X POST repos/o/r/issues/1/comments -f body="@coderabbitai full review"',
            'gh api repos/o/r/issues/1/comments -f body="@coderabbitai full review"',
            'gh api graphql -f query="mutation { x(body: \\"@coderabbitai full review\\") }"',
        ):
            with self.subTest(command=command):
                self.assertAsked(command)

    def test_later_write_method_is_not_masked_by_an_earlier_read(self):
        """`-X GET -X POST`を読み取り扱いにしない。**`gh`は最後の`-X`を使う。**

        `any()`で見ると先頭の`get`が後ろの`post`を隠し、**承認を経ずに素通りした**
        （2026-08-28に実測。PR #254のreviewで出た）。
        """
        self.assertAsked(
            "gh api -X GET -X POST repos/o/r/issues/1/comments"
            ' -f body="@coderabbitai full review"'
        )

    def test_read_method_declared_last_is_still_allowed(self):
        """**後勝ちは両方向へ効く。**最後が読み取りなら素通りさせる。

        `-X POST -X GET`で`gh`が投げるのはGETである。ここを止めると誤検知になる。
        """
        self.assertAllowed(
            "gh api -X POST -X GET repos/o/r/issues/1/comments"
            ' -f body="@coderabbitai full review"'
        )

    def test_input_file_is_read(self):
        """`gh api --input <file>`の本文を読む。

        **値がpathであって本文ではないため、`BODY_OPTIONS`では拾えない。**
        塞ぐまでは`--input`へ本文を置けば素通りした（2026-08-28に実測）。
        """
        with tempfile.TemporaryDirectory() as directory:
            body = Path(directory) / "body.json"
            body.write_text(
                '{"body": "@coderabbitai full review"}', encoding="utf-8"
            )
            self.assertAsked(
                f"gh api repos/o/r/issues/1/comments --input {body}"
            )
            harmless = Path(directory) / "ok.json"
            harmless.write_text(
                '{"body": "@coderabbitai rate limit"}', encoding="utf-8"
            )
            self.assertAllowed(
                f"gh api repos/o/r/issues/1/comments --input {harmless}"
            )

    def test_field_value_from_a_file_is_read(self):
        """`-F key=@<path>`の値をfileから読む。

        **`key=@path`という文字列のままでは`@coderabbitai`を含まないため素通りする**
        （2026-08-28に実測）。
        """
        with tempfile.TemporaryDirectory() as directory:
            body = Path(directory) / "body.txt"
            body.write_text("@coderabbitai full review", encoding="utf-8")
            for option in ("-F", "--field"):
                with self.subTest(option=option):
                    self.assertAsked(
                        f"gh api -X POST repos/o/r/issues/1/comments"
                        f" {option} body=@{body}"
                    )

    def test_raw_field_does_not_expand_at_sign(self):
        """`-f`／`--raw-field`の`@`をfileとして読まない。

        **`gh`は`-f`の値を文字列のまま送る**（`gh api --help`で確認した）。
        fileとして読むと、**実際には送られない内容でaskを出す**ことになる。
        """
        with tempfile.TemporaryDirectory() as directory:
            body = Path(directory) / "body.txt"
            body.write_text("@coderabbitai full review", encoding="utf-8")
            for option in ("-f", "--raw-field"):
                with self.subTest(option=option):
                    self.assertAllowed(
                        f"gh api -X POST repos/o/r/issues/1/comments"
                        f" {option} body=@{body}"
                    )

    def test_body_source_is_named_on_stderr(self):
        """読めなかった経路の名前を記録へ残す。

        **読める経路は3つある。**どれが読めなかったかを書かないと、記録から
        経路を辿れない。
        """
        for command, name in (
            (
                "gh api repos/o/r/issues/1/comments"
                " --input /nonexistent/b.json",
                "--input",
            ),
            (
                "gh api -X POST repos/o/r/issues/1/comments"
                " -F body=@/nonexistent/b.txt",
                "-F",
            ),
        ):
            with self.subTest(name=name):
                payload = json.dumps(
                    {"tool_name": "Bash", "tool_input": {"command": command}}
                )
                result = subprocess.run(
                    [sys.executable, CODERABBIT_GATE], input=payload,
                    capture_output=True, text=True,
                    encoding="utf-8", timeout=60,
                )
                # **askで止めたうえで**、経路の名前を記録へ残す。
                decision = json.loads(result.stdout.strip())
                self.assertEqual(
                    decision["hookSpecificOutput"]["permissionDecision"], "ask"
                )
                self.assertIn(name, result.stderr)

    def test_an_unreadable_body_is_recorded_on_stderr(self):
        """読めない`--body-file`をaskで止めつつ、**理由をstderrへも残す。**

        AGENTS.mdの「エラーを握りつぶさず、分類、ログ、カウンタを用意する」に従う。
        **askの診断文と、stderrの記録の両方に残す。**前者は人が読み、後者は後から辿る。
        """
        payload = json.dumps({
            "tool_name": "Bash",
            "tool_input": {"command": "gh pr comment 1 --body-file /nonexistent/x.md"},
        })
        result = subprocess.run(
            [sys.executable, CODERABBIT_GATE],
            input=payload, capture_output=True, text=True,
            encoding="utf-8", errors="replace", timeout=60,
        )
        self.assertEqual(result.returncode, 0)
        decision = json.loads(result.stdout.strip())
        self.assertEqual(
            decision["hookSpecificOutput"]["permissionDecision"], "ask"
        )
        self.assertIn("--body-file", _reason(decision))
        self.assertIn("coderabbit_gate:", result.stderr)
        self.assertIn("--body-file", result.stderr)

    def test_no_bypass_environment_variable(self):
        """**環境変数で無効化できないことを確認する。**

        他のhookは`DESKCAT_SKIP_*`を持つが、このhookは意図して持たない。
        **AIが自分で外せる抑制は抑制ではない。**#240で人間が決めた。
        """
        command = 'gh pr comment 239 --body "@coderabbitai full review"'
        for name in (
            "DESKCAT_SKIP_GH_GUARD",
            "DESKCAT_SKIP_CODERABBIT_GATE",
        ):
            with self.subTest(name=name):
                code, output = _invoke(CODERABBIT_GATE, command, environment={name: "1"})
                self.assertEqual(code, 0)
                self.assertIsNotNone(output, f"{name}で素通りした")
                self.assertEqual(
                    output["hookSpecificOutput"]["permissionDecision"], "ask"
                )


class GlobalOptionTests(unittest.TestCase):
    """`command_line`のglobal optionの読み飛ばし（#472）。"""

    def test_value_options_consume_their_value(self):
        for option in ("--git-dir", "--work-tree", "--namespace", "--config-env", "--super-prefix"):
            with self.subTest(option=option):
                self.assertEqual(
                    command_line.skip_global_options([option, "X", "push", "origin"], "git"),
                    ["push", "origin"],
                )
                self.assertEqual(
                    command_line.global_option_value([option, "X", "push"], "git", option),
                    "X",
                )
                self.assertEqual(
                    command_line.skip_global_options([f"{option}=X", "push"], "git"),
                    ["push"],
                )
                self.assertEqual(
                    command_line.global_option_value([f"{option}=X", "push"], "git", option),
                    "X",
                )

    def test_repeated_directory_option_joins_paths(self):
        """gitの`-C`は前の`-C`からの相対で解決する。絶対pathはそこから始め直す。"""
        value = command_line.global_option_value
        self.assertEqual(value(["-C", "a", "-C", "b", "push"], "git", "-C"), os.path.join("a", "b"))
        self.assertEqual(value(["-C", "a", "-C", "/abs", "push"], "git", "-C"), "/abs")
        self.assertEqual(value(["-C", "a", "-C", "", "push"], "git", "-C"), "a")
        self.assertEqual(value(["-C", "a", "push"], "git", "-C"), "a")

    def test_other_options_keep_the_last_value(self):
        self.assertEqual(
            command_line.global_option_value(["--repo", "a/x", "--repo", "b/y", "pr"], "gh", "--repo"),
            "b/y",
        )


class CommandFromTests(unittest.TestCase):
    """`command_line.command_from`のtest。**5本のhookが共有する入口である。**

    妥当なJSONでもmappingでない入力で`AttributeError`を出していた（#242）。
    **hookが例外で落ちると、止めているはずの判定が走らない。**
    """

    def test_valid_payload_returns_command(self):
        self.assertEqual(
            command_line.command_from({"tool_input": {"command": "gh pr create"}}),
            "gh pr create",
        )

    def test_non_mapping_payload_returns_none(self):
        """mappingでないpayloadで例外を出さない。"""
        for payload in ([], "text", None, 0, 1.5, True, ("a",)):
            with self.subTest(payload=payload):
                self.assertIsNone(command_line.command_from(payload))

    def test_non_mapping_tool_input_returns_none(self):
        """`tool_input`がmappingでない入力で例外を出さない。"""
        for value in (["x"], "x", 0, True):
            with self.subTest(value=value):
                self.assertIsNone(command_line.command_from({"tool_input": value}))

    def test_missing_pieces_return_none(self):
        for payload in ({}, {"tool_input": None}, {"tool_input": {}},
                        {"tool_input": {"command": None}},
                        {"tool_input": {"command": 1}},
                        {"tool_input": {"command": ["gh"]}}):
            with self.subTest(payload=payload):
                self.assertIsNone(command_line.command_from(payload))


class CommandLineSegmentsTests(unittest.TestCase):
    """`command_line.segments`のtest（#389）。**改行もcommandの区切りである。**

    `branch_base_guard`と`gh_metadata_guard`は
    `invocations`で呼び出しを探す。**2行目が見えないと、止める門が通常の書き方で
    反応しない。**

    **実測の条件（bash／python3／gitの版）は`hooks/command_line.py`のdocstringが持つ。**
    """

    def test_the_walk_without_segments_missed_the_second_line(self):
        """**直す前の挙動を固定する。**`segments`を通さない走査では2行目が消える。

        `_invocations_in`は改行を含まない1 commandを歩く内部関数であり、
        **#389より前の`invocations`と同じ走査である。**この差が#389の中身である。

        **固定するのは、下の7形で`push`から始まる並びが返らないことだけである。**
        **「空が返る」ではない。**1行目が`git`なら1件返り、2行目以降はその引数として
        飲み込まれる（`git fetch origin`改行`git push origin develop`は
        `[['fetch', 'origin', 'git', 'push', 'origin', 'develop']]`になる）。

        **複数行commandが必ずこうなる、という主張ではない。**行末へ区切り語を置いた
        `cat x &&`改行`git push origin develop`は、**旧走査でも拾えていた**
        （2026-09-14に実測）。#389が挙げたのは区切り語を置かない形である。

        """
        for command in (
            "cat x\ngit push origin develop",
            "cat x\r\ngit push origin develop",
            "cat x\n\n  git push origin develop",
            "git fetch origin\ngit push origin develop",
            "git add -A\ngit commit -m x\ngit push origin develop",
            "git status\ngit push origin HEAD:develop",
            "git status\ngit add -A\ngit commit -m x\ngit push origin develop",
        ):
            with self.subTest(command=command):
                def pushes(calls):
                    return [call for call in calls if call and call[0] == "push"]

                self.assertEqual(
                    pushes(command_line._invocations_in(command, "git")), [],
                    "旧走査がpushを拾ってしまう",
                )
                self.assertNotEqual(
                    pushes(command_line.invocations(command, "git")), [],
                    "新走査がpushを拾えていない",
                )

    def test_the_walk_without_segments_broke_the_line_continuation(self):
        r"""行継続では、旧走査は改行1文字を語として返していた。

        `git push \`改行`origin develop`の第1引数が改行1文字になるため、
        **引数を読むhookは`origin`を読めなかった。**
        """
        command = "git push \\\n  origin develop"
        self.assertEqual(
            command_line._invocations_in(command, "git"),
            [["push", "\n", "origin", "develop"]],
        )
        self.assertEqual(
            command_line.invocations(command, "git"), [["push", "origin", "develop"]]
        )

    def test_the_walk_without_segments_counted_a_comment_as_a_refspec(self):
        """コメントを落とさない走査では、`# develop`の`develop`が引数に残っていた。

        引数を読むhookは、これを引数として数えていた。同じ走査で直した。
        """
        command = "git push origin main # develop"
        self.assertEqual(
            command_line._invocations_in(command, "git"),
            [["push", "origin", "main", "#", "develop"]],
        )
        self.assertEqual(
            command_line.invocations(command, "git"), [["push", "origin", "main"]]
        )

    def test_heredoc_delimiter_of_an_arithmetic_shift(self):
        """算術の`<<`から読むdelimiterが何になるかを固定する。

        **機構まで固定する。**結果だけを見るtestは、delimiterが`2`でも`2))`でも通る。
        `)`は`DELIMITER_END`に入っているため`2`で切れる。
        """
        self.assertEqual(
            command_line._heredoc_delimiter("echo $((1 << 2))", len("echo $((1 <<")),
            ("2", len("echo $((1 << 2")),
        )
        self.assertEqual(
            command_line._heredoc_delimiter("(( i <<= 1 ))", len("(( i <<")),
            ("=", len("(( i <<=")),
        )

    def test_hash_after_a_brace_or_an_escaped_space_is_not_a_comment(self):
        r"""`${#x}`と`a\ #b`と`$(date)#x`の`#`をコメントと読まない。**bashも読まない。**

        `{`／`}`／`!`／`)`は語の区切りとして数えない。**数えると行末までを捨て、
        `git push`を取り落とす**（門が黙る側である）。`)`だけは形で意味が変わり
        （`(echo hi)#x`ではコメントになる）、**字句では区別できないため入れていない。**
        """
        for command in (
            "test ${#x} -gt 0 && git push origin develop",
            "echo ${x}#1 && git push origin develop",
            "echo a\\ #b && git push origin develop",
            "echo $(date)#x ; git push origin develop",
        ):
            with self.subTest(command=command):
                self.assertEqual(
                    command_line.invocations(command, "git"),
                    [["push", "origin", "develop"]],
                )

    def test_empty_heredoc_delimiter_is_not_treated_as_a_heredoc(self):
        """`<<''`はheredocとして扱わない。**bashは空行またはEOFまでをbodyとして読む。**

        この形には空行が無いため、bashはendまでbodyとして読み、`git`を実行しない
        （2026-09-14に実測。`here-document delimited by end-of-file`の警告が出る）。
        こちらはbodyを落とさず検査するため、**過検出の側である。**
        乖離そのものは`DIVERGENT_CASES`が持つ。
        """
        self.assertEqual(
            command_line.invocations("cat <<''\ngit push origin develop", "git"),
            [["push", "origin", "develop"]],
        )

    def test_partially_terminated_heredocs_keep_everything(self):
        """`cat <<A <<B`でAだけ終端している場合、Aのbodyごと検査する。

        **途中まで捨てると、どこまで捨てたかで判定が変わる。**
        1つでも終端行が無ければ1文字も捨てない（`_skip_heredoc_bodies`）。
        """
        self.assertEqual(
            command_line.invocations("cat <<A <<B\nx\nA\ngit push origin develop", "git"),
            [["push", "origin", "develop"]],
        )

    def test_control_flow_body_is_detected_even_if_bash_would_skip_it(self):
        """条件分岐の中の行も呼び出しとして返す。**実行されるかは判定しない。**

        `cat x && git push`の短絡と同じ扱いであり、**過検出の側である。**
        止める門としては安全側に出るため、合わせない。
        """
        for command in (
            "if false; then\ngit push origin develop\nfi",
            "for i in 1 2; do\ngit push origin develop\ndone",
        ):
            with self.subTest(command=command):
                self.assertEqual(
                    command_line.invocations(command, "git"),
                    [["push", "origin", "develop"]],
                )

    def test_control_flow_written_on_one_line_is_not_detected(self):
        """**1行で書いた場合は逆になる。**command位置が`git`まで戻らない。

        落ちる語は書き方で変わる。`false;`と続けると区切り語が現れず`if`で落ち、
        `false ; then`と空けると`;`で戻ってから`then`で落ちる。
        **どちらも取り落としであり、#389では直していない。**
        """
        for command in (
            "if false; then git push origin develop; fi",
            "if false ; then git push origin develop ; fi",
        ):
            with self.subTest(command=command):
                self.assertEqual(command_line.invocations(command, "git"), [])

    def test_separator_without_a_leading_space_is_still_missed(self):
        """**`;`／`&&`の直前に空白が無い形は今も素通りする。**#389では直していない。

        `shlex`は空白でしか語を切らないため、`x;`が1語になり`git`がcommand位置から
        外れる。**bashは両方を実行する。**
        **直したらこのtestが落ちる。**そのときはCONTRIBUTINGの「取り切れていないもの」も直す。
        """
        for command in (
            "cat x; git push origin develop",
            "cat x;git push origin develop",
            "cat x&&git push origin develop",
        ):
            with self.subTest(command=command):
                self.assertEqual(command_line.invocations(command, "git"), [])

    def test_command_substitution_on_one_line_is_still_missed(self):
        """**1行に収めた`$(...)`と`eval`の内側は今も素通りする。**#389では直していない。

        CONTRIBUTINGの「取り切れていないもの」に書いた形である。
        **改行を挟んだ`$( )`の中の行は、行として拾う。**その差を並べて固定する。
        **直したらこのtestが落ちる。**そのときはCONTRIBUTINGも直す。
        """
        for command in (
            "echo $(git push origin develop)",
            "eval 'git push origin develop'",
        ):
            with self.subTest(command=command):
                self.assertEqual(command_line.invocations(command, "git"), [])
        self.assertEqual(
            command_line.invocations("MSG=$(\ngit push origin develop\n)", "git"),
            [["push", "origin", "develop"]],
        )

    def test_word_initial_comment_is_dropped(self):
        """語頭の`#`から行末までを落とす。**bashと同じ扱いである。**"""
        self.assertEqual(
            command_line.invocations("git push origin main # develop", "git"),
            [["push", "origin", "main"]],
        )
        self.assertEqual(
            command_line.invocations("# git push origin develop\ngit status", "git"),
            [["status"]],
        )

    def test_hash_inside_a_word_or_quotes_is_kept(self):
        """語の中と引用の中の`#`は落とさない。**bashも落とさない。**"""
        self.assertEqual(
            command_line.invocations("git log --format=%h#x", "git"),
            [["log", "--format=%h#x"]],
        )
        self.assertEqual(
            command_line.invocations("git commit -m 'fix #389'", "git"),
            [["commit", "-m", "fix #389"]],
        )

    def test_arithmetic_shift_is_not_read_as_a_heredoc_that_eats_the_rest(self):
        """`$((1 << 2))`の`<<`でheredocと読み違えても、残りの行を捨てない。

        delimiterは`2`になり（`)`は`DELIMITER_END`にある）、終端行は現れない。
        **捨てると後続の`git push`が門から消える。**`_skip_heredoc_bodies`は
        終端行が無ければ1文字も捨てない。**delimiterの値そのものは
        `test_heredoc_delimiter_of_an_arithmetic_shift`が固定する。**
        """
        for command in (
            "echo $((1 << 2))\ngit push origin develop",
            "(( i <<= 1 ))\ngit push origin develop",
        ):
            with self.subTest(command=command):
                self.assertEqual(
                    command_line.invocations(command, "git"),
                    [["push", "origin", "develop"]],
                )

    def test_heredoc_without_a_terminator_does_not_hide_the_rest(self):
        """終端行の無いheredocでは、残りを検査する側へ倒す。

        **bashはEOFまでbodyとして読むため、bashは`git push`を実行しない。**
        それでも検査するのは、`<<`を読み違えた場合に門が黙るのを避けるためである
        （`_skip_heredoc_bodies`のdocstring）。**bashと一致しない側の判断であり、
        `BASH_CASES`へは入れていない。**
        """
        self.assertEqual(
            command_line.invocations("cat <<EOF\ngit push origin develop", "git"),
            [["push", "origin", "develop"]],
        )

    def test_newline_separates_commands(self):
        """改行、CRLF、空行＋字下げのいずれでも2行目を呼び出しとして見る。

        **4形とも#389より前は素通りしていた。**旧走査（`_invocations_in`）が
        `push`から始まる並びを返さないことを
        `test_the_walk_without_segments_missed_the_second_line`が固定しており、
        **この差が#389の中身である。**
        """
        for command in (
            "cat x\ngit push origin develop",
            "cat x\r\ngit push origin develop",
            "cat x\n\n  git push origin develop",
            "git status\ngit add -A\ngit commit -m x\ngit push origin develop",
        ):
            with self.subTest(command=command):
                self.assertEqual(
                    command_line.invocations(command, "git")[-1],
                    ["push", "origin", "develop"],
                )

    def test_existing_separators_still_split(self):
        """`;`／`&&`／`&`／`|`／`!`／`{`で検出できていた形を落とさない。"""
        for separator in (";", "&&", "&", "|", "!", "{"):
            command = f"cat x {separator} git push origin develop"
            with self.subTest(separator=separator):
                self.assertEqual(
                    command_line.invocations(command, "git"),
                    [["push", "origin", "develop"]],
                )

    def test_quoted_newline_does_not_split(self):
        """引用の中の改行では切らない。**素朴に改行で切ると引用符が途中で切れる。**

        切れた側は`tokenize`が失敗して空になり、**同じcommandのpushを取り落とす。**
        """
        command = 'git commit -m "題\n\n本文" && git push origin develop'
        self.assertEqual(
            command_line.invocations(command, "git"),
            [["commit", "-m", "題\n\n本文"], ["push", "origin", "develop"]],
        )

    def test_heredoc_body_is_not_a_command(self):
        """heredocのbodyを呼び出しとして数えない。**bashは実行しない。**

        `<<EOF`／`<<'EOF'`／`<<"EOF"`／`<< EOF`／`<<-EOF`のいずれの書き方でも落とす。
        """
        for command in (
            "cat > doc.md <<'EOF'\ngit push origin develop\nEOF\n",
            'cat > doc.md <<"EOF"\ngit push origin develop\nEOF\n',
            "cat > doc.md <<EOF\ngit push origin develop\nEOF\n",
            "cat > doc.md << 'EOF'\ngit push origin develop\nEOF\n",
            "cat > doc.md <<-EOF\n\tgit push origin develop\n\tEOF\n",
        ):
            with self.subTest(command=command):
                self.assertEqual(command_line.invocations(command, "git"), [])

    def test_command_after_a_heredoc_body_is_inspected(self):
        """bodyの後ろのcommandは見る。**bodyの引用符で検査が止まらない。**

        bodyを落とさずに`tokenize`へ渡すと、body の`'`1つで`shlex`が失敗し、
        **後ろの`git push`まで検査できない。**
        """
        command = (
            "cat > doc.md <<'EOF'\n"
            "それはできない。don't\n"
            "EOF\n"
            "git push origin develop"
        )
        self.assertEqual(
            command_line.invocations(command, "git"),
            [["push", "origin", "develop"]],
        )

    def test_two_heredocs_on_one_line_are_both_skipped(self):
        """1行に2つ開いたheredocのbodyを、開いた順に読み飛ばす。"""
        command = (
            "cat <<A <<B\n"
            "git push origin develop\n"
            "A\n"
            "gh pr merge 1\n"
            "B\n"
            "git push origin develop"
        )
        self.assertEqual(
            command_line.invocations(command, "git"),
            [["push", "origin", "develop"]],
        )
        self.assertEqual(command_line.invocations(command, "gh"), [])

    def test_carriage_return_is_not_a_separator(self):
        r"""`\r`は区切りにしない。**bashも語の一部として扱う。**

        `shlex`が`\r`を空白として落とすため、`develop\r`は`develop`として読む。
        bashへ渡せば`fatal: invalid refspec`で落ちる形であり（2026-09-12に実測）、
        **こちらはbashより厳しい側へ出る。**取り落としではないため合わせない。
        """
        self.assertEqual(
            command_line.segments("git push origin develop\r"),
            ["git push origin develop\r"],
        )
        self.assertEqual(
            command_line.invocations("git push origin develop\r", "git"),
            [["push", "origin", "develop"]],
        )

    def test_crlf_heredoc_body_is_skipped(self):
        r"""CRLFのfileでも、delimiterと終端行がどちらも`EOF\r`になり body を落とす。

        **走るcommandの並びはbashと同じである。**bashもbodyの`git push`を実行せず、
        `git tag`だけを実行する。**これは`DIVERGENT_CASES`の同じ形が、
        bashを起動して毎回測っている**（`test_known_divergences_from_bash_are_measured`）。
        **引数は一致しない。**bashは`v1\r`を渡すが、`shlex`が`\r`を落とすため`v1`として読む。
        **そのため`BASH_CASES`へは入れていない。**
        """
        command = "cat > d.md <<EOF\r\ngit push origin develop\r\nEOF\r\ngit tag v1\r\n"
        self.assertEqual(command_line.invocations(command, "git"), [["tag", "v1"]])

    def test_herestring_has_no_body(self):
        """`<<<`はherestringであり、bodyを持たない。次の行は普通のcommandである。"""
        self.assertEqual(
            command_line.invocations("grep x <<<'text'\ngit push origin develop", "git"),
            [["push", "origin", "develop"]],
        )

    def test_line_continuation_joins_the_command(self):
        r"""`\`改行はbashと同じく消す。**残すと引数の位置がずれる。**

        `shlex`は`\`改行を改行1文字の語として返すため、`git push \`改行`origin develop`の
        `origin`が第1引数でなくなる（2026-09-12に実測）。
        """
        for command in (
            "git push \\\n  origin develop",
            "git \\\n  push origin develop",
        ):
            with self.subTest(command=command):
                self.assertEqual(
                    command_line.invocations(command, "git"),
                    [["push", "origin", "develop"]],
                )

    def test_unterminated_quote_is_still_not_inspected(self):
        """引用符が閉じていないcommandは検査しない。**今と同じ扱いである。**

        引用の中で切らないため、後ろの行も引用の中として読む。`tokenize`が失敗し、
        `invocations`は空を返す。**取り落としであって誤検知ではない。**
        """
        self.assertEqual(
            command_line.invocations("echo 'x\ngit push origin develop", "git"), []
        )

    def test_tokenize_still_treats_newline_as_whitespace(self):
        """**`tokenize`は変えていない。**語の切り方をhookが共有している。

        改行を語として返すようにすると、それらの判定まで動く。
        **区切りの追加は`segments`側で持つ。**
        """
        self.assertEqual(
            command_line.tokenize("cat x\ngit push"), ["cat", "x", "git", "push"]
        )

    # `git`の呼び出しを記録するだけのshim。**PATHの先頭へ置いて本物を隠す。**
    # shebangは`sys.executable`で書く。**`python3`がPATHに無い環境でも記録が空にならない。**
    GIT_SHIM = (
        f"#!{sys.executable}\n"
        "import json, os, sys\n"
        "open(os.environ['GIT_SHIM_LOG'], 'a', encoding='utf-8')"
        ".write(json.dumps(sys.argv[1:]) + '\\n')\n"
    )

    # bashが実際に起動する`git`と突き合わせる形。改行（LF／CRLF／空行と字下げ）、
    # 既存の区切り、heredoc（`<<`／`<<-`／終端後の続き／2つ同時）、行継続、
    # コメント、算術の`<<`を含む。**この表は等値で突き合わせるため、
    # bashと一致しない形は入れない。**それらは`DIVERGENT_CASES`が持つ
    # （**数はそちらが正本である。ここへ書き写さない。**理由は各testが持つ）。
    BASH_CASES = (
        "cat x\ngit push origin develop",
        "cat x\r\ngit push origin develop",
        "cat x\n\n  git push origin develop",
        "git status\ngit add -A\ngit commit -m x\ngit push origin develop",
        "cat x ; git push origin develop",
        "cat x && git push origin develop",
        "cat x | git push origin develop",
        "git push \\\n  origin develop",
        "git \\\n  push origin develop",
        'git commit -m "題\n\n本文" && git push origin develop',
        "cat > doc.md <<'EOF'\ngit push origin develop\nEOF\ngit status",
        "cat > doc.md <<-EOF\n\tgit push origin develop\n\tEOF\ngit status",
        "echo $((1 << 2))\ngit push origin develop",
        "git push origin main # develop",
        # **`gh`は使わない。**本物の`gh`は内部で`git`を起動するため、shimがそれを拾う。
        "echo x # git push origin develop",
        # 変数を空にすると`&&`が短絡し、bashは`git`を起動しない。**値を入れて測る。**
        "x=abc\ntest ${#x} -gt 0 && git push origin develop",
        "echo ${x}#1\ngit status",
        "echo a\\ #b && git push origin develop",
        "echo $(date)#x ; git push origin develop",
        "if false; then git push origin develop; fi",
        "MSG=$(\ngit push origin develop\n)",
        "cat > doc.md <<'EOF'\nそれはできない。don't\nEOF\ngit push origin develop",
        "cat <<A <<B\ngit push origin develop\nA\ngit tag v1\nB\ngit push origin develop",
        "grep x <<<'text'\ngit push origin develop",
        "# git push origin develop\ngit status",
        "echo 'git push origin develop'\ngit status",
        "env FOO=1 git push origin develop\ngit status",
        "cd .\ngit push origin develop",
        "{ git push origin develop ; }\ngit status",
    )

    def test_invocations_match_what_bash_executes(self):
        """**bashが実際に起動した`git`と、`invocations`が返す並びを突き合わせる。**

        #389でずれていたのは、guardとbashでcommandの区切りが違ったためである。
        **ずれを直接測る。**`git`を記録するだけのshimでPATHの先頭を差し替え、
        `bash -c`で実行して、記録された引数の並びと比較する。

        **本物の`git`は走らない。**shimが同じ名前で先に見つかる。実行するdirectoryも
        tempdirであり、`cat > doc.md`はそこへ書く。
        """
        bash = shutil.which("bash")
        if bash is None:
            self.skipTest("bashが無い")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            shim = root / "git"
            shim.write_text(self.GIT_SHIM, encoding="utf-8")
            shim.chmod(0o755)
            (root / "x").write_text("x\n", encoding="utf-8")
            log = root / "log.jsonl"
            environment = dict(os.environ)
            environment["PATH"] = f"{root}{os.pathsep}{environment['PATH']}"
            environment["GIT_SHIM_LOG"] = str(log)
            for command in self.BASH_CASES:
                with self.subTest(command=command):
                    log.write_text("", encoding="utf-8")
                    subprocess.run(
                        [bash, "-c", command], cwd=str(root), env=environment,
                        capture_output=True, text=True, encoding="utf-8", timeout=60,
                    )
                    executed = [
                        json.loads(line)
                        for line in log.read_text(encoding="utf-8").splitlines()
                        if line.strip()
                    ]
                    self.assertEqual(
                        command_line.invocations(command, "git"), executed,
                        f"bashが実行したものと一致しない: {command!r}",
                    )

    # bashと一致しない形。**ずれていることを測る。**`BASH_CASES`の等値検査へは入れられない。
    DIVERGENT_CASES = (
        # `shlex`が`\r`を空白として落とすため、引数末尾の`\r`が消える。
        ("git push origin develop\r",
         [["push", "origin", "develop\r"]], [["push", "origin", "develop"]]),
        # 同じ理由。bodyの読み飛ばし自体はbashと同じ結果になる。
        ("cat > d.md <<EOF\r\ngit push origin develop\r\nEOF\r\ngit tag v1\r\n",
         [["tag", "v1\r"]], [["tag", "v1"]]),
        # 終端行の無いheredoc。bashはEOFまでbodyとして読み、`git`を実行しない。
        ("cat <<EOF\ngit push origin develop",
         [], [["push", "origin", "develop"]]),
        # `)`を語の区切りに数えないため、`#`をコメントと読まない。bashは読む。
        # **`)`は形で意味が変わり、字句では区別できない**（`WORD_BREAKS`のコメント）。
        ("(echo hi)#x ; git push origin develop",
         [], [["push", "origin", "develop"]]),
        # 多重度の乖離。**bashは2回起動するが、返すのは1件である。**
        ("for i in 1 2; do\ngit push origin develop\ndone",
         [["push", "origin", "develop"], ["push", "origin", "develop"]],
         [["push", "origin", "develop"]]),
        # 空delimiter。bashは空行かEOFまでbodyとして読み、`git`を実行しない。
        ("cat <<''\ngit push origin develop",
         [], [["push", "origin", "develop"]]),
        # 部分終端。Bのbodyがそのままendまで伸び、bashは`git`を実行しない。
        ("cat <<A <<B\nx\nA\ngit push origin develop",
         [], [["push", "origin", "develop"]]),
        # **門が黙る側。**算術の`<<`をheredocと読み違え、後から現れた`2`を終端行と見て
        # bodyごと落とす。**bashはpushを実行する。**
        ("echo $((1 << 2))\ngit push origin develop\n2\n",
         [["push", "origin", "develop"]], []),
        # **門が黙る側。**bodyを実行する側へ流す形。落としてよいのは、受け取る側が
        # bodyをcommandとして実行しない場合だけである。
        ("bash <<'EOF'\ncd . ; git push origin develop\nEOF\n",
         [["push", "origin", "develop"]], []),
        # 条件分岐。bashは走らせないが、字句だけで見るこちらは1件返す。
        ("if false; then\ngit push origin develop\nfi",
         [], [["push", "origin", "develop"]]),
    )

    def test_known_divergences_from_bash_are_measured(self):
        """**bashと一致しない形を、ずれたまま固定する。**

        `BASH_CASES`は等値で突き合わせるため、これらは入れられない。
        **入れずに黙っていると、ずれが記録から消える。**各形の判断の理由は
        `hooks/command_line.py`のdocstringと、対応する個別のtestが持つ。

        **`BASH_CASES`と同じ仕組みで、一致しないことの方を測る。**引数の中身だけがずれる形も、
        起動の有無や回数がずれる形も入れてある。

        **ここに在るのは乖離の全件ではない。**`;`の直前に空白が無い形は
        `test_separator_without_a_leading_space_is_still_missed`が、引用の中の`\\`改行は
        `CodeRabbitGateTests.test_line_continuation_inside_the_body_is_still_missed`が固定しており、
        **bashと突き合わせる形では測っていない。**
        """
        bash = shutil.which("bash")
        if bash is None:
            self.skipTest("bashが無い")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            shim = root / "git"
            shim.write_text(self.GIT_SHIM, encoding="utf-8")
            shim.chmod(0o755)
            log = root / "log.jsonl"
            environment = dict(os.environ)
            environment["PATH"] = f"{root}{os.pathsep}{environment['PATH']}"
            environment["GIT_SHIM_LOG"] = str(log)
            for command, by_bash, by_parse in self.DIVERGENT_CASES:
                with self.subTest(command=command):
                    log.write_text("", encoding="utf-8")
                    subprocess.run(
                        [bash, "-c", command], cwd=str(root), env=environment,
                        capture_output=True, text=True, encoding="utf-8", timeout=60,
                    )
                    executed = [
                        json.loads(line)
                        for line in log.read_text(encoding="utf-8").splitlines()
                        if line.strip()
                    ]
                    self.assertEqual(executed, by_bash, "bash側の観測が変わった")
                    self.assertEqual(
                        command_line.invocations(command, "git"), by_parse,
                        "こちら側の読みが変わった",
                    )

    def test_blank_and_empty_commands_have_no_segments(self):
        """空と空白だけのcommandは区切りを持たない。**空を1つのcommandと数えない。**"""
        for command in ("", "   ", "\n", "\n\n  \n"):
            with self.subTest(command=command):
                self.assertEqual(command_line.segments(command), [])


class HookPayloadShapeTests(unittest.TestCase):
    """**hookが、mappingでない入力で落ちないことを確かめる**（#242）。

    [PR #241](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/241)のreview指摘は`coderabbit_gate.py`に対するものだったが、
    **型で全数走査したら当時の5本すべてに同じ形が残っていた。**指摘は代表例であって全数ではない。

    **後から足したhookもこの一覧へ入れる。**一覧に無いhookは、壊れても静かに素通りする。
    """

    HOOKS = (GH_GUARD, BASE_GUARD, CODERABBIT_GATE, WORKTREE_GUARD)

    MALFORMED = (
        "[]",
        '"text"',
        "null",
        "0",
        '{"tool_input": ["x"]}',
        '{"tool_input": "x"}',
        '{"tool_input": {"command": 1}}',
        "{}",
    )

    def test_no_hook_raises_on_malformed_payload(self):
        """例外を出さず、素通りする。**return code 0かつstdoutが空である。**"""
        for script in self.HOOKS:
            for payload in self.MALFORMED:
                with self.subTest(script=Path(script).name, payload=payload):
                    result = subprocess.run(
                        [sys.executable, script],
                        input=payload, capture_output=True, text=True,
                        encoding="utf-8", errors="replace", timeout=120,
                        env={**os.environ,
                             "DESKCAT_SKIP_GH_GUARD": "",
                             "DESKCAT_SKIP_BASE_GUARD": "",
                             "DESKCAT_SKIP_WORKTREE_GUARD": ""},
                    )
                    self.assertEqual(result.returncode, 0,
                                     f"{Path(script).name} が落ちた: {result.stderr[:300]}")
                    self.assertNotIn("Traceback", result.stderr,
                                     f"{Path(script).name} が例外を出した")
                    self.assertEqual(result.stdout.strip(), "",
                                     f"{Path(script).name} が止めてしまった")


class ReviewerAgentToolsTests(unittest.TestCase):
    """**レビュー役の subagent は Bash を持たない。**読み取り専用を tools で絞る（#813、#814 の作業4）。"""

    def test_reviewer_agent_has_no_bash(self):
        text = (REPO_ROOT_FOR_TEMPLATES / ".claude" / "agents" / "fresh-context-reviewer.md").read_text(
            encoding="utf-8")
        front = text[4:text.index("\n---\n", 4) + 1]
        tools = [line for line in front.splitlines() if line.startswith("tools:")]
        self.assertEqual(len(tools), 1, "tools の行が1つでない")
        names = [name.strip() for name in tools[0][len("tools:"):].split(",")]
        self.assertEqual(sorted(names), ["Glob", "Grep", "Read"])
        self.assertNotIn("hooks:", front)


class SourceCompileWarningTests(unittest.TestCase):
    """**hooksに限らず、`git ls-files`の全`.py`を対象にする。**"""

    def test_no_python_source_warns_on_compile(self):
        """compileで警告を出す`.py`を置かない。

        **新しいPythonでは、invalid escape sequenceがSyntaxWarningとしてstderrへ出る。**
        hookの子processのstderrが空であることを確かめるtestがそれで落ち、
        push前の検査を通せなくなった。**CIでは出ない。**このfileが先に`command_line`を
        importして`.pyc`を作るため、子processはsourceをcompileし直さない。
        `PYTHONDONTWRITEBYTECODE=1`の端末でだけ表に出る。
        """
        result = subprocess.run(
            ["git", "ls-files", "-z", "*.py"],
            cwd=REPO_ROOT_FOR_TEMPLATES, capture_output=True, check=True,
        )
        paths = [p for p in result.stdout.decode("utf-8").split("\0") if p]
        self.assertTrue(paths, "git ls-filesが.pyを1本も返さない")
        found = []
        for path in paths:
            source = (REPO_ROOT_FOR_TEMPLATES / path).read_text(encoding="utf-8")
            with warnings.catch_warnings(record=True) as caught:
                warnings.simplefilter("always")
                compile(source, path, "exec")
            found += [f"{path}:{w.lineno}: {w.message}" for w in caught
                      if issubclass(w.category, (SyntaxWarning, DeprecationWarning))]
        self.assertEqual(found, [])


if __name__ == "__main__":
    unittest.main(verbosity=2)
