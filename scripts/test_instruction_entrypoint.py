#!/usr/bin/env python3
"""`validate_instruction_entrypoint.py`の回帰test。

validatorを子processとして起動し、exit codeと診断出力を検査する。fixtureはmode 120000を
Git indexへ直接登録するため、OSのsymlink作成権限に依存しない。Windowsの
`core.symlinks=false`でも同じcaseが走る。
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
import validate_instruction_entrypoint as entrypoint  # noqa: E402

REPOSITORY_ROOT = str(SCRIPTS_ROOT.parent)
VALIDATOR = str(SCRIPTS_ROOT / "validate_instruction_entrypoint.py")

# 期待値はvalidator側から取る。harnessが独自のbyte列を持つと、契約を変えたときに
# testだけが古い値のまま失敗する。
ENTRYPOINT = entrypoint.ENTRYPOINT
GOOD_CONTENT = entrypoint.EXPECTED_BLOB
# `core.symlinks=false`のcheckoutが実際に残す内容。link先のpath文字列だけになる。
WINDOWS_SYMLINK_CONTENT = b"AGENTS.md"


def _git(repository, *arguments):
    result = subprocess.run(
        ["git", "-C", repository, *arguments],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    if result.returncode != 0:
        raise RuntimeError(
            f"git {' '.join(arguments)} failed: {result.stdout}{result.stderr}"
        )
    return result.stdout


def _run_validator(arguments=(), cwd=None):
    return subprocess.run(
        [sys.executable, VALIDATOR, *arguments],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        cwd=cwd,
        timeout=60,
    )


class InstructionEntrypointTests(unittest.TestCase):
    def _fixture(self, content=GOOD_CONTENT, mode=None, present=True):
        """fixture repositoryを作り、rootを返す。

        `mode`を渡した場合は、`git add`が記録したblobを同じpathへそのmodeで
        再登録する。内容とmodeを独立に指定できるため、どちらの検査が働いたかを
        診断文で切り分けられる。
        """
        root = tempfile.mkdtemp(prefix="deskcat-entrypoint-")
        self.addCleanup(self._remove, root)
        _git(root, "init", "--quiet")
        # 改行の正規化をglobal設定へ委ねない。fixtureのbyte列がそのままindexへ入ること
        # が前提であり、autocrlfが有効な環境ではCRLF caseが成立しない。
        _git(root, "config", "--local", "core.autocrlf", "false")
        if present:
            with open(os.path.join(root, ENTRYPOINT), "wb") as handle:
                handle.write(content)
            _git(root, "add", "--", ENTRYPOINT)
            if mode is not None:
                blob = _git(root, "rev-parse", "--verify", f":{ENTRYPOINT}").strip()
                _git(
                    root,
                    "update-index",
                    "--add",
                    "--cacheinfo",
                    f"{mode},{blob},{ENTRYPOINT}",
                )
        # `present=False`では何もaddしない。`git ls-files`はcommitが無くても成功して
        # 0件を返すため、validatorが0件をerrorとして扱うことをそのまま確認できる。
        return root

    def _remove(self, path):
        # `.git/objects`はread-onlyで作られるため、素のrmtreeでは消えない。
        # 削除の中身はpublish_guardsが持つ。harnessごとに複製しない。
        if not guards.remove_tree(path):
            print(
                f"warning: failed to remove the test fixture {os.path.basename(path)}",
                file=sys.stderr,
            )

    def test_repository_index_satisfies_the_contract(self):
        """このrepository自身のindexが契約を満たすこと。"""
        result = _run_validator(["--repository-root", REPOSITORY_ROOT])
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("ENTRYPOINT=CLAUDE.md", result.stdout)
        self.assertIn("MODE=100644", result.stdout)

    def test_default_root_needs_no_argument_and_no_current_directory(self):
        """引数もcurrent directoryも与えずに、script位置からrepositoryを解決すること。

        CIはharnessをrunnerの一時directoryから起動する。
        """
        with tempfile.TemporaryDirectory(prefix="deskcat-entrypoint-cwd-") as elsewhere:
            result = _run_validator(cwd=elsewhere)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_symlink_mode_fails_even_when_the_content_is_right(self):
        """modeの検査が内容の検査と独立していること。"""
        root = self._fixture(mode="120000")
        result = _run_validator(["--repository-root", root])
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertIn("mode 120000", result.stderr)
        self.assertIn("git rm --cached", result.stderr)
        self.assertNotIn("content does not match", result.stderr)

    def test_windows_symlink_checkout_content_fails(self):
        """mode 100644でも、内容がlink先のpath文字列なら失敗すること。"""
        root = self._fixture(content=WINDOWS_SYMLINK_CONTENT)
        result = _run_validator(["--repository-root", root])
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertIn("content does not match", result.stderr)
        self.assertNotIn("is recorded with mode", result.stderr)

    def test_crlf_content_fails(self):
        """改行の変化が通らないこと。"""
        root = self._fixture(content=GOOD_CONTENT.replace(b"\n", b"\r\n"))
        result = _run_validator(["--repository-root", root])
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertIn("content does not match", result.stderr)

    def test_symlink_entry_reports_both_problems(self):
        """実際の壊れた状態では、modeと内容の両方を報告すること。

        片方だけを直して通ることがないようにする。
        """
        root = self._fixture(content=WINDOWS_SYMLINK_CONTENT, mode="120000")
        result = _run_validator(["--repository-root", root])
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertIn("mode 120000", result.stderr)
        self.assertIn("content does not match", result.stderr)

    def test_missing_entrypoint_fails(self):
        """追跡されていない場合に、0件を成功として通さないこと。"""
        root = self._fixture(present=False)
        result = _run_validator(["--repository-root", root])
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertIn("single index entry", result.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
