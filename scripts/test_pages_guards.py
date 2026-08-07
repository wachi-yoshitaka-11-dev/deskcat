#!/usr/bin/env python3
"""`prepare_pages.py`の公開境界が、想定した入力で失敗することを確認する。

各caseは一時fileまたはmanifestの一時改変で異常状態を作り、`tearDown`で必ず元へ戻す。
Gitのindexは変更しない。追跡状態を操作するcaseは、indexを複製して`GIT_INDEX_FILE`で
切り替える。環境変数は子processへ引き継がれ、`prepare_pages.py`内の`git ls-files`も
この複製indexを読む。

Windowsでのsymlink作成にはDeveloper Modeまたは管理者権限が必要なため、作成できない
環境では`skipTest`とする。実行していないものを成功として数えない。
CIのubuntu runnerでは常に実行される。

repository root以外のcurrent directoryでも成功しなければならない。Pages CIはこの
harnessをrunnerの一時directoryから絶対pathで起動する。`PAGES_SOURCE=.pages-src`は
callerのCWDではなくrepository root基準で解決する。
"""

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
import uuid
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))

import publish_guards as guards  # noqa: E402

SCRIPT_DIRECTORY = Path(__file__).resolve().parent
REPOSITORY_ROOT = guards.full_path(str(SCRIPT_DIRECTORY.parent))
PREPARE_SCRIPT = str(SCRIPT_DIRECTORY / "prepare_pages.py")

STAGING_ROOT = os.path.join(REPOSITORY_ROOT, ".pages-src")
MANIFEST_PATH = os.path.join(REPOSITORY_ROOT, "pages", "assets-manifest.json")
ASSETS_ROOT = os.path.join(REPOSITORY_ROOT, "pages", "assets")
DOCS_ROOT = os.path.join(REPOSITORY_ROOT, "docs")
PORTAL_PAGE_PATH = os.path.join(REPOSITORY_ROOT, "pages", "404.md")


