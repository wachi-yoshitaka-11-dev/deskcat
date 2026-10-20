#!/usr/bin/env python3
"""`check_wiring_figure_sync.py`の回帰test。

fixture repositoryへ実際にcommitし、検出と宣言の受理・拒否を測る。
見逃し（Wiring revisionを変えたのに図が無い）と誤検知（関係ない行・語・空範囲）の
両方を持つ。
"""

import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS_ROOT))

import check_wiring_figure_sync as sync  # noqa: E402

SCRIPT = str(SCRIPTS_ROOT / "check_wiring_figure_sync.py")
GIT_IDENTITY = (
    "-c", "user.name=test",
    "-c", "user.email=test@example.invalid",
    "-c", "commit.gpgsign=false",
)
ROW = "| Wiring revision | 基準の配線 |"
ROW_CHANGED = "| Wiring revision | 基準の配線から変更 |"
LONG_REASON = "配線を変えていない読み取り試験"


def git(root, *arguments):
    result = subprocess.run(
        ["git", "-C", root, *GIT_IDENTITY, *arguments],
        capture_output=True, text=True, encoding="utf-8", errors="replace",
    )
    if result.returncode != 0:
        raise AssertionError(f"git {arguments} failed: {result.stdout}{result.stderr}")
    return result.stdout.strip()


