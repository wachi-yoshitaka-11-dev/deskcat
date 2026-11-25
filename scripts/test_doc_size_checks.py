#!/usr/bin/env python3
"""`check_doc_size.py`と`check_condense_loss.py`のtest（#806）。

単体testは関数を直接呼ぶ。子processのtestはCLIを起動し、exit codeと診断まで検査する。
サイズの検査は一時directoryへ書いたfileを、消えた値の一覧はfixture repositoryへ実際にcommitして
起動する。ページ数はChromeが無い端末でも検査できるよう、小さなPDFのfixtureで数える関数を試す。
"""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPTS_ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(SCRIPTS_ROOT))

import check_condense_loss as loss  # noqa: E402
import check_doc_size as size  # noqa: E402

SIZE = str(SCRIPTS_ROOT / "check_doc_size.py")
LOSS = str(SCRIPTS_ROOT / "check_condense_loss.py")
IDENTITY = (
    "-c", "user.name=test",
    "-c", "user.email=test@example.invalid",
    "-c", "commit.gpgsign=false",
)


def _git(root, *arguments):
    result = subprocess.run(
        ["git", "-C", str(root), *IDENTITY, *arguments],
        capture_output=True, text=True, encoding="utf-8", errors="replace",
    )
    if result.returncode != 0:
        raise RuntimeError(f"git {arguments} failed: {result.stdout}{result.stderr}")
    return result.stdout.strip()


def _write(root, relative, text):
    path = Path(root) / relative
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def _run(script, root, *arguments):
    return subprocess.run(
        [sys.executable, script, "--repository-root", str(root), *arguments],
        capture_output=True, text=True, encoding="utf-8", errors="replace",
        timeout=60,
    )


def _pdf(pages, count=None):
    """ページ数だけが意味を持つ最小のPDF風のbytes（Chromeの出力と同じ`/Type /Page`の形）。"""
    body = b"".join(b"%d 0 obj\n<</Type /Page /Parent 1 0 R>>\nendobj\n" % (i + 2)
                    for i in range(pages))
    root = b"1 0 obj\n<</Type /Pages /Count %d>>\nendobj\n" % (pages if count is None else count)
    return b"%PDF-1.4\n" + root + body + b"%%EOF\n"


class ClassifyTests(unittest.TestCase):
    def test_first_matching_row_decides(self):
        self.assertEqual(size.classify("docs/hardware/a.md"), "design")
        self.assertEqual(size.classify("docs/hardware/a-history.md"), "exempt")
        self.assertEqual(size.classify("docs/hardware/README.md"), "readme")
        self.assertEqual(size.classify("docs/decisions/0001-x.md"), "adr")
        self.assertEqual(size.classify("docs/governance/a.md"), "governance")
        self.assertEqual(size.classify("docs/runbooks/a.md"), "procedure")
        self.assertEqual(size.classify("docs/toolchains/a.md"), "procedure")
        self.assertEqual(size.classify("CONTRIBUTING.md"), "contributing")
        self.assertEqual(size.classify("AGENTS.md"), "entry")
        self.assertEqual(size.classify("hardware/universal-board/plan.md"), "design")

    def test_unknown_markdown_is_none(self):
        self.assertIsNone(size.classify("notes/new.md"))

    def test_every_kind_in_the_table_has_a_limit_or_is_exempt(self):
        for _, kind in size.KINDS:
            self.assertTrue(kind in size.LIMITS or kind == "exempt", kind)


class PdfPageTests(unittest.TestCase):
    def test_counts_page_objects_not_the_pages_root(self):
        self.assertEqual(size.count_pdf_pages(_pdf(1)), 1)
        self.assertEqual(size.count_pdf_pages(_pdf(5)), 5)

    def test_disagreement_with_count_is_an_error(self):
        with self.assertRaises(ValueError):
            size.count_pdf_pages(_pdf(3, count=4))

    def test_no_page_is_an_error(self):
        with self.assertRaises(ValueError):
            size.count_pdf_pages(b"%PDF-1.4\n")


