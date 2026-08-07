#!/usr/bin/env python3
"""公開対象を`.pages-src/`へ複製し、公開禁止情報を検査する。

診断のfile pathはstaging-root相対で出力する。localの絶対pathをCI logへ残さない。
"""

import json
import os
import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))

import publish_guards as guards  # noqa: E402

# Staging対象の全fileへ適用するsize上限。extension条件を付けない。
# `.svg`はtext扱いだがimage同様に大きくなり得るため、除外すると検査から漏れる。
FILE_SIZE_LIMIT = 1024 * 1024

# GitHub Pagesのthemeが各pageから参照するfaviconを、依存toolなしで生成する。
# 1 x 1 pixel、32-bit BGRAの最小ICOであり、公開文書のbuild成否だけに影響する。
FAVICON_BYTES = bytes(
    [
        0x00, 0x00, 0x01, 0x00, 0x01, 0x00,
        0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x20, 0x00,
        0x30, 0x00, 0x00, 0x00, 0x16, 0x00, 0x00, 0x00,
        0x28, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
        0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x20, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x66, 0x99, 0xCC, 0xFF, 0x00, 0x00, 0x00, 0x00,
    ]
)

PORTAL_FILES = ("_config.yml", "index.md", "404.md")

# hashで固定しないtext asset。diff reviewと内容scanの対象であり、
# 編集ごとにhashが変わるだけなのでmanifestへ記録しない。
TEXT_ASSET_EXTENSIONS = (".css", ".scss", ".svg", ".txt")


def _copy(source, destination):
    os.makedirs(os.path.dirname(destination), exist_ok=True)
    shutil.copyfile(source, destination)


def _stage_assets(repository_root, portal_root, output_root):
    """Pages固有のassetを、review済みmanifestが列挙したexact pathだけ公開する。

    再帰copyにすると、許可拡張子でありreviewを経ていないfileまで公開され得る。
    特にbinaryは内容scanが効かないため、hashで同一性を固定する。
    """
    assets_source = os.path.join(portal_root, "assets")
    assets_destination = os.path.join(output_root, "assets")
    manifest_path = os.path.join(portal_root, "assets-manifest.json")

    if not os.path.isdir(assets_source):
        raise guards.ValidationError(
            "Required Pages assets directory is missing: pages/assets"
        )
    if not os.path.isfile(manifest_path):
        raise guards.ValidationError(
            "Required Pages asset manifest is missing: pages/assets-manifest.json"
        )

    # JSONはdataだけを表現し、manifest内のcodeを実行する余地がない。
    with open(manifest_path, "r", encoding="utf-8") as handle:
        manifest = json.load(handle)
    if "assets" not in manifest:
        raise guards.ValidationError(
            "Pages asset manifest has no Assets key: pages/assets-manifest.json"
        )

    # Git追跡対象だけを公開する。追跡外のfileをmanifestへ書いても公開しない。
    # manifestのpathはslash区切りで比較するため、区切り文字を変換しない。
    tracked_assets = guards.get_tracked_files(repository_root, "pages/assets")

    problems = []
    declared_assets = set()

    os.makedirs(assets_destination, exist_ok=True)
    for entry in manifest["assets"]:
        if "path" not in entry:
            problems.append("Asset manifest entry has no Path.")
            continue
        relative = str(entry["path"])

        # Manifestの`path`でstaging先を`assets/`の外へ逃がせないようにする。
        if (
            not relative.strip()
            or ".." in relative
            or relative.startswith("/")
            or relative.startswith("\\")
            or os.path.isabs(relative)
        ):
            problems.append(
                f"Asset manifest Path is not a safe relative path: {relative}"
            )
            continue

        normalized = relative.replace("\\", "/")
        declared_assets.add(normalized)
        repository_relative = f"pages/assets/{normalized}"
        source = os.path.join(assets_source, *normalized.split("/"))

        if not os.path.isfile(source):
            problems.append(f"Declared asset is missing: {repository_relative}")
            continue
        if repository_relative not in tracked_assets:
            problems.append(
                f"Declared asset is not tracked by Git: {repository_relative}"
            )
            continue

        extension = guards.get_extension(normalized).lower()
        is_text_asset = extension in TEXT_ASSET_EXTENSIONS
        has_hash = "sha256" in entry

        if is_text_asset and has_hash:
            problems.append(
                f"Text asset must not declare Sha256: {repository_relative}"
            )
            continue
        if not is_text_asset:
            if not has_hash:
                problems.append(
                    f"Binary asset must declare Sha256: {repository_relative}"
                )
                continue
            actual_hash = guards.file_sha256(source)
            if actual_hash.lower() != str(entry["sha256"]).lower():
                problems.append(
                    "Asset SHA-256 does not match the manifest: "
                    f"{repository_relative} (actual {actual_hash})"
                )
                continue

        _copy(source, os.path.join(assets_destination, *normalized.split("/")))

    # Manifestに無いfileを検知する。追跡状態にかかわらず失敗させ、localとCIで
    # 同じ結果にする。`pages/assets/`は少数の選定済みassetだけを置く場所である。
    for path in guards.iter_files(assets_source):
        on_disk = guards.path_relative_to_root(path, assets_source)
        if on_disk not in declared_assets:
            problems.append(
                f"Asset is not declared in the manifest: pages/assets/{on_disk}"
            )

    if problems:
        for problem in problems:
            print(problem, file=sys.stderr)
        raise guards.ValidationError(
            f"Pages asset manifest validation failed with {len(problems)} problem(s)."
        )


