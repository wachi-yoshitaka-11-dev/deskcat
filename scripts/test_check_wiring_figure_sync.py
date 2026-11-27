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

    def test_row_with_deleted_figure_fails(self):
        self.write(sync.LOG_PATH, f"# log\n\n{ROW_CHANGED}\n| Other | x |\n")
        (Path(self.root) / sync.FIGURE_PATH).unlink()
        ok, _ = self.run_check(self.commit())
        self.assertFalse(ok)

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

    def test_commit_message_declaration_is_not_read(self):
        head = self.changed_head(
            f"change\n\nWiring-Figure: not-applicable {LONG_REASON}"
        )
        ok, _ = self.run_check(head)
        self.assertFalse(ok)

    def test_body_declaration_with_short_reason_fails(self):
        head = self.changed_head()
        ok, _ = self.run_check(head, "Wiring-Figure: not-applicable なし")
        self.assertFalse(ok)

    def test_body_declaration_with_placeholder_reason_fails(self):
        head = self.changed_head()
        ok, _ = self.run_check(head, "Wiring-Figure: not-applicable <配線を変えていない理由>")
        self.assertFalse(ok)

    def test_body_declaration_with_other_value_fails(self):
        head = self.changed_head()
        ok, _ = self.run_check(head, f"Wiring-Figure: skip {LONG_REASON}")
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
        head = self.changed_head()
        ok, _ = self.run_check(head, "Wiring-Figure: not-applicable なし")
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


class Bench18NodeCheckTests(unittest.TestCase):
    """`hardware/bench-18/generate.py`の節点の検査（#751）。"""

    def load(self):
        import importlib.util
        path = SCRIPTS_ROOT.parent / "hardware" / "bench-18" / "generate.py"
        spec = importlib.util.spec_from_file_location("bench18_generate", path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def test_current_data_passes(self):
        self.assertEqual(self.load().run_checks(), [])

    def test_consistent_rename_to_a_duplicate_identifier_is_rejected(self):
        """N4をN1へ一貫して付け替えても、PSUの＋と−が同じ識別子になるため落ちる。"""
        module = self.load()
        module.NETS = [(("N1" if nid == "N4" else nid), sig, pts)
                       for nid, sig, pts in module.NETS]
        module.PROBES = [(pid, what, ("N1" if net == "N4" else net))
                         for pid, what, net in module.PROBES]
        errors = module.run_checks()
        self.assertTrue(any("識別子" in e for e in errors), errors)
        self.assertTrue(any("PSU の＋と−" in e for e in errors), errors)

    def test_missing_psu_points_are_not_reported_as_a_short(self):
        """PSUの端子が節点に無いときは、短絡ではなく「節点に無い」と報告する。"""
        module = self.load()
        module.NETS = [(nid, sig, [p for p in pts if not p.startswith("PSU の")])
                       for nid, sig, pts in module.NETS]
        errors = module.run_checks()
        self.assertTrue(errors)
        self.assertFalse(any("PSU の＋と−" in e for e in errors), errors)

    def test_psu_terminals_in_one_net_are_rejected(self):
        """識別子が4つ異なっていても、PSUの両端子を同じ節点へ入れたら落ちる。"""
        module = self.load()
        nets = [list(n) for n in module.NETS]
        minus = "PSU の−端子"
        for n in nets:
            n[2] = [p for p in n[2] if p != minus]
        nets[0][2].append(minus)
        module.NETS = [tuple(n) for n in nets]
        errors = module.run_checks()
        self.assertTrue(any("PSU の＋と−" in e for e in errors), errors)


if __name__ == "__main__":
    unittest.main()
