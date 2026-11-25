#!/usr/bin/env python3
"""文書の「目標の量」をすべて検査する（#806）。

文書は決定された最終形だけを残す。大きくなること自体を、merge前に止める。

- **数値と出典をこのscriptの`LIMITS`の1か所だけに持つ。**意味と根拠は#806の「目標の量」の表が
  正本である（ここへ別の数字を作らない）。
- **種類の表（`KINDS`）**: path → 種類。上から順に最初に当たった行を使う。どの行にも当たらない
  Markdownは**失敗にして知らせる**（`UNCLASSIFIED`）。文書を足すときは、種類を先に決める。
  記録（追記だけ）と#806で消す文書は`exempt`に置き、消えるまで対象外にする。
- 測り方は種類で決まる。`lines`（行数）、`chars`（Unicodeの文字数）、`pages`（A4のページ数）。
  ページは、Pagesのbuildが出したHTMLからサイトの装飾（stylesheet・font・bar・footer）を外した
  本文を、runnerのGoogle Chrome（headless）の既定の組みでA4に印刷して数える（`--site-root`を
  渡したときだけ。Chromeの無い端末では測らない）。余白・文字の大きさ・行間の数値を足さない。
- 今上限を超えている文書は`doc_size_baseline.json`に現在値を記録する（棚卸し）。**下げる方向にしか
  動かさない。**縮約のPRが小さくしたら、同じPRで下げる（下げ忘れは失敗にする）。baselineが
  空になることが#806の完了条件である。
"""

import argparse
import fnmatch
import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

SCRIPTS_ROOT = Path(__file__).resolve().parent

# 目標の量（#806「目標の量」の表）。キーは種類。値は(測り方, 上限)。
# 出典:
# - entry: Claude Codeの公式文書（https://code.claude.com/docs/en/memory）の
#   「target under 200 lines per CLAUDE.md file」。AGENTS.mdとCLAUDE.mdの合計行数で測る。
# - design / adr: 1文書あたりのA4ページ数。設計文書の合計は`DESIGN_TOTAL_PAGES`。
# - contributing / governance / procedure / readme: #806の「実測した公開repoの表」の
#   文字数（Unicode）。
LIMITS = {
    "entry": ("lines", 199),
    "design": ("pages", 3),
    "adr": ("pages", 2),
    "contributing": ("chars", 3346),
    "governance": ("chars", 8104),
    "procedure": ("chars", 26842),
    "readme": ("chars", 8397),
}
DESIGN_TOTAL_PAGES = 20
ENTRY_KEY = "AGENTS.md+CLAUDE.md"
DESIGN_TOTAL_KEY = "total:design-pages"
ENTRY_FILES = ("AGENTS.md", "CLAUDE.md")

# path → 種類（`fnmatch`。`*`は`/`も越える）。どのMarkdownにも上限を当てる（量の無い種類は作らない）。
KINDS = (
    # 記録（追記だけ）と、#806で消す文書。消えるまで対象外にする。
    ("docs/hardware/*-history.md", "exempt"),
    ("docs/runbooks/*-history.md", "exempt"),
    ("docs/hardware/experiment-log.md", "exempt"),
    ("docs/toolchains/version-records/*", "exempt"),
    ("docs/planning/*", "exempt"),
    ("docs/backlog/*", "exempt"),
    ("docs/runbooks/coderabbit-review-observations.md", "exempt"),
    ("AGENTS.md", "entry"),
    ("CLAUDE.md", "entry"),
    ("README.md", "readme"),
    ("*/README.md", "readme"),
    ("CONTRIBUTING.md", "contributing"),
    ("docs/governance/*", "governance"),
    ("SECURITY.md", "governance"),
    (".claude/*", "governance"),
    (".github/*", "governance"),
    ("pages/*", "readme"),
    ("docs/runbooks/*", "procedure"),
    ("docs/toolchains/*", "procedure"),
    ("docs/decisions/*", "adr"),
    ("docs/hardware/*", "design"),
    ("docs/protocol/*", "design"),
    ("docs/DeskCat_Microcontroller_Development_Guide.md", "design"),
    ("docs/fonts/*", "design"),
    ("hardware/*", "design"),
    ("docs/architecture/*", "design"),
)

