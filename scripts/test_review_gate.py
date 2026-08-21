#!/usr/bin/env python3
"""`review_gate.py`の回帰test。

fixture repositoryを作り、validatorを子processとして起動して、分類結果・exit code・
診断出力を検査する。**規則表が偽陽性側へ倒れていないこと**、つまり軽微でないものを
`minor`と言わないことを主に見る。
"""

import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS_ROOT / "lib"))
sys.path.insert(0, str(SCRIPTS_ROOT))

import publish_guards as guards  # noqa: E402
import review_gate as gate  # noqa: E402

REPOSITORY_ROOT = str(SCRIPTS_ROOT.parent)
SCRIPT = str(SCRIPTS_ROOT / "review_gate.py")

# 軽微経路に残るpath。`INSTRUCTION_SOURCES`のどれにも当たらないMarkdownである。
PLAIN_DOC = "docs/runbooks/plain-note.md"
BASE_TEXT = "見出しのない散文である。\nここに説明を書く。\n"


def _git(root, *arguments, stdin_text=None):
    result = subprocess.run(
        ["git", "-C", root, *arguments],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        input=stdin_text,
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"git {' '.join(arguments)} failed: {result.stdout}{result.stderr}"
        )
    return result.stdout


def _run(arguments, cwd=None):
    return subprocess.run(
        [sys.executable, SCRIPT, *arguments],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        cwd=cwd,
        timeout=60,
    )


