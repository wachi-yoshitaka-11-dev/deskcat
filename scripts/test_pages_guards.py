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
import struct
import subprocess
import sys
import tempfile
import unittest
import uuid
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))

sys.path.insert(0, str(Path(__file__).resolve().parent))

import prepare_pages  # noqa: E402
import publish_guards as guards  # noqa: E402

SCRIPT_DIRECTORY = Path(__file__).resolve().parent
REPOSITORY_ROOT = guards.full_path(str(SCRIPT_DIRECTORY.parent))
PREPARE_SCRIPT = str(SCRIPT_DIRECTORY / "prepare_pages.py")

STAGING_ROOT = os.path.join(REPOSITORY_ROOT, ".pages-src")
MANIFEST_PATH = os.path.join(REPOSITORY_ROOT, "pages", "assets-manifest.json")
ASSETS_ROOT = os.path.join(REPOSITORY_ROOT, "pages", "assets")
LAYOUTS_ROOT = os.path.join(REPOSITORY_ROOT, "pages", "_layouts")
DOCS_ROOT = os.path.join(REPOSITORY_ROOT, "docs")
PORTAL_PAGE_PATH = os.path.join(REPOSITORY_ROOT, "pages", "404.md")

# stagingが止まったときに、CI jobのtimeoutまで待たずにそのcaseで落とすための上限。
# 実測は1秒未満であり、遅いrunnerでも十分な余裕がある。
PREPARE_TIMEOUT_SECONDS = 120


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


def _remove_fixture(path):
    """fixtureを取り除き、消せなかった事実だけを報告する。

    削除の中身は`publish_guards.remove_tree`が持つ。2つのharnessで実装を複製すると、
    片方のchmodや警告だけを変えたときに、もう片方でfixtureが黙って溜まる。
    警告にはfile名だけを出す。一時directoryの絶対pathをlogへ書かない。
    """
    if not guards.remove_tree(path):
        print(
            f"warning: failed to remove the test fixture {os.path.basename(path)}",
            file=sys.stderr,
        )


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