BASELINE_NAME = "doc_size_baseline.json"


def classify(relative):
    """`relative`（repository root相対、`/`区切り）の種類。当たる行が無ければ`None`。"""
    for pattern, kind in KINDS:
        if fnmatch.fnmatchcase(relative, pattern):
            return kind
    return None


def uses_measure_build(relative):
    """Pagesに載らず、計測用のbuild（`_measure`）で数える文書（`hardware/`の計画と配線表）。"""
    return relative.startswith("hardware/")


def site_path(relative):
    """Markdownをbuildしたあとの、出力root相対のHTML。"""
    stem = str(Path(relative).with_suffix(""))
    if stem.endswith("/README"):
        stem = stem[: -len("README")] + "index"
    return stem + ".html"


def count_pdf_pages(data):
    """PDF（bytes）のページ数。`/Type /Page`の数を数え、根の`/Count`と食い違えば例外。"""
    pages = len(re.findall(rb"/Type\s*/Page(?![A-Za-z])", data))
    counts = [int(n) for n in re.findall(rb"/Count\s+(\d+)", data)]
    if pages == 0 or (counts and max(counts) != pages):
        raise ValueError(
            f"Cannot count PDF pages reliably (page objects={pages}, /Count={counts})."
        )
    return pages


_MAIN = re.compile(r'<main\b[^>]*\bid="content"[^>]*>(.*)</main>', re.S)
_ARTICLE = re.compile(r'<article\b[^>]*\bclass="dc-doc"[^>]*>(.*)</article>', re.S)


def printable_html(text):
    """印刷用のHTML。サイトのlayoutの本文（`<main id="content">`の中身。pageの
    layoutなら`<article class="dc-doc">`の中身）だけを取り出し、stylesheet・font・bar・footerを
    外した最小のHTMLで包む。余白・文字の大きさ・行間はChromeの既定のままにする（数値を足さない）。
    本文が見つからなければ例外（測れないものを黙って通さない）。"""
    main = _MAIN.search(text)
    if main is None:
        raise ValueError('<main id="content"> not found in the built HTML.')
    body = main.group(1)
    article = _ARTICLE.search(body)
    if article is not None:
        body = article.group(1)
    return (
        '<!doctype html><html lang="ja"><head><meta charset="utf-8">'
        "<style>@page { size: A4 }</style></head><body>" + body + "</body></html>"
    )


def render_pages(chrome, html_path):
    """HTML fileの本文をChrome headlessでA4のPDFに印刷し、ページ数を返す。"""
    html_path = Path(html_path)
    printable = printable_html(html_path.read_text(encoding="utf-8"))
    with tempfile.TemporaryDirectory() as temporary:
        # 一時directoryへ置く（出力directoryはDocker actionが作り、書けるとは限らない）。
        copy = Path(temporary) / "print.html"
        pdf = Path(temporary) / "out.pdf"
        copy.write_text(printable, encoding="utf-8")
        result = subprocess.run(
            [chrome, "--headless", "--no-sandbox", "--disable-gpu",
             "--no-pdf-header-footer", f"--print-to-pdf={pdf}", copy.as_uri()],
            capture_output=True, text=True, timeout=120,
        )
        if result.returncode != 0 or not pdf.exists():
            raise RuntimeError(f"Chrome failed on {html_path}: {result.stderr[-300:]}")
        return count_pdf_pages(pdf.read_bytes())


PORTAL_ENTRIES = ("_config.yml", "_layouts", "assets", "favicon.ico")


def stage_measure_source(root, pages_source, destination, names):
    """`hardware/`の設計文書だけを、公開と同じlayout・設定でbuildするためのsourceを作る。"""
    destination = Path(destination)
    if destination.exists():
        shutil.rmtree(destination)
    destination.mkdir(parents=True)
    for entry in PORTAL_ENTRIES:
        source = Path(pages_source) / entry
        if source.is_dir():
            shutil.copytree(source, destination / entry)
        elif source.is_file():
            shutil.copy2(source, destination / entry)
    staged = []
    for name in names:
        if uses_measure_build(name) and classify(name) == "design":
            target = destination / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(Path(root) / name, target)
            staged.append(name)
    return staged


