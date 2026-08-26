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
PUSH_GATE = str(SCRIPTS_ROOT / "hooks" / "push_gate.py")

sys.path.insert(0, str(SCRIPTS_ROOT / "hooks"))

import push_gate  # noqa: E402

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
    env.pop("DESKCAT_SKIP_PUSH_GATE", None)
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


class PushGateTests(unittest.TestCase):
    """`develop`への直接pushを`gate`で見るhookのtest。

    **fixture repositoryへ`review_gate.py`を複製して実際に実行させる。**
    hookが呼ぶのはsubprocessであり、呼べているかどうかを模擬では確かめられない。
    """

    # `docs/decisions/`は`INSTRUCTION_SOURCES`に入る。`Instruction-Change`が要る。
    INSTRUCTION_FILE = "docs/decisions/0001-x.md"

    def setUp(self):
        self.directory = tempfile.mkdtemp()
        self.addCleanup(guards.remove_tree, Path(self.directory))
        root = Path(self.directory)
        self.root = root
        _git(str(root), "init", "--quiet", ".")
        (root / "scripts").mkdir()
        (root / "scripts" / "review_gate.py").write_text(
            (SCRIPTS_ROOT / "review_gate.py").read_text(encoding="utf-8"),
            encoding="utf-8",
        )
        (root / "note.md").write_text("最初\n", encoding="utf-8")
        _git(str(root), "add", "-A")
        _git(str(root), "commit", "--quiet", "-m", "first")
        # 自分自身をoriginにする。networkへ出ずに`origin/develop`を成立させる。
        _git(str(root), "branch", "--force", "trunk", "HEAD")
        _git(str(root), "remote", "add", "origin", str(root))
        _git(
            str(root), "config", "remote.origin.fetch",
            "+refs/heads/trunk:refs/remotes/origin/develop",
        )
        _git(str(root), "fetch", "--quiet", "origin")

    def _instruction_commit(self, declared):
        """指示sourceを触るcommitを1つ積む。`declared`で宣言の有無を切り替える。"""
        target = self.root / self.INSTRUCTION_FILE
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text("# 決定\n\n本文\n", encoding="utf-8")
        trailers = [
            f"{gate.TRAILER_CLASS}: {gate.CLASS_FIXUP}",
            *(
                f"{gate.TRAILER_REVIEW}: {value}"
                for value in gate.REVIEW_DECLARATIONS
            ),
            f"{gate.TRAILER_REFS}: #1",
        ]
        if declared:
            trailers.append(
                f"{gate.TRAILER_INSTRUCTION}: {gate.INSTRUCTION_ACK}"
            )
        _git(str(self.root), "add", "-A")
        _git(
            str(self.root), "commit", "--quiet",
            "-m", "決定を足す\n\n" + "\n".join(trailers) + "\n",
        )

    def _review_gate(self, command):
        return subprocess.run(
            [
                sys.executable, str(self.root / "scripts" / "review_gate.py"),
                command, "--repository-root", str(self.root),
                "--base", "origin/develop", "--head", "HEAD",
            ],
            capture_output=True, text=True, encoding="utf-8", cwd=str(self.root),
        ).returncode

    def assertDenied(self, command, cwd=None):
        code, output = _invoke(PUSH_GATE, command, cwd=cwd or str(self.root))
        self.assertEqual(code, 0, command)
        self.assertIsNotNone(output, f"通してしまった: {command}")
        return _reason(output)

    def assertAllowed(self, command, cwd=None):
        code, output = _invoke(PUSH_GATE, command, cwd=cwd or str(self.root))
        self.assertEqual(code, 0, command)
        self.assertIsNone(output, f"止めてしまった: {command}")

    def test_undeclared_instruction_change_is_denied(self):
        """`Instruction-Change`を持たないまま`develop`へ押すのを止める。

        **`b93b309`で実際に起きた形である。**
        """
        self._instruction_commit(declared=False)
        reason = self.assertDenied("git push origin HEAD:develop")
        self.assertIn(gate.TRAILER_INSTRUCTION, reason)

    def test_receipt_alone_would_have_passed(self):
        """同じcommitが`receipt`だけなら通る。**この差がhookの存在理由である。**"""
        self._instruction_commit(declared=False)
        self.assertEqual(self._review_gate("receipt"), 0)
        self.assertEqual(self._review_gate("gate"), 1)

    def test_declared_instruction_change_is_allowed(self):
        """宣言が揃っていれば通す。"""
        self._instruction_commit(declared=True)
        self.assertAllowed("git push origin HEAD:develop")

    def test_refspec_forms_that_update_develop_are_inspected(self):
        """`develop`を更新する書き方をひととおり拾う。"""
        self._instruction_commit(declared=False)
        _git(str(self.root), "branch", "--force", "develop", "HEAD")
        for command in (
            "git push origin HEAD:develop",
            "git push origin develop",
            "git push origin HEAD:refs/heads/develop",
            "git push origin +HEAD:develop",
            "git push --force-with-lease origin HEAD:develop",
            "cd . && git push origin HEAD:develop",
            "/usr/bin/git push origin HEAD:develop",
            "git -c core.pager=cat push origin HEAD:develop",
            "git --no-pager push origin HEAD:develop",
        ):
            self.assertDenied(command)

    def test_deleting_develop_is_not_read_as_pushing_head(self):
        """`git push origin :develop`はbranchの削除であり、押すcommitが無い。

        `HEAD`を押すものとして扱うと、**無関係な範囲を検査する。**
        """
        self._instruction_commit(declared=False)
        self.assertIsNone(push_gate.pushed_source("git push origin :develop"))
        self.assertAllowed("git push origin :develop")

    def test_tag_option_does_not_hide_an_explicit_refspec(self):
        """`--tags`を併記しても、`develop`を指すrefspecは見る。

        **`--tags`をskip対象に置くと、この形を取り落とす。**
        """
        self._instruction_commit(declared=False)
        self.assertDenied("git push --tags origin HEAD:develop")

    def test_forms_without_a_refspec_are_not_inspected(self):
        """refspecを書かない形は対象にならない。**gapとして文書に書いてある。**"""
        self._instruction_commit(declared=False)
        for command in (
            "git push --mirror origin",
            "git push --all origin",
            "git push --tags origin",
        ):
            self.assertIsNone(push_gate.pushed_source(command), command)

    def test_prefix_of_develop_is_not_matched(self):
        """`develop`で始まるだけのbranchを取り違えない。"""
        self._instruction_commit(declared=False)
        for command in (
            "git push origin HEAD:developer",
            "git push origin HEAD:develop-2",
            "git push origin HEAD:feature/develop",
        ):
            self.assertAllowed(command)

    def test_directory_option_selects_the_repository(self):
        """`git -C <dir> push`は、そのdirectoryのrepositoryを検査する。

        **cwdの側を検査すると、別treeの結果で判断することになる。**
        """
        self._instruction_commit(declared=False)
        other = tempfile.mkdtemp()
        self.addCleanup(guards.remove_tree, Path(other))
        source, directory = push_gate.pushed_source(
            f"git -C {self.root} push origin HEAD:develop"
        )
        self.assertEqual((source, directory), ("HEAD", str(self.root)))
        # cwdをrepositoryの外に置いても、`-C`の先を見て拒否する。
        reason = self.assertDenied(
            f"git -C {self.root} push origin HEAD:develop", cwd=other
        )
        self.assertIn(gate.TRAILER_INSTRUCTION, reason)

    def test_directory_option_outside_a_repository_is_not_inspected(self):
        """`-C`の先がrepositoryでなければ見ない。どこを検査すべきか決まらない。"""
        self._instruction_commit(declared=False)
        other = tempfile.mkdtemp()
        self.addCleanup(guards.remove_tree, Path(other))
        self.assertAllowed(f"git -C {other} push origin HEAD:develop")

    def test_other_destinations_are_out_of_scope(self):
        """`develop`以外へのpushは見ない。Pull Requestが`gate`を通す。"""
        self._instruction_commit(declared=False)
        for command in (
            "git push origin HEAD:feature/x",
            "git push origin HEAD:main",
            "git push fork HEAD:develop",
            "git push --dry-run origin HEAD:develop",
            "git push --delete origin develop",
            "git push --tags origin",
            "git status",
        ):
            self.assertAllowed(command)

    def test_the_word_push_in_an_argument_is_not_an_invocation(self):
        """`echo git push origin HEAD:develop`を呼び出しと読まない。"""
        self._instruction_commit(declared=False)
        self.assertAllowed("echo git push origin HEAD:develop")

    def test_nothing_to_push_is_allowed(self):
        """押すcommitが無ければ検査しない。範囲が空でheadのtrailerを問わない。"""
        _git(str(self.root), "checkout", "--quiet", "-B", "work", "origin/develop")
        self.assertAllowed("git push origin HEAD:develop")

    def test_bare_push_follows_the_upstream(self):
        """`git push`だけの形は、upstreamが`origin/develop`のときだけ対象になる。"""
        self._instruction_commit(declared=False)
        _git(str(self.root), "branch", "--set-upstream-to", "origin/develop")
        self.assertDenied("git push")

    def test_bare_push_to_another_upstream_is_out_of_scope(self):
        """upstreamが`origin/develop`でなければ見ない。"""
        self._instruction_commit(declared=False)
        branch = _git(str(self.root), "rev-parse", "--abbrev-ref", "HEAD").strip()
        # `--set-upstream-to`はremote-tracking refの実在を要求するため、configで置く。
        _git(str(self.root), "config", f"branch.{branch}.remote", "origin")
        _git(str(self.root), "config", f"branch.{branch}.merge", "refs/heads/other")
        self.assertAllowed("git push")

    def test_skip_environment_disables_the_guard(self):
        """逃げ道が効く。**使ったら理由を残すのは人間の側の規則である。**"""
        self._instruction_commit(declared=False)
        code, output = _invoke(
            PUSH_GATE, "git push origin HEAD:develop", cwd=str(self.root),
            environment={"DESKCAT_SKIP_PUSH_GATE": "1"},
        )
        self.assertEqual(code, 0)
        self.assertIsNone(output)

    def test_broken_input_does_not_block(self):
        """壊れた入力で作業を止めない。"""
        result = subprocess.run(
            [sys.executable, PUSH_GATE], input="{壊れている",
            capture_output=True, text=True, encoding="utf-8",
            timeout=60, cwd=str(self.root),
        )
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.strip(), "")



if __name__ == "__main__":
    unittest.main(verbosity=2)
