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

sys.path.insert(0, str(SCRIPTS_ROOT / "hooks"))

import command_line  # noqa: E402

GH_GUARD = str(SCRIPTS_ROOT / "hooks" / "gh_metadata_guard.py")
BASE_GUARD = str(SCRIPTS_ROOT / "hooks" / "branch_base_guard.py")
MERGE_REPORT = str(SCRIPTS_ROOT / "hooks" / "merge_trailer_report.py")
PUSH_GATE = str(SCRIPTS_ROOT / "hooks" / "push_gate.py")
CODERABBIT_GATE = str(SCRIPTS_ROOT / "hooks" / "coderabbit_gate.py")
MERGE_REPORT = str(SCRIPTS_ROOT / "hooks" / "merge_trailer_report.py")

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



class CodeRabbitGateTests(unittest.TestCase):
    """CodeRabbitのreviewを起動するcommandを見るhookのtest。

    **このhookだけ`deny`ではなく`ask`を返す。**他のhookのtestは`deny`を期待するため、
    検査する経路を別に持つ。**止めたいのはAIの独断であって、人間の依頼ではない。**
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
        """reviewを起動する語を止める。**枠を消費するのはこの2つだけである。**"""
        for command in (
            'gh pr comment 239 --body "@coderabbitai full review"',
            'gh pr comment 239 --body "@coderabbitai review"',
            'gh issue comment 240 --body "@coderabbitai full review"',
            'gh pr comment 239 -b "@coderabbitai full review"',
            'gh pr comment 239 --body="@coderabbitai full review"',
        ):
            with self.subTest(command=command):
                self.assertAsked(command)

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

    def test_unreadable_body_file_is_allowed(self):
        """読めない`--body-file`で止めない。

        **hookが読めないことを、AIの独断の証拠として扱わない。**
        この方向の取りこぼしは意図である。
        """
        self.assertAllowed("gh pr comment 239 --body-file /nonexistent/body.md")
        self.assertAllowed("gh pr comment 239 --body-file -")

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
            'gh api --method HEAD repos/o/r -f body="@coderabbitai review"',
        ):
            with self.subTest(command=command):
                self.assertAllowed(command)

    def test_write_api_method_is_still_inspected(self):
        """書き込みのmethodと、method未指定は引き続き見る。"""
        for command in (
            'gh api -X POST repos/o/r/issues/1/comments -f body="@coderabbitai full review"',
            'gh api repos/o/r/issues/1/comments -f body="@coderabbitai full review"',
            'gh api graphql -f query="mutation { x(body: \\"@coderabbitai review\\") }"',
        ):
            with self.subTest(command=command):
                self.assertAsked(command)

    def test_unreadable_body_file_is_recorded_on_stderr(self):
        """読めない`--body-file`を素通りさせるが、**握りつぶさない。**

        AGENTS.mdの「エラーを握りつぶさず、分類、ログ、カウンタを用意する」に従う。
        判定は変えない（素通り）が、理由をstderrへ残す。
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
        self.assertEqual(result.stdout.strip(), "", "止めてしまった")
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
            "DESKCAT_SKIP_PUSH_GATE",
        ):
            with self.subTest(name=name):
                code, output = _invoke(CODERABBIT_GATE, command, environment={name: "1"})
                self.assertEqual(code, 0)
                self.assertIsNotNone(output, f"{name}で素通りした")
                self.assertEqual(
                    output["hookSpecificOutput"]["permissionDecision"], "ask"
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


class HookPayloadShapeTests(unittest.TestCase):
    """**5本のhookが、mappingでない入力で落ちないことを確かめる**（#242）。

    [PR #241](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/241)のreview指摘は`coderabbit_gate.py`に対するものだったが、
    **型で全数走査したら5本すべてに同じ形が残っていた。**指摘は代表例であって全数ではない。
    """

    HOOKS = (GH_GUARD, BASE_GUARD, PUSH_GATE, MERGE_REPORT, CODERABBIT_GATE)

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
                             "DESKCAT_SKIP_PUSH_GATE": ""},
                    )
                    self.assertEqual(result.returncode, 0,
                                     f"{Path(script).name} が落ちた: {result.stderr[:300]}")
                    self.assertNotIn("Traceback", result.stderr,
                                     f"{Path(script).name} が例外を出した")
                    self.assertEqual(result.stdout.strip(), "",
                                     f"{Path(script).name} が止めてしまった")


if __name__ == "__main__":
    unittest.main(verbosity=2)
