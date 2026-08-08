#!/usr/bin/env python3
"""リポジトリ内のMarkdown相対linkを検査する。

Pages workflowのlink checkは、公開対象（rootのMarkdownと`docs/`）だけを、
HTML生成後に検査する。このscriptはそれを補い、`.github/`や各componentの
READMEを含むリポジトリ全体を、生成前のMarkdownとして検査する。

`pages/index.md`は`.pages-src/`のroot基準で書かれているため、
repositoryのdirectory構造では解決できない。専用の基準pathで検査する。
"""

import argparse
import hashlib
import os
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))

import publish_guards as guards  # noqa: E402

HEADING_RE = re.compile(r"^#{1,6}\s+(?P<heading>.+?)\s*$")
EXTERNAL_SCHEME_RE = re.compile(r"^(?:https?|mailto|tel|data):", re.IGNORECASE)
PORTAL_RE = re.compile(r"^pages/(?:index|404)\.md$")


def _zero_tracked_diagnostics(root):
    """0件のときは、gitが何を見ているかを添える。

    pathspecの解釈、repository rootの解決、indexの中身のどれが原因かをlogから切り分ける。
    """
    all_tracked = subprocess.run(
        ["git", "-C", root, "ls-files"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    # stderrも混ぜる。gitがrepositoryを見つけられない場合、原因はstdoutではなく
    # error文言にしか現れない。切り分けのための診断なので握りつぶさない。
    tracked_lines = [
        line
        for line in (all_tracked.stdout + all_tracked.stderr).splitlines()
        if line
    ]
    top_level = subprocess.run(
        ["git", "-C", root, "rev-parse", "--show-toplevel"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    top_level_output = " ".join(top_level.stdout.split())
    top_level_matches_root = False
    if top_level.returncode == 0 and top_level_output.strip():
        candidate = guards.full_path(top_level_output)
        if os.name == "nt":
            top_level_matches_root = candidate.lower() == root.lower()
        else:
            top_level_matches_root = candidate == root
    git_version = subprocess.run(
        ["git", "--version"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
    )
    return (
        "Unable to enumerate tracked Markdown files.\n"
        f"  ls-files(all) exit={all_tracked.returncode} count={len(tracked_lines)}"
        f" -> {', '.join(tracked_lines)}\n"
        f"  rev-parse exit={top_level.returncode} matches-root={top_level_matches_root}\n"
        f"  {' '.join(git_version.stdout.split())}"
    )


def _passes_through_symlink(target_normalized, tracked_symlinks):
    """target自身か、その途中のdirectoryがGit上のsymlinkかを返す。

    `prepare_pages.py`はreparse point配下へ降りないため、symlinkを経由するpathは
    複製されない。一方`os.path.exists`はlinkを辿るので、存在確認だけでは公開対象と
    区別できない。判定はfilesystemではなくGitのmode（120000）で行う。作業ツリー上の
    実体は`core.symlinks=false`のcheckoutで変わるが、indexのmodeは環境に依存しない。

    `target_normalized`はroot相対のslash区切りである。途中のdirectoryも見るのは、
    `docs/linkdir/target.md`のようにlinkがdirectory側に付いている場合を拾うため。
    """
    parts = target_normalized.split("/")
    for index in range(1, len(parts) + 1):
        if "/".join(parts[:index]) in tracked_symlinks:
            return True
    return False


def _collect_anchors(markdown_files):
    """各fileの見出しから生成されるanchor集合を作る。

    linkのfragmentがここに無ければ、生成siteでpage内jumpが解決しない。
    fileが存在するだけでは検出できないため、link先の存在確認とは別に突き合わせる。
    過去に見出しの改名で2度壊している。
    """
    anchors_by_file = {}
    for path in markdown_files:
        anchors = set()
        # GitHubは同一textの見出しが繰り返されると`-1`、`-2`と採番する。
        # 出現回数を数えないと、2つ目以降の見出しへのlinkを未解決と誤判定する。
        seen = {}
        # fenced code block内の`#`で始まる行はshell commentであり見出しではない。
        # 数えると偽のanchorが増え、同名見出しの`-1`／`-2`採番がずれる。
        # このrepositoryのgithub-wiki-home.mdは、実際にbash commentを10行以上含む。
        for line in guards.markdown_outside_fences(guards.get_file_text(path)):
            match = HEADING_RE.match(line)
            if not match:
                continue
            base = guards.heading_anchor(match.group("heading"))
            count = seen.get(base, 0)
            anchors.add(base if count == 0 else f"{base}-{count}")
            seen[base] = count + 1
        anchors_by_file[path] = anchors
    return anchors_by_file


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

    # 検査対象はGitが追跡しているMarkdownだけとする。
    # directory走査にすると、生成物、worktree（`.claude/worktrees/`等）、vendor、
    # 未追跡の作業用copyまで拾い、repositoryに存在しないfileで失敗する。
    problems = []
    checked = 0
    # 検査したlinkの一覧。件数だけでは、環境によって走査範囲が変わっても
    # `BROKEN=0`が揃うため同等性を確認できない。実際にWindowsで229件、
    # Linux CIで243件と食い違った。内容のdigestを出して環境間で突き合わせる。
    checked_targets = []
    scanned = 0

    # 追跡fileの列挙はpublish_guardsの共有helperを使う。ここで`git ls-files`を
    # 直接呼ぶと、同じ呼び出しとerror処理を2箇所で持つことになる。
    # helperはcase-sensitiveなsetを返すため、未解決merge中にstage 1/2/3で
    # 重複するpathもここで解消される。
    #
    # 拡張子の絞り込みはgitのpathspecに任せず、script側で行う。
    # 過去のLinux CIでglob pathspecを渡したhelperの結果が0件になったため、
    # pathspecのglob解釈や引数展開に依存しない形にして、環境差で走査対象が消えるのを防ぐ。
    tracked_symlinks = guards.get_tracked_symlinks(root)
    tracked_all = guards.get_tracked_files(root, ".")
    tracked = {
        entry
        for entry in tracked_all
        if entry not in tracked_symlinks
        and guards.get_extension(entry).lower() in guards.MARKDOWN_EXTENSIONS
    }
    if not tracked:
        raise guards.ValidationError(_zero_tracked_diagnostics(root))

    # symlinkは走査しない。`CLAUDE.md`は`AGENTS.md`へのsymlinkであり、
    # symlinkを解決する環境では同じ内容を2回走査して件数が二重になる。
    # 実際にWindows（`core.symlinks=false`）で229件、Linux CIで243件と食い違い、
    # 差の14件はすべて`CLAUDE.md`だった。
    #
    # 判定はfile属性ではなくGitのmode（120000）で行う。作業ツリー上の実体は
    # checkout環境で変わるため、属性で判定すると走査対象そのものが環境ごとに変わる。
    markdown_files = []
    for entry in tracked:
        candidate = guards.full_path(os.path.join(root, entry))
        if os.path.isfile(candidate):
            markdown_files.append(candidate)

    # 走査対象が0件なら、検査が働いていない。追跡fileの列挙か絞り込みが壊れている。
    # 実際に一度、helperの戻り値を配列で包んだことで列挙が集合自身になり、
    # 0件のまま「BROKEN=0」を報告した。0件を正常として通さない。
    if not markdown_files:
        raise guards.ValidationError(
            "No tracked Markdown files resolved. The link check is not working."
        )

    # Pagesへ複製されるfileと、そのうちHTML化されないもの。
    # 定義はpublish_guardsにあり、prepare_pages.pyのcopy対象と同一の値を使う。
    published_root_documents = guards.ROOT_DOCUMENTS
    unrendered_root_documents = guards.UNRENDERED_ROOT_DOCUMENTS

    anchors_by_file = _collect_anchors(markdown_files)

    for path in markdown_files:
        scanned += 1
        relative_file = guards.path_relative_to_root(path, root)
        normalized_file = relative_file
        # fenced code block内はlinkの例示でありlinkではない。走査対象から除く。
        # 見出し検出と同じhelperを使い、両者が同じ行を見ることを保証する。
        content = "\n".join(
            guards.markdown_outside_fences(guards.get_file_text(path))
        )

        # `pages/index.md`と`pages/404.md`はstaging後のroot基準で解決する。
        is_portal = PORTAL_RE.match(normalized_file) is not None
        base_directory = root if is_portal else os.path.dirname(path)

        # 公開対象の文書か。ここからのlinkは生成site上でも解決できなければならない。
        # 比較はcase-sensitiveにする。CIのcase-sensitive filesystemでは`Docs/`のような
        # pathを公開対象と誤判定しうる。一致しなければ「非公開」へ倒すfail-closedとする。
        is_published_source = (
            is_portal
            or normalized_file.startswith("docs/")
            or normalized_file in published_root_documents
        )

        # 生成siteでHTMLになる文書か。HTML化されない文書のlinkは生成siteに現れないため、
        # 「HTML化されないtargetを参照するな」という制約の対象外とする。
        # それらのlinkはGitHubのrepository画面でだけ解決され、そこでは正しく動く。
        is_rendered_source = is_published_source and (
            normalized_file not in unrendered_root_documents
        )

        for target_value in guards.markdown_link_targets(content):
            target = target_value.strip()

            if (
                EXTERNAL_SCHEME_RE.match(target)
                or target.startswith("//")
                or target.startswith("{{")
            ):
                continue

            parts = target.split("#", 1)
            link_path = parts[0]
            fragment = parts[1] if len(parts) > 1 else ""
            is_same_page_fragment = not link_path.strip() and bool(fragment.strip())
            if not link_path.strip() and not is_same_page_fragment:
                continue

            if is_same_page_fragment:
                candidate = path
            else:
                link_path = guards.unescape_data_string(link_path)
                if link_path.startswith("/"):
                    candidate = os.path.join(root, link_path.lstrip("/"))
                else:
                    candidate = os.path.join(base_directory, link_path)

            checked += 1
            checked_targets.append(f"{normalized_file}|{target}")
            if not os.path.exists(candidate):
                hint = " (resolved against staging root)" if is_portal else ""
                problems.append(f"Broken link in {relative_file}: {target}{hint}")
                continue

            # fragmentが見出しに対応するか。
            #
            # `anchors_by_file`は追跡下でsymlinkでないMarkdownだけを持つ。targetがそこに
            # 無いままskipすると、`CLAUDE.md`のようなsymlinkや追跡外のMarkdownへの
            # fragmentが無検査で通る。このscriptの他の判定はfail-closedであり、
            # ここだけfail-openにしない。検査できない事実を報告する。
            if fragment.strip():
                candidate_full = guards.full_path(candidate)
                wanted = guards.unescape_data_string(fragment).lower()
                if candidate_full in anchors_by_file:
                    if wanted not in anchors_by_file[candidate_full]:
                        problems.append(
                            f"Broken anchor in {relative_file}: {target}"
                            " (no matching heading)"
                        )
                        continue
                elif (
                    guards.get_extension(candidate_full).lower()
                    in guards.MARKDOWN_EXTENSIONS
                ):
                    problems.append(
                        f"Unverifiable anchor in {relative_file}: {target} "
                        "(target Markdown is not a tracked non-symlink file)"
                    )
                    continue

            # 同一page内のfragmentは上で検証済みであり、公開先もsource自身である。
            # repository rootからtarget pathを再分類する必要はない。
            if is_same_page_fragment:
                continue

            # 公開文書から非公開pathへの相対linkを、Jekyll build前に検出する。
            # `.github/`や`scripts/`はPagesへ複製されないため、生成siteでは解決できない。
            # validate_pages_output.pyも検出するが、そちらはbuild後でfeedbackが遅い。
            if not is_published_source:
                continue

            # repository外に実在するfileへのrelative link（`../../outside.md`など）は
            # 存在確認を通過する。root基準のrelative pathを作る前に範囲を確認しないと、
            # helperが例外になり、意図した診断ではなく検査自体のcrashになる。
            candidate_full = guards.full_path(candidate)
            if not guards.path_within_root(candidate_full, root):
                problems.append(
                    f"Published doc {relative_file} links outside the repository:"
                    f" {target} (use an absolute URL)"
                )
                continue
            target_normalized = guards.path_relative_to_root(candidate_full, root)
            # `docs/`配下でも、prepare_pages.pyが複製するのはMarkdownだけである。
            # 存在するだけで公開対象とみなすと、docs/配下の画像等へのlinkが
            # 生成siteで404になる。directory targetは生成siteのindexへ解決される。
            target_is_directory = os.path.isdir(candidate)
            target_extension = guards.get_extension(target_normalized).lower()
            # `docs`単体（`docs`、`docs/`、`./docs/`）はdirectory自身を指す。
            # prefix一致だけでは区切り以降を要求するため一致せず、実在するdirectoryを
            # 未公開と誤判定する。比較はcase-sensitiveにして、一致しなければ
            # 「未公開」へ倒すfail-closedを保つ。
            is_published_target = (
                (
                    (target_normalized == "docs" or target_normalized.startswith("docs/"))
                    and (
                        target_is_directory
                        or target_extension in guards.DOCS_COPY_EXTENSIONS
                    )
                )
                or target_normalized in published_root_documents
            )
            # symlinkを経由するpathは、存在しても複製されない。stagingはreparse point
            # 配下へ降りないためである。ここで見ないと、`os.path.exists`がlinkを辿って
            # 「公開対象」と判定し、生成siteで404になるlinkを通してしまう。
            # 生成site側の検査も`Unconverted Markdown link`として拾うが、そのmessageは
            # 「拡張子を直せ」と読めるため、原因である「未公開」をここで正しく報告する。
            if is_published_target and _passes_through_symlink(
                target_normalized, tracked_symlinks
            ):
                is_published_target = False
            if not is_published_target:
                problems.append(
                    f"Published doc {relative_file} links to unpublished path:"
                    f" {target} (use an absolute URL)"
                )
            elif is_rendered_source and target_normalized in unrendered_root_documents:
                problems.append(
                    f"Published doc {relative_file} links to a root document that is"
                    f" not rendered as HTML: {target} (use an absolute URL)"
                )

    if problems:
        for problem in guards.sort_unique(problems):
            print(problem, file=sys.stderr)
        raise guards.ValidationError(
            f"Documentation link validation failed with {len(problems)} problem(s)."
        )

    # 並び替えはOrdinalで行う。culture依存の比較はWindowsとLinuxで順序が変わり、
    # digestが一致しない。
    checked_targets.sort(key=guards.ordinal_sort_key)
    digest_source = "\n".join(checked_targets)
    digest = hashlib.sha256(digest_source.encode("utf-8")).hexdigest().upper()
    print(f"MARKDOWN={scanned} LINKS={checked} BROKEN=0 DIGEST={digest[:16]}")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except guards.ValidationError as error:
        print(error, file=sys.stderr)
        sys.exit(1)
