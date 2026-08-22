#!/usr/bin/env python3
"""公開対象を`.pages-src/`へ複製し、公開禁止情報を検査する。

診断のfile pathはstaging-root相対で出力する。localの絶対pathをCI logへ残さない。
"""

import json
import os
import re
import shutil
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / "lib"))

import publish_guards as guards  # noqa: E402

# staging先として許すdirectory名。`_remove_staging`が再帰削除を行う前に、
# 消そうとしている対象が本当にこれかを確認する。
STAGING_DIRECTORY_NAME = ".pages-src"

# Windowsのdrive指定。manifestの`path`判定で使う。
DRIVE_LETTER_RE = re.compile(r"^[A-Za-z]:")

PORTAL_FILES = ("_config.yml", "index.md", "404.md")

# 自前layoutは`pages/_layouts/`へ置き、ここに列挙したexact pathだけを公開する
# （ADR-0009）。`pages/assets/`と同じ考え方であり、列挙外のfileがdirectoryに
# あればbuildを失敗させる。「置いたのに公開されない」も「置いたら黙って公開
# される」も作らない。
#
# `default.html`は必須である。`jekyll-default-layout`はfront matterを持たない
# Markdownへ`page`が無ければ`default`を割り当てるため、これが欠けると`docs/`
# 配下の約40 pageがlayoutなしで生成される。
LAYOUT_DIRECTORY = "_layouts"
PORTAL_LAYOUTS = ("default.html", "home.html", "page.html")

# faviconの意匠。concept画像（pages/assets/deskcat-concept.jpg）の猫顔を
# pixel artへ落としたものであり、1文字が1 pixelである。巨大なbyte列を貼ると
# diffでreviewできないため、この形のままsourceへ置いて組み立てる。
#
# browserの縮小は汚いため、32 x 32と16 x 16を別々に描いて1つのICOへ入れる。
# 16 x 16は頭の輪郭、内耳、目だけへ簡略化している。
FAVICON_PALETTE = {
    ".": (0x00, 0x00, 0x00, 0x00),  # 透過
    "o": (0x8A, 0x6A, 0x5C, 0xFF),  # 輪郭。cream地を明るいtabから切り離す
    "C": (0xFA, 0xF0, 0xE8, 0xFF),  # 頭部のcream
    "c": (0xEC, 0xD9, 0xCD, 0xFF),  # 顎の陰
    "P": (0xF0, 0xA6, 0xB8, 0xFF),  # 内耳
    "D": (0x2B, 0x21, 0x1E, 0xFF),  # face panel
    "W": (0xFF, 0xFF, 0xFF, 0xFF),  # ハート型の目
    "p": (0xE0, 0x8B, 0xA0, 0xFF),  # 鼻
}

FAVICON_ART_32 = (
    "................................",
    ".........o............o.........",
    "........oCo..........oCo........",
    ".......oCCCo........oCCCo.......",
    ".......oCCCo........oCCCo.......",
    "......oCCPCCo......oCCPCCo......",
    "......oCPPPCo......oCPPPCo......",
    ".....oCCPPPCCooooooCCPPPCCo.....",
    "....oCCPCCCCCCCCCCCCCCCCPCCo....",
    "....oCCCCCCCCCCCCCCCCCCCCCCo....",
    "...oCCCCCCCCCCCCCCCCCCCCCCCCo...",
    "...oCCCCCCCCCCCCCCCCCCCCCCCCo...",
    "..oCCCCCCCCCCCCCCCCCCCCCCCCCCo..",
    "..oCCCCCCCCCCCCCCCCCCCCCCCCCCo..",
    ".oCCCCCCCCCDDDDDDDDDDCCCCCCCCCo.",
    ".oCCCCCCCDDDDDDDDDDDDDDCCCCCCCo.",
    ".oCCCCCCDDDDDDDDDDDDDDDDCCCCCCo.",
    ".oCCCCCCDDDWDWDDDDWDWDDDCCCCCCo.",
    ".oCCCCCDDDWWWWWDDWWWWWDDDCCCCCo.",
    ".oCCCCCDDDWWWWWDDWWWWWDDDCCCCCo.",
    ".oCCCCCDDDDWWWDDDDWWWDDDDCCCCCo.",
    ".oCCCCCDDDDDWDDDDDDWDDDDDCCCCCo.",
    ".oCCCCCCDDDDDDDDDDDDDDDDCCCCCCo.",
    ".oCCCCCCDDDDDDDppDDDDDDDCCCCCCo.",
    "..oCCCCCCDDDDDDDDDDDDDDCCCCCCo..",
    "..oCCCCCCCCDDDDDDDDDDCCCCCCCCo..",
    "...oCCCCCCCCCCCCCCCCCCCCCCCCo...",
    "....oCCCCCccccccccccccCCCCCo....",
    ".....oCCCccccccccccccccCCCo.....",
    "......ooCCccccccccccccCCoo......",
    "........oooooooooooooooo........",
    "................................",
)

