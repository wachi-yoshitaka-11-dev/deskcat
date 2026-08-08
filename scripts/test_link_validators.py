#!/usr/bin/env python3
"""source／生成siteのlink validatorに対する回帰test。

同一page内のanchorを見逃さないことを、source Markdownと生成HTMLの両方で確認する。
Jekyllを追加せず、各validatorを一時directoryに対して子processで実行する。
子processにするのは、exit codeとstdout／stderrという公開contractまで検査するためであり、
関数を直接呼ぶとvalidator側のprocess終了経路を検証できない。

link作成不可の環境では、対象caseを`skipTest`で成功件数と分ける。実行していないものを
成功として数えない。

repository root以外のcurrent directoryでも成功しなければならない。Pages CIはこの
harnessをrunnerの一時directoryから絶対pathで起動する。
"""

import os
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest
import uuid
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import publish_guards as guards  # noqa: E402
import validate_pages_output  # noqa: E402

SCRIPT_DIRECTORY = Path(__file__).resolve().parent
REPOSITORY_ROOT = str(SCRIPT_DIRECTORY.parent)
VALIDATE_DOCS = str(SCRIPT_DIRECTORY / "validate_doc_links.py")
VALIDATE_OUTPUT = str(SCRIPT_DIRECTORY / "validate_pages_output.py")

# validatorへ渡すlocal pathは、診断へ再掲されてはならない。値を取る option を
# 個別caseではなくharness側で列挙し、新しい診断を追加した際の漏れを防ぐ。
PRIVATE_PATH_OPTIONS = ("--repository-root", "--site-root", "--pages-config-path")

VALID_PAGE = '<html><body><h1 id="existing">Existing</h1></body></html>'

# NBSP。URL parserもYAMLもASCII空白としては扱わないが、Unicode whitespace判定では
# 空白になる。両者を取り違えていないことを確かめるfixtureで使う。
# code pointで書く。source中に生のNBSPを置くと、目視でもdiffでもASCII空白と区別できない。
NON_URL_WHITESPACE = chr(0x00A0)

REQUIRED_HTML = (
    "index.html",
    "404.html",
    "docs/architecture/index.html",
    "docs/backlog/index.html",
    "docs/governance/index.html",
    "docs/governance/hardware-safety-policy.html",
    "docs/decisions/index.html",
    "docs/hardware/index.html",
    "docs/planning/index.html",
    "docs/protocol/index.html",
    "docs/runbooks/index.html",
    "docs/toolchains/index.html",
)

REQUIRED_ASSETS = ("favicon.ico", "assets/css/style.css", "assets/deskcat-concept.jpg")

_state = {}


def _write(path, content):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8", newline="") as handle:
        handle.write(content)


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


class ValidatorRun:
    def __init__(self, exit_code, timed_out, output, private_paths):
        self.exit_code = exit_code
        self.timed_out = timed_out
        self.output = output
        self.private_paths = private_paths