def measure(files, page_counter=None):
    """`files`は`{相対path: 本文}`。`(測定値, 未分類)`を返す。

    測定値のキーは相対path（ページ数は`page_counter(相対path)`、無ければ測らない）、
    `ENTRY_KEY`、`DESIGN_TOTAL_KEY`。
    """
    values = {}
    unclassified = []
    entry_lines = 0
    design_total = 0
    for relative, text in sorted(files.items()):
        kind = classify(relative)
        if kind is None:
            unclassified.append(relative)
            continue
        if kind == "entry":
            entry_lines += len(text.splitlines())
            continue
        if kind == "exempt":
            continue
        unit = LIMITS[kind][0]
        if unit == "chars":
            values[relative] = len(text)
        elif page_counter is not None:
            pages = page_counter(relative)
            values[relative] = pages
            if kind == "design":
                design_total += pages
    values[ENTRY_KEY] = entry_lines
    if page_counter is not None:
        values[DESIGN_TOTAL_KEY] = design_total
    return values, unclassified


def limit_of(key, files_kinds):
    if key == ENTRY_KEY:
        return LIMITS["entry"][1]
    if key == DESIGN_TOTAL_KEY:
        return DESIGN_TOTAL_PAGES
    return LIMITS[files_kinds[key]][1]


def load_baseline(path):
    """`{キー: 現在値}`を読む（値は0以上の整数）。"""
    if not path.exists():
        return {}
    data = json.loads(path.read_text(encoding="utf-8"))
    valid = isinstance(data, dict) and all(
        isinstance(k, str) and isinstance(v, int) and not isinstance(v, bool) and v >= 0
        for k, v in data.items()
    )
    if not valid:
        raise ValueError(f"{path.name} must map keys to non-negative integers.")
    return data


def kinds_of(files):
    return {r: classify(r) for r in files if classify(r) not in (None, "exempt")}


def check(values, kinds, baseline, unclassified=(), pages_measured=True):
    """違反のlistを返す。

    違反: 未分類／上限とbaselineの大きい方を超える／baselineが上限以内で不要／baselineの対象が
    もう無い／実測より大きい（下げ忘れ）。ページを測っていないときは、ページのキーの
    baselineを検査しない。
    """
    problems = [
        f"{r}: UNCLASSIFIED. Add a row to KINDS in check_doc_size.py "
        "(decide the document's kind first)."
        for r in unclassified
    ]
    for key, value in sorted(values.items()):
        limit = limit_of(key, kinds)
        entry = baseline.get(key)
        allowed = max(limit, entry or 0)
        if value > allowed:
            problems.append(f"{key}: {value}, allowed {allowed}.")
        if entry is not None:
            if entry <= limit:
                problems.append(f"{key}: baseline {entry} is within the limit {limit}; remove it.")
            elif value < entry:
                problems.append(
                    f"{key}: now {value}, baseline is {entry}; lower the baseline to {value}."
                )
    for key in sorted(set(baseline) - set(values)):
        if not pages_measured and (key == DESIGN_TOTAL_KEY or key in kinds and
                                   LIMITS[kinds[key]][0] == "pages"):
            continue
        problems.append(f"{key}: baseline entry has no measured target.")
    return problems


def ratchet(current, base):
    """baselineを下げる方向にしか動かさない。`base`に無いキーの追加と、値の増加は違反。
    `base`が`None`（比較元にbaselineのfileが無い。新設のPR）のときは比べない。"""
    if base is None:
        return []
    problems = []
    for key, value in sorted(current.items()):
        if key not in base:
            problems.append(f"{key}: baseline entry is new ({value}); baseline may only go down.")
        elif value > base[key]:
            problems.append(f"{key}: baseline rose from {base[key]} to {value}.")
    return problems