FAVICON_ART_16 = (
    "...oCo....oCo...",
    "...oCo....oCo...",
    "..oCPCo..oCPCo..",
    ".oCCPCCooCCPCCo.",
    ".oCPPPCooCPPPCo.",
    ".oCCCCCCCCCCCCo.",
    "oCCCCCCCCCCCCCCo",
    "oCCCDDDDDDDDCCCo",
    "oCCDDDDDDDDDDCCo",
    "oCCDWDWDDWDWDCCo",
    "oCCDWWWDDWWWDCCo",
    "oCCDDWDDDDWDDCCo",
    "oCCCDDDDDDDDCCCo",
    ".oCCCCCCCCCCCCo.",
    "..oCCCCCCCCCCo..",
    "...oooooooooo...",
)

# hashで固定しないtext asset。diff reviewと内容scanの対象であり、
# 編集ごとにhashが変わるだけなのでmanifestへ記録しない。
TEXT_ASSET_EXTENSIONS = (".css", ".scss", ".svg", ".txt")


def _copy(source, destination):
    os.makedirs(os.path.dirname(destination), exist_ok=True)
    shutil.copyfile(source, destination)


def is_unsafe_manifest_path(relative):
    """manifestの`path`がstaging先を`assets/`の外へ逃がしうる形かを判定する。

    規則をOSに依存させない。`os.path.isabs`はPOSIXで`C:\\x`を相対pathとして通すため、
    それに任せると判定がOSごとに変わる。drive letter、slash、backslashを明示して、
    どのOSでも同じ形を拒否する。公開境界の判定は環境非依存でfail-closedに保つ。

    `os.path.isabs`は使わない。上の3つが、それが拒否していた形（`/x`、`\\x`、UNC、
    `C:\\x`）をすべて覆う。冗長に残すとWindowsでは常にそちらが先に一致し、明示rule
    のどれを壊しても結果が変わらないため、回帰をtestで捕まえられなくなる。

    関数として切り出してあるのは、この規則をOSに依存せず直接testできるようにするため。
    """
    return bool(
        not relative.strip()
        or ".." in relative
        or relative.startswith("/")
        or relative.startswith("\\")
        or DRIVE_LETTER_RE.match(relative)
    )