class PrintableHtmlTests(unittest.TestCase):
    def test_site_absolute_paths_become_file_urls_and_a4_is_set(self):
        out = size.printable_html(
            '<html><head><link href="/deskcat/assets/a.css"></head><body></body></html>',
            "/tmp/site")
        self.assertIn('href="file:///tmp/site/assets/a.css"', out)
        self.assertIn("@page { size: A4 }", out)
        self.assertLess(out.index("@page"), out.index("</head>"))

    def test_html_without_head_still_gets_the_page_rule(self):
        self.assertIn("@page { size: A4 }", size.printable_html("<p>x</p>", "/tmp/site"))


class StageMeasureSourceTests(unittest.TestCase):
    def test_only_design_documents_under_hardware_are_staged_with_the_portal(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "repo"
            pages = Path(temporary) / "pages"
            _write(root, "hardware/a/plan.md", "x")
            _write(root, "hardware/a/README.md", "readme")
            _write(root, "docs/hardware/b.md", "y")
            _write(pages, "_config.yml", "title: t\n")
            _write(pages, "_layouts/default.html", "{{ content }}")
            _write(pages, "docs/other.md", "z")
            staged = size.stage_measure_source(
                root, pages, Path(temporary) / "out",
                ["hardware/a/plan.md", "hardware/a/README.md", "docs/hardware/b.md"])
            self.assertEqual(staged, ["hardware/a/plan.md"])
            out = Path(temporary) / "out"
            self.assertTrue((out / "hardware/a/plan.md").is_file())
            self.assertTrue((out / "_config.yml").is_file())
            self.assertTrue((out / "_layouts/default.html").is_file())
            self.assertFalse((out / "docs").exists())


class SitePathTests(unittest.TestCase):
    def test_published_paths(self):
        self.assertEqual(size.site_path("docs/hardware/a.md"), "docs/hardware/a.html")
        self.assertEqual(size.site_path("docs/hardware/README.md"), "docs/hardware/index.html")
        self.assertEqual(size.site_path("CONTRIBUTING.md"), "CONTRIBUTING.html")

    def test_hardware_plans_have_a_path_in_the_measure_build(self):
        self.assertEqual(size.site_path("hardware/universal-board/plan.md"),
                         "hardware/universal-board/plan.html")


class MeasureTests(unittest.TestCase):
    def test_characters_not_bytes(self):
        values, _ = size.measure({"docs/governance/a.md": "あ" * 10})
        self.assertEqual(values["docs/governance/a.md"], 10)

    def test_entry_lines_are_summed_over_agents_and_claude(self):
        values, _ = size.measure({"AGENTS.md": "a\nb\n", "CLAUDE.md": "c\n"})
        self.assertEqual(values[size.ENTRY_KEY], 3)

    def test_unclassified_is_reported(self):
        _, unclassified = size.measure({"notes/x.md": "x"})
        self.assertEqual(unclassified, ["notes/x.md"])

    def test_pages_use_the_counter_and_total_design_pages(self):
        files = {"docs/hardware/a.md": "", "docs/protocol/b.md": "", "docs/decisions/c.md": ""}
        values, _ = size.measure(files, lambda r: 2)
        self.assertEqual(values[size.DESIGN_TOTAL_KEY], 4)
        self.assertEqual(values["docs/decisions/c.md"], 2)

    def test_hardware_design_docs_are_paged_through_the_measure_build(self):
        seen = []
        values, _ = size.measure({"hardware/x/plan.md": ""}, lambda r: seen.append(r) or 2)
        self.assertEqual(seen, ["hardware/x/plan.md"])
        self.assertEqual(values["hardware/x/plan.md"], 2)
        self.assertTrue(size.uses_measure_build("hardware/x/plan.md"))
        self.assertFalse(size.uses_measure_build("docs/hardware/a.md"))

    def test_pages_are_skipped_without_a_counter(self):
        values, _ = size.measure({"docs/hardware/a.md": ""})
        self.assertNotIn("docs/hardware/a.md", values)
        self.assertNotIn(size.DESIGN_TOTAL_KEY, values)


class CheckTests(unittest.TestCase):
    KINDS = {"docs/governance/a.md": "governance", "docs/hardware/a.md": "design"}

    def test_over_limit_fails_and_within_passes(self):
        problems = size.check({"docs/governance/a.md": 8105}, self.KINDS, {})
        self.assertEqual(len(problems), 1)
        problems = size.check({"docs/governance/a.md": 8104}, self.KINDS, {})
        self.assertEqual(problems, [])

    def test_baseline_allows_the_recorded_value_but_not_growth(self):
        base = {"docs/governance/a.md": 9000}
        self.assertEqual(size.check({"docs/governance/a.md": 9000}, self.KINDS, base), [])
        self.assertEqual(len(size.check({"docs/governance/a.md": 9001}, self.KINDS, base)), 1)

    def test_shrinking_without_lowering_the_baseline_fails(self):
        problems = size.check(
            {"docs/governance/a.md": 8500}, self.KINDS, {"docs/governance/a.md": 9000})
        self.assertEqual(len(problems), 1)
        self.assertIn("lower the baseline to 8500", problems[0])

    def test_baseline_within_limit_or_without_target_fails(self):
        self.assertEqual(len(size.check(
            {"docs/governance/a.md": 100}, self.KINDS, {"docs/governance/a.md": 100})), 1)
        self.assertEqual(len(size.check({}, self.KINDS, {"docs/governance/gone.md": 9000})), 1)

    def test_page_baseline_is_ignored_when_pages_are_not_measured(self):
        base = {"docs/hardware/a.md": 5, size.DESIGN_TOTAL_KEY: 25}
        problems = size.check({}, self.KINDS, base, pages_measured=False)
        self.assertEqual(problems, [])

    def test_entry_lines_and_design_total(self):
        self.assertEqual(len(size.check({size.ENTRY_KEY: 200}, {}, {})), 1)
        self.assertEqual(size.check({size.ENTRY_KEY: 199}, {}, {}), [])
        self.assertEqual(len(size.check({size.DESIGN_TOTAL_KEY: 21}, {}, {})), 1)

    def test_unclassified_fails(self):
        problems = size.check({}, {}, {}, unclassified=["notes/x.md"])
        self.assertIn("UNCLASSIFIED", problems[0])

    def test_make_baseline_keeps_only_excess(self):
        values = {"docs/governance/a.md": 9000, "docs/hardware/a.md": 2}
        self.assertEqual(size.make_baseline(values, self.KINDS), {"docs/governance/a.md": 9000})


class RatchetTests(unittest.TestCase):
    def test_new_key_and_increase_fail_decrease_and_same_pass(self):
        base = {"a.md": 10, "b.md": 10, "c.md": 10}
        problems = size.ratchet({"a.md": 11, "b.md": 9, "c.md": 10, "d.md": 5}, base)
        self.assertEqual(len(problems), 2)
        self.assertTrue(any("a.md" in p for p in problems))
        self.assertTrue(any("d.md" in p for p in problems))

    def test_no_base_baseline_means_nothing_to_compare(self):
        self.assertEqual(size.ratchet({"a.md": 11}, None), [])


class SizeCommandTests(unittest.TestCase):
    def _repo(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        _git(root, "init", "-q", "-b", "main")
        return root

    def test_over_limit_exits_1(self):
        root = self._repo()
        _write(root, "docs/governance/a.md", "x" * 8105)
        _git(root, "add", "-A")
        result = _run(SIZE, root)
        self.assertEqual(result.returncode, 1)
        self.assertIn("DOC-TOO-LARGE docs/governance/a.md", result.stdout)

    def test_within_baseline_passes(self):
        root = self._repo()
        _write(root, "docs/governance/a.md", "x" * 9000)
        _write(root, "scripts/doc_size_baseline.json", json.dumps({"docs/governance/a.md": 9000}))
        _git(root, "add", "-A")
        self.assertEqual(_run(SIZE, root).returncode, 0)

    def test_unclassified_markdown_exits_1(self):
        root = self._repo()
        _write(root, "notes/new.md", "x\n")
        _git(root, "add", "-A")
        result = _run(SIZE, root)
        self.assertEqual(result.returncode, 1)
        self.assertIn("UNCLASSIFIED", result.stdout)

    def test_print_baseline_lists_only_excess(self):
        root = self._repo()
        _write(root, "docs/governance/a.md", "x" * 9000)
        _write(root, "docs/governance/b.md", "ok\n")
        _git(root, "add", "-A")
        result = _run(SIZE, root, "--print-baseline")
        self.assertEqual(json.loads(result.stdout), {"docs/governance/a.md": 9000})

    def test_this_repositorys_baseline_matches_the_documents(self):
        """baselineが実測とずれると、CIが落ちる。文書を直したあとに作り直し忘れるのを止める。"""
        result = subprocess.run(
            [sys.executable, SIZE], capture_output=True, text=True, encoding="utf-8")
        self.assertEqual(result.returncode, 0, result.stdout)

    def _base_repo(self, base_baseline, current_baseline):
        root = self._repo()
        _write(root, "docs/governance/a.md", "x" * 9000)
        _write(root, "scripts/doc_size_baseline.json", json.dumps(base_baseline))
        _git(root, "add", "-A")
        _git(root, "commit", "-q", "-m", "base")
        _git(root, "branch", "base")
        _write(root, "scripts/doc_size_baseline.json", json.dumps(current_baseline))
        _git(root, "add", "-A")
        _git(root, "commit", "-q", "--allow-empty", "-m", "change")
        return root

    def test_base_flag_rejects_a_raised_baseline(self):
        root = self._base_repo({"docs/governance/a.md": 9000}, {"docs/governance/a.md": 9500})
        result = _run(SIZE, root, "--base", "base")
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertIn("baseline rose", result.stdout)

    def test_base_flag_accepts_an_unchanged_baseline(self):
        root = self._base_repo({"docs/governance/a.md": 9000}, {"docs/governance/a.md": 9000})
        self.assertEqual(_run(SIZE, root, "--base", "base").returncode, 0)

    def test_unknown_base_is_an_error(self):
        root = self._base_repo({"docs/governance/a.md": 9000}, {"docs/governance/a.md": 9000})
        self.assertEqual(_run(SIZE, root, "--base", "no-such-ref").returncode, 2)

    def test_markdown_extension_files_are_classified_too(self):
        root = self._repo()
        _write(root, "notes/new.markdown", "x\n")
        _git(root, "add", "-A")
        self.assertIn("UNCLASSIFIED", _run(SIZE, root).stdout)

    def test_no_markdown_is_an_error(self):
        self.assertEqual(_run(SIZE, self._repo()).returncode, 2)

    def test_a_design_doc_without_html_is_an_error_not_a_pass(self):
        root = self._repo()
        _write(root, "docs/hardware/a.md", "x\n")
        _git(root, "add", "-A")
        (root / "_site").mkdir()
        result = _run(SIZE, root, "--site-root", str(root / "_site"), "--chrome", "true")
        self.assertEqual(result.returncode, 2, result.stdout)

    def test_stage_mode_fails_when_nothing_is_staged(self):
        root = self._repo()
        _write(root, "docs/hardware/a.md", "x\n")
        _git(root, "add", "-A")
        result = _run(SIZE, root, "--stage-measure-source", str(root / "out"),
                      "--pages-source", str(root / "none"))
        self.assertEqual(result.returncode, 2)

    def test_missing_chrome_is_an_error_not_a_pass(self):
        root = self._repo()
        _write(root, "docs/hardware/a.md", "x\n")
        _git(root, "add", "-A")
        site = root / "_site" / "docs" / "hardware"
        site.mkdir(parents=True)
        (site / "a.html").write_text("<html><head></head><body>x</body></html>")
        result = _run(SIZE, root, "--site-root", str(root / "_site"), "--chrome", "no-such-chrome")
        self.assertEqual(result.returncode, 2, result.stdout)


class TokenTests(unittest.TestCase):
    def test_values_ids_issues_urls_and_codes_are_tokens(self):
        found = loss.tokens(
            "`GPIO21` HW-TBD-034 #805 0.896 A 3.3 V https://example.invalid/a?b=1 です"
        )
        for expected in ("`GPIO21`", "HW-TBD-034", "#805", "0.896 A", "3.3 V",
                         "https://example.invalid/a?b=1"):
            self.assertIn(expected, found)

    def test_units_used_by_the_safety_values_are_tokens(self):
        """正本の安全の値で実際に使う書き方（power-budget.md・hardware-bom.md）を拾う。"""
        cases = {
            "標準46 mΩ・最大60 mΩ": ("46 mΩ", "60 mΩ"),
            "5W定格に対し": ("5W",),
            "外径1.55 mm（AWG22相当）": ("1.55 mm", "AWG22"),
            "0.3054 mm²＝**0.3 sq**": ("0.3054 mm²", "0.3 sq"),
            "30 min. at": ("30 min",),
            "Rθja（88 ℃/W）、25 ℃": ("88 ℃/W", "25 ℃"),
            "最大0.5 mW": ("0.5 mW",),
            "測定は5 sと200 ms": ("5 s", "200 ms"),
        }
        for text, expected in cases.items():
            for token in expected:
                with self.subTest(text=text, token=token):
                    self.assertIn(token, loss.tokens(text))

    def test_units_missing_in_the_first_version_are_tokens(self):
        """正本の安全の値にある書き方（レビューで漏れが見つかったもの）。"""
        cases = {
            "Dead band width 1 us": ("1 us",),
            "IIH ≤ 1 µA、702 µA": ("1 µA", "702 µA"),
            "bus clock 6 MHz／10 MHz": ("6 MHz", "10 MHz"),
            "22µF と 100 nF と 10 pF": ("22µF", "100 nF", "10 pF"),
            "36 k–67 kΩ、20k": ("67 kΩ", "20k"),
            "1 MΩ": ("1 MΩ",),
        }
        for text, expected in cases.items():
            for token in expected:
                with self.subTest(text=text, token=token):
                    self.assertIn(token, loss.tokens(text))

    def test_lower_bound_of_a_range_and_the_minus_sign_are_tokens(self):
        found = loss.tokens("周期 19.97〜20.07 ms、パルス 1.56〜1.66 ms、36 k–67 kΩ、VGS=−10 V、-3 V")
        for token in ("19.97", "20.07 ms", "1.56", "1.66 ms", "36 k", "−10 V", "-3 V"):
            with self.subTest(token=token):
                self.assertIn(token, found)

    def test_a_range_lower_bound_change_is_visible(self):
        lost, added = loss.compare("パルス 1.56〜1.66 ms", "パルス 1.50〜1.66 ms")
        self.assertEqual((lost, added), (["1.56"], ["1.50"]))

    def test_a_flipped_sign_is_visible(self):
        lost, added = loss.compare("VGS=−10 V", "VGS=10 V")
        self.assertEqual((lost, added), (["−10 V"], ["10 V"]))

    def test_ids_next_to_japanese_text_keep_their_number(self):
        """`\\b`はCJK文字を語の文字と見なす。`HW-TBD-035の`が`HW-TBD`へ縮んではならない。"""
        found = loss.tokens("HW-TBD-035の台帳、EXP-028では、ADR-0027の決定、線AWG22")
        for token in ("HW-TBD-035", "EXP-028", "ADR-0027", "AWG22"):
            with self.subTest(token=token):
                self.assertIn(token, found)
        self.assertNotIn("HW-TBD", found)

    def test_a_removed_hw_tbd_number_is_lost_even_when_another_remains(self):
        lost, _ = loss.compare("HW-TBD-035の条件。HW-TBD-036の条件", "HW-TBD-036の条件")
        self.assertIn("HW-TBD-035", lost)
        self.assertNotIn("HW-TBD-036", lost)

    def test_second_review_findings_are_tokens(self):
        """巡2の指摘（桁区切り・VAC・素のGPIO番号・型番・全角チルダ）。正本の実際の書き方。"""
        cases = {
            "max 1,100 mA": ("1,100 mA",),
            "35 A @ 250 VAC": ("35 A", "250 VAC"),
            "GPIO12・GPIO0": ("GPIO12", "GPIO0"),
            "TowerPro SG90、2SJ334、AO3401A、秋月 M-12001": ("SG90", "2SJ334", "AO3401A", "M-12001"),
            "DC1.71～3.6V": ("1.71", "3.6V"),
            "I2Cは0x76、3V3 pin": ("0x76", "3V3"),
            "115200 baud、9600": ("115200", "115200 baud", "9600"),
            "29.80 GiB、16 bit、76.9 kSPS、3.9 mg/LSB、2 kV、31,994,216,448 B": (
                "29.80 GiB", "16 bit", "76.9 kSPS", "3.9 mg/LSB", "2 kV", "31,994,216,448 B"),
        }
        for text, expected in cases.items():
            for token in expected:
                with self.subTest(text=text, token=token):
                    self.assertIn(token, loss.tokens(text))

    def test_third_review_findings_are_tokens(self):
        """巡3の指摘（日本語に隣接した整数、`-3.3`の版、度・秒・count・m・hPa、単位なしの小数）。"""
        cases = {
            "トリップ時間が1000秒前後": ("1000", "1000秒"),
            "秋月 通販110972、通販113013": ("110972", "113013"),
            "UMW LD1117-3.3 と AMS1117-5.0": ("LD1117-3.3", "AMS1117-5.0"),
            "90度、既定15度、15 deg、7.3秒、既定10秒": ("90度", "15度", "15 deg", "7.3秒", "10秒"),
            "9 count 以下、4 count": ("9 count", "4 count"),
            "4.8vしか無い": ("4.8v",),
            "0.5 m、100 m、1 m": ("0.5 m", "100 m", "1 m"),
            "300〜1100 hPa、±1.0 hPa、0.18 Pa": ("300", "1100 hPa", "1.0 hPa", "0.18 Pa"),
            "0.073 以下、0.800 未満、4.80、4.92 を超えて": ("0.073", "0.800", "4.80", "4.92"),
            "0.75×VDD、0.7×VDDI、約20倍、0.8倍": ("0.75", "0.7", "20倍", "0.8倍"),
            "穴 (4,12) と (15,10)": ("(4,12)", "(15,10)"),
            "KiCad 8.0.9、§24.5.2、idf-hal 0.46.2": ("8.0.9", "24.5.2", "0.46.2"),
            "62,488,704 sector、Figure 31-365": ("62,488,704", "365"),
        }
        for text, expected in cases.items():
            for token in expected:
                with self.subTest(text=text, token=token):
                    self.assertIn(token, loss.tokens(text))

    def test_a_changed_hyphenated_part_number_version_is_visible(self):
        lost, added = loss.compare("U2 は LD1117-3.3", "U2 は LD1117-5.0")
        self.assertIn("LD1117-3.3", lost)
        self.assertIn("LD1117-5.0", added)

    def test_a_changed_breadboard_coordinate_is_visible(self):
        lost, added = loss.compare("穴 (4,12) へ", "穴 (4,13) へ")
        self.assertEqual((lost, added), (["(4,12)"], ["(4,13)"]))

    def test_no_partial_tokens_from_the_middle_of_a_value_or_an_id(self):
        found = loss.tokens("1,100 mA と DC1.71～3.6V と AO3401A と 3V3")
        for partial in ("100 mA", "6V", "DC1", "3401A", "3V", "LD1117-3"):
            with self.subTest(partial=partial):
                self.assertNotIn(partial, found)

    def test_a_thousands_digit_change_is_visible(self):
        lost, added = loss.compare("max 1,100 mA", "max 2,100 mA")
        self.assertIn("1,100 mA", lost)
        self.assertIn("2,100 mA", added)

    def test_years_and_dates_are_not_bare_number_tokens(self):
        self.assertEqual(loss.tokens("2026-11-25 に決めた。2026年"), set())

    def test_a_longer_unit_is_not_cut_to_a_shorter_one(self):
        found = loss.tokens("60 mΩ と 200 ms と 30 min")
        self.assertNotIn("60 Ω", found)
        self.assertNotIn("200 s", found)
        self.assertNotIn("30 m", found)

    def test_english_word_after_a_number_is_not_a_unit(self):
        self.assertEqual(loss.tokens("5 samples と 3 sets"), set())

    def test_unbalanced_backticks_do_not_make_long_cross_span_tokens(self):
        text = "`a` 本文 " + "あ" * 90 + " `b`"
        for token in loss.tokens(text):
            self.assertLessEqual(len(token), 80)


class CompareTests(unittest.TestCase):
    def test_removed_token_is_lost(self):
        lost, added = loss.compare("値 `X1` 0.5 A", "要約 `X1`")
        self.assertIn("0.5 A", lost)
        self.assertNotIn("`X1`", lost)
        self.assertEqual(added, [])

    def test_new_value_is_added(self):
        lost, added = loss.compare("a", "a 0.7 A")
        self.assertEqual(lost, [])
        self.assertIn("0.7 A", added)

    def test_unchanged_text_has_neither(self):
        self.assertEqual(loss.compare("値 `X1` 0.5 A #12", "値 `X1` 0.5 A #12"), ([], []))



class CommandTests(unittest.TestCase):
    def _repo(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        _git(root, "init", "-q", "-b", "main")
        return root

    def _rewrite_repo(self, new_text, other=None):
        root = self._repo()
        _write(root, "docs/a.md", "値 `X1` 0.5 A #12\n")
        _write(root, "docs/b.md", "変えない 9 V\n")
        _git(root, "add", "-A")
        _git(root, "commit", "-q", "-m", "base")
        _git(root, "branch", "base")
        _write(root, "docs/a.md", new_text)
        if other is not None:
            _write(root, "docs/c.md", other)
        _git(root, "add", "-A")
        _git(root, "commit", "-q", "-m", "rewrite")
        return root

    def test_loss_command_lists_removed_values_without_failing(self):
        root = self._rewrite_repo("要約 `X1` #12\n")
        result = _run(LOSS, root, "--base", "base")
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertIn("LOST docs/a.md: 0.5 A", result.stdout)
        self.assertRegex(result.stdout, r"docs/a\.md: lost=\d+ added=0")
        self.assertNotIn("docs/b.md", result.stdout)

    def test_loss_command_lists_added_values(self):
        root = self._rewrite_repo("値 `X1` 0.5 A #12 と 0.9 A\n")
        result = _run(LOSS, root, "--base", "base")
        self.assertIn("ADDED docs/a.md: 0.9 A", result.stdout)

    def test_loss_command_skips_new_files(self):
        root = self._rewrite_repo("値 `X1` 0.5 A #12\n", other="新しい 7 V\n")
        result = _run(LOSS, root, "--base", "base")
        self.assertIn("checked=0", result.stdout)

    def test_loss_command_checks_changed_files_even_without_any_history_file(self):
        """履歴の変化を条件にしない。内容が変わったMarkdown全部が対象。"""
        root = self._rewrite_repo("置き換え #12\n")
        result = _run(LOSS, root, "--base", "base")
        self.assertIn("checked=1", result.stdout)
        self.assertIn("LOST docs/a.md: `X1`", result.stdout)

    def test_show_returns_none_for_an_unreadable_file(self):
        """読めないfileは空文字ではなく`None`。空文字だと全tokenがLOST／ADDEDに出たまま0で終わる。"""
        root = self._rewrite_repo("置き換え #12\n")
        self.assertIsNone(loss._show(root, "base", "docs/none.md"))
        self.assertEqual(loss._show(root, "base", "docs/b.md"), "変えない 9 V\n")

    def test_non_ascii_file_names_are_checked_not_an_error(self):
        root = self._repo()
        _write(root, "docs/日本語.md", "値 0.5 A\n")
        _git(root, "add", "-A")
        _git(root, "commit", "-q", "-m", "base")
        _git(root, "branch", "base")
        _write(root, "docs/日本語.md", "要約\n")
        _git(root, "add", "-A")
        _git(root, "commit", "-q", "-m", "rewrite")
        result = _run(LOSS, root, "--base", "base")
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertIn("LOST docs/日本語.md: 0.5 A", result.stdout)

    def test_markdown_extension_files_are_checked(self):
        root = self._repo()
        _write(root, "docs/a.markdown", "値 0.5 A\n")
        _git(root, "add", "-A")
        _git(root, "commit", "-q", "-m", "base")
        _git(root, "branch", "base")
        _write(root, "docs/a.markdown", "要約\n")
        _git(root, "add", "-A")
        _git(root, "commit", "-q", "-m", "rewrite")
        self.assertIn("LOST docs/a.markdown: 0.5 A", _run(LOSS, root, "--base", "base").stdout)

    def test_unknown_base_is_an_error_not_a_pass(self):
        root = self._rewrite_repo("要約\n")
        self.assertEqual(_run(LOSS, root, "--base", "no-such-ref").returncode, 2)



if __name__ == "__main__":
    unittest.main()
