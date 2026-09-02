#!/usr/bin/env python3
"""GFM表のheader行とセル数が食い違う行を検出する。

不足セルは空で埋まり、超過セルは捨てられるため、GFMは列数の食い違いを描画時に
黙って吸収する。ここで検出しなければ、review・自己レビュー・昇格の照合の
いずれも気付かないまま`main`へ入る（[#289](https://github.com/wachi-yoshitaka-11-dev/deskcat/issues/289)）。

この検査は過去2回、手で数えて逆向きに外れている。書き直すならこの2点を先に読むこと。

- `\\|`はセル内の文字であり、区切りではない。GFMではセル内にパイプを書くには
  `\\|`とエスケープする。これを区切りとして数えると、存在しない不整合を報告する
  （2026-08-31に`gpio-assignment.md`へ2件そう報告された）。
- 末尾のパイプは省略できる。`|`の数から1を引く数え方は、末尾パイプが無い行で
  1セルずれる。これで実在した破損を「誤検知」と判定した（2026-08-28。
  `hardware-bom.md`の10セル目が落ち、短絡安全の断り書きが描画されないまま
  昇格しかけた）。

方向が逆の2つの誤りなので、注意の向きを変えても直らない。セルを1文字ずつ
割ることで両方を同時に扱う。

不足セルが空で埋まるのか超過セルが捨てられるのかは判定しない。どちらの向きに
壊れているかは、報告された行を開いて読む必要がある。列がずれているだけで数が
合っている行、HTMLの`<table>`、header行そのものの誤り（列名の妥当性）は
この検査の対象外である。

full reviewで指摘された、意図して直さない2点（[PR #307](https://github.com/wachi-yoshitaka-11-dev/deskcat/pull/307)）:

- **fenceの判定は開始文字種・長さを見ない。**4個以上のbacktick fenceの内側に
  3個のbacktick行があると、そこで閉じたと誤認する。GFMの厳密な規則ではない。
  `validate_doc_links.py`の`markdown_outside_fences`と同じ`_FENCE_RE`を意図して
  共有しており（このscriptだけを直すと、2つのscriptが別の行をfenceと見なし
  片方だけがfence内の表を走査する不整合を生む）、現在の追跡下Markdownに
  該当する書き方は無い。
- **パイプを含まない行は、直前まで表であっても表の終わりとして扱う。**GFMの
  厳密な規則では、空行または別のblock構造の開始までは1セルの行として
  表が続く。この単純化により、パイプの無い継続行を見逃す可能性がある。
  ただし2026-09-02の実測で、追跡下362表すべてが空行区切りを持ち、
  該当する行は0件だった。正しく実装するにはblock構造の開始判定
  （見出し・list・blockquote等）が要り、誤判定すると見出しや通常の段落を
  誤って1セルの表行として報告しうる。実例が無い状態でこの複雑さを追加する
  リスクを取らない。
"""

import argparse
import os
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))

import publish_guards as guards  # noqa: E402

_DELIMITER_RE = re.compile(r"^:?-+:?$")
# fence判定はvalidate_doc_links.pyの`markdown_outside_fences`と同じ`_FENCE_RE`に
# 揃える。2つのscriptが別の行をfenceと見なすと、片方だけがfence内の表を
# 走査してしまう。
_FENCE_RE = re.compile(r"^\s*(?:```|~~~)")


