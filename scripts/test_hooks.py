#!/usr/bin/env python3
"""`hooks/`配下の回帰test。

hookを子processとして起動し、stdinへhookの入力JSONを渡して、**拒否したか通したか**と
診断文を検査する。基点の検査はfixture repositoryを作って実際に`git fetch`させる。

**主に見るのは、素通りしないことである。**guardが黙って通るようになると、
止めているはずの失敗（boardへのitem追加漏れ、trailerの引き継ぎ漏れ、古い基点）が
また起きる。**そしてhookは失敗しても静かなため、壊れたことに気付けない。**
"""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS_ROOT / "lib"))

import publish_guards as guards  # noqa: E402

sys.path.insert(0, str(SCRIPTS_ROOT))

import review_gate as gate  # noqa: E402

GH_GUARD = str(SCRIPTS_ROOT / "hooks" / "gh_metadata_guard.py")
BASE_GUARD = str(SCRIPTS_ROOT / "hooks" / "branch_base_guard.py")
MERGE_REPORT = str(SCRIPTS_ROOT / "hooks" / "merge_trailer_report.py")

# fixtureのcommitに使うidentity。実行者の設定に依存させない。
GIT_IDENTITY = (
    "-c", "user.name=test",
    "-c", "user.email=test@example.invalid",
    "-c", "commit.gpgsign=false",
)


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


def _invoke(script, command, cwd=None, environment=None):
    """hookへcommandを渡し`(exit code, 解釈した出力)`を返す。

    出力が空のときは`None`を返す。**空は「通した」を意味する。**
    """
    payload = json.dumps({"tool_name": "Bash", "tool_input": {"command": command}})
    env = dict(os.environ)
    # 呼び出し側の環境に無効化用の変数が残っていても、testの前提を壊さない。
    env.pop("DESKCAT_SKIP_GH_GUARD", None)
    env.pop("DESKCAT_SKIP_BASE_GUARD", None)
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

    def test_create_without_project_is_denied(self):
        """`--project`が無いIssue／Pull Request作成を止める。"""
        for command in (
            "gh pr create --title t --body b",
            "gh issue create --title t",
        ):
            with self.subTest(command=command):
                self.assertDenied(command, contains="--project")

    def test_create_with_project_is_allowed(self):
        """`--project value`と`--project=value`のどちらも通す。"""
        for command in (
            "gh pr create --title t --project deskcat",
            "gh pr create --title t --project=deskcat",
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
            "gh pr create --title t -p deskcat",
            "gh issue create --title t -p deskcat",
            "gh pr create --title t -pdeskcat",
            "gh pr create --title t -p=deskcat",
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
        """messageを渡さないmergeを止める。GitHubが合成したmessageにはtrailerが入らない。"""
        self.assertDenied("gh pr merge 1 --squash", contains="18298ae")

    def test_merge_with_unreadable_message_is_denied(self):
        """本文を読めない経路を「入っている」と扱わない。"""
        self.assertDenied("gh pr merge 1 --squash -F -", contains="stdin")
        self.assertDenied(
            "gh pr merge 1 --squash --body-file /nonexistent/body.txt",
            contains="読み出しに失敗",
        )

    def test_merge_message_trailers_are_checked(self):
        """squash messageのtrailerを、fileとinlineの両方で見る。"""
        with tempfile.TemporaryDirectory() as directory:
            complete = Path(directory) / "complete.txt"
            complete.write_text(
                f"body\n\n{gate.TRAILER_CLASS}: x\n{gate.TRAILER_REVIEW}: y\n",
                encoding="utf-8",
            )
            partial = Path(directory) / "partial.txt"
            partial.write_text(
                f"body\n\n{gate.TRAILER_REVIEW}: y\n", encoding="utf-8"
            )
            self.assertAllowed(f"gh pr merge 1 --squash --body-file {complete}")
            self.assertDenied(
                f"gh pr merge 1 --squash --body-file {partial}",
                contains=gate.TRAILER_CLASS,
            )
        self.assertAllowed(
            "gh pr merge 1 --squash --body "
            f'"x\n\n{gate.TRAILER_CLASS}: c\n{gate.TRAILER_REVIEW}: s"'
        )
        self.assertDenied('gh pr merge 1 --squash --body "本文だけ"')

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

    def test_broken_input_does_not_block(self):
        """hookの入力やcommandが壊れていることを、対象commandの問題として扱わない。"""
        result = subprocess.run(
            [sys.executable, GH_GUARD], input="{ではないJSON",
            capture_output=True, text=True, encoding="utf-8", timeout=60,
        )
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.strip(), "")
        # 引用符が閉じていないcommandは、shell自身が落とす。ここで二重に報告しない。
        self.assertAllowed('gh pr create --title "閉じていない')


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


class MergeTrailerReportTests(unittest.TestCase):
    """merge後の確認hookのtest。

    **`gh`と実際のPull Requestを要する経路は検査しない。**testがnetworkとGitHubの
    状態に依存すると、落ちた理由がhookの誤りか環境かを区別できなくなる。
    ここで見るのは、対象外のcommandで`gh`を呼ばずに抜けることである。
    """

    def test_non_merge_commands_are_ignored(self):
        for command in (
            "git status",
            "gh pr view 1 --json state",
            "gh pr create --title t --project deskcat",
            "echo gh pr merge",
        ):
            with self.subTest(command=command):
                code, output = _invoke(MERGE_REPORT, command)
                self.assertEqual(code, 0, command)
                self.assertIsNone(output, f"報告してしまった: {command}")

    def test_broken_input_is_ignored(self):
        result = subprocess.run(
            [sys.executable, MERGE_REPORT], input="{ではないJSON",
            capture_output=True, text=True, encoding="utf-8", timeout=60,
        )
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.strip(), "")


if __name__ == "__main__":
    unittest.main(verbosity=2)