def run_validator(script_path, arguments=(), timeout_seconds=20):
    """validatorを子processとして実行し、結果とlocal pathの一覧を返す。

    stdoutとstderrを同時に読み、pipe bufferの飽和でwaitがdeadlockしないようにする。
    timeoutではprocess treeを終了させ、設定値だけのtimeoutにしない。
    """
    effective = list(arguments)
    if script_path == VALIDATE_OUTPUT and "--pages-config-path" not in effective:
        # output validatorのunit fixtureをrepository固有の`pages/_config.yml`から分離する。
        # baseurl変更時にvalidator logicと無関係なcaseが壊れないよう、既知のconfigを明示する。
        effective += ["--pages-config-path", _state["default_pages_config"]]

    private_paths = []
    for index, argument in enumerate(effective):
        if argument in PRIVATE_PATH_OPTIONS and index + 1 < len(effective):
            private_paths.append(guards.full_path(effective[index + 1]))

    process = subprocess.Popen(
        [sys.executable, script_path, *effective],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    try:
        stdout, stderr = process.communicate(timeout=timeout_seconds)
    except subprocess.TimeoutExpired:
        process.kill()
        process.communicate()
        return ValidatorRun(
            -1,
            True,
            f"Validator timed out after {timeout_seconds} second(s).",
            private_paths,
        )
    combined = [part.rstrip() for part in (stdout, stderr) if part and part.strip()]
    return ValidatorRun(process.returncode, False, "\n".join(combined), private_paths)


def output_exposes_private_path(run):
    """validatorの出力がuser指定のlocal pathを含むかを判定する。

    separator表記を揃え、Windowsではfilesystemと同じくcase違いも検出する。
    """
    normalized_output = run.output.replace("\\", "/")
    if os.name == "nt":
        normalized_output = normalized_output.lower()
    for private_path in run.private_paths:
        normalized = private_path.replace("\\", "/")
        if os.name == "nt":
            normalized = normalized.lower()
        if normalized and normalized in normalized_output:
            return True
    return False


def setUpModule():
    temporary_parent = guards.full_path(tempfile.gettempdir())
    temporary_root = os.path.join(
        temporary_parent, f"deskcat-link-validator-tests-{uuid.uuid4().hex}"
    )
    os.makedirs(temporary_root)
    _state["temporary_parent"] = temporary_parent
    _state["temporary_root"] = temporary_root
    try:
        _build_fixtures(temporary_root)
    except BaseException:
        # `setUpModule`が失敗すると`tearDownModule`は呼ばれない。ここで消さないと、
        # 下の`_git`が作ったread-onlyのgit objectを含むfixtureが一時directoryへ残る。
        # 失敗するたびに1件ずつ溜まる。
        _remove_fixture(temporary_root)
        _state.clear()
        raise


def _build_fixtures(temporary_root):
    _state["default_pages_config"] = os.path.join(
        temporary_root, "default-pages-config.yml"
    )
    _write(_state["default_pages_config"], "baseurl: /deskcat")

    # source validator向けfixture。
    source_root = os.path.join(temporary_root, "source")
    docs_root = os.path.join(source_root, "docs")
    os.makedirs(docs_root)
    _state["source_root"] = source_root
    _state["source_docs"] = docs_root
    _state["source_page"] = os.path.join(docs_root, "page.md")
    _write(_state["source_page"], "# Existing\n\n[valid](#existing)")
    _git(source_root, "init", "--quiet")
    _git(source_root, "config", "core.autocrlf", "false")
    _git(source_root, "add", "--", "docs/page.md")

    # fixtureがindexへ入ったことを、validatorと同じhelper呼び出しで確認する。
    # ここが空だと、validatorは「追跡fileが無い」として失敗し、anchor検査の結果ではなく
    # setupの失敗をtestの失敗として報告してしまう。
    indexed = guards.get_tracked_files(source_root, ".")
    if "docs/page.md" not in indexed:
        listing = _git(source_root, "ls-files")
        raise RuntimeError(
            "Temporary fixture is not listed by the validator's tracked-file helper. "
            f"ls-files(all)=[{listing}]"
        )

    # 追跡fileはあるがMarkdownが無いrepository。
    zero_source_root = os.path.join(temporary_root, "zero-source")
    os.makedirs(zero_source_root)
    _write(os.path.join(zero_source_root, "tracked.txt"), "fixture")
    _git(zero_source_root, "init", "--quiet")
    _git(zero_source_root, "add", "--", "tracked.txt")
    _state["zero_source_root"] = zero_source_root

    # output validator向けfixture。
    site_root = os.path.join(temporary_root, "site")
    for relative_path in REQUIRED_HTML:
        _write(os.path.join(site_root, *relative_path.split("/")), VALID_PAGE)
    for relative_path in REQUIRED_ASSETS:
        _write(os.path.join(site_root, *relative_path.split("/")), "fixture")

    extra_root = os.path.join(site_root, "extra")
    os.makedirs(extra_root)
    extra_count = guards.MINIMUM_PUBLISHED_COUNT - len(REQUIRED_HTML)
    if extra_count < 1:
        raise RuntimeError(
            "The HTML-count boundary fixture requires at least one non-required HTML file."
        )
    for index in range(extra_count):
        _write(os.path.join(extra_root, f"page-{index}.html"), VALID_PAGE)

    _state["site_root"] = site_root
    _state["extra_root"] = extra_root
    _state["output_index"] = os.path.join(site_root, "index.html")
    _state["nested_output_index"] = os.path.join(
        site_root, "docs", "architecture", "index.html"
    )


def tearDownModule():
    # GUID付きの専用directoryだけを削除し、一時directory root自体は対象にしない。
    temporary_root = _state.get("temporary_root")
    if not temporary_root:
        return
    if guards.path_within_root(
        temporary_root, _state["temporary_parent"]
    ) and os.path.basename(temporary_root).startswith("deskcat-link-validator-tests-"):
        if os.path.exists(temporary_root):
            _remove_fixture(temporary_root)


def _remove_fixture(path):
    """fixtureを取り除き、消せなかった事実だけを報告する。

    削除の中身は`publish_guards.remove_tree`が持つ。file、directory、reparse point、
    read-onlyのいずれも扱う。2つのharnessで実装を複製すると、片方のchmodや警告だけを
    変えたときに、もう片方でfixtureが黙って溜まる。
    警告にはfile名だけを出す。一時directoryの絶対pathをlogへ書かない。
    """
    if not guards.remove_tree(path):
        print(
            f"warning: failed to remove the test fixture {os.path.basename(path)}",
            file=sys.stderr,
        )


class ValidatorAssertions(unittest.TestCase):
    def assert_outcome(
        self, run, should_succeed, expected_message="", forbidden_message=""
    ):
        # 機密fixtureがchild outputへ現れた場合は、他の診断より先に検出し、
        # `run.output`をこのtest processから再表示しない。
        self.assertFalse(run.timed_out, "validator process timed out")
        # validatorの成否に関係なく、user指定のlocal pathをstdout／stderrへ出さない。
        self.assertFalse(
            output_exposes_private_path(run),
            "local absolute path was exposed in validator output",
        )
        if forbidden_message:
            self.assertNotIn(
                forbidden_message,
                run.output,
                "forbidden text was exposed in validator output",
            )
        if should_succeed:
            self.assertEqual(run.exit_code, 0, run.output)
        else:
            self.assertNotEqual(run.exit_code, 0, "expected failure, but validation succeeded")
        if expected_message:
            self.assertIn(expected_message, run.output)


class HarnessSelfTests(ValidatorAssertions):
    """harness自身の検出力を先に固定する。

    timeoutやprivate-path検出がno-opでも、各validator caseのoutputがたまたま
    cleanなら全件成功してしまう。
    """

    def test_harness_bounds_child_process_runtime(self):
        probe = os.path.join(_state["temporary_root"], "timeout_probe.py")
        _write(probe, "import time\ntime.sleep(5)\n")
        try:
            run = run_validator(probe, timeout_seconds=1)
            self.assertTrue(run.timed_out)
            self.assertEqual(run.exit_code, -1)
            self.assertIn("timed out after 1 second", run.output)
        finally:
            os.remove(probe)

    def test_harness_detects_private_paths_without_false_positives(self):
        probe_path = os.path.join(_state["temporary_root"], "private-path-probe")
        rendered = probe_path.replace("\\", "/")
        if os.name == "nt":
            # WindowsでOrdinalへ退化した場合もpositive controlが失敗するようcaseを変える。
            rendered = rendered.upper()
        leaking = ValidatorRun(1, False, f"synthetic diagnostic: {rendered}", [probe_path])
        clean = ValidatorRun(1, False, "synthetic diagnostic: extra/page.html", [probe_path])
        self.assertTrue(output_exposes_private_path(leaking))
        self.assertFalse(output_exposes_private_path(clean))


class SourceValidatorTests(ValidatorAssertions):
    def tearDown(self):
        _write(_state["source_page"], "# Existing\n\n[valid](#existing)")

    def test_missing_repository_root_is_not_exposed(self):
        missing = os.path.join(_state["temporary_root"], "missing-repository")
        run = run_validator(VALIDATE_DOCS, ["--repository-root", missing])
        self.assertEqual(run.private_paths, [guards.full_path(missing)])
        self.assert_outcome(
            run, False, expected_message="Repository root does not exist."
        )

    def test_zero_tracked_markdown_is_reported(self):
        run = run_validator(
            VALIDATE_DOCS, ["--repository-root", _state["zero_source_root"]]
        )
        self.assert_outcome(
            run, False, expected_message="Unable to enumerate tracked Markdown files."
        )

    def test_zero_resolved_markdown_is_reported(self):
        """indexにはあるがworking treeに無いMarkdownを、0件のまま成功にしない。"""
        root = _state["zero_source_root"]
        missing_markdown = os.path.join(root, "missing.md")
        _write(missing_markdown, "# Missing")
        _git(root, "add", "--", "missing.md")
        try:
            os.remove(missing_markdown)
            run = run_validator(VALIDATE_DOCS, ["--repository-root", root])
            self.assert_outcome(
                run, False, expected_message="No tracked Markdown files resolved."
            )
        finally:
            _git(root, "update-index", "--force-remove", "missing.md")

    def test_broken_same_page_anchor_is_rejected(self):
        _write(_state["source_page"], "# Existing\n\n[broken](#missing)")
        run = run_validator(
            VALIDATE_DOCS, ["--repository-root", _state["source_root"]]
        )
        self.assert_outcome(run, False, expected_message="Broken anchor")

    def test_valid_same_page_anchor_is_accepted(self):
        run = run_validator(
            VALIDATE_DOCS, ["--repository-root", _state["source_root"]]
        )
        self.assert_outcome(run, True, expected_message="BROKEN=0")

    def test_unverifiable_anchor_is_reported(self):
        """anchor集合を持たないMarkdownへのfragment linkを、無検査で通さない。

        追跡外やsymlinkのMarkdownはanchor集合に入らない。skipすると、このscriptで
        ここだけがfail-openになる。
        """
        untracked = os.path.join(_state["source_docs"], "untracked.md")
        _write(untracked, "# Other\n")
        _write(_state["source_page"], "# Existing\n\n[out](untracked.md#other)")
        try:
            run = run_validator(
                VALIDATE_DOCS, ["--repository-root", _state["source_root"]]
            )
            self.assert_outcome(run, False, expected_message="Unverifiable anchor")
        finally:
            os.remove(untracked)


class OutputValidatorTestCase(ValidatorAssertions):
    """`_site` fixtureを共有し、各caseの前後で入口pageを既知の状態へ戻す。"""

    def setUp(self):
        self._temporary_paths = []
        _write(_state["output_index"], VALID_PAGE)

    def tearDown(self):
        _write(_state["output_index"], VALID_PAGE)
        _write(_state["nested_output_index"], VALID_PAGE)
        for path in reversed(self._temporary_paths):
            _remove_fixture(path)

    def track(self, path):
        self._temporary_paths.append(path)
        return path

    def run_site(self, arguments=None):
        return run_validator(
            VALIDATE_OUTPUT, ["--site-root", _state["site_root"], *(arguments or [])]
        )

    def check_index(self, html, should_succeed, expected_message="", forbidden=""):
        _write(_state["output_index"], html)
        self.assert_outcome(
            self.run_site(), should_succeed, expected_message, forbidden
        )


class OutputValidatorArgumentTests(OutputValidatorTestCase):
    def test_missing_site_root_is_not_exposed(self):
        missing = os.path.join(_state["temporary_root"], "missing-site")
        run = run_validator(VALIDATE_OUTPUT, ["--site-root", missing])
        self.assertEqual(
            run.private_paths,
            [guards.full_path(missing), guards.full_path(_state["default_pages_config"])],
        )
        self.assert_outcome(
            run, False, expected_message="Pages output directory does not exist."
        )

    def test_missing_config_path_is_not_exposed(self):
        missing = os.path.join(_state["temporary_root"], "missing-pages-config.yml")
        run = run_validator(
            VALIDATE_OUTPUT,
            ["--site-root", _state["site_root"], "--pages-config-path", missing],
        )
        self.assertEqual(
            run.private_paths,
            [guards.full_path(_state["site_root"]), guards.full_path(missing)],
        )
        self.assert_outcome(
            run, False, expected_message="Pages config file does not exist."
        )


class OutputValidatorArtifactTests(OutputValidatorTestCase):
    def test_broken_same_page_anchor_in_nested_page(self):
        _write(
            _state["nested_output_index"],
            '<html><body><h1 id="existing">Existing</h1>'
            '<a href="#missing">broken</a></body></html>',
        )
        self.assert_outcome(
            self.run_site(),
            False,
            expected_message=(
                "Broken anchor in generated site: docs/architecture/index.html -> #missing"
            ),
        )

    def test_missing_required_output(self):
        """required outputとHTML件数下限を個別に回帰させる。

        別のfailure messageではなく、それぞれのguardが実際に動いたことまで確認する。
        """
        required_asset = os.path.join(_state["site_root"], "favicon.ico")
        os.remove(required_asset)
        try:
            self.assert_outcome(
                self.run_site(),
                False,
                expected_message="Required output is missing: favicon.ico",
            )
        finally:
            _write(required_asset, "fixture")

    def test_html_set_below_minimum(self):
        boundary_page = os.path.join(_state["extra_root"], "page-0.html")
        os.remove(boundary_page)
        try:
            self.assert_outcome(
                self.run_site(), False, expected_message="Unexpectedly small HTML set"
            )
        finally:
            _write(boundary_page, VALID_PAGE)

    def test_summary_reports_the_output_breakdown(self):
        """`_site`の内訳をsummaryへ出す。

        `FILES=`と`HTML=`だけでは非HTMLが何であるか分からない。実際に公開される
        artifactは`_site`であり、その構成はJekyllとpluginが決めるためsourceからは辿れない。
        `UNSCANNED=`は、secret／個人pathのscan対象外になっている拡張子を明示する。
        """
        # 拡張子なしのfileも内訳へ出る。`LICENSE`はfile名で許可され、scanもされる。
        license_file = self.track(os.path.join(_state["extra_root"], "LICENSE"))
        _write(license_file, "license text")
        run = self.run_site()
        self.assert_outcome(run, True, expected_message="EXTENSIONS=")

        extensions = next(
            line for line in run.output.splitlines() if line.startswith("EXTENSIONS=")
        )
        unscanned = next(
            line for line in run.output.splitlines() if line.startswith("UNSCANNED=")
        )
        self.assertIn(".html=", extensions)
        self.assertIn("(none)=1", extensions)
        # binaryはscanしても意味が無いためUNSCANNEDへ現れる。textは現れない。
        self.assertIn(".ico=1", unscanned)
        self.assertIn(".jpg=1", unscanned)
        self.assertNotIn(".html", unscanned)
        self.assertNotIn(".css", unscanned)
        # `LICENSE`はscan対象なので、拡張子なしでもUNSCANNEDへ入らない。
        self.assertNotIn("(none)", unscanned)
        self.assertRegex(run.output, r"LARGEST=\d+ \S+")

    def test_unscanned_extensions_are_really_not_scanned(self):
        """`UNSCANNED=`の表示と、実際のscan範囲が食い違わないこと。

        `is_scanned_for_secrets`をscan本体と診断の両方で使っているが、scan側が
        その関数を通さなくなっても、範囲が広がる方向なら既存testは落ちない。
        その場合`UNSCANNED=`だけが嘘になる。ここで両者の一致を直接固定する。

        binaryを内容scanしないのは意図した範囲である。`.png`はallowlist上は許可だが
        `TEXT_EXTENSIONS`には無いため、secretらしき文字列を入れても検出されない。
        """
        binary_like = self.track(os.path.join(_state["extra_root"], "binary.png"))
        _write(binary_like, "ghp_" + "f" * 24)
        run = self.run_site()
        # scan対象外なので検出されない。ここが失敗するならscan範囲が広がっている。
        self.assert_outcome(run, True)
        unscanned = next(
            line for line in run.output.splitlines() if line.startswith("UNSCANNED=")
        )
        self.assertIn(".png=1", unscanned)

    def test_disallowed_output_file_type_is_rejected(self):
        """`.pages-src`側と同じ拡張子allowlistを`_site`へも課す。

        2026-08-08時点の実`_site`は`.html .md .css .ico .jpg`と`LICENSE`だけであり、
        この判定は現状no-opである。Jekyllやpluginが将来別の拡張子を生成したときに、
        気付かないまま公開せず止めるために置いている。
        """
        for name, content in (("feed.xml", "<feed/>"), ("app.js", "//"), ("NOTICE", "x")):
            with self.subTest(name=name):
                target = self.track(os.path.join(_state["extra_root"], name))
                _write(target, content)
                self.assert_outcome(
                    self.run_site(),
                    False,
                    expected_message=f"File type is not approved for Pages: extra/{name}",
                )
                os.remove(target)

    def test_oversized_output_file_is_rejected(self):
        """size上限も`.pages-src`側と同じ`FILE_SIZE_LIMIT`を使う。

        実`_site`の最大は205894 byteであり、こちらも現状no-opである。
        testが独自の閾値を持つと、上限を変えたときにtestだけが古い前提で失敗する。
        """
        oversized = self.track(os.path.join(_state["extra_root"], "big.html"))
        _write(oversized, VALID_PAGE + "x" * (guards.FILE_SIZE_LIMIT + 1024))
        self.assert_outcome(
            self.run_site(),
            False,
            expected_message="File exceeds the Pages size limit: extra/big.html",
        )

    def test_pdf_output_is_reported_with_a_relative_path(self):
        pdf_asset = self.track(os.path.join(_state["extra_root"], "manual.PDF"))
        _write(pdf_asset, "synthetic PDF fixture")
        self.assert_outcome(
            self.run_site(),
            False,
            expected_message="PDF output is not allowed: extra/manual.PDF",
        )

    def test_existing_target_outside_site_root_is_rejected(self):
        """repository外targetが実在しても、_siteのlexical boundary外なら拒否する。"""
        outside = self.track(os.path.join(_state["temporary_root"], "outside-target.html"))
        _write(outside, '<html><body><h1 id="outside">Outside</h1></body></html>')
        self.check_index(
            '<html><body><h1 id="existing">Existing</h1>'
            '<a href="../outside-target.html">outside</a></body></html>',
            False,
            "Link escapes Pages output in index.html",
        )


class OutputValidatorSecretTests(OutputValidatorTestCase):
    """秘密情報と個人pathのguardを回帰させる。

    この2つは公開直前の最終確認であり、patternが退化しても他のcaseはすべて通るため、
    専用のcaseが無いと無検査になる。fixtureの値は明確に合成のものを使う。
    """

    def test_secret_like_html_content(self):
        guard_page = self.track(os.path.join(_state["extra_root"], "guard.html"))
        secret = "ghp_" + "a" * 24
        _write(
            guard_page,
            f'<html><body><h1 id="{secret}">Sensitive</h1>'
            f'<a href="missing.html?token={secret}">fixture</a></body></html>',
        )
        _write(
            _state["output_index"],
            '<html><body><h1 id="existing">Existing</h1>'
            '<a href="/deskcat/extra/guard.html#missing">sensitive target</a></body></html>',
        )
        self.assert_outcome(
            self.run_site(),
            False,
            expected_message="Secret-like content detected: extra/guard.html",
            forbidden_message=secret,
        )

    def test_personal_absolute_path_in_html(self):
        guard_page = self.track(os.path.join(_state["extra_root"], "guard.html"))
        personal_path = "/home/exampleuser/notes.md"
        _write(guard_page, f'<html><body><a href="{personal_path}">fixture</a></body></html>')
        self.assert_outcome(
            self.run_site(),
            False,
            expected_message="Personal absolute path detected: extra/guard.html",
            forbidden_message=personal_path,
        )

    def test_secret_like_generated_css(self):
        """最終artifactではHTML以外のtext出力もscanする。"""
        guard_asset = self.track(os.path.join(_state["extra_root"], "guard.css"))
        secret = "ghp_" + "b" * 24
        _write(guard_asset, f"/* {secret} */")
        self.assert_outcome(
            self.run_site(),
            False,
            expected_message="Secret-like content detected: extra/guard.css",
            forbidden_message=secret,
        )

    def test_personal_absolute_path_in_generated_css(self):
        guard_asset = self.track(os.path.join(_state["extra_root"], "guard.css"))
        personal_path = "/home/exampleuser/generated.css"
        _write(guard_asset, f"/* {personal_path} */")
        self.assert_outcome(
            self.run_site(),
            False,
            expected_message="Personal absolute path detected: extra/guard.css",
            forbidden_message=personal_path,
        )

    def test_extensionless_license_output_is_scanned(self):
        guard_license = self.track(os.path.join(_state["extra_root"], "LICENSE"))
        secret = "ghp_" + "c" * 24
        _write(guard_license, secret)
        self.assert_outcome(
            self.run_site(),
            False,
            expected_message="Secret-like content detected: extra/LICENSE",
            forbidden_message=secret,
        )


class OutputValidatorHtmlScannerTests(OutputValidatorTestCase):
    """HTML走査の境界。tag、comment、raw-text、属性tokenizerを個別に固定する。"""

    CASES = (
        (
            "unquoted link attribute",
            # raw HTMLではunquoted属性も有効である。quoted属性だけを抽出すると、
            # このlinkを無検査で通す。
            '<html><body><h1 id="existing">Existing</h1>'
            "<a href=/deskcat/extra/missing.svg>missing asset</a></body></html>",
            False,
            "Broken local link",
        ),
        (
            "data-href metadata is ignored",
            # data-hrefはnavigation属性ではないため対象にしない。
            '<html><body><h1 id="existing">Existing</h1>'
            '<div data-href="/deskcat/extra/missing.svg">metadata</div></body></html>',
            True,
            "BROKEN_LINKS=0",
        ),
        (
            "attribute-like text inside a quoted value is ignored",
            # quoted value内の`href=`や`id=`は実属性ではない。開始tag全体へのregexでは
            # `/missing.svg`をlinkとして誤検出する。
            '<html><body><h1 id="existing">Existing</h1>'
            '<div data-note="text href=/deskcat/extra/missing.svg" id=fake>metadata</div>'
            '<a href="#existing">valid</a></body></html>',
            True,
            "BROKEN_LINKS=0",
        ),
        (
            "quoted value does not create an id",
            '<html><body><h1 id="existing">Existing</h1>'
            '<div data-note="id=phantom">metadata</div>'
            '<a href="#phantom">invalid</a></body></html>',
            False,
            "Broken anchor in generated site",
        ),
        (
            "duplicate id attribute is dropped",
            # HTML tokenizerは同名attributeの先頭だけを採用する。2個目のidを集合へ
            # 入れると、browserには存在しないanchorをvalidatorだけが有効と誤認する。
            '<html><body><h1 id="existing" ID="phantom">Existing</h1>'
            '<a href="#phantom">invalid</a></body></html>',
            False,
            "Broken anchor in generated site",
        ),
        (
            "duplicate link attribute is dropped",
            '<html><body><h1 id="existing">Existing</h1>'
            '<a href="#existing" HREF="/deskcat/extra/missing.svg">valid first value</a>'
            "</body></html>",
            True,
            "BROKEN_LINKS=0",
        ),
        (
            "namespaced SVG link resolves",
            # SVG 1.1のnamespaced linkも実navigation属性として検査する。
            '<html><body><h1 id="existing">Existing</h1>'
            '<svg><use xlink:href="/deskcat/extra/sprite.svg#cat"></use></svg></body></html>',
            True,
            "BROKEN_LINKS=0",
        ),
        (
            "missing namespaced SVG link is rejected",
            '<html><body><h1 id="existing">Existing</h1>'
            '<svg><use xlink:href="/deskcat/extra/missing.svg#cat"></use></svg></body></html>',
            False,
            "Broken local link",
        ),
        (
            "empty same-page href is accepted",
            '<html><body><h1 id="existing">Existing</h1><a href="">same page</a></body></html>',
            True,
            "BROKEN_LINKS=0",
        ),
        (
            "src without a resource path is rejected",
            '<html><body><h1 id="existing">Existing</h1><img src="#existing"></body></html>',
            False,
            "Source attribute has no resource path",
        ),
        (
            "displayed and commented link examples are ignored",
            # text nodeに表示された属性例とHTML comment内のlinkはrendered navigationではない。
            '<html><body><h1 id="existing">Existing</h1>'
            "<code>href=&quot;/deskcat/extra/missing.svg&quot;</code>"
            '<!-- <a href="/deskcat/extra/missing.svg">comment</a> --></body></html>',
            True,
            "BROKEN_LINKS=0",
        ),
        (
            "attributes after a quoted greater-than sign are scanned",
            '<html><body><h1 id="existing">Existing</h1>'
            '<a title="1 > 0" href="/deskcat/extra/missing.svg">missing</a></body></html>',
            False,
            "Broken local link",
        ),
        (
            "link-like text in raw-text and RCDATA bodies is ignored",
            '<html><head><TITLE><a href="/deskcat/extra/missing.svg">title text</a></TITLE>'
            '</head><body><h1 id="existing">Existing</h1>'
            "<SCRIPT>const sample = '<a href=\"/deskcat/extra/missing.svg\">';</SCRIPT>"
            '<TEXTAREA><a href="/deskcat/extra/missing.svg">textarea text</a></TEXTAREA>'
            "</body></html>",
            True,
            "BROKEN_LINKS=0",
        ),
        (
            "real links survive comment-like raw text",
            # commentを先に除去すると、後続の実linkまで`<!--`と`-->`の間として消える。
            '<html><body><h1 id="existing">Existing</h1>'
            '<script>const marker = "<!--";</script>'
            '<a href="/deskcat/extra/missing.svg">missing</a><!-- actual comment --></body></html>',
            False,
            "Broken local link",
        ),
        (
            "comments raw text and quoted attributes stay separate",
            '<html><body><h1 id="existing">Existing</h1><!-- <script>fake</script> -->'
            '<a title="<!--" href="/deskcat/extra/missing.svg">missing</a>'
            "<!-- actual comment --></body></html>",
            False,
            "Broken local link",
        ),
        (
            "scanning continues after a text less-than sign",
            '<html><body><h1 id="existing">Existing</h1><p>1 < 2</p>'
            '<a href="/deskcat/extra/missing.svg">missing</a></body></html>',
            False,
            "Broken local link",
        ),
        (
            "every supported raw-text element body is ignored",
            # deprecated要素を含め、browserがtextとして扱うfallback内のtag-like textを
            # navigationに数えない。
            '<html><body><h1 id="existing">Existing</h1>'
            '<iframe><a href="/deskcat/extra/missing.svg">iframe</a></iframe>'
            '<xmp><a href="/deskcat/extra/missing.svg">xmp</a></xmp>'
            '<noembed><a href="/deskcat/extra/missing.svg">noembed</a></noembed>'
            '<noframes><a href="/deskcat/extra/missing.svg">noframes</a></noframes>'
            '<a href="#existing">valid</a></body></html>',
            True,
            "BROKEN_LINKS=0",
        ),
        (
            "content after plaintext is treated as text",
            '<html><body><h1 id="existing">Existing</h1>'
            '<plaintext><a href="/deskcat/extra/missing.svg">text only</a></plaintext>'
            "</body></html>",
            True,
            "BROKEN_LINKS=0",
        ),
        (
            "scanning continues after a self-closing raw-text tag",
            # namespaceを追跡しないscannerで`<style/>`を未閉鎖HTML raw-textと決めつけると、
            # inline SVG後の実linkをEOFまで隠す。
            '<html><body><h1 id="existing">Existing</h1><svg><style /></svg>'
            '<a href="/deskcat/extra/missing.svg">missing</a></body></html>',
            False,
            "Broken local link",
        ),
        (
            "scanning resumes after an abruptly closed empty comment",
            # `<!-->`と`--!>`もHTML parser上はcommentを終了する。
            '<html><body><h1 id="existing">Existing</h1><!-->'
            '<a href="/deskcat/extra/missing.svg">missing</a></body></html>',
            False,
            "Broken local link",
        ),
        (
            "scanning resumes after a comment end bang",
            '<html><body><h1 id="existing">Existing</h1><!-- ignored --!>'
            '<a href="/deskcat/extra/missing.svg">missing</a></body></html>',
            False,
            "Broken local link",
        ),
        (
            "scanning resumes after a processing instruction",
            # processing instructionと未知のmarkup declarationはbogus commentとして
            # 最初の`>`で終わる。quoteを尊重すると後続の実linkまでskipする。
            '<html><body><h1 id="existing">Existing</h1><?fixture ">'
            '<a href="/deskcat/extra/missing.svg">missing</a></body></html>',
            False,
            "Broken local link",
        ),
        (
            "scanning resumes after an unknown markup declaration",
            '<html><body><h1 id="existing">Existing</h1><!fixture ">'
            '<a href="/deskcat/extra/missing.svg">missing</a></body></html>',
            False,
            "Broken local link",
        ),
        (
            "scanning resumes after an invalid end-tag opener",
            '<html><body><h1 id="existing">Existing</h1></ ">'
            '<a href="/deskcat/extra/missing.svg">missing</a></body></html>',
            False,
            "Broken local link",
        ),
        (
            "malformed long-tag parsing stays bounded",
            # 閉じていない長大な開始tagでも、再走査やbacktrackingを起こさず線形に完了する。
            '<html><body><h1 id="existing">Existing</h1><a ' + "x" * 100000,
            True,
            "BROKEN_LINKS=0",
        ),
        (
            "matching unquoted id and link attributes are accepted",
            "<html><body><h1 id=unquoted-anchor>Existing</h1>"
            "<a href=#unquoted-anchor>valid</a></body></html>",
            True,
            "BROKEN_LINKS=0",
        ),
    )

    def setUp(self):
        super().setUp()
        # 非HTML assetのfragmentはHTML id検査の対象外だが、asset自体の存在は
        # href/src検査で必須とする。存在するassetを1つ用意しておく。
        sprite = self.track(os.path.join(_state["extra_root"], "sprite.svg"))
        _write(sprite, '<svg xmlns="http://www.w3.org/2000/svg"><symbol id="cat" /></svg>')

    def test_html_scanner_cases(self):
        for name, html, should_succeed, expected in self.CASES:
            with self.subTest(case=name):
                self.check_index(html, should_succeed, expected)


class OutputValidatorUrlSchemeTests(OutputValidatorTestCase):
    def test_obfuscated_javascript_url_is_rejected(self):
        self.check_index(
            '<html><body><h1 ID="existing">Existing</h1>'
            '<a href="JaVa&#x0a;ScRiPt:alert(1)">unsafe</a></body></html>',
            False,
            "Unsafe URL scheme",
            forbidden="JaVa\nScRiPt",
        )

    def test_unsafe_schemes_are_rejected(self):
        cases = (
            # data URIはasset manifest／size guardを迂回する。
            ("data URI", '<img src="data:image/svg+xml,fixture">'),
            # 明示allowlist外のURI schemeをlocal pathへ落とさない。
            ("file URI", '<a href="file:fixture.html">file</a>'),
            ("vbscript URI", '<a href="vbscript:msgbox(1)">vbscript</a>'),
            ("custom scheme", '<a href="deskcat-custom:resource">custom</a>'),
        )
        for name, markup in cases:
            with self.subTest(case=name):
                self.check_index(
                    f'<html><body><h1 id="existing">Existing</h1>{markup}</body></html>',
                    False,
                    "Unsafe URL scheme",
                )

    def test_allowlisted_external_schemes_are_accepted(self):
        """外部URLのallowlist側も固定し、unsafe scheme追加時に正常系を巻き込まない。"""
        self.check_index(
            '<html><body><h1 id="existing">Existing</h1>'
            '<a href="//example.com/path">relative</a>'
            '<a href="http://example.com/">http</a>'
            '<a href="https://example.com/">https</a>'
            '<a href="mailto:example@example.com">mail</a>'
            '<a href="tel:+10000000000">tel</a>'
            '<a href="h&#x0a;ttps://example.com/">normalized</a></body></html>',
            True,
            "BROKEN_LINKS=0",
        )

    def test_non_url_unicode_whitespace_is_preserved(self):
        """URL parserはNBSPをASCII空白として捨てない。

        先頭NBSPを消して既存local pathへ読み替えないことを回帰させる。
        """
        self.check_index(
            '<html><body><h1 id="existing">Existing</h1>'
            f'<a href="{NON_URL_WHITESPACE}/deskcat/docs/architecture/">invalid</a>'
            "</body></html>",
            False,
            "Broken local link",
        )


class OutputValidatorLinkResolutionTests(OutputValidatorTestCase):
    def test_percent_encoded_markdown_extension_is_rejected(self):
        """拡張子のpercent-encodingでunconverted Markdown禁止を迂回させない。

        targetを実在させ、単なるmissing-file errorとの取り違えも防ぐ。
        """
        asset = self.track(os.path.join(_state["extra_root"], "source.md"))
        _write(asset, "# Source")
        self.check_index(
            '<html><body><h1 id="existing">Existing</h1>'
            '<a href="/deskcat/extra/source%2Emd">encoded markdown</a></body></html>',
            False,
            "Unconverted Markdown link in index.html",
        )

    def test_shared_markdown_extension_is_rejected(self):
        asset = self.track(os.path.join(_state["extra_root"], "source.markdown"))
        _write(asset, "# Source")
        self.check_index(
            '<html><body><h1 id="existing">Existing</h1>'
            '<a href="/deskcat/extra/source.markdown">markdown source</a></body></html>',
            False,
            "Unconverted Markdown link in index.html",
        )

    def test_encoded_path_separator_resolves_consistently(self):
        """encoded backslashもdecode後にURL separatorへ正規化し、OS間で同じ結果にする。"""
        self.check_index(
            '<html><body><h1 id="existing">Existing</h1>'
            '<a href="/deskcat/docs%5Carchitecture/">encoded separator</a></body></html>',
            True,
            "BROKEN_LINKS=0",
        )

    def test_ambiguous_extensionless_link_is_rejected(self):
        """extensionless URLに`.html`とdirectory indexが同時に対応する場合、

        配列順で片方を選ばない。実serverの解決順に依存するため曖昧として停止する。
        """
        ambiguous_file = self.track(os.path.join(_state["extra_root"], "ambiguous.html"))
        _write(ambiguous_file, '<html><body><h1 id="file">File</h1></body></html>')
        ambiguous_directory = self.track(os.path.join(_state["extra_root"], "ambiguous"))
        _write(
            os.path.join(ambiguous_directory, "index.html"),
            '<html><body><h1 id="directory">Directory</h1></body></html>',
        )
        self.check_index(
            '<html><body><h1 id="existing">Existing</h1>'
            '<a href="/deskcat/extra/ambiguous">ambiguous</a></body></html>',
            False,
            "Ambiguous local link",
        )

    def test_directory_link_without_index_is_rejected(self):
        """directoryの存在だけではlinkを解決済みにしない。"""
        empty_directory = self.track(os.path.join(_state["extra_root"], "empty-directory"))
        os.makedirs(empty_directory)
        self.check_index(
            '<html><body><h1 id="existing">Existing</h1>'
            '<a href="/deskcat/extra/empty-directory/">missing index</a></body></html>',
            False,
            "Broken local link",
        )

    def test_directory_link_with_index_is_accepted(self):
        self.check_index(
            '<html><body><h1 id="existing">Existing</h1>'
            '<a href="/deskcat/docs/architecture/">directory index</a></body></html>',
            True,
            "BROKEN_LINKS=0",
        )

    def test_case_mismatched_generated_path_is_rejected(self):
        """Windowsのcase-insensitive filesystemでも、Linux Pagesで404になる

        case違いを通さない。実fileは`docs/architecture/index.html`である。
        """
        self.check_index(
            '<html><body><h1 id="existing">Existing</h1>'
            '<a href="/deskcat/docs/Architecture/">wrong case</a></body></html>',
            False,
            "Broken local link",
        )

    def test_root_absolute_link_outside_base_path_is_rejected(self):
        """project Pagesのrootは`/deskcat`である。

        `/docs/...`をsite rootへ読み替えると、fileが存在するfixtureでは
        実際の公開URLが404でもvalidatorが通ってしまう。
        """
        self.check_index(
            '<html><body><h1 id="existing">Existing</h1>'
            '<a href="/docs/architecture/">outside base path</a></body></html>',
            False,
            "Root-absolute link is outside Pages base path",
        )

    def test_fragment_on_existing_non_html_asset_is_accepted(self):
        sprite = self.track(os.path.join(_state["extra_root"], "sprite.svg"))
        _write(sprite, '<svg xmlns="http://www.w3.org/2000/svg"><symbol id="cat" /></svg>')
        self.check_index(
            '<html><body><h1 id="existing">Existing</h1>'
            '<a href="/deskcat/extra/sprite.svg#cat">valid asset fragment</a></body></html>',
            True,
            "BROKEN_LINKS=0",
        )

    def test_fragment_on_missing_non_html_asset_is_rejected(self):
        self.check_index(
            '<html><body><h1 id="existing">Existing</h1>'
            '<a href="/deskcat/extra/missing.svg#cat">missing asset</a></body></html>',
            False,
            "Broken local link",
        )

    def test_broken_fragment_containing_a_question_mark(self):
        """URIのfragmentでは`?`もdataである。

        query delimiterとして先にsplitすると、fragmentが空へ退化して
        壊れたanchorを無検査で通す。
        """
        self.check_index(
            '<html><body><h1 id="existing">Existing</h1>'
            '<a href="#missing?part">broken fragment</a></body></html>',
            False,
            "Broken anchor in generated site",
        )

    def test_valid_fragment_containing_a_question_mark(self):
        self.check_index(
            '<html><body><h1 id="existing?part">Existing</h1>'
            '<a href="#existing?part">valid fragment</a></body></html>',
            True,
            "BROKEN_LINKS=0",
        )

    def test_valid_same_page_anchor(self):
        self.check_index(
            '<html><body><h1 id="existing">Existing</h1>'
            '<a href="#existing">valid</a></body></html>',
            True,
            "BROKEN_LINKS=0",
        )


class OutputValidatorBaseUrlTests(OutputValidatorTestCase):
    """base pathはvalidator内の定数ではなくPages configの正本から読む。"""

    NON_YAML_WHITESPACE = NON_URL_WHITESPACE

    def setUp(self):
        super().setUp()
        self.config_path = self.track(
            os.path.join(_state["temporary_root"], "alternate-pages-config.yml")
        )

    def check_config(self, config, html, should_succeed, expected, forbidden=""):
        _write(self.config_path, config)
        _write(_state["output_index"], html)
        run = run_validator(
            VALIDATE_OUTPUT,
            [
                "--site-root",
                _state["site_root"],
                "--pages-config-path",
                self.config_path,
            ],
        )
        self.assert_outcome(run, should_succeed, expected, forbidden)

    def test_base_path_comes_from_config(self):
        self.check_config(
            "baseurl: /alternate",
            '<html><body><h1 id="existing">Existing</h1>'
            '<a href="/alternate/docs/architecture/">alternate base</a></body></html>',
            True,
            "BROKEN_LINKS=0",
        )

    def test_differently_cased_yaml_keys_are_ignored(self):
        self.check_config(
            "baseurl: /alternate\nBaseUrl: /ignored",
            '<html><body><h1 id="existing">Existing</h1>'
            '<a href="/alternate/docs/architecture/">alternate base</a></body></html>',
            True,
            "BROKEN_LINKS=0",
        )

    def test_accepted_baseurl_forms(self):
        deskcat_link = (
            '<html><body><h1 id="existing">Existing</h1>'
            '<a href="/deskcat/docs/architecture/">base link</a></body></html>'
        )
        root_link = (
            '<html><body><h1 id="existing">Existing</h1>'
            '<a href="/docs/architecture/">root Pages</a></body></html>'
        )
        cases = (
            ("trailing slash", "baseurl: /deskcat/", deskcat_link),
            ("unquoted with comment", "baseurl: /deskcat # project Pages", deskcat_link),
            ("double quoted with comment", 'baseurl: "/deskcat" # project Pages', deskcat_link),
            ("single quoted with comment", "baseurl: '/deskcat' # project Pages", deskcat_link),
            ("empty for root Pages", 'baseurl: ""', root_link),
            ("empty before a comment", "baseurl: # root Pages", root_link),
            # colonの後ろが空のkeyは、YAMLではnull＝空文字列であり、Jekyllも
            # root Pagesとして扱う。PowerShell実装はここでparameter binding error
            # になっていたが、それは意図した拒否ではなく事故である。
            # Python実装は`baseurl: ""`と同じく受理する。
            ("bare key with no value", "baseurl:", root_link),
        )
        for name, config, html in cases:
            with self.subTest(case=name):
                self.check_config(config, html, True, "BROKEN_LINKS=0")

    def test_rejected_baseurl_forms(self):
        cases = (
            # `#`の直前がASCII空白でなければYAML commentは始まらない。
            (
                "non-YAML whitespace is not a comment boundary",
                f"baseurl: /deskcat{self.NON_YAML_WHITESPACE}#not-a-comment",
                "contains an unsafe baseurl",
            ),
            (
                "non-YAML whitespace after the key is not consumed",
                f"baseurl:{self.NON_YAML_WHITESPACE}/deskcat",
                "must define exactly one baseurl",
            ),
            (
                "value separation is required",
                "baseurl:/deskcat",
                "must define exactly one baseurl",
            ),
            (
                "Unicode whitespace inside the value",
                f"baseurl: /deskcat{self.NON_YAML_WHITESPACE}",
                "contains an unsafe baseurl",
            ),
            (
                "a hash inside a quoted value is preserved",
                'baseurl: "/deskcat#fragment" # project Pages',
                "contains an unsafe baseurl",
            ),
            (
                "relative baseurl",
                "baseurl: relative/path",
                "contains an unsafe baseurl",
            ),
            (
                "duplicate top-level keys",
                "baseurl: /first\nbaseurl: /second",
                "must define exactly one baseurl",
            ),
            (
                "no top-level baseurl",
                "site:\n  baseurl: /nested",
                "must define exactly one baseurl",
            ),
        )
        for name, config, expected in cases:
            with self.subTest(case=name):
                self.check_config(config, VALID_PAGE, False, expected)

    def test_unsafe_baseurl_value_is_not_exposed(self):
        secret = "ghp_" + "d" * 24
        self.check_config(
            f"baseurl: {secret}",
            VALID_PAGE,
            False,
            "contains an unsafe baseurl",
            forbidden=secret,
        )


class OutputValidatorReparseTests(OutputValidatorTestCase):
    """reparse-point配下の実体を、_site内の通常fileとして受理しない。

    Windowsでは権限不要のjunction、他環境ではsymbolic linkを使う。
    作成できない環境ではskipし、実行していないものを成功として数えない。
    """

    def _create_link(self, link_path, target):
        try:
            if os.name == "nt":
                subprocess.run(
                    ["cmd", "/c", "mklink", "/J", link_path, target],
                    capture_output=True,
                    check=True,
                )
            else:
                os.symlink(target, link_path)
        except (OSError, subprocess.CalledProcessError):
            if os.path.exists(link_path) or guards.is_reparse_point(link_path):
                os.remove(link_path)
            self.skipTest("link creation is not permitted")
        self.track(link_path)

    def test_reparse_point_directory_is_rejected(self):
        target = self.track(os.path.join(_state["temporary_root"], "reparse-target"))
        _write(
            os.path.join(target, "outside.html"),
            '<html><body><h1 id="outside">Outside</h1></body></html>',
        )
        link_path = os.path.join(_state["extra_root"], "reparse-directory")
        self._create_link(link_path, target)
        self.check_index(
            '<html><body><h1 id="existing">Existing</h1>'
            '<a href="/deskcat/extra/reparse-directory/outside.html">reparse target</a>'
            "</body></html>",
            False,
            "Symbolic or reparse-point output is not allowed: extra/reparse-directory",
        )

    def test_reparse_point_site_root_is_rejected(self):
        alias = os.path.join(_state["temporary_root"], "site-root-alias")
        self._create_link(alias, _state["site_root"])
        run = run_validator(VALIDATE_OUTPUT, ["--site-root", alias])
        self.assert_outcome(
            run,
            False,
            expected_message="Symbolic or reparse-point output is not allowed at root: .",
        )


class FixtureCleanupTests(unittest.TestCase):
    """後始末が read-only file を残さないこと。

    このharnessはgit repositoryをfixtureとして作る。Gitのobjectはread-onlyで
    作られるため、素の`shutil.rmtree`では消えず、`ignore_errors=True`と組み合わせると
    失敗が握りつぶされて一時directoryへ溜まり続ける。実際に1実行あたり3件溜めていた。
    """

    def test_remove_tree_deletes_read_only_files(self):
        root = os.path.join(_state["temporary_root"], f"cleanup-probe-{uuid.uuid4().hex}")
        nested = os.path.join(root, "objects", "ab")
        os.makedirs(nested)
        target = os.path.join(nested, "read-only")
        _write(target, "x")
        os.chmod(target, stat.S_IREAD)
        self.assertFalse(os.access(target, os.W_OK), "fixture precondition: read-only")
        if os.name == "nt":
            # positive control。Windowsではread-only属性そのものが削除を拒む。
            # POSIXではunlinkの可否は親directoryの権限で決まり、read-onlyのfileでも
            # 消せるため、この対照は成立しない。
            shutil.rmtree(root, ignore_errors=True)
            self.assertTrue(
                os.path.exists(root), "read-only fileが素のrmtreeで消えてしまった"
            )
        _remove_fixture(root)
        self.assertFalse(os.path.exists(root))

    def test_git_fixture_directories_are_removed(self):
        root = os.path.join(_state["temporary_root"], f"cleanup-git-{uuid.uuid4().hex}")
        os.makedirs(root)
        _write(os.path.join(root, "a.md"), "# A\n")
        _git(root, "init", "--quiet")
        _git(root, "add", "--all")
        _remove_fixture(root)
        self.assertFalse(os.path.exists(root), "git fixtureが消し残された")


class ScanScopeTests(unittest.TestCase):
    """診断の`UNSCANNED=`と、実際のscan範囲が食い違わないこと。

    2箇所で条件を書くと、片方だけ変えたときに「scanされている」と読める診断が出て、
    実際にはされていない状態になる。`is_scanned_for_secrets`へ集約してある。
    """

    def test_scan_scope_matches_the_declared_extensions(self):
        for name in ("a.html", "a.css", "a.md", "a.svg", "a.txt", "a.yml", "LICENSE"):
            with self.subTest(name=name):
                self.assertTrue(validate_pages_output.is_scanned_for_secrets(name))
        for name in ("a.xml", "a.json", "a.js", "NOTICE", "a.png", "a.ico"):
            with self.subTest(name=name):
                self.assertFalse(validate_pages_output.is_scanned_for_secrets(name))

    def test_license_is_matched_by_file_name_not_extension(self):
        self.assertTrue(validate_pages_output.is_scanned_for_secrets("docs/LICENSE"))
        self.assertFalse(validate_pages_output.is_scanned_for_secrets("docs/LICENSE.bin"))
        # 大文字小文字は区別する。`license`は対象にしない。
        self.assertFalse(validate_pages_output.is_scanned_for_secrets("docs/license"))


class SharedHelperTests(unittest.TestCase):
    """publish_guardsのhelperを直接検査する。

    validator越しでは、helperが壊れても他の判定が拾って成功し続ける組合せがある。
    """

    def test_markdown_link_parser_returns_deterministic_targets(self):
        """link抽出がruntimeや改行形式に依存せず、validatorの対象だけを返すこと。"""
        fixture = (
            "[local](docs/page.md)\r\n"
            "![image](assets/image.png)\n"
            "[same](#section)\n"
            '[multi\nline](docs/other.md "title")\n'
            "[external](https://example.com)\n"
            "[invalid](docs/invalid.md trailing)"
        )
        self.assertEqual(
            list(guards.markdown_link_targets(fixture)),
            ["docs/page.md", "#section", "docs/other.md", "https://example.com"],
        )

    def test_fence_helper_is_shared_by_heading_and_link_scans(self):
        """fence判定が見出し走査とlink走査で共通であること。

        別々に持つと、片方だけを変えたときにanchor集合とlink集合が別の行を見る。
        """
        fixture = (
            "# Heading\n"
            "```bash\n"
            "# not a heading\n"
            "[not a link](fenced.md)\n"
            "```\n"
            "[real](real.md)\n"
            "~~~\n"
            "# also fenced\n"
            "~~~\n"
            "## Tail"
        )
        outside = "\n".join(guards.markdown_outside_fences(fixture))
        self.assertEqual(outside, "# Heading\n[real](real.md)\n## Tail")
        self.assertEqual(list(guards.markdown_link_targets(outside)), ["real.md"])

    def test_publication_path_helper_normalizes_and_rejects(self):
        self.assertEqual(
            guards.path_relative_to_root(
                os.path.join(REPOSITORY_ROOT, "scripts", "test_link_validators.py"),
                REPOSITORY_ROOT,
            ),
            "scripts/test_link_validators.py",
        )
        outside = os.path.join(
            os.path.dirname(REPOSITORY_ROOT), "deskcat-outside-probe.txt"
        )
        with self.assertRaises(guards.ValidationError) as context:
            guards.path_relative_to_root(outside, REPOSITORY_ROOT)
        self.assertEqual(
            str(context.exception), "Cannot format a path outside the publication root."
        )

    def test_tracked_symlink_helper_uses_the_git_index_mode(self):
        """file属性ではcheckout環境で結果が変わり、走査対象と公開物が環境ごとに変わる。

        `CLAUDE.md`はindex上mode 120000である。
        """
        symlinks = guards.get_tracked_symlinks(REPOSITORY_ROOT)
        self.assertIsInstance(symlinks, set)
        self.assertIn("CLAUDE.md", symlinks)
        self.assertNotIn("AGENTS.md", symlinks)

    def test_tracked_file_helper_returns_expected_sets(self):
        cases = (
            {"name": "one match", "pathspec": "LICENSE", "exact": "LICENSE", "minimum": 1},
            {"name": "many matches", "pathspec": "scripts", "root": "scripts", "minimum": 2},
            {"name": "no match", "pathspec": "no-such-path-xyz", "count": 0},
        )
        for case in cases:
            with self.subTest(case=case["name"]):
                tracked = guards.get_tracked_files(REPOSITORY_ROOT, case["pathspec"])
                self.assertIsInstance(tracked, set)
                if "count" in case:
                    self.assertEqual(len(tracked), case["count"])
                if "minimum" in case:
                    self.assertGreaterEqual(len(tracked), case["minimum"])
                if "exact" in case:
                    self.assertEqual(tracked, {case["exact"]})
                if "root" in case:
                    prefix = case["root"].rstrip("/") + "/"
                    unexpected = [
                        entry
                        for entry in tracked
                        if entry != case["root"] and not entry.startswith(prefix)
                    ]
                    self.assertEqual(unexpected, [])

    def test_tracked_file_helper_matches_exactly(self):
        """部分一致へ退化していないこと。`LICENSE`は`LIC`を含むが要素ではない。"""
        tracked = guards.get_tracked_files(REPOSITORY_ROOT, "LICENSE")
        self.assertNotIn("LIC", tracked)
        self.assertIn("LICENSE", tracked)


class NonAsciiQuotingTests(unittest.TestCase):
    """非ASCIIのpathが、escape sequenceではなく実pathとして返ること。

    `git ls-files`は既定（`core.quotePath=true`）で非ASCIIをdouble quoteと
    octal escapeにする。escapeされたまま集合へ入ると、実pathと一致せず、
    追跡済みfileをsymlink除外や公開判定で取りこぼす。
    両helperが同じquoting設定で読むことも併せて確認する。
    """

    NON_ASCII_NAME = "日本語ファイル.md"
    NON_ASCII_SYMLINK_NAME = "日本語リンク.md"

    def setUp(self):
        parent = guards.full_path(tempfile.gettempdir())
        self.parent = parent
        self.root = os.path.join(parent, f"deskcat-quote-{uuid.uuid4().hex}")
        os.makedirs(self.root)
        # 作った直後に登録する。`unittest`は`setUp`が例外を投げたときや
        # `skipTest`したときに`tearDown`を呼ばない。下の`_git`はどれも失敗しうるし、
        # quotingのpositive controlはskipへ抜ける。`tearDown`に任せると、
        # そのどの経路でもfixtureが一時directoryへ残る。
        self.addCleanup(self._remove_fixture_root)
        _write(os.path.join(self.root, self.NON_ASCII_NAME), "# 見出し")

        _git(self.root, "init", "--quiet")
        # global設定に依存させない。helperの`core.quotePath=false`を外すと、
        # local設定が有効になって非ASCII pathがescapeされ、必ずtestが失敗する。
        _git(self.root, "config", "--local", "core.quotePath", "true")
        _git(self.root, "add", "--all")

        # helperが無効化すべきGit quotingが、fixtureで実際に発生していることを先に確認する。
        # このpositive controlがなければ、helperから`core.quotePath=false`が消えても
        # testが何も検証せず成功しうる。
        raw_listing = _git(self.root, "ls-files").splitlines()
        if len(raw_listing) != 1 or not re.match(r'^".*\\[0-7]{3}.*"$', raw_listing[0]):
            self.skipTest(
                "git ls-files did not quote the non-ASCII path in this environment"
            )

        # OSのsymlink作成権限に依存させず、Git indexへmode 120000を直接登録する。
        # 通常file列挙とは別の`ls-files -s`経路でも実pathを返すことを確認する。
        blob = _git(self.root, "rev-parse", "--verify", f":{self.NON_ASCII_NAME}").strip()
        _git(
            self.root,
            "update-index",
            "--add",
            "--cacheinfo",
            f"120000,{blob},{self.NON_ASCII_SYMLINK_NAME}",
        )

    def _remove_fixture_root(self):
        # 一時directoryの直下にある、自分で作った名前のものだけを消す。
        if guards.path_within_root(self.root, self.parent) and os.path.basename(
            self.root
        ).startswith("deskcat-quote-"):
            _remove_fixture(self.root)

    def test_tracked_file_helper_returns_the_exact_unquoted_set(self):
        tracked = guards.get_tracked_files(self.root, ".")
        self.assertEqual(
            tracked, {self.NON_ASCII_NAME, self.NON_ASCII_SYMLINK_NAME}
        )

    def test_tracked_symlink_helper_returns_only_the_symlink(self):
        symlinks = guards.get_tracked_symlinks(self.root)
        self.assertEqual(symlinks, {self.NON_ASCII_SYMLINK_NAME})


if __name__ == "__main__":
    unittest.main(verbosity=2)