def split_cells(line):
    r"""1行をセルへ割る。`\|`は文字として扱い、区切りにしない。

    連続するbackslashは、直前の連続数が奇数のときだけ次のパイプをエスケープする
    （GFM）。`\\|`（backslash 2個）はエスケープされたbackslashに続く実区切りであり、
    `\\\|`（3個）はエスケープされたbackslashとエスケープされたパイプになる。
    2個目のbackslashだけを見て判定すると、この2つを取り違える
    （full reviewの指摘。この repositoryの現行文書に該当行は無いが、将来の
    書き方を制限しないため正しく扱う）。

    行頭・行末のパイプが作る空セルは、あれば落とす。末尾パイプは省略できるため
    「あれば落とす」であって「必ず1引く」ではない。
    """
    out = []
    current = []
    index = 0
    length = len(line)
    while index < length:
        char = line[index]
        if char == "\\":
            run_start = index
            while index < length and line[index] == "\\":
                index += 1
            run_length = index - run_start
            # 2個で1つの literal backslash。奇数分の最後の1個は次のパイプへ回す。
            current.append("\\" * (run_length // 2))
            if run_length % 2 == 1:
                if index < length and line[index] == "|":
                    current.append("|")
                    index += 1
                else:
                    current.append("\\")
            continue
        if char == "|":
            out.append("".join(current))
            current = []
            index += 1
            continue
        current.append(char)
        index += 1
    out.append("".join(current))
    if out and out[0].strip() == "":
        out = out[1:]
    if out and out[-1].strip() == "":
        out = out[:-1]
    return out


def is_delimiter_row(line):
    """`---`／`:--`／`--:`／`:-:`だけからなる行（header区切り）かを返す。"""
    cells = split_cells(line)
    return bool(cells) and all(_DELIMITER_RE.match(cell.strip()) for cell in cells)


def find_mismatches(text):
    """`(行番号, header行番号, header列数, この行の列数, 行内容)`のtupleを返す。

    表の開始は「直前の非空行」+「区切り行」の対で検出する。ここでの列数は
    header行のセル数であり、以降の行はそれと比較する。fence内は表として
    読まない（fence内の`|`はcode例示であり表ではない）。
    """
    mismatches = []
    tables = 0
    rows = 0
    in_fence = False
    previous_line = None
    column_count = 0
    header_line_number = 0
    for number, line in enumerate(text.split("\n"), start=1):
        if _FENCE_RE.match(line):
            in_fence = not in_fence
            column_count = 0
            previous_line = None
            continue
        if in_fence:
            continue
        stripped = line.strip()
        if "|" not in stripped:
            column_count = 0
            previous_line = None
            continue
        if column_count == 0:
            if previous_line is not None and is_delimiter_row(stripped):
                column_count = len(split_cells(previous_line))
                header_line_number = number - 1
                tables += 1
            else:
                previous_line = stripped
            continue
        rows += 1
        actual = len(split_cells(stripped))
        if actual != column_count:
            mismatches.append(
                (number, header_line_number, column_count, actual, stripped)
            )
    return mismatches, tables, rows


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository-root", default="")
    arguments = parser.parse_args(argv)

    repository_root = arguments.repository_root
    if not repository_root.strip():
        repository_root = str(Path(__file__).resolve().parent.parent)
    root = guards.full_path(repository_root)

    if not os.path.isdir(root):
        raise guards.ValidationError("Repository root does not exist.")

    # 追跡fileの列挙はvalidate_doc_links.pyと同じ考え方に揃える。symlinkは
    # 走査しない（実体と二重に数える）。走査対象はGitが追跡するMarkdownだけとし、
    # 生成物・worktree・未追跡の作業用copyを拾わない。
    tracked_symlinks = guards.get_tracked_symlinks(root)
    tracked_all = guards.get_tracked_files(root, ".")
    tracked = sorted(
        entry
        for entry in tracked_all
        if entry not in tracked_symlinks
        and guards.get_extension(entry).lower() in guards.MARKDOWN_EXTENSIONS
    )
    if not tracked:
        raise guards.ValidationError(
            "No tracked Markdown files resolved. The table check is not working."
        )

    problems = []
    total_tables = 0
    total_rows = 0
    for entry in tracked:
        path = guards.full_path(os.path.join(root, entry))
        if not os.path.isfile(path):
            continue
        text = guards.get_file_text(path)
        mismatches, tables, rows = find_mismatches(text)
        total_tables += tables
        total_rows += rows
        for number, header_number, expected, actual, content in mismatches:
            preview = content if len(content) <= 60 else content[:60] + "…"
            problems.append(
                f"{entry}:{number}  header L{header_number}={expected}列"
                f"  この行={actual}列  {preview}"
            )

    if problems:
        for problem in guards.sort_unique(problems):
            print(problem, file=sys.stderr)
        raise guards.ValidationError(
            f"Table column validation failed with {len(problems)} mismatch(es)."
        )

    print(f"TABLES={total_tables} ROWS={total_rows} MISMATCHES=0")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except guards.ValidationError as error:
        print(error, file=sys.stderr)
        sys.exit(1)
