#!/usr/bin/env python3
"""文書の「目標の量」をすべて検査する（#806）。

文書は決定された最終形だけを残す。大きくなること自体を、merge前に止める。

- **数値と出典をこのscriptの`LIMITS`の1か所だけに持つ。**意味と根拠は#806の「目標の量」の表が
  正本である（ここへ別の数字を作らない）。
- **種類の表（`KINDS`）**: path → 種類。上から順に最初に当たった行を使う。どの行にも当たらない
  Markdownは**失敗にして知らせる**（`UNCLASSIFIED`）。文書を足すときは、種類を先に決める。
  記録（追記だけ）と#806で消す文書は`exempt`に置き、消えるまで対象外にする。
- 測り方は種類で決まる。`lines`（行数）、`chars`（Unicodeの文字数）、`pages`（A4のページ数）。
  ページは、GitHubのMarkdown API（GFM）が作ったHTMLを、stylesheetを付けない最小のHTMLで包み、
  Firefox（headless）をgeckodriverのW3C WebDriverの`print`（A4。縮小は切る。ほかは既定）で印刷して数える。
  縮小は幅の広い要素の有無で倍率が変わり、ページ数が文書の量を表さなくなるため切る。
  文書のページを測るためサイトの装飾は付けず、余白・文字の大きさ・行間の数値を足さない。
  CIはrunnerのFirefox・geckodriver（Ubuntu 24.04のimage:
  https://github.com/actions/runner-images/blob/main/images/ubuntu/Ubuntu2404-Readme.md）と、
  日本語fontの`fonts-noto-cjk`を使う。手元でも同じcommandで測れる（`--measure-pages`）。
  描画の部品（font・Firefoxの版）が違えばページ数がずれるため、CIと手元で同じ部品に揃える。
- 今上限を超えている文書は`doc_size_baseline.json`に現在値を記録する（棚卸し）。**下げる方向にしか
  動かさない。**縮約のPRが小さくしたら、同じPRで下げる（下げ忘れは失敗にする）。baselineが
  空になることが#806の完了条件である。
"""

import argparse
import base64
import fnmatch
import json
import re
import socket
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
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


def count_pdf_pages(data):
    """PDF（bytes）のページ数。`/Type /Page`の数を数え、根の`/Count`と食い違えば例外。"""
    pages = len(re.findall(rb"/Type\s*/Page(?![A-Za-z])", data))
    counts = [int(n) for n in re.findall(rb"/Count\s+(\d+)", data)]
    if pages == 0 or (counts and max(counts) != pages):
        raise ValueError(
            f"Cannot count PDF pages reliably (page objects={pages}, /Count={counts})."
        )
    return pages


REPOSITORY = "wachi-yoshitaka-11-dev/deskcat"
A4_CM = {"width": 21.0, "height": 29.7}
# 縮小（shrinkToFit。既定はtrue）は、幅の広い表やcodeの有無で倍率が変わり、ページ数が文書の量を表さなくなるため切る。
PRINT_OPTIONS = {"page": A4_CM, "shrinkToFit": False}


API_LIMIT_BYTES = 300_000  # Markdown APIは400 KBまで。余裕を持たせる。


def split_markdown(text, limit=API_LIMIT_BYTES):
    """`limit` byte以下のかたまりへ、fence外の空行で分ける（表は空行を含まないため割れない）。"""
    if len(text.encode("utf-8")) <= limit:
        return [text]
    chunks = []
    current = []
    size = 0
    in_fence = False
    for line in text.split("\n"):
        if line.lstrip().startswith(("```", "~~~")):
            in_fence = not in_fence
        current.append(line)
        size += len(line.encode("utf-8")) + 1
        if not in_fence and not line.strip() and size >= limit * 2 // 3:
            chunks.append("\n".join(current))
            current, size = [], 0
    if current:
        chunks.append("\n".join(current))
    return chunks


def _render(text, gh):
    with tempfile.TemporaryDirectory() as temporary:
        source = Path(temporary) / "source.md"
        source.write_text(text, encoding="utf-8")
        result = subprocess.run(
            [gh, "api", "markdown", "-f", "mode=gfm", "-f", f"context={REPOSITORY}",
             "-F", f"text=@{source}"],
            capture_output=True, text=True, encoding="utf-8", timeout=120,
        )
    if result.returncode != 0 or not result.stdout.strip():
        raise RuntimeError(f"Markdown API failed: {result.stderr[-300:]}")
    return result.stdout