class ReviewGateTests(unittest.TestCase):
    def _repository(self):
        root = tempfile.mkdtemp(prefix="deskcat-gate-")
        self.addCleanup(self._remove, root)
        _git(root, "init", "--quiet")
        _git(root, "config", "--local", "core.autocrlf", "false")
        _git(root, "config", "--local", "user.email", "fixture@example.invalid")
        _git(root, "config", "--local", "user.name", "fixture")
        self._write(root, PLAIN_DOC, BASE_TEXT)
        self._write(root, "AGENTS.md", "# Rules\n\n本文である。\n")
        self._write(root, "scripts/tool.py", "value = 1\n")
        self._write(
            root, "docs/runbooks/with-fence.md", "散文である。\n\n```text\nplain\n```\n"
        )
        _git(root, "add", "--all")
        _git(root, "commit", "--quiet", "-m", "base")
        return root

    def _write(self, root, relative, text):
        path = os.path.join(root, relative)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8", newline="\n") as handle:
            handle.write(text)

    def _commit(self, root, message):
        _git(root, "add", "--all")
        _git(root, "commit", "--quiet", "-F", "-", stdin_text=message)

    def _remove(self, path):
        # `.git/objects`はread-onlyで作られるため、素のrmtreeでは消えない。
        # 削除の中身はpublish_guardsが持つ。harnessごとに複製しない。
        if not guards.remove_tree(path):
            print(
                f"warning: failed to remove the test fixture {os.path.basename(path)}",
                file=sys.stderr,
            )

    def _classify(self, root):
        result = _run(["classify", "--repository-root", root, "--base", "HEAD~1"])
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return result.stdout

    def test_prose_only_edit_is_minor(self):
        """規則を持たないfileの散文だけの修正は軽微と証明できる。"""
        root = self._repository()
        self._write(root, PLAIN_DOC, "見出しのない散文である。\nここに解説を書く。\n")
        self._commit(root, "散文の言い回しを直す")
        self.assertIn(f"CLASS={gate.CLASS_MINOR}", self._classify(root))

    def test_lexical_deny_rules_block_minor(self):
        """数値・inline code・link・表・見出し・checkboxに触れたら軽微にしない。"""
        cases = (
            ("digit", "ここに3件と書く。\n"),
            ("inline code", "ここに `cargo build` と書く。\n"),
            ("link", "ここに [doc](../index.md) と書く。\n"),
            ("table", "| a | b |\n"),
            ("heading", "## 見出しを足す\n"),
            ("checkbox", "- [ ] 項目を足す\n"),
            ("html comment", "<!-- 注記 -->\n"),
        )
        for expected, addition in cases:
            with self.subTest(rule=expected):
                root = self._repository()
                self._write(root, PLAIN_DOC, BASE_TEXT + addition)
                self._commit(root, "行を足す")
                output = self._classify(root)
                self.assertIn(f"CLASS={gate.CLASS_REVIEW}", output)
                self.assertIn(expected, output)

    def test_fenced_line_is_not_minor(self):
        """fence内の行は、数値もbacktickも無くても軽微にしない。"""
        root = self._repository()
        self._write(
            root, "docs/runbooks/with-fence.md", "散文である。\n\n```text\nother\n```\n"
        )
        self._commit(root, "fence内を書き換える")
        output = self._classify(root)
        self.assertIn(f"CLASS={gate.CLASS_REVIEW}", output)
        self.assertIn("fenced block", output)

    def test_instruction_source_is_never_minor(self):
        """規則を持つfileは、散文だけの修正でも軽微にしない。"""
        root = self._repository()
        self._write(root, "AGENTS.md", "# Rules\n\n本文をなおす。\n")
        self._commit(root, "AGENTS.mdの言い回しを直す")
        output = self._classify(root)
        self.assertIn(f"CLASS={gate.CLASS_REVIEW}", output)
        self.assertIn("instruction source", output)

    def test_non_markdown_edit_is_not_minor(self):
        """Markdown以外のfileは、内容を見る前に軽微から外す。"""
        root = self._repository()
        path = "docs/runbooks/note.txt"
        self._write(root, path, "text\n")
        self._commit(root, "add txt")
        self._write(root, path, "other\n")
        self._commit(root, "edit txt")
        output = self._classify(root)
        self.assertIn(f"CLASS={gate.CLASS_REVIEW}", output)
        self.assertIn("not Markdown", output)

    def test_added_file_is_not_minor(self):
        """新規fileの追加は、散文だけでもtypoの修正ではない。"""
        root = self._repository()
        self._write(root, "docs/runbooks/new-note.md", "散文である。\n")
        self._commit(root, "add md")
        output = self._classify(root)
        self.assertIn(f"CLASS={gate.CLASS_REVIEW}", output)
        self.assertIn("is not a content edit", output)

    def test_empty_range_is_not_minor(self):
        """対象が無いことは、軽微であることの証明ではない。"""
        root = self._repository()
        result = _run(["classify", "--repository-root", root, "--base", "HEAD"])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(f"CLASS={gate.CLASS_REVIEW}", result.stdout)
        self.assertIn("no changed path in range", result.stdout)

    def test_expect_mismatch_fails(self):
        root = self._repository()
        self._write(root, PLAIN_DOC, BASE_TEXT + "ここに追記する。\n")
        self._commit(root, "追記する")
        result = _run(
            [
                "classify",
                "--repository-root",
                root,
                "--base",
                "HEAD~1",
                "--expect",
                gate.CLASS_REVIEW,
            ]
        )
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertIn("expected CLASS=", result.stderr)

    def test_receipt_requires_both_trailers(self):
        root = self._repository()
        self._write(root, PLAIN_DOC, BASE_TEXT + "ここに追記する。\n")
        self._commit(root, "追記する")
        result = _run(["receipt", "--repository-root", root, "--base", "HEAD~1"])
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertIn(gate.TRAILER_CLASS, result.stderr)
        self.assertIn(gate.TRAILER_REVIEW, result.stderr)

    def test_receipt_declaration_cannot_widen_the_minor_path(self):
        """`minor`と宣言しても、範囲がreview必須なら通さない。"""
        root = self._repository()
        self._write(root, "AGENTS.md", "# Rules\n\n本文をなおす。\n")
        self._commit(
            root,
            "AGENTS.mdを直す\n\n"
            f"{gate.TRAILER_CLASS}: {gate.CLASS_MINOR}\n"
            f"{gate.TRAILER_REVIEW}: {gate.REVIEW_CONVERGED}\n",
        )
        result = _run(["receipt", "--repository-root", root, "--base", "HEAD~1"])
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertIn("cannot widen the minor path", result.stderr)

    def test_receipt_accepts_a_matching_declaration(self):
        root = self._repository()
        self._write(root, PLAIN_DOC, BASE_TEXT + "ここに追記する。\n")
        self._commit(
            root,
            "追記する\n\n"
            f"{gate.TRAILER_CLASS}: {gate.CLASS_REVIEW}\n"
            f"{gate.TRAILER_REVIEW}: {gate.REVIEW_CONVERGED}\n",
        )
        result = _run(["receipt", "--repository-root", root, "--base", "HEAD~1"])
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_a_later_commit_without_trailers_invalidates_the_receipt(self):
        """reviewの後にdiffが変わったら宣言が無効になること。

        trailerはcommitへ結び付いているため、新しいheadが宣言を持たなければ落ちる。
        """
        root = self._repository()
        self._write(root, PLAIN_DOC, BASE_TEXT + "ここに追記する。\n")
        self._commit(
            root,
            "追記する\n\n"
            f"{gate.TRAILER_CLASS}: {gate.CLASS_REVIEW}\n"
            f"{gate.TRAILER_REVIEW}: {gate.REVIEW_CONVERGED}\n",
        )
        self._write(root, PLAIN_DOC, BASE_TEXT + "ここに別の追記をする。\n")
        self._commit(root, "review後に書き換える")
        result = _run(["receipt", "--repository-root", root, "--base", "HEAD~2"])
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertIn(gate.TRAILER_CLASS, result.stderr)

    def test_instruction_change_needs_an_acknowledgement(self):
        root = self._repository()
        self._write(root, "AGENTS.md", "# Rules\n\n本文をなおす。\n")
        self._commit(root, "AGENTS.mdを直す")
        result = _run(["instructions", "--repository-root", root, "--base", "HEAD~1"])
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertIn(gate.TRAILER_INSTRUCTION, result.stderr)
        self.assertIn("INSTRUCTION_SOURCES_TOUCHED=1", result.stdout)

    def test_instruction_change_passes_with_the_acknowledgement(self):
        root = self._repository()
        self._write(root, "AGENTS.md", "# Rules\n\n本文をなおす。\n")
        self._commit(
            root,
            "AGENTS.mdを直す\n\n"
            f"{gate.TRAILER_INSTRUCTION}: {gate.INSTRUCTION_ACK}\n",
        )
        result = _run(["instructions", "--repository-root", root, "--base", "HEAD~1"])
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_gate_requires_every_declaration_together(self):
        root = self._repository()
        self._write(root, "AGENTS.md", "# Rules\n\n本文をなおす。\n")
        self._commit(
            root,
            "AGENTS.mdを直す\n\n"
            f"{gate.TRAILER_CLASS}: {gate.CLASS_REVIEW}\n"
            f"{gate.TRAILER_REVIEW}: {gate.REVIEW_CONVERGED}\n"
            f"{gate.TRAILER_INSTRUCTION}: {gate.INSTRUCTION_ACK}\n",
        )
        result = _run(["gate", "--repository-root", root, "--base", "HEAD~1"])
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

        # 宣言を1つ落とすと落ちること。3つが揃っていることを見ているのを確認する。
        self._write(root, "AGENTS.md", "# Rules\n\nさらになおす。\n")
        self._commit(
            root,
            "AGENTS.mdをさらに直す\n\n"
            f"{gate.TRAILER_CLASS}: {gate.CLASS_REVIEW}\n"
            f"{gate.TRAILER_REVIEW}: {gate.REVIEW_CONVERGED}\n",
        )
        result = _run(["gate", "--repository-root", root, "--base", "HEAD~1"])
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertIn(gate.TRAILER_INSTRUCTION, result.stderr)

    def test_instruction_source_matching_is_exact_on_path_boundaries(self):
        """前方一致で無関係なpathを巻き込まないこと。"""
        cases = (
            ("AGENTS.md", True),
            ("docs/AGENTS.md", False),
            ("AGENTS.md.bak", False),
            (".github/workflows/pages.yml", True),
            (".githubbing/note.md", False),
            ("docs/governance/README.md", True),
            ("docs/governance-notes/README.md", False),
            ("docs/runbooks/note.md", False),
        )
        for path, expected in cases:
            with self.subTest(path=path):
                self.assertEqual(gate._is_instruction_source(path), expected)

    def test_runs_against_this_repository(self):
        """実repositoryでもerrorなく分類できること。結果の値は主張しない。"""
        result = _run(
            ["classify", "--repository-root", REPOSITORY_ROOT, "--base", "HEAD"]
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("CLASS=", result.stdout)


if __name__ == "__main__":
    unittest.main(verbosity=2)