def baseline_at(root, ref):
    """`ref`とHEADのmerge-baseにおけるbaseline。`(辞書かNone, エラー文)`を返す。"""
    merge_base = subprocess.run(
        ["git", "-C", str(root), "merge-base", ref, "HEAD"],
        capture_output=True, text=True, encoding="utf-8",
    )
    if merge_base.returncode != 0:
        return None, f"Cannot resolve merge-base of {ref} and HEAD."
    shown = subprocess.run(
        ["git", "-C", str(root), "show", f"{merge_base.stdout.strip()}:scripts/{BASELINE_NAME}"],
        capture_output=True, text=True, encoding="utf-8",
    )
    if shown.returncode != 0:
        return None, ""  # 比較元にfileが無い（新設）。
    try:
        data = json.loads(shown.stdout)
    except ValueError:
        return None, f"{BASELINE_NAME} at {ref} is not valid JSON."
    return data, ""


def make_baseline(values, kinds):
    return {k: v for k, v in sorted(values.items()) if v > limit_of(k, kinds)}


def list_markdown(root):
    result = subprocess.run(
        ["git", "-c", "core.quotePath=false", "-C", str(root),
         "ls-files", "--cached", "-z", "--", "*.md", "*.markdown"],
        capture_output=True,
    )
    if result.returncode != 0:
        raise RuntimeError("git ls-files failed.")
    names = [n for n in result.stdout.decode("utf-8").split("\0") if n]
    return sorted(n for n in names if (Path(root) / n).is_file())


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository-root", default="")
    parser.add_argument("--site-root", default="", help="ページ数を測る。Pagesのbuild出力（`_site`）")
    parser.add_argument("--measure-root", default="",
                        help="`hardware/`の設計文書を計測用にbuildした出力（`_measure`）")
    parser.add_argument("--stage-measure-source", default="",
                        help="計測用buildのsourceをここへ作って終わる（`--pages-source`が要る）")
    parser.add_argument("--pages-source", default=".pages-src")
    parser.add_argument("--chrome", default="google-chrome")
    parser.add_argument("--base", default="",
                        help="比較元のrefまたはsha。baselineが増えた・キーが増えたら失敗にする")
    parser.add_argument("--print-baseline", action="store_true",
                        help="上限を超える現在値をJSONで出す（fileは書かない）")
    arguments = parser.parse_args(argv)
    root = Path(arguments.repository_root or SCRIPTS_ROOT.parent).resolve()

    try:
        names = list_markdown(root)
    except RuntimeError as error:
        print(str(error))
        return 2
    files = {name: (root / name).read_text(encoding="utf-8") for name in names}
    if not files:
        print("No Markdown files resolved. The size check is not working.")
        return 2

    if arguments.stage_measure_source:
        staged = stage_measure_source(
            root, arguments.pages_source, arguments.stage_measure_source, names)
        print(f"STAGED={len(staged)}")
        return 0 if staged else 2

    counter = None
    if arguments.site_root:
        site = Path(arguments.site_root)
        measure_site = Path(arguments.measure_root) if arguments.measure_root else None

        def counter(relative):
            base = measure_site if uses_measure_build(relative) else site
            if base is None:
                raise RuntimeError(f"{relative} needs --measure-root.")
            # 出力が無いのは失敗（測れないものを黙って通さない）。
            return render_pages(arguments.chrome, base / site_path(relative))

    try:
        values, unclassified = measure(files, counter)
    except (RuntimeError, ValueError, OSError, subprocess.SubprocessError) as error:
        print(f"Page counting failed: {error}")
        return 2
    kinds = kinds_of(files)

    if arguments.print_baseline:
        print(json.dumps(make_baseline(values, kinds), ensure_ascii=False, indent=2))
        return 0
    try:
        baseline = load_baseline(root / "scripts" / BASELINE_NAME)
    except ValueError as error:
        print(str(error))
        return 2
    problems = check(values, kinds, baseline, unclassified, counter is not None)
    if arguments.base:
        base_baseline, error = baseline_at(root, arguments.base)
        if error:
            print(error)
            return 2
        problems += ratchet(baseline, base_baseline)
    for line in problems:
        print(f"DOC-TOO-LARGE {line}")
    print(f"files={len(files)} measured={len(values)} problems={len(problems)}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