def markdown_to_html(text, gh="gh"):
    """GitHubのMarkdown API（GFM）でHTMLにする。`gh`の認証（CIでは`GH_TOKEN`）を使う。
    APIの上限（400 KB）を超える文書は、fence外の空行で分けて作り、つなぐ。"""
    return "\n".join(_render(chunk, gh) for chunk in split_markdown(text))


def wrap_html(body):
    """stylesheetを付けない最小のHTML。余白・文字の大きさ・行間はブラウザの既定のままにする。"""
    return ('<!doctype html><html lang="ja"><head><meta charset="utf-8"></head><body>'
            + body + "</body></html>")


class PrintSession:
    """geckodriverのW3C WebDriverでFirefox（headless）を起動し、HTMLをA4のPDFへ印刷する。

    `base_url`を渡すと、起動済みのWebDriverに接続する（test用）。無ければ`geckodriver`を起動する。
    """

    def __init__(self, geckodriver="geckodriver", base_url=""):
        self.process = None
        self.session = ""
        self.base_url = base_url
        if not base_url:
            with socket.socket() as probe:
                probe.bind(("127.0.0.1", 0))
                port = probe.getsockname()[1]
            self.process = subprocess.Popen(
                [geckodriver, "--port", str(port)],
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            )
            self.base_url = f"http://127.0.0.1:{port}"
        self._wait_ready()
        created = self._request("POST", "/session", {"capabilities": {"alwaysMatch": {
            "browserName": "firefox",
            "moz:firefoxOptions": {"args": ["-headless"]},
        }}})
        self.session = created["value"]["sessionId"]

    def _request(self, method, path, body=None):
        request = urllib.request.Request(
            self.base_url + path,
            data=None if body is None else json.dumps(body).encode("utf-8"),
            method=method, headers={"Content-Type": "application/json"},
        )
        try:
            with urllib.request.urlopen(request, timeout=120) as response:
                return json.loads(response.read().decode("utf-8"))
        except urllib.error.HTTPError as error:
            raise RuntimeError(f"WebDriver {method} {path}: {error.read().decode('utf-8')[:300]}")

    def _wait_ready(self):
        for _ in range(100):
            try:
                self._request("GET", "/status")
                return
            except (OSError, RuntimeError):
                time.sleep(0.1)
        raise RuntimeError("WebDriver did not become ready.")

    def pages(self, html):
        """HTMLをA4で印刷したときのページ数。"""
        url = "data:text/html;charset=utf-8;base64," + base64.b64encode(
            html.encode("utf-8")).decode("ascii")
        self._request("POST", f"/session/{self.session}/url", {"url": url})
        printed = self._request("POST", f"/session/{self.session}/print", PRINT_OPTIONS)
        return count_pdf_pages(base64.b64decode(printed["value"]))

    def close(self):
        try:
            if self.session:
                self._request("DELETE", f"/session/{self.session}")
        except (OSError, RuntimeError):
            pass
        if self.process is not None:
            self.process.terminate()
            try:
                self.process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.process.kill()

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close()


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
    parser.add_argument("--measure-pages", action="store_true",
                        help="ページ数を測る（Firefox・geckodriver・`gh`の認証が要る）")
    parser.add_argument("--geckodriver", default="geckodriver")
    parser.add_argument("--gh", default="gh")
    parser.add_argument("--base", default="",
                        help="比較元のrefまたはsha。baselineが増えた・キーが増えたら失敗にする")
    parser.add_argument("--print-baseline", action="store_true",
                        help="上限を超える現在値をJSONで出す（fileは書かない）")
    parser.add_argument("--print-values", action="store_true",
                        help="測った全文書の現在値をJSONで出す（fileは書かない）")
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

    session = None
    try:
        counter = None
        if arguments.measure_pages:
            session = PrintSession(arguments.geckodriver)

            def counter(relative):
                return session.pages(wrap_html(markdown_to_html(files[relative], arguments.gh)))

        values, unclassified = measure(files, counter)
    except (RuntimeError, ValueError, OSError, KeyError, subprocess.SubprocessError) as error:
        print(f"Page counting failed: {error}")
        return 2
    finally:
        if session is not None:
            session.close()
    kinds = kinds_of(files)

    if arguments.print_values:
        print(json.dumps(values, ensure_ascii=False, indent=2, sort_keys=True))
        return 0
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