def _stage_docs(repository_root, output_root, tracked_symlinks):
    """`docs/` はMarkdownだけを複製する。

    再帰的な一括copyにすると、docs/へ置いたfileが人手のreviewを経ずに公開される。
    特に画像はbinaryのため内容scanが効かず、EXIFや写り込みを検出できない。
    """
    docs_source = os.path.join(repository_root, "docs")
    docs_destination = os.path.join(output_root, "docs")
    if not os.path.isdir(docs_source):
        raise guards.ValidationError("Required docs directory is missing: docs")

    os.makedirs(docs_destination, exist_ok=True)
    skipped = []
    copied = 0

    # Gitが追跡しているfileだけを公開する。
    # CIはclean checkoutのため差は出ないが、local実行では未追跡の下書きがdocs/に
    # 残っていることがある。追跡状態で絞り、localとCIのstaging結果を一致させる。
    tracked_docs = guards.get_tracked_files(repository_root, "docs")

    for path in guards.iter_files(docs_source):
        relative = guards.path_relative_to_root(path, docs_source)
        repository_relative = guards.path_relative_to_root(path, repository_root)

        # Gitのmodeを先に見る。属性だけだと`core.symlinks=false`のcheckoutで
        # symlinkがregular fileとして複製され、環境ごとに公開物が変わる。
        if repository_relative in tracked_symlinks:
            skipped.append(f"{relative} (symlink in Git)")
            continue

        if guards.is_reparse_point(path):
            skipped.append(f"{relative} (reparse point)")
            continue

        if repository_relative not in tracked_docs:
            skipped.append(f"{relative} (not tracked by Git)")
            continue

        if guards.get_extension(path).lower() not in guards.DOCS_COPY_EXTENSIONS:
            skipped.append(f"{relative} (not Markdown)")
            continue

        _copy(path, os.path.join(docs_destination, *relative.split("/")))
        copied += 1

    return copied, skipped