def link_rejection(repository_relative, source, tracked_symlinks):
    """copyしてよいsourceかを判定し、駄目なら理由を返す。問題なければNone。

    `.pages-src/`へfileを入れる経路は4つある（portal file、asset、root document、
    `docs/`）。どの経路でも、symlinkとreparse pointは同じ理由で拒否する。
    `os.path.isfile`はlinkを辿り、copyはtarget側の内容を書き出す。staging先は通常file
    になるため、stagingの最後にあるreparse point検査では捕まらない。binaryのSHA-256も
    辿った先から計算されて一致する。link 1本でrepository外の内容が公開できてしまう。

    判定を1箇所に集めるのは、経路ごとに書くと今回のように一部だけguardが抜けるからである。
    実際、移行元のPowerShell実装は`docs/`にだけguardを持っていた。

    Gitのmodeを先に見る。`core.symlinks=false`のcheckoutでは、working tree上の実体が
    regular fileになり、属性だけでは判定が環境ごとに変わる。
    """
    if repository_relative in tracked_symlinks:
        return "is a symlink in Git"
    if guards.is_reparse_point(source):
        return "is a reparse point"
    return None


def _remove_staging(path, repository_root):
    """既存のstaging pathを、file・directory・reparse pointのどれでも取り除く。

    再帰削除の前に、消そうとしている対象がrepository内の`.pages-src`であることを
    確認する。pathの組み立てを将来変えたときに、別のdirectoryを消してしまわないための
    guardである。比較はcase-sensitiveにする。`path_within_root`が同じ方針であり、
    ここだけcase-insensitiveにすると判定基準が食い違う。
    """
    if os.path.basename(path) != STAGING_DIRECTORY_NAME or not guards.path_within_root(
        path, repository_root
    ):
        raise guards.ValidationError("Unexpected Pages staging path.")
    if not guards.remove_tree(path):
        raise guards.ValidationError("Unable to clear the Pages staging directory.")


