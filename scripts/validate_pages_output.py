#!/usr/bin/env python3
"""生成済み`_site/`のlinkと公開禁止情報を検査する。

診断のfile pathはsite-root相対で出力する。localの絶対pathをCI logへ残さない。
"""

import argparse
import os
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))

import publish_guards as guards  # noqa: E402

REQUIRED_FILES = (
    "index.html",
    "404.html",
    "favicon.ico",
    "assets/css/style.css",
    "assets/deskcat-concept.jpg",
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

# HTML tokenizerがseparatorとして扱うASCII whitespaceだけを受理する。
# Unicodeのwhitespace判定はNBSP等も含み、browserの属性境界とずれる。
HTML_SPACE = frozenset("\t\n\x0c\r ")

# browserがtextとして扱うfallback本文を持つ要素。ここに`<a href=...>`が
# 書かれていても実際のnavigationではない。
RAW_TEXT_ELEMENTS = frozenset(
    {"script", "style", "textarea", "title", "xmp", "iframe", "noembed", "noframes"}
)

NAVIGATION_ATTRIBUTES = frozenset({"href", "src", "xlink:href"})

# `Trim()`へ渡すURL前後の文字。ASCII制御文字と空白だけを対象にする。
URL_TRIM_CHARACTERS = "".join(chr(code) for code in range(0x00, 0x21))

BASEURL_LINE_RE = re.compile(r"^baseurl[ \t]*:(?P<after>.*)$")
UNSAFE_BASEURL_RE = re.compile(r"[\x00-\x1f\x7f?#\\]")
BASEURL_DOT_SEGMENT_RE = re.compile(r"(?:^|/)\.\.?(?:/|$)")
SCHEME_RE = re.compile(r"^[A-Za-z][A-Za-z0-9+.-]*:")
CONTROL_RUN_RE = re.compile(r"[\x00-\x20\x7f]+")


def _yaml_scalar_without_comment(value):
    """YAMLのplain scalarから、ASCII space／tabの後ろにある`#`以降を取り除く。

    quoted scalar内の`#`や、double quote内のescape、single quoteの二重化は
    comment境界として扱わない。
    """
    quote = ""
    index = 0
    while index < len(value):
        character = value[index]
        if quote == '"':
            if character == "\\":
                index += 1
            elif character == '"':
                quote = ""
            index += 1
            continue
        if quote == "'":
            if character == "'":
                if index + 1 < len(value) and value[index + 1] == "'":
                    index += 1
                else:
                    quote = ""
            index += 1
            continue
        if character in ('"', "'"):
            quote = character
            index += 1
            continue
        if character == "#" and (index == 0 or value[index - 1] in (" ", "\t")):
            return value[:index].rstrip(" \t")
        index += 1

    return value.rstrip(" \t")


def _read_base_path(pages_config_path):
    """Pages base URLの正本はJekyll設定である。

    validatorへrepository名を重複記述すると、renameやcustom baseurl変更後に
    古いpathを正しいものとして検査してしまう。
    """
    base_url_values = []
    for line in re.split(r"\r?\n", guards.get_file_text(pages_config_path)):
        # Jekyllのbaseurlはcase-sensitiveなtop-level keyである。indentされた同名keyや
        # `BaseUrl`のような別keyを正本として読まない。
        match = BASEURL_LINE_RE.match(line)
        if not match:
            continue
        after_colon = match.group("after")
        # block mappingのcolon後が非空なら、valueとのseparatorはASCII space／tabである。
        # `baseurl:/deskcat`をkey/valueとして受理するとJekyllのYAML解釈とずれる。
        if after_colon and after_colon[0] not in (" ", "\t"):
            continue
        base_url_values.append(
            _yaml_scalar_without_comment(after_colon.strip(" \t"))
        )
    if len(base_url_values) != 1:
        raise guards.ValidationError(
            f"Pages config must define exactly one baseurl (found {len(base_url_values)})."
        )

    base_path = base_url_values[0].strip(" \t")
    if len(base_path) >= 2 and (
        (base_path[0] == '"' and base_path[-1] == '"')
        or (base_path[0] == "'" and base_path[-1] == "'")
    ):
        base_path = base_path[1:-1]
    if base_path and (
        not base_path.startswith("/")
        or "//" in base_path
        or UNSAFE_BASEURL_RE.search(base_path)
        or re.search(r"\s", base_path)
        or BASEURL_DOT_SEGMENT_RE.search(base_path)
    ):
        # config値自体がsecretや個人pathである可能性があるため、値をCI logへ再掲しない。
        raise guards.ValidationError("Pages config contains an unsafe baseurl.")
    return base_path.rstrip("/")


def _is_ascii_letter(character):
    return ("A" <= character <= "Z") or ("a" <= character <= "z")


def _html_start_tags(content):
    """開始tagだけを、出現順に文字列として返す。

    comment、quoted属性、raw-text／RCDATAを同じ走査状態で扱う。これらを別々の
    global regexで除去すると、comment内の偽scriptや属性値内の`<!--`が後続の
    実tagまで飲み込み、navigation属性を無検査にし得る。
    """
    length = len(content)
    index = 0
    while index < length:
        open_index = content.find("<", index)
        if open_index < 0:
            break

        if content[open_index : open_index + 4] == "<!--":
            comment_body_start = open_index + 4
            comment_end = -1
            # `<!-->`と`<!--->`はparse errorだが、その`>`でcommentが終了する。
            # 通常の`-->`だけを探すと、後続の実tagをcommentとして飲み込む。
            if comment_body_start < length and content[comment_body_start] == ">":
                comment_end = comment_body_start
            elif (
                comment_body_start + 1 < length
                and content[comment_body_start] == "-"
                and content[comment_body_start + 1] == ">"
            ):
                comment_end = comment_body_start + 1
            normal_comment_end = content.find("-->", comment_body_start)
            if normal_comment_end >= 0:
                normal_comment_end += 2
                if comment_end < 0 or normal_comment_end < comment_end:
                    comment_end = normal_comment_end
            # `--!>`もHTML tokenizerではcomment終端になる。
            bang_comment_end = content.find("--!>", comment_body_start)
            if bang_comment_end >= 0:
                bang_comment_end += 3
                if comment_end < 0 or bang_comment_end < comment_end:
                    comment_end = bang_comment_end
            if comment_end < 0:
                # 閉じていないcommentでは残り全体がmarkupではない。
                break
            index = comment_end + 1
            continue

        name_start = open_index + 1
        if name_start >= length:
            break
        if not _is_ascii_letter(content[name_start]):
            marker = content[name_start]
            if marker not in ("/", "!", "?"):
                # `1 < 2`のようなtextは次の`>`までを構文要素にしない。1文字だけ進め、
                # その後ろにある実開始tagを引き続き探索する。
                index = open_index + 1
                continue

            if marker in ("!", "?"):
                # comment以外のmarkup declarationとprocessing instructionは、HTMLでは
                # bogus comment等として最初の`>`でdataへ戻る。quoteに特別な意味はない。
                bogus_end = content.find(">", name_start + 1)
                if bogus_end < 0:
                    break
                index = bogus_end + 1
                continue

            # `</`の直後がASCII letterでなければclosing tag tokenは始まらない。
            # textとして1文字だけ進め、後続の実開始tagを探索する。
            closing_name_start = name_start + 1
            if closing_name_start >= length:
                break
            if not _is_ascii_letter(content[closing_name_start]):
                if content[closing_name_start] == ">":
                    index = closing_name_start + 1
                else:
                    index = open_index + 1
                continue

            # closing tagのattribute-like部分ではquoted value中の`>`を終端にしない。
            quote = ""
            declaration_end = -1
            for scan in range(closing_name_start, length):
                character = content[scan]
                if quote:
                    if character == quote:
                        quote = ""
                    continue
                if character in ('"', "'"):
                    quote = character
                elif character == ">":
                    declaration_end = scan
                    break
            if declaration_end < 0:
                break
            index = declaration_end + 1
            continue

        tag_name_end = name_start
        while (
            tag_name_end < length
            and content[tag_name_end] not in HTML_SPACE
            and content[tag_name_end] != "/"
            and content[tag_name_end] != ">"
        ):
            tag_name_end += 1
        tag_name = content[name_start:tag_name_end].lower()

        quote = ""
        tag_end = -1
        for scan in range(tag_name_end, length):
            character = content[scan]
            if quote:
                if character == quote:
                    quote = ""
                continue
            if character in ('"', "'"):
                quote = character
            elif character == ">":
                tag_end = scan
                break
        if tag_end < 0:
            # 閉じていない開始tagの後ろをtextとして再解釈しない。
            break

        self_closing_probe = tag_end - 1
        while (
            self_closing_probe > open_index
            and content[self_closing_probe] in HTML_SPACE
        ):
            self_closing_probe -= 1
        has_self_closing_marker = (
            self_closing_probe > open_index and content[self_closing_probe] == "/"
        )

        yield content[open_index : tag_end + 1]
        index = tag_end + 1

        if tag_name == "plaintext":
            if has_self_closing_marker:
                # scannerはHTML／foreign-contentのnamespace stackを持たない。`/>`を
                # text開始として残り全体を隠すより、後続tagを検査する安全側へ倒す。
                continue
            # plaintext要素の開始後はEOFまでtextであり、終了tagも認識されない。
            break
        if tag_name in RAW_TEXT_ELEMENTS:
            if has_self_closing_marker:
                # foreign contentではself-closingになる一方、HTML namespaceではparse errorに
                # なり得る。namespace非追跡のscannerでは後続の実linkを隠さない方を選ぶ。
                continue
            # 対応する終了tagまでをtextとしてskipし、終了tagが無ければ残りを再走査しない。
            closing_pattern = re.compile(re.escape(f"</{tag_name}"), re.IGNORECASE)
            closing_end = -1
            while index < length:
                closing_match = closing_pattern.search(content, index)
                if closing_match is None:
                    break
                after_name = closing_match.end()
                if after_name < length and (
                    content[after_name] in HTML_SPACE
                    or content[after_name] == "/"
                    or content[after_name] == ">"
                ):
                    closing_end = content.find(">", after_name)
                    break
                index = after_name
            if closing_end < 0:
                break
            index = closing_end + 1


def _start_tag_attributes(start_tag):
    """開始tagを先頭からtokenizeし、`(name, value)`を出現順に返す。

    tag全体へのattribute regexは、quoted value内の`href=`や`id=`まで実属性として
    拾うため使わない。quoted／unquoted／boolean属性と`xlink:href`のような
    namespace付き属性名を同じ境界規則で扱う。
    """
    if len(start_tag) < 3 or start_tag[0] != "<":
        return

    seen_names = set()
    length = len(start_tag)
    index = 1
    while (
        index < length
        and start_tag[index] not in HTML_SPACE
        and start_tag[index] != "/"
        and start_tag[index] != ">"
    ):
        index += 1

    while index < length:
        while index < length and start_tag[index] in HTML_SPACE:
            index += 1
        if index >= length or start_tag[index] == ">":
            break
        if (
            start_tag[index] == "/"
            and index + 1 < length
            and start_tag[index + 1] == ">"
        ):
            break

        name_start = index
        while (
            index < length
            and start_tag[index] not in HTML_SPACE
            and start_tag[index] not in ("=", "/", ">", "<", '"', "'")
        ):
            index += 1
        if index == name_start:
            # malformedな1文字で停止せず、次の境界を検査する。開始tag抽出側が
            # quoteを閉じたtagだけを渡すため、ここでquoted valueへ入ることはない。
            index += 1
            continue

        name = start_tag[name_start:index]
        while index < length and start_tag[index] in HTML_SPACE:
            index += 1

        value = ""
        if index < length and start_tag[index] == "=":
            index += 1
            while index < length and start_tag[index] in HTML_SPACE:
                index += 1
            if index < length and start_tag[index] in ('"', "'"):
                quote = start_tag[index]
                index += 1
                value_start = index
                while index < length and start_tag[index] != quote:
                    index += 1
                value = start_tag[value_start:index]
                if index < length:
                    index += 1
            else:
                value_start = index
                while (
                    index < length
                    and start_tag[index] not in HTML_SPACE
                    and start_tag[index] != ">"
                ):
                    index += 1
                value = start_tag[value_start:index]

        # HTML tokenizerは同じ開始tag内の後続duplicate属性をdropする。すべてを返すと、
        # browserが採用しない2個目のidをanchor集合へ加えてbroken linkを通してしまう。
        key = name.lower()
        if key not in seen_names:
            seen_names.add(key)
            yield name, value


def _check_links(
    html_files,
    attributes_by_file,
    ids_by_file,
    sensitive_text_files,
    published_file_paths,
    site_root_path,
    base_path,
    problems,
):
    for html_path in html_files:
        relative_html = guards.path_relative_to_root(html_path, site_root_path)
        # secret／個人pathを検出済みのfileは、raw URLを含むlink診断を追加で出さない。
        # file pathだけを報告して停止し、機密値そのものをCI logへ二次露出させない。
        if html_path in sensitive_text_files:
            continue

        for name, raw_value in attributes_by_file[html_path]:
            attribute_name = name.lower()
            if attribute_name not in NAVIGATION_ATTRIBUTES:
                continue
            value = guards.html_decode(raw_value).strip(URL_TRIM_CHARACTERS)
            if not value.strip():
                if attribute_name == "src":
                    problems.append(
                        f"Source attribute has no resource path in {relative_html}"
                    )
                continue
            diagnostic_value = guards.diagnostic_text(value)
            # data URIはasset manifest／size guardを迂回し、javascript URIは公開pageで
            # codeを実行できる。browserがscheme中のASCII制御文字／空白を正規化する場合も
            # あるため、それらを除いたprobeで判定し、外部linkとしてskipしない。
            scheme_probe = CONTROL_RUN_RE.sub("", value)
            lowered_probe = scheme_probe.lower()
            if lowered_probe.startswith("data:") or lowered_probe.startswith(
                "javascript:"
            ):
                problems.append(
                    f"Unsafe URL scheme in {relative_html}: {diagnostic_value}"
                )
                continue
            if scheme_probe.startswith("//") or lowered_probe.startswith(
                ("http:", "https:", "mailto:", "tel:")
            ):
                continue
            # URI schemeを持つ値は上のallowlistだけを外部linkとして受理する。file:、
            # vbscript:、custom scheme等をlocal pathへ落とすと、同名fileの存在だけで
            # browser上の別解釈を有効と誤判定する。
            if SCHEME_RE.match(scheme_probe):
                problems.append(
                    f"Unsafe URL scheme in {relative_html}: {diagnostic_value}"
                )
                continue

            # fragment内では`?`も通常の文字である。`[?#]`で同時にsplitすると
            # `#missing?part`のfragmentを空として扱い、anchor検査をskipしてしまう。
            # 先に最初の`#`でfragmentを分離し、その手前だけからqueryを除く。
            fragment_index = value.find("#")
            if fragment_index >= 0:
                before_fragment = value[:fragment_index]
                fragment = value[fragment_index + 1 :]
            else:
                before_fragment = value
                fragment = ""
            link_path = before_fragment.split("?", 1)[0]
            is_same_page_fragment = not link_path.strip() and bool(fragment.strip())
            if attribute_name == "src" and not link_path.strip():
                problems.append(
                    f"Source attribute has no resource path in {relative_html}:"
                    f" {diagnostic_value}"
                )
                continue
            if not link_path.strip() and not is_same_page_fragment:
                continue
            if not is_same_page_fragment:
                # encoded拡張子やseparatorも判定対象へ含める。decode前に`.md`を調べると
                # `source%2Emd`が禁止を迂回し、decode前だけ`\`を置換すると`%5C`の
                # path解釈がWindowsとLinuxで食い違う。
                link_path = guards.unescape_data_string(link_path).replace("\\", "/")
            path_extension = (
                "" if is_same_page_fragment else guards.get_extension(link_path).lower()
            )
            if path_extension in guards.MARKDOWN_EXTENSIONS:
                problems.append(
                    f"Unconverted Markdown link in {relative_html}: {diagnostic_value}"
                )
                continue

            if is_same_page_fragment:
                candidate_base = html_path
            elif link_path == base_path or link_path == f"{base_path}/":
                candidate_base = site_root_path
            elif link_path.startswith(f"{base_path}/"):
                candidate_base = os.path.join(
                    site_root_path, link_path[len(base_path) + 1 :]
                )
            elif link_path.startswith("/"):
                # project Pagesで`/docs/...`はdomain rootを指し、`/deskcat/docs/...`とは
                # 別URLである。site内の同名fileへ読み替えると404を見逃す。
                problems.append(
                    "Root-absolute link is outside Pages base path in "
                    f"{relative_html}: {diagnostic_value}"
                )
                continue
            else:
                candidate_base = os.path.join(os.path.dirname(html_path), link_path)

            candidate_base = guards.full_path(candidate_base)
            if not guards.path_within_root(candidate_base, site_root_path):
                problems.append(
                    f"Link escapes Pages output in {relative_html}: {diagnostic_value}"
                )
                continue

            candidates = [candidate_base]
            if not is_same_page_fragment and link_path.endswith("/"):
                candidates.append(os.path.join(candidate_base, "index.html"))
            elif not is_same_page_fragment and not guards.get_extension(
                candidate_base
            ).strip():
                candidates.append(f"{candidate_base}.html")
                candidates.append(os.path.join(candidate_base, "index.html"))

            resolved_candidates = []
            for candidate in candidates:
                # directory自体をlink先として受理しない。末尾`/`などのdirectory URLは
                # 別candidateとして追加した`index.html`が存在する場合だけ解決する。
                # 列挙済みfileのcase-sensitiveな集合と突き合わせ、Windowsでも
                # case違いを存在扱いにしない。
                candidate_full = guards.full_path(candidate)
                if candidate_full in published_file_paths and os.path.isfile(
                    candidate_full
                ):
                    resolved_candidates.append(candidate_full)
            if not resolved_candidates:
                problems.append(
                    f"Broken local link in {relative_html}: {diagnostic_value}"
                )
                continue
            if len(resolved_candidates) > 1:
                relative_targets = ", ".join(
                    guards.path_relative_to_root(candidate, site_root_path)
                    for candidate in resolved_candidates
                )
                problems.append(
                    f"Ambiguous local link in {relative_html}: {diagnostic_value}"
                    f" (matches: {relative_targets})"
                )
                continue
            resolved = resolved_candidates[0]

            # fragmentが生成HTMLのidに存在するか。
            if not fragment.strip():
                continue
            # 非HTMLのLeafへのfragmentは、この検査の対象外として通す。
            # `sprite.svg#icon`や`file.pdf#page=3`のfragmentはasset内部への参照であり、
            # HTMLのid検査では判定できない。fail-openではなく検査領域の境界である。
            # HTML拡張子なのにid集合へ無い場合だけを、検査できない事実として報告する。
            if guards.get_extension(resolved).lower() != ".html":
                continue
            if resolved in sensitive_text_files:
                problems.append(
                    f"Unverifiable anchor in {relative_html}:"
                    " target HTML contains sensitive content"
                )
                continue
            if resolved not in ids_by_file:
                problems.append(
                    f"Unverifiable anchor in {relative_html}: {diagnostic_value}"
                    " (resolved file is not a scanned HTML)"
                )
                continue
            wanted = guards.unescape_data_string(fragment)
            if wanted not in ids_by_file[resolved]:
                available = ", ".join(
                    guards.diagnostic_text(identifier)
                    for identifier in sorted(ids_by_file[resolved])[:6]
                )
                problems.append(
                    f"Broken anchor in generated site: {relative_html} ->"
                    f" {diagnostic_value} (ids present: {available})"
                )


def _is_license(path):
    """`LICENSE`はMarkdownではないため拡張子を持たない。file名で判定する。

    比較はcase-sensitiveにする。`license`のような別fileを例外へ通さない。
    """
    return os.path.basename(path) == "LICENSE"


def is_scanned_for_secrets(path):
    """その`_site`内のfileが、secret／個人pathのscan対象かを返す。

    scanの実処理と同じ条件をここへ書く。診断が実際のscan範囲とずれると、
    「scanされている」と読めるのにされていない、という誤読を生む。
    """
    if _is_license(path):
        return True
    return guards.get_extension(path).lower() in guards.TEXT_EXTENSIONS


def _summarize_output(files, site_root_path):
    """`_site`の内訳を要約する。

    `FILES=`と`HTML=`だけでは、非HTMLが何であるかが分からない。実際に公開される
    artifactは`_site`であり、その構成はJekyllとPagesが有効化するpluginが決めるため、
    sourceからは辿れない。scan対象外の拡張子と最大file sizeを出して、公開物の実態を
    log へ残す。判定には使わない。

    path は site-root 相対で出す。localの絶対pathをCI logへ書かない。
    """
    counts = {}
    unscanned = {}
    largest_size = -1
    largest_path = ""
    for path in files:
        extension = guards.get_extension(path).lower() or "(none)"
        counts[extension] = counts.get(extension, 0) + 1
        if not is_scanned_for_secrets(path):
            unscanned[extension] = unscanned.get(extension, 0) + 1
        try:
            size = os.path.getsize(path)
        except OSError:
            continue
        if size > largest_size:
            largest_size = size
            largest_path = path

    def render(mapping):
        # 並びを固定する。環境やfilesystemの列挙順でlogが変わると突き合わせられない。
        return ",".join(f"{key}={mapping[key]}" for key in sorted(mapping)) or "(none)"

    lines = [f"EXTENSIONS={render(counts)}", f"UNSCANNED={render(unscanned)}"]
    if largest_path:
        relative = guards.path_relative_to_root(largest_path, site_root_path)
        lines.append(f"LARGEST={largest_size} {relative}")
    return lines


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--site-root", default="")
    parser.add_argument("--pages-config-path", default="")
    arguments = parser.parse_args(argv)

    repository_root = guards.full_path(str(Path(__file__).resolve().parent.parent))
    site_root = arguments.site_root
    if not site_root.strip():
        site_root = os.path.join(repository_root, "_site")
    site_root_path = guards.full_path(site_root)

    if not os.path.isdir(site_root_path):
        # user指定pathにはusername等が含まれ得るため、local絶対pathをCI logへ出さない。
        raise guards.ValidationError("Pages output directory does not exist.")

    pages_config_path = arguments.pages_config_path
    if not pages_config_path.strip():
        pages_config_path = os.path.join(repository_root, "pages/_config.yml")
    pages_config_full_path = guards.full_path(pages_config_path)
    if not os.path.isfile(pages_config_full_path):
        raise guards.ValidationError("Pages config file does not exist.")
    # base pathの比較はcase-sensitiveにする。GitHub PagesのURLはcase-sensitiveであり、
    # 設定とcaseが異なるpathは実際には404になる。case-insensitiveに比較すると、
    # そのlinkをsite root基準で解決して「有効」と誤判定する。
    base_path = _read_base_path(pages_config_full_path)

    problems = []
    # rootがreparse pointなら、target treeを走査する前に停止する。走査後の報告では
    # SiteRoot外の内容を読んだ後になり、公開境界の検査順として遅い。
    if guards.is_reparse_point(site_root_path):
        raise guards.ValidationError(
            "Symbolic or reparse-point output is not allowed at root: "
            + guards.path_relative_to_root(site_root_path, site_root_path)
        )

    output_items = list(guards.iter_tree(site_root_path))
    for path, _is_directory in output_items:
        # fileだけでなくdirectoryとSiteRoot自身も検査する。directory reparse pointを
        # 見落とすと、lexicalには_site内でも実体が外にあるfileをlink先として受理し得る。
        if guards.is_reparse_point(path):
            problems.append(
                "Symbolic or reparse-point output is not allowed: "
                + guards.path_relative_to_root(path, site_root_path)
            )

    files = [path for path, is_directory in output_items if not is_directory]
    published_file_paths = set()
    for path in files:
        published_file_paths.add(guards.full_path(path))
        relative_file = guards.path_relative_to_root(path, site_root_path)

        # 拡張子の判定はcase-insensitiveのままにする。`.PDF`も拒否し、`.HTML`も
        # scan対象に含めるためであり、広く拾う方向がfail-safeになる。
        # 一方、pathのcontainment、base path、存在するfileの突き合わせはcase-sensitiveにする。
        extension = guards.get_extension(path).lower()
        if extension == ".pdf":
            # allowlistでも弾けるが、専用の診断を残す。PDFは過去に明示的な拒否対象と
            # されており、理由の分かるmessageで落とす方が調べやすい。
            problems.append(f"PDF output is not allowed: {relative_file}")
        elif extension not in guards.ALLOWED_EXTENSIONS and not _is_license(path):
            # `.pages-src`側と同じallowlistを最終artifactへも課す。2026-08-08時点の
            # `_site`は`.html .md .css .ico .jpg`と`LICENSE`だけであり、すべて許可済みの
            # ためこの判定は現状no-opである。Jekyllやpluginが将来別の拡張子を生成した
            # ときに、気付かないまま公開せずfail-closedで止めるために置く。
            problems.append(f"File type is not approved for Pages: {relative_file}")

        if os.path.getsize(path) > guards.FILE_SIZE_LIMIT:
            # 上限は`.pages-src`側と同じ`FILE_SIZE_LIMIT`を使う。2026-08-08時点の
            # `_site`の最大は205894 byteであり、こちらも現状no-opである。
            problems.append(f"File exceeds the Pages size limit: {relative_file}")

    # 存在確認だけではWindows上でcase違いのfileを存在扱いにする。実際に列挙した
    # fileのcase-sensitiveな集合と突き合わせ、LinuxのPagesと同じ結果にする。
    for relative_path in REQUIRED_FILES:
        candidate = guards.full_path(os.path.join(site_root_path, relative_path))
        if candidate not in published_file_paths or not os.path.isfile(candidate):
            problems.append(f"Required output is missing: {relative_path}")

    # 下限はpublish_guardsが持つ。ここで再定義しない。
    html_files = [
        path for path in files if guards.get_extension(path).lower() == ".html"
    ]
    if len(html_files) < guards.MINIMUM_PUBLISHED_COUNT:
        problems.append(
            f"Unexpectedly small HTML set: {len(html_files)}"
            f" (minimum {guards.MINIMUM_PUBLISHED_COUNT})"
        )

    # 公開禁止patternはHTMLだけでなく、生成CSS／SVG等を含む全text出力へ適用する。
    # source側はprepare_pages.pyが検査するが、Jekyll変換後にだけ現れる内容もあるため、
    # 最終artifactを同じ共有拡張子で再検査する。
    sensitive_text_files = set()
    for path in files:
        # 判定は`is_scanned_for_secrets`へ集約する。summaryの`UNSCANNED=`が同じ関数を
        # 使うため、診断と実際のscan範囲が食い違わない。
        if not is_scanned_for_secrets(path):
            continue
        content = guards.get_file_text(path)
        relative_file = guards.path_relative_to_root(path, site_root_path)
        if guards.secret_like(content):
            problems.append(f"Secret-like content detected: {relative_file}")
            sensitive_text_files.add(path)
        if guards.personal_path(content):
            problems.append(f"Personal absolute path detected: {relative_file}")
            sensitive_text_files.add(path)

    # 生成HTMLのid属性。fragment付きlinkの検証に使う。
    # 元Markdownのanchorはvalidate_doc_links.pyが検査するが、生成側のid生成規則
    # （kramdownのauto_ids）がGitHubのanchor規則と一致する保証はない。
    # 食い違えば、sourceで通ったlinkが生成siteで解決しない。
    ids_by_file = {}
    attributes_by_file = {}
    scannable_html_count = 0
    for html_path in html_files:
        if html_path in sensitive_text_files:
            attributes_by_file[html_path] = []
            ids_by_file[html_path] = set()
            continue
        scannable_html_count += 1
        content = guards.get_file_text(html_path)
        # text nodeに表示された`href=&quot;...&quot;`やcomment内の例示を、実際の
        # navigation属性として扱わない。scannerはcomment、raw-text／RCDATA、quoted属性を
        # 一度の線形走査で区別し、raw-text要素自身の`src`等は開始tagとして保持する。
        identifiers = set()
        attributes = []
        for tag in _html_start_tags(content):
            for name, value in _start_tag_attributes(tag):
                attributes.append((name, value))
                if name.lower() == "id" and value:
                    identifiers.add(guards.html_decode(value))
        attributes_by_file[html_path] = attributes
        ids_by_file[html_path] = identifiers

    # idが1件も取れない場合、生成物ではなく検査側が壊れている可能性が高い。
    # 実際に一度、patternへ制御文字が混入して全fileで0件になり、
    # 25件のanchorをすべて誤検出した。0件を正常として通さない。
    if scannable_html_count > 0 and sum(
        len(identifiers) for identifiers in ids_by_file.values()
    ) == 0:
        raise guards.ValidationError(
            "No id attributes found in any generated HTML. The anchor check is not working."
        )

    _check_links(
        html_files,
        attributes_by_file,
        ids_by_file,
        sensitive_text_files,
        published_file_paths,
        site_root_path,
        base_path,
        problems,
    )

    if problems:
        for problem in guards.sort_unique(problems):
            print(problem, file=sys.stderr)
        raise guards.ValidationError(
            f"Pages output validation failed with {len(problems)} problem(s)."
        )

    print("SITE_ROOT=.")
    print(f"FILES={len(files)} HTML={len(html_files)} BROKEN_LINKS=0")
    for line in _summarize_output(files, site_root_path):
        print(line)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except guards.ValidationError as error:
        print(error, file=sys.stderr)
        sys.exit(1)