class FixtureCase(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.root = self._tmp.name
        git(self.root, "init", "-q", "-b", "main")
        self.write(sync.LOG_PATH, f"# log\n\n{ROW}\n| Other | x |\n")
        self.write(sync.FIGURE_PATH, "<svg/>\n")
        git(self.root, "add", "-A")
        git(self.root, "commit", "-q", "-m", "base")
        self.base = git(self.root, "rev-parse", "HEAD")

    def write(self, path, text):
        full = Path(self.root) / path
        full.parent.mkdir(parents=True, exist_ok=True)
        full.write_text(text, encoding="utf-8")

    def commit(self, message="change"):
        git(self.root, "add", "-A")
        git(self.root, "commit", "-q", "-m", message)
        return git(self.root, "rev-parse", "HEAD")

    def run_check(self, head, body=""):
        return sync.check(self.root, self.base, head, body)


class DetectionTests(FixtureCase):
    def test_added_row_without_figure_fails(self):
        self.write(sync.LOG_PATH, f"# log\n\n{ROW}\n{ROW}\n| Other | x |\n")
        ok, _ = self.run_check(self.commit())
        self.assertFalse(ok)

    def test_edited_row_without_figure_fails(self):
        self.write(sync.LOG_PATH, f"# log\n\n{ROW_CHANGED}\n| Other | x |\n")
        ok, _ = self.run_check(self.commit())
        self.assertFalse(ok)

    def test_row_with_figure_passes(self):
        self.write(sync.LOG_PATH, f"# log\n\n{ROW_CHANGED}\n| Other | x |\n")
        self.write(sync.FIGURE_PATH, "<svg><g/></svg>\n")
        ok, _ = self.run_check(self.commit())
        self.assertTrue(ok)

    def test_other_row_only_passes(self):
        self.write(sync.LOG_PATH, f"# log\n\n{ROW}\n| Other | y |\n")
        ok, _ = self.run_check(self.commit())
        self.assertTrue(ok)

    def test_phrase_inside_other_row_passes(self):
        self.write(
            sync.LOG_PATH,
            f"# log\n\n{ROW}\n| Notes | Wiring revision を見た |\nWiring revision の説明\n",
        )
        ok, _ = self.run_check(self.commit())
        self.assertTrue(ok)

    def test_deleting_row_passes(self):
        self.write(sync.LOG_PATH, "# log\n\n| Other | x |\n")
        ok, _ = self.run_check(self.commit())
        self.assertTrue(ok)

    def test_empty_range_passes(self):
        ok, _ = self.run_check(self.base)
        self.assertTrue(ok)

    def test_other_file_row_is_not_detected(self):
        self.write("docs/hardware/other.md", f"{ROW}\n")
        ok, _ = self.run_check(self.commit())
        self.assertTrue(ok)

    def test_base_side_change_is_not_blamed_on_the_pr(self):
        git(self.root, "checkout", "-q", "-b", "feature")
        self.write("note.txt", "pr\n")
        head = self.commit("pr")
        git(self.root, "checkout", "-q", "main")
        self.write(sync.LOG_PATH, f"# log\n\n{ROW_CHANGED}\n| Other | x |\n")
        new_base = self.commit("main moves")
        ok, _ = sync.check(self.root, new_base, head, "")
        self.assertTrue(ok)


class DeclarationTests(FixtureCase):
    def changed_head(self, message="change"):
        self.write(sync.LOG_PATH, f"# log\n\n{ROW_CHANGED}\n| Other | x |\n")
        return self.commit(message)

    def test_trailer_with_reason_passes(self):
        head = self.changed_head(
            f"change\n\nWiring-Figure: not-applicable {LONG_REASON}"
        )
        ok, _ = self.run_check(head)
        self.assertTrue(ok)

    def test_trailer_without_reason_fails(self):
        head = self.changed_head("change\n\nWiring-Figure: not-applicable")
        ok, _ = self.run_check(head)
        self.assertFalse(ok)

    def test_trailer_with_short_reason_fails(self):
        head = self.changed_head("change\n\nWiring-Figure: not-applicable なし")
        ok, _ = self.run_check(head)
        self.assertFalse(ok)

    def test_trailer_with_placeholder_reason_fails(self):
        head = self.changed_head(
            "change\n\nWiring-Figure: not-applicable <配線を変えていない理由>"
        )
        ok, _ = self.run_check(head)
        self.assertFalse(ok)

    def test_body_declaration_with_placeholder_reason_fails(self):
        head = self.changed_head()
        ok, _ = self.run_check(head, "Wiring-Figure: not-applicable <配線を変えていない理由>")
        self.assertFalse(ok)

    def test_trailer_with_other_value_fails(self):
        head = self.changed_head(f"change\n\nWiring-Figure: skip {LONG_REASON}")
        ok, _ = self.run_check(head)
        self.assertFalse(ok)

    def test_body_declaration_with_reason_passes(self):
        head = self.changed_head()
        ok, _ = self.run_check(head, f"概要\n\nWiring-Figure: not-applicable {LONG_REASON}\n")
        self.assertTrue(ok)

    def test_body_declaration_without_reason_fails(self):
        head = self.changed_head()
        ok, _ = self.run_check(head, "Wiring-Figure: not-applicable\n")
        self.assertFalse(ok)

    def test_body_declaration_mid_sentence_fails(self):
        head = self.changed_head()
        ok, _ = self.run_check(
            head, f"次は書かない: Wiring-Figure: not-applicable {LONG_REASON}"
        )
        self.assertFalse(ok)

    def test_declaration_is_harmless_when_figure_changed(self):
        self.write(sync.FIGURE_PATH, "<svg><g/></svg>\n")
        head = self.changed_head("change\n\nWiring-Figure: not-applicable なし")
        ok, _ = self.run_check(head)
        self.assertTrue(ok)


class CommandLineTests(FixtureCase):
    def run_cli(self, head, body=None):
        env = dict(os.environ)
        arguments = [
            sys.executable, SCRIPT, "--repository-root", self.root,
            "--base", self.base, "--head", head,
        ]
        if body is not None:
            env["TEST_PR_BODY"] = body
            arguments += ["--pr-body-env", "TEST_PR_BODY"]
        return subprocess.run(
            arguments, capture_output=True, text=True, encoding="utf-8", env=env,
        )

    def test_exit_codes(self):
        self.write(sync.LOG_PATH, f"# log\n\n{ROW_CHANGED}\n| Other | x |\n")
        head = self.commit()
        failed = self.run_cli(head)
        self.assertEqual(failed.returncode, 1, failed.stderr)
        self.assertIn(sync.FIGURE_PATH, failed.stderr)
        passed = self.run_cli(head, f"Wiring-Figure: not-applicable {LONG_REASON}")
        self.assertEqual(passed.returncode, 0, passed.stderr)
        self.assertEqual(self.run_cli(self.base).returncode, 0)


if __name__ == "__main__":
    unittest.main()