def _favicon_image(art):
    """1枚のASCII artを、32-bit BGRAのBMP-in-ICO imageへ変換する。

    ICO内のBMPは行がbottom-upで、`biHeight`はXOR maskとAND maskの合計を表すため
    実寸の2倍になる。AND maskは1 bit per pixelで、行を4 byte境界へ揃える。
    32-bit BGRAではalpha channelが効くが、alphaを見ない古いrendererのために
    maskも正しく書く。
    """
    height = len(art)
    if not height:
        raise guards.ValidationError("Favicon art has no rows.")
    width = len(art[0])
    if any(len(row) != width for row in art):
        raise guards.ValidationError("Favicon art rows have inconsistent width.")
    if width != height:
        raise guards.ValidationError("Favicon art must be square.")
    # ICOのdirectoryは寸法を1 byteで持ち、256だけを0で表す。257以上は表現できず、
    # 黙って0（=256）として書き出すと寸法の宣言が実体と食い違う。
    if width > 256:
        raise guards.ValidationError("Favicon art must not exceed 256 pixels.")

    pixels = bytearray()
    mask = bytearray()
    mask_row_bytes = ((width + 31) // 32) * 4
    for row in reversed(art):
        for character in row:
            if character not in FAVICON_PALETTE:
                raise guards.ValidationError(
                    "Favicon art uses a character that is not in the palette."
                )
            red, green, blue, alpha = FAVICON_PALETTE[character]
            pixels += bytes((blue, green, red, alpha))
        bits = bytearray(mask_row_bytes)
        for index, character in enumerate(row):
            # AND maskは1が透過、0が不透過である。
            if FAVICON_PALETTE[character][3] == 0:
                bits[index // 8] |= 0x80 >> (index % 8)
        mask += bits

    header = struct.pack(
        "<IiiHHIIiiII",
        40,  # biSize
        width,
        height * 2,  # biHeight。XORとANDの合計
        1,  # biPlanes
        32,  # biBitCount
        0,  # biCompression
        len(pixels) + len(mask),  # biSizeImage
        0,  # biXPelsPerMeter
        0,  # biYPelsPerMeter
        0,  # biClrUsed
        0,  # biClrImportant
    )
    return header + bytes(pixels) + bytes(mask)


def build_favicon(arts=(FAVICON_ART_32, FAVICON_ART_16)):
    """複数寸法のpixel artを1つのICOへまとめる。

    以前のfaviconは1 x 1の単色placeholderで、themeのlayoutが`<link rel="icon">`を
    コメントアウトしていたため実際には使われていなかった。自前layoutがlinkを持つ
    ため、ここで生成する内容がそのままtabへ出る。
    """
    images = [_favicon_image(art) for art in arts]
    offset = 6 + 16 * len(images)
    directory = bytearray(struct.pack("<HHH", 0, 1, len(images)))
    for art, image in zip(arts, images):
        size = len(art)
        # 256だけを0で表す。257以上は`_favicon_image`が拒否している。
        stored = 0 if size == 256 else size
        directory += struct.pack(
            "<BBBBHHII", stored, stored, 0, 0, 1, 32, len(image), offset
        )
        offset += len(image)
    return bytes(directory) + b"".join(images)


def _stage_layouts(repository_root, portal_root, output_root, tracked_symlinks):
    """自前layoutを、`PORTAL_LAYOUTS`が列挙したexact pathだけ公開する。

    `pages/assets/`と同じ規則を課す。存在、Gitの追跡、symlink／reparse point、
    拡張子を確認し、列挙外のfileがdirectoryにあれば失敗させる。再帰copyにすると、
    reviewを経ていないlayoutが公開経路へ入る。layoutは全pageのHTMLを決めるため、
    assetよりも影響が大きい。
    """
    layouts_source = os.path.join(portal_root, LAYOUT_DIRECTORY)
    layouts_destination = os.path.join(output_root, LAYOUT_DIRECTORY)

    if not os.path.isdir(layouts_source):
        raise guards.ValidationError(
            f"Required Pages layouts directory is missing: pages/{LAYOUT_DIRECTORY}"
        )

    tracked_layouts = guards.get_tracked_files(
        repository_root, f"pages/{LAYOUT_DIRECTORY}"
    )

    problems = []
    os.makedirs(layouts_destination, exist_ok=True)
    for name in PORTAL_LAYOUTS:
        repository_relative = f"pages/{LAYOUT_DIRECTORY}/{name}"
        source = os.path.join(layouts_source, name)

        if guards.get_extension(name).lower() != ".html":
            problems.append(f"Declared layout must be HTML: {repository_relative}")
            continue
        if not os.path.isfile(source):
            problems.append(f"Declared layout is missing: {repository_relative}")
            continue
        if repository_relative not in tracked_layouts:
            problems.append(
                f"Declared layout is not tracked by Git: {repository_relative}"
            )
            continue
        rejection = link_rejection(repository_relative, source, tracked_symlinks)
        if rejection:
            problems.append(f"Declared layout {rejection}: {repository_relative}")
            continue

        _copy(source, os.path.join(layouts_destination, name))

    # 列挙外のfileを検知する。追跡状態にかかわらず失敗させ、localとCIで同じ結果に
    # する。`pages/_layouts/`は少数のreview済みlayoutだけを置く場所である。
    for path in guards.iter_files(layouts_source):
        on_disk = guards.path_relative_to_root(path, layouts_source)
        if on_disk not in PORTAL_LAYOUTS:
            problems.append(
                "Layout is not declared in PORTAL_LAYOUTS:"
                f" pages/{LAYOUT_DIRECTORY}/{on_disk}"
            )

    if problems:
        for problem in problems:
            print(problem, file=sys.stderr)
        raise guards.ValidationError(
            f"Pages layout validation failed with {len(problems)} problem(s)."
        )


def _stage_assets(repository_root, portal_root, output_root, tracked_symlinks):
    """Pages固有のassetを、review済みmanifestが列挙したexact pathだけ公開する。

    再帰copyにすると、許可拡張子でありreviewを経ていないfileまで公開され得る。
    特にbinaryは内容scanが効かないため、hashで同一性を固定する。

    symlinkとreparse pointは`docs/`側と同じく拒否する。`os.path.isfile`はlinkを辿り、
    copyはtarget側の内容を書き出すため、`pages/assets/`配下のsymlink 1本で
    repository外の内容が公開される。staged側は通常fileになるので、`main`の
    reparse point検査では捕まえられない。binaryのSHA-256も辿った先から計算されるため
    一致してしまう。ここで止めるしかない。
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
    # 壊れたmanifestはtracebackではなく診断で落とす。例外文にはlocal絶対pathや
    # manifestの中身が載りうるため、repository相対pathだけを報告する。
    try:
        manifest = json.loads(guards.get_file_text(manifest_path))
    except (json.JSONDecodeError, ValueError) as error:
        raise guards.ValidationError(
            "Pages asset manifest is not valid JSON: pages/assets-manifest.json"
        ) from error
    if not isinstance(manifest, dict) or "assets" not in manifest:
        raise guards.ValidationError(
            "Pages asset manifest has no Assets key: pages/assets-manifest.json"
        )
    # 形を先に確認する。`assets`がlistでない、entryがobjectでない場合、
    # 下の`"path" not in entry`はstringに対する部分一致へ退化し、宣言していない
    # assetを宣言済みとして通しうる。公開境界の判定を型で守る。
    declared_entries = manifest["assets"]
    if not isinstance(declared_entries, list) or not all(
        isinstance(entry, dict) for entry in declared_entries
    ):
        raise guards.ValidationError(
            "Pages asset manifest Assets must be a list of objects:"
            " pages/assets-manifest.json"
        )

    # Git追跡対象だけを公開する。追跡外のfileをmanifestへ書いても公開しない。
    # manifestのpathはslash区切りで比較するため、区切り文字を変換しない。
    tracked_assets = guards.get_tracked_files(repository_root, "pages/assets")

    problems = []
    declared_assets = set()

    os.makedirs(assets_destination, exist_ok=True)
    for entry in declared_entries:
        if "path" not in entry:
            problems.append("Asset manifest entry has no Path.")
            continue
        relative = str(entry["path"])

        # Manifestの`path`でstaging先を`assets/`の外へ逃がせないようにする。
        if is_unsafe_manifest_path(relative):
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
        rejection = link_rejection(repository_relative, source, tracked_symlinks)
        if rejection:
            problems.append(f"Declared asset {rejection}: {repository_relative}")
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

        # `docs/`は必須fileではないため、失敗ではなくskipとして記録する。
        # 判定そのものは他経路と同じhelperを使う。
        rejection = link_rejection(repository_relative, path, tracked_symlinks)
        if rejection == "is a symlink in Git":
            skipped.append(f"{relative} (symlink in Git)")
            continue
        if rejection == "is a reparse point":
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
    output_root = guards.full_path(
        os.path.join(repository_root, STAGING_DIRECTORY_NAME)
    )

    _remove_staging(output_root, repository_root)
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
        rejection = link_rejection(f"pages/{name}", source, tracked_symlinks)
        if rejection:
            raise guards.ValidationError(
                f"Required Pages source {rejection}: pages/{name}"
            )
        _copy(source, os.path.join(output_root, name))

    _stage_layouts(repository_root, portal_root, output_root, tracked_symlinks)
    _stage_assets(repository_root, portal_root, output_root, tracked_symlinks)

    with open(os.path.join(output_root, "favicon.ico"), "wb") as handle:
        handle.write(build_favicon())

    for name in guards.ROOT_DOCUMENTS:
        source = os.path.join(repository_root, name)
        if not os.path.isfile(source):
            raise guards.ValidationError(f"Required root document is missing: {name}")
        if name not in tracked_repository_files:
            raise guards.ValidationError(
                f"Required root document is not tracked by Git: {name}"
            )
        rejection = link_rejection(name, source, tracked_symlinks)
        if rejection:
            raise guards.ValidationError(f"Required root document {rejection}: {name}")
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

        if os.path.getsize(path) > guards.FILE_SIZE_LIMIT:
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