def _git(*arguments, check=True):
    result = subprocess.run(
        ["git", "-C", REPOSITORY_ROOT, *arguments],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    if check and result.returncode != 0:
        raise RuntimeError(
            f"git {' '.join(arguments)} failed: {result.stdout}{result.stderr}"
        )
    return result.stdout


def _read(path):
    with open(path, "r", encoding="utf-8", newline="") as handle:
        return handle.read()


def _write(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="") as handle:
        handle.write(content)


def _resolve_git_index_path():
    """追跡状態のguardを検証するために、実indexの場所を解決しておく。

    worktreeでは`.git`がfileのため、pathは`rev-parse`に決めさせる。
    """
    index_path = _git("rev-parse", "--git-path", "index").strip()
    if not index_path:
        raise RuntimeError("Unable to locate the Git index. Run inside a Git checkout.")
    if not os.path.isabs(index_path):
        index_path = os.path.join(REPOSITORY_ROOT, index_path)
    return guards.full_path(index_path)


class PrepareRun:
    def __init__(self, exit_code, output):
        self.exit_code = exit_code
        self.output = output


def run_prepare():
    process = subprocess.run(
        [sys.executable, PREPARE_SCRIPT],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    combined = [
        part.rstrip() for part in (process.stdout, process.stderr) if part and part.strip()
    ]
    return PrepareRun(process.returncode, "\n".join(combined))


def output_exposes_staging_root(output):
    normalized_output = output.replace("\\", "/")
    normalized_staging_root = STAGING_ROOT.replace("\\", "/")
    if os.name == "nt":
        normalized_output = normalized_output.lower()
        normalized_staging_root = normalized_staging_root.lower()
    return normalized_staging_root in normalized_output


class PublishGuardTestCase(unittest.TestCase):
    """各caseの後始末を共通化する。

    manifestとportal pageはbackupから戻し、`GIT_INDEX_FILE`は必ず解除する。
    次のcaseが実indexから始まらないと、後続の判定が前のcaseの改変を引きずる。
    """

    @classmethod
    def setUpClass(cls):
        cls.git_index_path = _resolve_git_index_path()

    def setUp(self):
        self.manifest_backup = _read(MANIFEST_PATH)
        self.portal_page_backup = _read(PORTAL_PAGE_PATH)
        self.temporary_paths = []
        self.addCleanup(self._restore)

    def _restore(self):
        os.environ.pop("GIT_INDEX_FILE", None)
        _write(MANIFEST_PATH, self.manifest_backup)
        _write(PORTAL_PAGE_PATH, self.portal_page_backup)
        for path in reversed(self.temporary_paths):
            if guards.is_reparse_point(path):
                try:
                    os.remove(path)
                except OSError:
                    os.rmdir(path)
            elif os.path.isdir(path):
                shutil.rmtree(path, ignore_errors=True)
            elif os.path.exists(path):
                os.remove(path)

    def track(self, path):
        self.temporary_paths.append(path)
        return path

    # -- fixture helpers -------------------------------------------------

    def new_detached_index(self):
        """indexを複製して`GIT_INDEX_FILE`で切り替える。

        実indexとworking treeはどちらも変更しない。
        """
        copy_path = self.track(
            os.path.join(
                tempfile.gettempdir(), f"deskcat-guard-index-{uuid.uuid4().hex}"
            )
        )
        shutil.copyfile(self.git_index_path, copy_path)
        os.environ["GIT_INDEX_FILE"] = copy_path

    def detached_index_without(self, repository_path):
        """「fileは存在するがindexに無い」状態を作る。"""
        self.new_detached_index()
        _git("update-index", "--force-remove", repository_path)

    def detached_index_with(self, repository_path):
        """working treeの一時fileを追跡済みとして見せる。

        追跡checkを通過しないと到達しない判定（拡張子check等）を試験するために使う。
        """
        self.new_detached_index()
        _git("update-index", "--add", "--", repository_path)

    def add_manifest_entry(self, entry):
        manifest = json.loads(self.manifest_backup)
        manifest["assets"].append(entry)
        _write(MANIFEST_PATH, json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")

    def edit_manifest(self, mutate):
        manifest = json.loads(self.manifest_backup)
        mutate(manifest)
        _write(MANIFEST_PATH, json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")

    # -- assertions ------------------------------------------------------

    def assert_staging_fails(self, expected_message, forbidden_messages=()):
        run = run_prepare()
        self._assert_no_local_leak(run, forbidden_messages)
        self.assertNotEqual(run.exit_code, 0, "expected failure, but staging succeeded")
        self.assertIn(expected_message, run.output)
        return run

    def assert_staging_succeeds(self, forbidden_messages=()):
        run = run_prepare()
        self._assert_no_local_leak(run, forbidden_messages)
        self.assertEqual(run.exit_code, 0, run.output)

        # 成功caseはすべて、prepareが報告した相対pathをrepository rootから解決する。
        # callerのcurrent directoryから解決すると、repository外からtestを起動した
        # ときだけ存在しないpathを見て誤判定する。
        match = re.search(r"PAGES_SOURCE=(?P<path>.+)", run.output)
        self.assertIsNotNone(match, "PAGES_SOURCE not found in output")
        reported = match.group("path").strip("\r\n ")
        self.assertFalse(
            os.path.isabs(reported), "PAGES_SOURCE must be repository-relative"
        )
        staged_root = guards.full_path(os.path.join(REPOSITORY_ROOT, reported))
        if os.name == "nt":
            self.assertEqual(staged_root.lower(), STAGING_ROOT.lower())
        else:
            self.assertEqual(staged_root, STAGING_ROOT)
        self.assertTrue(
            os.path.isdir(staged_root), "reported staging root does not exist"
        )
        return staged_root, run.output

    def _assert_no_local_leak(self, run, forbidden_messages):
        self.assertFalse(
            output_exposes_staging_root(run.output),
            "local staging path was exposed in prepare output",
        )
        for message in forbidden_messages:
            if message:
                self.assertNotIn(
                    message, run.output, "forbidden text was exposed in prepare output"
                )


class HarnessSelfTests(PublishGuardTestCase):
    """staging path検出器のpositive／negative control。

    検出器がno-opでも、prepare側の診断がたまたまcleanなら全caseが成功するため、
    検出力を先に確認する。
    """

    def test_detects_private_staging_paths_without_false_positives(self):
        probe = STAGING_ROOT.replace("\\", "/")
        if os.name == "nt":
            probe = probe.upper()
        self.assertTrue(output_exposes_staging_root(f"synthetic: {probe}"))
        self.assertFalse(output_exposes_staging_root("synthetic: .pages-src"))


class BaselineTests(PublishGuardTestCase):
    """改変なしのstagingが成功すること。

    PowerShell版はこれと同じcaseを末尾にも置き、全caseの後始末が効いたことを
    確認していた。unittestでは`addCleanup`が失敗時にも必ず走るため、末尾の
    再実行を置かずに済む。復元が壊れれば、後続classの`assert_staging_succeeds`が
    そこで落ちる。
    """

    def test_baseline_staging_succeeds(self):
        self.assert_staging_succeeds()


class AssetManifestGuardTests(PublishGuardTestCase):
    def test_undeclared_asset_on_disk_fails(self):
        path = self.track(os.path.join(ASSETS_ROOT, "__guardtest-undeclared.png"))
        _write(path, "not a real png")
        self.assert_staging_fails("Asset is not declared in the manifest")

    def test_declared_asset_missing_on_disk_fails(self):
        self.add_manifest_entry({"path": "__guardtest-absent.png", "sha256": "00"})
        self.assert_staging_fails("Declared asset is missing")

    def test_declared_asset_not_tracked_by_git_fails(self):
        path = self.track(os.path.join(ASSETS_ROOT, "__guardtest-untracked.png"))
        _write(path, "not a real png")
        self.add_manifest_entry({"path": "__guardtest-untracked.png", "sha256": "00"})
        self.assert_staging_fails("Declared asset is not tracked by Git")

    def test_asset_sha256_mismatch_fails(self):
        def tamper(manifest):
            replaced = False
            for entry in manifest["assets"]:
                if "sha256" in entry:
                    entry["sha256"] = "0" * 64
                    replaced = True
            if not replaced:
                raise RuntimeError(
                    "Unable to tamper with the recorded SHA-256. The manifest layout changed."
                )

        self.edit_manifest(tamper)
        self.assert_staging_fails("Asset SHA-256 does not match the manifest")

    def test_binary_asset_without_sha256_fails(self):
        def remove_hashes(manifest):
            removed = False
            for entry in manifest["assets"]:
                if entry.pop("sha256", None) is not None:
                    removed = True
            if not removed:
                raise RuntimeError(
                    "Unable to remove the recorded SHA-256. The manifest layout changed."
                )

        self.edit_manifest(remove_hashes)
        self.assert_staging_fails("Binary asset must declare Sha256")

    def test_unsafe_manifest_path_fails(self):
        """Manifestの`path`でstaging先を`assets/`の外へ逃がせないこと。"""
        for unsafe in ("../escape.png", "/absolute.png"):
            with self.subTest(path=unsafe):
                self.edit_manifest(
                    lambda manifest, value=unsafe: manifest["assets"].append(
                        {"path": value, "sha256": "00"}
                    )
                )
                self.assert_staging_fails(
                    "Asset manifest Path is not a safe relative path"
                )

    def test_disallowed_staged_file_type_fails_with_a_relative_path(self):
        path = self.track(os.path.join(ASSETS_ROOT, "__guardtest-disallowed.pdf"))
        _write(path, "synthetic PDF fixture")
        self.detached_index_with("pages/assets/__guardtest-disallowed.pdf")
        self.add_manifest_entry(
            {
                "path": "__guardtest-disallowed.pdf",
                "sha256": guards.file_sha256(path),
            }
        )
        self.assert_staging_fails(
            "File type is not approved for Pages: assets/__guardtest-disallowed.pdf"
        )


class StagedContentGuardTests(PublishGuardTestCase):
    """Size上限と公開禁止patternは`pages/404.md`で検証する。

    `docs/`配下の一時fileはGit追跡外のため複製されず、これらの検査へ到達しない。
    `pages/404.md`は追跡済みのため、内容を書き換えてもstagingされ、検査へ到達する。
    """

    def test_oversized_staged_file_fails(self):
        filler = ("\n" + "x" * 1000) * 1100
        _write(PORTAL_PAGE_PATH, self.portal_page_backup + filler)
        self.assert_staging_fails("File exceeds the Pages size limit: 404.md")

    def test_secret_like_content_fails_without_exposing_values(self):
        secret = "ghp_" + "e" * 24
        _write(PORTAL_PAGE_PATH, f"# Synthetic\n{secret}")
        self.assert_staging_fails(
            "Secret-like content detected: 404.md", forbidden_messages=(secret,)
        )

    def test_personal_path_fails_without_exposing_values(self):
        personal_path = "/home/exampleuser/staged.md"
        _write(PORTAL_PAGE_PATH, f"# Synthetic\n{personal_path}")
        self.assert_staging_fails(
            "Personal absolute path detected: 404.md",
            forbidden_messages=(personal_path,),
        )


class DocsBoundaryTests(PublishGuardTestCase):
    """`docs/`側の公開境界。`pages/assets/`の境界と対で維持する。"""

    def test_untracked_docs_file_is_not_published(self):
        path = self.track(os.path.join(DOCS_ROOT, "__guardtest-untracked.md"))
        _write(path, "# guard test")
        staged_root, _ = self.assert_staging_succeeds()
        self.assertFalse(
            os.path.exists(os.path.join(staged_root, "docs", "__guardtest-untracked.md")),
            "Untracked docs file was published.",
        )

    def test_non_markdown_docs_file_is_not_published(self):
        """`prepare_pages.py`の判定順は4段である。

        Gitのmode 120000によるsymlink → file属性のreparse point → Git追跡 → 拡張子。
        未追跡のfileは追跡checkで弾かれ、拡張子checkへ到達しない。拡張子checkを
        実際に通すため、複製indexへ追加して追跡済みに見せる。
        """
        path = self.track(os.path.join(DOCS_ROOT, "__guardtest-note.txt"))
        _write(path, "guard test")
        self.detached_index_with("docs/__guardtest-note.txt")
        staged_root, _ = self.assert_staging_succeeds()
        self.assertFalse(
            os.path.exists(os.path.join(staged_root, "docs", "__guardtest-note.txt")),
            "Non-Markdown docs file was published.",
        )


class RequiredSourceTrackingTests(PublishGuardTestCase):
    """portal fileとroot documentの追跡guard。

    `docs/`と`pages/assets/`の境界と対で維持する。これらは公開に必須のため、
    skipではなく失敗として止める。
    """

    def test_untracked_portal_file_fails(self):
        self.detached_index_without("pages/404.md")
        self.assert_staging_fails(
            "Required Pages source is not tracked by Git: pages/404.md",
            forbidden_messages=(PORTAL_PAGE_PATH,),
        )

    def test_untracked_root_document_fails(self):
        self.detached_index_without("AGENTS.md")
        self.assert_staging_fails(
            "Required root document is not tracked by Git: AGENTS.md",
            forbidden_messages=(os.path.join(REPOSITORY_ROOT, "AGENTS.md"),),
        )


class SymlinkBoundaryTests(PublishGuardTestCase):
    """symlinkとreparse pointの拒否経路を、働くguardごとに分けて確認する。

    どちらも「公開されない」は同じだが、働くguardが違う。skip理由まで確認しないと、
    片方のguardを壊しても、もう片方が拾って両caseが成功し続ける。
    """

    LINK_NAME = "__guardtest-link.md"

    def _create_symlink(self):
        link_path = os.path.join(DOCS_ROOT, self.LINK_NAME)
        target = os.path.join(REPOSITORY_ROOT, "README.md")
        # 作る前に登録する。作成と削除の間で異常終了すると、`docs/`配下に
        # symlinkが残り、commitへ入りうる。cleanupが拾えるようにしておく。
        self.track(link_path)
        try:
            os.symlink(target, link_path)
        except (OSError, NotImplementedError):
            self.skipTest("symlink creation unavailable")
        return link_path

    def test_symlink_recorded_in_the_index_is_not_published(self):
        self._create_symlink()
        # `update-index --add`はsymlinkをmode 120000で記録する。
        # 追跡checkで弾かれると判定の由来が区別できないため、追跡済みにする。
        self.detached_index_with(f"docs/{self.LINK_NAME}")
        staged_root, output = self.assert_staging_succeeds()
        self.assertFalse(
            os.path.exists(os.path.join(staged_root, "docs", self.LINK_NAME)),
            "Symlink docs file was published.",
        )
        self.assertRegex(
            output,
            rf"{re.escape(self.LINK_NAME)} \(symlink in Git\)",
            f"Expected the Git mode guard to skip it. Output: {output}",
        )

    def test_reparse_point_recorded_as_a_regular_file_is_not_published(self):
        """indexがregular file（mode 100644）として記録しているのに、

        working tree上はsymlinkという状態。`core.symlinks=false`のcheckoutと実体が
        食い違う場合に相当し、Gitのmode判定では拾えない。ここでだけreparse point
        checkへ到達する。
        """
        self._create_symlink()
        blob = _git("rev-parse", "HEAD:README.md").strip()
        if not blob:
            raise RuntimeError("Unable to resolve a blob for the reparse point fixture.")
        self.new_detached_index()
        _git("update-index", "--add", "--cacheinfo", f"100644,{blob},docs/{self.LINK_NAME}")
        staged_root, output = self.assert_staging_succeeds()
        self.assertFalse(
            os.path.exists(os.path.join(staged_root, "docs", self.LINK_NAME)),
            "Reparse point docs file was published.",
        )
        self.assertRegex(
            output,
            rf"{re.escape(self.LINK_NAME)} \(reparse point\)",
            f"Expected the reparse point guard to skip it. Output: {output}",
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)