def main(argv=None):
    del argv
    repository_root = guards.full_path(str(Path(__file__).resolve().parent.parent))
    output_root = guards.full_path(os.path.join(repository_root, ".pages-src"))
    expected_output = guards.full_path(os.path.join(repository_root, ".pages-src"))

    if output_root.lower() != expected_output.lower():
        raise guards.ValidationError("Unexpected Pages staging path.")

    if os.path.exists(output_root):
        shutil.rmtree(output_root)
    os.makedirs(output_root)

    # Gitのindexにpathが存在することを確認するguardを、portal fileとroot documentにも
    # 適用する。これは追跡外pathを公開対象にしないための判定であり、working treeの内容が
    # commit済み・review済みであることまでは証明しない。Production deployはmainのcleanな
    # CI checkoutだけから行い、内容のreview境界はそちらで担保する。
    tracked_repository_files = guards.get_tracked_files(repository_root, ".")

    # Gitがsymlinkとして記録しているpath。file属性で判定すると、`core.symlinks=false`の
    # checkoutではregular fileに見えるため、複製するかどうかが環境ごとに変わる。
    # indexのmodeは環境に依存しない。
    tracked_symlinks = guards.get_tracked_symlinks(repository_root)

    portal_root = os.path.join(repository_root, "pages")
    for name in PORTAL_FILES:
        source = os.path.join(portal_root, name)
        if not os.path.isfile(source):
            raise guards.ValidationError(f"Required Pages source is missing: pages/{name}")
        if f"pages/{name}" not in tracked_repository_files:
            raise guards.ValidationError(
                f"Required Pages source is not tracked by Git: pages/{name}"
            )
        _copy(source, os.path.join(output_root, name))

    _stage_assets(repository_root, portal_root, output_root)

    with open(os.path.join(output_root, "favicon.ico"), "wb") as handle:
        handle.write(FAVICON_BYTES)

    for name in guards.ROOT_DOCUMENTS:
        source = os.path.join(repository_root, name)
        if not os.path.isfile(source):
            raise guards.ValidationError(f"Required root document is missing: {name}")
        if name not in tracked_repository_files:
            raise guards.ValidationError(
                f"Required root document is not tracked by Git: {name}"
            )
        _copy(source, os.path.join(output_root, name))

    copied, skipped = _stage_docs(repository_root, output_root, tracked_symlinks)

    problems = []
    files = list(guards.iter_files(output_root))
    license_path = os.path.join(output_root, "LICENSE")
    for path in files:
        relative_file = guards.path_relative_to_root(path, output_root)
        if guards.is_reparse_point(path):
            problems.append(
                f"Symbolic or reparse-point file is not allowed: {relative_file}"
            )

        extension = guards.get_extension(path).lower()
        # 拡張子allowlistの例外はroot直下の`LICENSE`1 fileに限る。file名だけで判定すると、
        # manifestが宣言した`assets/...license`のような拡張子なしpathも例外を通る。
        is_license = path == license_path
        if extension not in guards.ALLOWED_EXTENSIONS and not is_license:
            problems.append(f"File type is not approved for Pages: {relative_file}")
            continue

        if os.path.getsize(path) > FILE_SIZE_LIMIT:
            problems.append(f"File exceeds the Pages size limit: {relative_file}")

        if extension in guards.TEXT_EXTENSIONS or is_license:
            content = guards.get_file_text(path)
            if guards.secret_like(content):
                problems.append(f"Secret-like content detected: {relative_file}")
            if guards.personal_path(content):
                problems.append(f"Personal absolute path detected: {relative_file}")

    # 大量削除に気付くための下限。stagingするMarkdownが減ったら検知する。
    # 実際の件数から余裕を取った値であり、増えた分に追従して上げる必要はない。
    #
    # `docs/`の複製条件と同じ拡張子集合を使う。Jekyllがrenderするのはこの集合であり、
    # 二箇所で列挙すると、拡張子を増やしたときに件数checkだけ追従しない。
    markdown_count = sum(
        1
        for path in files
        if guards.get_extension(path).lower() in guards.DOCS_COPY_EXTENSIONS
    )
    if markdown_count < guards.MINIMUM_PUBLISHED_COUNT:
        problems.append(
            f"Unexpectedly small Markdown set: {markdown_count}"
            f" (minimum {guards.MINIMUM_PUBLISHED_COUNT})"
        )

    if skipped:
        print("Skipped files under docs/ (not published):", file=sys.stderr)
        for entry in skipped:
            print(f"  {entry}", file=sys.stderr)

    if problems:
        for problem in problems:
            print(problem, file=sys.stderr)
        raise guards.ValidationError(
            f"Pages staging validation failed with {len(problems)} problem(s)."
        )

    print("PAGES_SOURCE=.pages-src")
    print(
        f"FILES={len(files)} MARKDOWN={markdown_count}"
        f" DOCS_COPIED={copied} DOCS_SKIPPED={len(skipped)}"
    )
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except guards.ValidationError as error:
        print(error, file=sys.stderr)
        sys.exit(1)