def run_prepare(timeout_seconds=PREPARE_TIMEOUT_SECONDS):
    """`prepare_pages.py`を子processとして実行する。

    時間を区切る。stagingが止まったときに、CI jobのtimeoutまでこのsuiteが
    block するのではなく、そのcaseで落として原因をtestの結果として残す。
    """
    try:
        process = subprocess.run(
            [sys.executable, PREPARE_SCRIPT],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout_seconds,
        )
    except subprocess.TimeoutExpired:
        return PrepareRun(
            -1, f"Prepare timed out after {timeout_seconds} second(s)."
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
        # 復元をstepごとに登録する。1つの関数へまとめると、前半の`_write`が
        # 失敗した時点で残りが実行されず、実repositoryのfileが書き換わったまま
        # 残ってcommitへ入りうる。`unittest`は登録した全cleanupを、失敗しても
        # 最後まで実行する。実行はLIFOのため、登録は「最後に戻したいもの」から行う。
        self.addCleanup(self._remove_temporary_paths)
        self.addCleanup(_write, PORTAL_PAGE_PATH, self.portal_page_backup)
        self.addCleanup(_write, MANIFEST_PATH, self.manifest_backup)
        self.addCleanup(os.environ.pop, "GIT_INDEX_FILE", None)

    def _remove_temporary_paths(self):
        for path in reversed(self.temporary_paths):
            try:
                _remove_fixture(path)
            except OSError:
                # 1件の失敗で残りのfixtureを残さない。
                print(
                    "warning: failed to remove the test fixture "
                    f"{os.path.basename(path)}",
                    file=sys.stderr,
                )

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


class StagingPathGuardTests(unittest.TestCase):
    """staging pathとmanifest pathの規則を、OSに依存せず直接検査する。

    子process越しの検査だけでは検出力がOSに依存する。`C:\\x`はWindowsでは
    `os.path.isabs`が先に弾くため、drive letterの規則を外しても結果が変わらず、
    Linuxでしか回帰を捕まえられない。規則そのものを呼んで両OSで固定する。
    """

    UNSAFE_PATHS = (
        "",
        "   ",
        "../escape.png",
        "..\\escape.png",
        "nested/../../escape.png",
        "/absolute.png",
        "\\absolute.png",
        "C:\\escape.png",
        "c:/escape.png",
    )
    SAFE_PATHS = ("style.css", "css/style.scss", "deskcat-concept.jpg")

    def test_unsafe_manifest_paths_are_rejected(self):
        for value in self.UNSAFE_PATHS:
            with self.subTest(path=value):
                self.assertTrue(prepare_pages.is_unsafe_manifest_path(value))

    def test_safe_manifest_paths_are_accepted(self):
        for value in self.SAFE_PATHS:
            with self.subTest(path=value):
                self.assertFalse(prepare_pages.is_unsafe_manifest_path(value))

    def test_staging_removal_rejects_an_unexpected_path(self):
        """再帰削除の前に、対象がrepository内の`.pages-src`であることを確かめる。

        pathの組み立てを将来変えたときに、別のdirectoryを消してしまわないための
        guardである。実際に消えては困るので、**存在しないpathだけ**で規則を見る。
        実在するdirectoryを渡すと、guardが将来弱まったときにこのtestが先に
        そのdirectoryを再帰削除し、失敗はその後で報告されることになる。
        `assertRaises`は削除を止められない。
        """
        for value in (
            os.path.join(REPOSITORY_ROOT, "__guardtest-not-staging"),
            os.path.join(REPOSITORY_ROOT, ".pages-src-old"),
            os.path.join(os.path.dirname(REPOSITORY_ROOT), ".pages-src"),
        ):
            with self.subTest(path=value):
                with self.assertRaises(guards.ValidationError) as context:
                    prepare_pages._remove_staging(value, REPOSITORY_ROOT)
                self.assertEqual(
                    str(context.exception), "Unexpected Pages staging path."
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

    def test_staging_path_occupied_by_a_file_is_replaced(self):
        """staging pathにfileが置かれていても、stagingは成立する。

        `shutil.rmtree`だけで消そうとするとtracebackになる。`.pages-src`は
        生成物であり、形が違っても作り直してよい。
        """
        self.track(STAGING_ROOT)
        _remove_fixture(STAGING_ROOT)
        _write(STAGING_ROOT, "staging path occupied by a file")
        staged_root, _ = self.assert_staging_succeeds()
        self.assertTrue(os.path.isdir(staged_root))


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

    def test_broken_manifest_fails_without_exposing_local_paths(self):
        """壊れたmanifestを、tracebackではなく診断で落とす。

        JSONのparse例外をそのまま抜けさせると「問題を報告して失敗」ではなく
        「検査自体がcrash」になる。例外文にはlocal絶対pathやmanifestの中身が
        載りうるため、repository相対pathだけを報告する。
        """
        _write(MANIFEST_PATH, '{ "assets": [ ')
        run = self.assert_staging_fails(
            "Pages asset manifest is not valid JSON: pages/assets-manifest.json",
            forbidden_messages=(MANIFEST_PATH, "Traceback (most recent call last)"),
        )
        self.assertNotIn("Traceback", run.output)

    def test_unsafe_manifest_path_fails(self):
        """Manifestの`path`でstaging先を`assets/`の外へ逃がせないこと。

        backslashとdrive letterも含める。このharnessはWindowsでも走り、
        `..\\escape.png`や`C:\\escape.png`は現実的な逃げ道の形である。
        POSIXの`os.path.isabs`は`C:\\x`を相対pathとして通すため、実装側は
        drive letterを別に見て、判定がOSごとに変わらないようにしている。
        """
        for unsafe in (
            "../escape.png",
            "/absolute.png",
            "..\\escape.png",
            "\\absolute.png",
            "C:\\escape.png",
            "c:/escape.png",
        ):
            with self.subTest(path=unsafe):
                self.edit_manifest(
                    lambda manifest, value=unsafe: manifest["assets"].append(
                        {"path": value, "sha256": "00"}
                    )
                )
                self.assert_staging_fails(
                    "Asset manifest Path is not a safe relative path"
                )

    def test_symlinked_asset_does_not_publish_outside_content(self):
        """`pages/assets/`配下のsymlinkでrepository外の内容を公開しない。

        `os.path.isfile`はlinkを辿り、copyはtarget側の内容を書き出す。staged側は
        通常fileになるため、stagingの最後にあるreparse point検査では捕まえられない。
        binaryならSHA-256も辿った先から計算されて一致する。asset loopで止めるしかない。
        """
        outside = self.track(
            os.path.join(tempfile.gettempdir(), f"deskcat-outside-{uuid.uuid4().hex}.txt")
        )
        secret = "OUTSIDE-CONTENT-THAT-MUST-NOT-BE-PUBLISHED"
        _write(outside, secret)
        link = self.track(os.path.join(ASSETS_ROOT, "__guardtest-link.txt"))
        try:
            os.symlink(outside, link)
        except (OSError, NotImplementedError):
            self.skipTest("symlink creation unavailable")
        # 追跡checkで弾かれると、どのguardが働いたのか区別できない。追跡済みにする。
        self.detached_index_with("pages/assets/__guardtest-link.txt")
        self.add_manifest_entry({"path": "__guardtest-link.txt"})
        run = self.assert_staging_fails(
            "Declared asset is a symlink in Git: pages/assets/__guardtest-link.txt",
            forbidden_messages=(secret,),
        )
        staged = os.path.join(STAGING_ROOT, "assets", "__guardtest-link.txt")
        self.assertFalse(os.path.exists(staged), "symlink assetが公開された")
        self.assertNotIn(secret, run.output)

    def test_reparse_point_asset_does_not_publish_outside_content(self):
        """indexがregular fileとして記録していても、実体がlinkなら公開しない。

        `core.symlinks=false`のcheckoutと実体が食い違う場合に相当し、Gitのmode判定
        では拾えない。ここでだけreparse point checkへ到達する。
        """
        outside = self.track(
            os.path.join(tempfile.gettempdir(), f"deskcat-outside-{uuid.uuid4().hex}.txt")
        )
        _write(outside, "OUTSIDE")
        link = self.track(os.path.join(ASSETS_ROOT, "__guardtest-reparse.txt"))
        try:
            os.symlink(outside, link)
        except (OSError, NotImplementedError):
            self.skipTest("symlink creation unavailable")
        blob = _git("rev-parse", "HEAD:README.md").strip()
        self.new_detached_index()
        _git(
            "update-index", "--add", "--cacheinfo",
            f"100644,{blob},pages/assets/__guardtest-reparse.txt",
        )
        self.add_manifest_entry({"path": "__guardtest-reparse.txt"})
        self.assert_staging_fails(
            "Declared asset is a reparse point: pages/assets/__guardtest-reparse.txt"
        )
        staged = os.path.join(STAGING_ROOT, "assets", "__guardtest-reparse.txt")
        self.assertFalse(os.path.exists(staged), "reparse point assetが公開された")

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
        # 上限は`publish_guards.FILE_SIZE_LIMIT`が正本である。testが独自の値を持つと、
        # 上限を上げたときにtestだけが古い前提のまま、理由の分からない失敗になる。
        filler = "x" * (guards.FILE_SIZE_LIMIT + 1024)
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

    def test_symlinked_portal_file_and_root_document_fail(self):
        """必須fileの経路でも、symlinkでrepository外の内容を公開しない。

        `.pages-src/`へfileを入れる経路は4つあり、symlinkとreparse pointは
        どの経路でも同じ理由で拒否する。asset経路だけを塞いでも、portal fileと
        root documentから同じことができる。実際に両方で外部内容が公開できた。
        """
        outside = self.track(
            os.path.join(tempfile.gettempdir(), f"deskcat-outside-{uuid.uuid4().hex}.txt")
        )
        secret = "OUTSIDE-CONTENT-THAT-MUST-NOT-BE-PUBLISHED"
        _write(outside, secret)
        cases = (
            ("pages/404.md", PORTAL_PAGE_PATH, "404.md",
             "Required Pages source is a symlink in Git: pages/404.md"),
            ("SECURITY.md", os.path.join(REPOSITORY_ROOT, "SECURITY.md"), "SECURITY.md",
             "Required root document is a symlink in Git: SECURITY.md"),
        )
        for repository_path, real_path, staged_name, expected in cases:
            with self.subTest(route=repository_path):
                backup = _read(real_path)
                try:
                    os.remove(real_path)
                    try:
                        os.symlink(outside, real_path)
                    except (OSError, NotImplementedError):
                        _write(real_path, backup)
                        self.skipTest("symlink creation unavailable")
                    self.detached_index_with(repository_path)
                    run = self.assert_staging_fails(
                        expected, forbidden_messages=(secret,)
                    )
                    staged = os.path.join(STAGING_ROOT, staged_name)
                    self.assertFalse(
                        os.path.exists(staged) and secret in _read(staged),
                        f"{repository_path} 経由で外部内容が公開された",
                    )
                    self.assertNotIn(secret, run.output)
                finally:
                    if guards.is_reparse_point(real_path):
                        os.remove(real_path)
                    _write(real_path, backup)

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


class LayoutBoundaryTests(PublishGuardTestCase):
    """`pages/_layouts/`の公開境界（ADR-0009）。

    layoutは全pageのHTMLを決めるため、assetより影響が大きい。`pages/assets/`と
    同じく、列挙したexact pathだけを公開し、列挙外は失敗させる。
    """

    def test_declared_layouts_are_staged(self):
        staged_root, _ = self.assert_staging_succeeds()
        staged_layouts = os.path.join(staged_root, "_layouts")
        for name in prepare_pages.PORTAL_LAYOUTS:
            self.assertTrue(
                os.path.isfile(os.path.join(staged_layouts, name)),
                f"declared layout was not staged: {name}",
            )
        self.assertEqual(
            sorted(os.listdir(staged_layouts)),
            sorted(prepare_pages.PORTAL_LAYOUTS),
            "staged layouts do not match PORTAL_LAYOUTS",
        )

    def test_undeclared_layout_on_disk_fails(self):
        path = self.track(os.path.join(LAYOUTS_ROOT, "__guardtest-extra.html"))
        _write(path, "<p>undeclared layout</p>")
        self.assert_staging_fails("Layout is not declared in PORTAL_LAYOUTS")

    def test_declared_layout_missing_on_disk_fails(self):
        path = os.path.join(LAYOUTS_ROOT, "page.html")
        backup = _read(path)
        self.addCleanup(_write, path, backup)
        os.remove(path)
        self.assert_staging_fails("Declared layout is missing: pages/_layouts/page.html")

    def test_declared_layout_not_tracked_by_git_fails(self):
        self.detached_index_without("pages/_layouts/page.html")
        self.assert_staging_fails(
            "Declared layout is not tracked by Git: pages/_layouts/page.html"
        )

    def test_symlinked_layout_does_not_publish_outside_content(self):
        """layoutがsymlinkならrepository外の内容を公開しない。

        `os.path.isfile`はlinkを辿り、copyはtarget側の内容を書き出す。staged側は
        通常fileになるため、stagingの最後にあるreparse point検査では捕まえられない。
        """
        outside = self.track(
            os.path.join(
                tempfile.gettempdir(), f"deskcat-outside-{uuid.uuid4().hex}.html"
            )
        )
        secret = "OUTSIDE-LAYOUT-THAT-MUST-NOT-BE-PUBLISHED"
        _write(outside, secret)

        layout = os.path.join(LAYOUTS_ROOT, "page.html")
        backup = _read(layout)
        # 復元はLIFOで最後に走らせる。symlinkを消す前に書き戻すと、link越しに
        # 一時fileを書き換えてしまい、repositoryにはsymlinkが残る。
        self.addCleanup(_write, layout, backup)
        os.remove(layout)
        try:
            os.symlink(outside, layout)
        except (OSError, NotImplementedError):
            self.skipTest("symlink creation unavailable")
        self.addCleanup(_remove_fixture, layout)

        # 追跡checkで弾かれると、どのguardが働いたのか区別できない。追跡済みにする。
        self.detached_index_with("pages/_layouts/page.html")
        run = self.assert_staging_fails(
            "Declared layout is a symlink in Git: pages/_layouts/page.html",
            forbidden_messages=(secret,),
        )
        staged = os.path.join(STAGING_ROOT, "_layouts", "page.html")
        self.assertFalse(os.path.exists(staged), "symlink layoutが公開された")
        self.assertNotIn(secret, run.output)


class FaviconTests(unittest.TestCase):
    """faviconをASCII artから組み立てる処理を、子processを介さず直接検査する。

    以前のfaviconは1 x 1の単色placeholderで、内容を見るtestが無かった。
    自前layoutが`<link rel="icon">`を持つため、生成物がそのままtabへ出る。
    """

    def test_art_is_square_and_uses_the_declared_palette(self):
        for art, expected in (
            (prepare_pages.FAVICON_ART_32, 32),
            (prepare_pages.FAVICON_ART_16, 16),
        ):
            self.assertEqual(len(art), expected)
            for row in art:
                self.assertEqual(len(row), expected)
                for character in row:
                    self.assertIn(character, prepare_pages.FAVICON_PALETTE)

    def test_ragged_art_is_rejected(self):
        with self.assertRaises(guards.ValidationError):
            prepare_pages._favicon_image(("..", "."))

    def test_non_square_art_is_rejected(self):
        with self.assertRaises(guards.ValidationError):
            prepare_pages._favicon_image(("..", "..", ".."))

    def test_undeclared_character_is_rejected(self):
        with self.assertRaises(guards.ValidationError):
            prepare_pages._favicon_image(("X.", ".."))

    def test_empty_art_is_rejected(self):
        with self.assertRaises(guards.ValidationError):
            prepare_pages._favicon_image(())

    def test_art_larger_than_256_is_rejected(self):
        """ICOのdirectoryは寸法を1 byteで持ち、256だけを0で表す。

        257以上を黙って0（=256）として書き出すと、宣言した寸法が実体と食い違う。
        """
        oversized = tuple("." * 257 for _ in range(257))
        with self.assertRaises(guards.ValidationError):
            prepare_pages._favicon_image(oversized)

    def test_icon_declares_two_square_images_with_consistent_sizes(self):
        data = prepare_pages.build_favicon()
        reserved, kind, count = struct.unpack_from("<HHH", data, 0)
        self.assertEqual(reserved, 0)
        self.assertEqual(kind, 1, "ICO type must be 1 (icon)")
        self.assertEqual(count, 2, "expected a 32x32 and a 16x16 image")

        seen = []
        for index in range(count):
            entry = struct.unpack_from("<BBBBHHII", data, 6 + 16 * index)
            width, height, colors, entry_reserved = entry[0:4]
            planes, bits, byte_count, offset = entry[4:8]
            self.assertEqual(width, height, "favicon images must be square")
            self.assertEqual(colors, 0)
            self.assertEqual(entry_reserved, 0)
            self.assertEqual(planes, 1)
            self.assertEqual(bits, 32)
            seen.append(width)

            # 宣言したbyte数が、header + XOR + AND maskの実寸と一致すること。
            mask_row_bytes = ((width + 31) // 32) * 4
            expected = 40 + width * width * 4 + mask_row_bytes * width
            self.assertEqual(byte_count, expected)
            self.assertLessEqual(
                offset + byte_count, len(data), "image extends past the file"
            )

            header = struct.unpack_from("<IiiHHIIiiII", data, offset)
            self.assertEqual(header[0], 40, "BITMAPINFOHEADER size")
            self.assertEqual(header[1], width)
            # `biHeight`はXOR maskとAND maskの合計を表すため実寸の2倍になる。
            self.assertEqual(header[2], width * 2)
            self.assertEqual(header[4], 32, "biBitCount")
            self.assertEqual(header[6], expected - 40, "biSizeImage")

        self.assertEqual(sorted(seen), [16, 32])

    def test_staged_favicon_matches_the_generated_bytes(self):
        staged = os.path.join(STAGING_ROOT, "favicon.ico")
        if not os.path.isfile(staged):
            self.skipTest("staging has not run yet")
        with open(staged, "rb") as handle:
            self.assertEqual(handle.read(), prepare_pages.build_favicon())


if __name__ == "__main__":
    unittest.main(verbosity=2)
