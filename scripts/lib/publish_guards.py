"""公開前検査で共有する定数と判定。

prepare_pages.py、validate_pages_output.py、validate_doc_links.py、
test_link_validators.py がimportする。
同じpatternを複数scriptで再定義しない（GovernanceのSingle Source of Truth）。

このmoduleは単体で実行しない。
"""

import hashlib
import html
import os
import re
import shutil
import stat
import subprocess
import unicodedata
import urllib.parse

# Provider既知のtoken形式と秘密鍵。
# GitHubのsecret scanningと重複するが、ここではPagesへ出す直前の最終確認として使う。
#
# `sk-(?:[A-Za-z0-9]+-)*[A-Za-z0-9]{20,}`はReDoSに見えるが、そうではない。
# 反復部が末尾に`-`を要求し、`-`は`[A-Za-z0-9]`に含まれないため、各反復の範囲は
# 次の`-`の位置で一意に決まる。分割の曖昧性がなく、`(a+)+`型のbacktrackが起きない。
#
# 2026-07-30の実測（.NET regex、非マッチ入力）:
#   `sk-` + `a-` x320 + `!`（644 char）  0.08 ms
#   `sk-` + `aaaaaaaaaa-` x80 + `!`（884 char）  0.08 ms
# 入力長を16倍にしても横ばいで、指数的増加は観測されない。
# 反復部の文字クラスへ`-`を追加する場合は曖昧になるため、この前提が崩れる。
#
# 最後の1件だけがcase-insensitiveかつ行頭起点である。PowerShell版の`(?im)`は
# その位置以降にだけ効いていた。Pythonは式の途中のglobal flagを許さないため、
# scoped group `(?i:...)`で同じ範囲へ限定し、`^`のためにMULTILINEをmodule全体へ与える。
SECRET_PATTERN = "|".join(
    [
        r"gh[pousr]_[A-Za-z0-9]{20,}",
        r"github_pat_[A-Za-z0-9_]{20,}",
        r"AKIA[0-9A-Z]{16}",
        r"AIza[0-9A-Za-z_\-]{35}",
        r"sk-(?:[A-Za-z0-9]+-)*[A-Za-z0-9]{20,}",
        r"xox[baprs]-[A-Za-z0-9-]{10,}",
        r"glpat-[A-Za-z0-9_\-]{20,}",
        r"-----BEGIN (?:RSA |EC |DSA |OPENSSH |PGP )?PRIVATE KEY-----",
        r"(?i:^\s*(?:password|passwd|secret|api[_-]?key|access[_-]?token|token)"
        r"\s*[:=]\s*\S{8,})",
    ]
)
SECRET_RE = re.compile(SECRET_PATTERN, re.MULTILINE)

# 個人を特定しうる絶対path。Windows、Linux、macOS、UNC、file scheme。
# `github.com/users/<name>`はGitHubのuser-owned resource（Projects等）の正規URL構造であり、
# ローカルのホームディレクトリpathではないため、直前がgithub.comの場合は除外する。
PERSONAL_PATH_PATTERN = "|".join(
    [
        r"[A-Za-z]:\\Users\\[^\\\s]+",
        r"/home/[^/\s]+",
        r"(?<!github\.com)/Users/[^/\s]+",
        r"\\\\[A-Za-z0-9._-]+\\[A-Za-z0-9$._-]+",
        r"file://",
    ]
)
PERSONAL_PATH_RE = re.compile(PERSONAL_PATH_PATTERN, re.IGNORECASE)

# Pagesへ出してよいfile拡張子。
ALLOWED_EXTENSIONS = (
    ".css",
    ".gif",
    ".html",
    ".ico",
    ".jpeg",
    ".jpg",
    ".markdown",
    ".md",
    ".png",
    ".scss",
    ".svg",
    ".txt",
    ".webp",
    ".yaml",
    ".yml",
)

# 内容scanの対象とするtext拡張子。
TEXT_EXTENSIONS = (
    ".css",
    ".html",
    ".markdown",
    ".md",
    ".scss",
    ".svg",
    ".txt",
    ".yaml",
    ".yml",
)

# Pagesのroot直下へ複製する文書。
# `prepare_pages.py`が複製し、`validate_doc_links.py`が公開対象の判定に使う。
# 二箇所で列挙すると、追加・削除の一方だけが追従する。追加を落とせば公開済み文書への
# linkを不当に失敗させ、削除を落とせば公開していない文書へのlinkを通してしまう。
ROOT_DOCUMENTS = ("README.md", "AGENTS.md", "CONTRIBUTING.md", "SECURITY.md", "LICENSE")

# 上のうち、複製はされるが生成siteでHTMLにならないもの。
# `README.md`はJekyllのreadme-indexとroot `index.md`が競合してpage URLを持たない。
# `CONTRIBUTING.md`も同様にHTML化されない（2026-07-29に公開siteで実測）。
# `LICENSE`はMarkdownではないためrender対象にならない。
# relative linkを張ると`.md`のまま残り、Pages output validationで失敗する。
# 参照する場合は絶対URLを使う。
UNRENDERED_ROOT_DOCUMENTS = ("README.md", "CONTRIBUTING.md", "LICENSE")

# 大量欠落に気付くための下限。stagingするMarkdownと、生成siteのHTMLの両方へ適用する。
# 実際の件数から余裕を取った値であり、増えた分に追従して上げる必要はない。
# 2箇所で別々に持つと、片方だけ更新して検知力が食い違う。
MINIMUM_PUBLISHED_COUNT = 35

# `docs/` から複製してよい拡張子。
# 画像はbinaryのため内容scanが効かない。承認なしに公開されることを防ぐため、
# ここでは複製せず、公開したい図版は pages/ 配下へ明示的に置く運用とする。
DOCS_COPY_EXTENSIONS = (".md", ".markdown")

# Markdownとして扱う拡張子。link検査の対象選別と、anchor検査が可能かどうかの
# 判定に使う。`DOCS_COPY_EXTENSIONS`とは目的が違うため別に定義する。
# 一方だけを変えたときに、もう一方が黙って追従しないようにする。
MARKDOWN_EXTENSIONS = (".md", ".markdown")

# Staging対象の全fileへ適用するsize上限。extension条件を付けない。
# `.svg`はtext扱いだがimage同様に大きくなり得るため、除外すると検査から漏れる。
# 上限を超えるfixtureを作るtestもこの値から大きさを決める。testが独自の定数を
# 持つと、上限を上げたときにtestだけが古い値のまま失敗する。
FILE_SIZE_LIMIT = 1024 * 1024


class ValidationError(Exception):
    """検査を継続できない状態。各scriptのmainがstderrへ出してexit 1にする。

    PowerShell版の`throw`に相当する。問題を1件ずつ集計するpathとは区別し、
    検査自体が成立しない場合にだけ使う。
    """


def get_file_text(path):
    """fileをtextとして読む。空fileは空文字列にする。

    PowerShellの`Get-Content -Raw`は空fileに対して`$null`を返し、そのまま正規表現へ
    渡すと例外になって「問題を報告して失敗」ではなく「検査自体がcrash」した。
    Python側でも同じ事故を避けるため、読み出しはここへ集約する。

    `newline=''`で改行を変換せず、fenceと見出しの走査がCRLFでも同じ行を見るようにする。
    UTF-8のBOMはPowerShellと同様に取り除く。PowerShellはUTF-16のBOMも判別するが、
    ここでは合わせない。`.gitattributes`が追跡textをUTF-8／LFへ正規化しており、
    UTF-16のsourceは想定しない。不正byteは置換文字にして、読み出しでは失敗させない。
    """
    with open(path, "r", encoding="utf-8-sig", newline="", errors="replace") as handle:
        return handle.read()


def _run_git(arguments):
    """gitを呼ぶ。実行file自体が無い場合も診断として報告する。

    `FileNotFoundError`のまま抜けるとtracebackになり、報告すべき前提条件の不足が
    stack traceに埋もれる。呼び出し側のmessageが想定しているのと同じ「gitが要る」
    という失敗なので、ここでValidationErrorへ揃える。
    """
    try:
        return subprocess.run(
            ["git", *arguments],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
        )
    except FileNotFoundError as error:
        raise ValidationError("Git is not available on PATH.") from error


def get_tracked_files(repository_root, pathspec):
    """指定pathspec配下でGitが追跡しているfileの集合を返す。

    比較はcase-sensitiveにする。CIのubuntu-24.04はcase-sensitive filesystemであり、
    case-insensitiveだとcase違いの未追跡fileを「追跡済み」と誤判定し、reviewを
    経ていないfileが公開される。一致しなければ「未追跡」へ倒すfail-closedとする。

    `--`でpathspecを明示する。省くと、環境によってpathspecが引数として解釈されず
    結果が空になることがある。`-`で始まるpathspecがoptionと誤解される問題も同時に防ぐ。

    `core.quotePath=false`を明示する。既定では非ASCIIのpathがdouble quoteと
    octal escapeで出力され、実pathではなくescape sequenceを保持してしまう。
    `get_tracked_symlinks`も同じ設定で読み、両者のpath表記を揃える。
    """
    if isinstance(pathspec, str):
        pathspec = [pathspec]
    result = _run_git(
        ["-C", repository_root, "-c", "core.quotePath=false", "ls-files", "--", *pathspec]
    )
    if result.returncode != 0:
        raise ValidationError(
            "Unable to enumerate tracked files under "
            f"{', '.join(pathspec)}. Run inside a Git checkout."
        )
    return {line for line in result.stdout.splitlines() if line}


_INDEX_SYMLINK_RE = re.compile(r"^120000\s+\S+\s+\d+\t(?P<path>.+)$")


def get_tracked_symlinks(repository_root):
    """Gitがsymlink（mode 120000）として記録しているpathの集合を返す。

    作業ツリー上の実体はcheckout環境で変わる。`core.symlinks=false`のWindowsでは
    link先pathを内容とするregular fileになるため、file属性で判定すると環境ごとに
    結果が変わる。indexのmodeは環境に依存しない。

    quotingは`get_tracked_files`と同じ設定にする。片方だけがescapeされたpathを返すと、
    symlink除外の突き合わせが非ASCII pathで一致しなくなる。
    """
    result = _run_git(
        ["-C", repository_root, "-c", "core.quotePath=false", "ls-files", "-s"]
    )
    if result.returncode != 0:
        raise ValidationError("Unable to read the Git index. Run inside a Git checkout.")

    symlinks = set()
    for line in result.stdout.splitlines():
        # `<mode> <sha> <stage>\t<path>`
        match = _INDEX_SYMLINK_RE.match(line)
        if match:
            symlinks.add(match.group("path"))
    return symlinks


def markdown_link_targets(content):
    """Markdown inline linkのtargetを出現順に返す。

    正規表現だけで文書全体から抽出すると、runtimeのpatch版が異なる環境で同一treeに
    対する走査件数が変わった。ここでは、このrepositoryが使用する`[text](target)`と
    `[text](target "title")`を線形走査し、runtime差を避ける。
    imageの`![text](target)`はlink検査の対象外とする。既存validatorと同じく、
    nested parenthesis、escaped closing parenthesis、single-quote titleは対象外である。
    """
    length = len(content)
    index = 0
    while index < length:
        if content[index] != "[":
            index += 1
            continue
        if index > 0 and content[index - 1] == "!":
            index += 1
            continue

        close_bracket = content.find("]", index + 1)
        if close_bracket < 0:
            break
        if close_bracket + 1 >= length or content[close_bracket + 1] != "(":
            index = close_bracket + 1
            continue

        target_start = close_bracket + 2
        target_end = target_start
        while (
            target_end < length
            and content[target_end] != ")"
            and not content[target_end].isspace()
        ):
            target_end += 1
        if target_end == target_start:
            index = close_bracket + 1
            continue

        close_parenthesis = -1
        if target_end < length and content[target_end] == ")":
            close_parenthesis = target_end
        elif target_end < length and content[target_end].isspace():
            cursor = target_end
            while cursor < length and content[cursor].isspace():
                cursor += 1
            if cursor < length and content[cursor] == '"':
                close_quote = content.find('"', cursor + 1)
                if (
                    close_quote >= 0
                    and close_quote + 1 < length
                    and content[close_quote + 1] == ")"
                ):
                    close_parenthesis = close_quote + 1

        if close_parenthesis < 0:
            index = close_bracket + 1
            continue

        yield content[target_start:target_end]
        index = close_parenthesis + 1


_FENCE_RE = re.compile(r"^\s*(?:```|~~~)")


def markdown_outside_fences(content):
    """fenced code block内の行を除いた行を、出現順に返す。

    fence内の`#`はshell commentであり見出しではない。fence内の`[a](b)`もlinkの例示である。
    見出し走査とlink走査で同じ判定を使う。2箇所で同じstate machineを持つと、
    片方だけを変えたときにanchor集合とlink集合が別の行を見ることになる。
    """
    in_fence = False
    for line in content.split("\n"):
        if _FENCE_RE.match(line):
            in_fence = not in_fence
            continue
        if not in_fence:
            yield line


# GitHubはunderscoreをslugへ残す。除去すると`#some_heading`が解決しない。
_ANCHOR_STRIP_RE = re.compile(r"[`*\[\]()!\"'.,:;/|<>?~^{}+=&%$#@]")
_ANCHOR_FULLWIDTH = ("／", "、", "。", "（", "）")
_ANCHOR_SPACE_RE = re.compile(r"\s+")


def heading_anchor(heading):
    """Markdown見出しからGitHubが生成するanchorを求める。

    小文字化し、記号を除去し、空白をhyphenへ変換する。CJKはそのまま残る。
    `validate_doc_links.py`がlinkのfragmentと突き合わせるために使う。
    """
    text = _ANCHOR_STRIP_RE.sub("", heading)
    for character in _ANCHOR_FULLWIDTH:
        text = text.replace(character, "")
    return _ANCHOR_SPACE_RE.sub("-", text.strip().lower())


def secret_like(content):
    return SECRET_RE.search(content) is not None


def personal_path(content):
    return PERSONAL_PATH_RE.search(content) is not None


def unescape_data_string(value):
    """`[System.Uri]::UnescapeDataString`と同じく、percent-encodingをUTF-8で戻す。

    reserved文字も含めてすべて戻す。`%2E`のようなencodingで拡張子判定を
    迂回されないよう、decode後に拡張子とseparatorを見る前提の判定で使う。
    """
    return urllib.parse.unquote(value, encoding="utf-8", errors="replace")


def html_decode(value):
    """`[System.Net.WebUtility]::HtmlDecode`に相当するHTML entityのdecode。"""
    return html.unescape(value)


def _directory_separators():
    separators = {os.sep}
    if os.altsep:
        separators.add(os.altsep)
    return separators


def full_path(path):
    """`[System.IO.Path]::GetFullPath`と同じく、filesystemを見ずに正規化する。

    symlinkを解決する`realpath`は使わない。公開境界の判定はlexicalに行い、
    reparse pointは別のguardが拒否する。解決してしまうと、`_site`の外を指す
    reparse pointが「内側のpath」として通ってしまう。
    """
    return os.path.abspath(path)


def path_within_root(path, root):
    """path が root の内側かを判定する。

    StartsWith だけでは `_site` と `_site-old` を区別できないため、区切り文字まで
    含めて比較する。比較はcase-sensitiveにする。CIのubuntu-24.04はcase-sensitive
    filesystemであり、case-insensitiveだと`_SITE`を`_site`の内側と誤判定する。
    判定を誤ると、artifactに含まれないfileへのlinkを検査が通してしまう。
    一致しなければ「範囲外」へ倒すfail-closedとする。
    """
    resolved = full_path(path)
    root_resolved = full_path(root).rstrip("".join(_directory_separators()))
    if resolved == root_resolved:
        return True
    return resolved.startswith(root_resolved + os.sep)


def path_relative_to_root(path, root):
    """root内のpathを、CIのOSに依存しない`/`区切りの相対pathへ変換する。

    validatorごとに切り出しを複製すると、root表記やseparatorの扱いが食い違い、
    local絶対pathを診断へ戻してしまうため、公開scriptで共有する。
    """
    resolved = full_path(path)
    root_resolved = full_path(root)
    if not path_within_root(resolved, root_resolved):
        # helper自体がlocal pathを再掲すると、診断を相対化する目的を破る。
        raise ValidationError("Cannot format a path outside the publication root.")
    return os.path.relpath(resolved, root_resolved).replace("\\", "/")


def get_extension(path):
    """`[System.IO.Path]::GetExtension`と同じ規則で拡張子を返す。

    `os.path.splitext`とは`.gitignore`のようなleading-dot名で結果が違う。
    拡張子allowlistの判定に使うため、旧実装と同じ規則を保つ。
    """
    separators = _directory_separators()
    for index in range(len(path) - 1, -1, -1):
        character = path[index]
        if character == ".":
            if index != len(path) - 1:
                return path[index:]
            return ""
        if character in separators:
            break
    return ""


def is_reparse_point(path):
    """symlink、junction、その他のreparse pointかを判定する。

    Windowsではfile属性、POSIXではlstatのmodeで判定する。追跡状態ではなく
    working tree上の実体を見る判定であり、Gitのmodeによる判定とは目的が違う。
    """
    try:
        info = os.lstat(path)
    except OSError:
        return False
    attributes = getattr(info, "st_file_attributes", None)
    if attributes is not None:
        return bool(attributes & stat.FILE_ATTRIBUTE_REPARSE_POINT)
    return stat.S_ISLNK(info.st_mode)


def iter_tree(root):
    """root配下の全entryを`(path, is_directory)`で返す。

    reparse pointは項目として列挙するが、その配下へは降りない。
    PowerShellの`Get-ChildItem -Recurse -Force`と同じ挙動であり、`_site`の外に
    実体があるfileを内側のfileとして数えないために必要である。
    `is_directory`はreparse pointを解決した結果で決める。PowerShellの
    `PSIsContainer`と同じく、directoryへのjunctionはcontainerとして扱う。
    """
    stack = [root]
    while stack:
        current = stack.pop()
        try:
            # `with`で閉じる。Windowsではdirectory handleが開いたままだと、
            # 走査後にそのdirectoryを削除できず、test fixtureの後始末が失敗する。
            with os.scandir(current) as scan:
                entries = sorted(scan, key=lambda entry: entry.name)
        except OSError:
            continue
        for entry in entries:
            try:
                is_directory = entry.is_dir()
            except OSError:
                is_directory = False
            yield entry.path, is_directory
            if is_directory and not is_reparse_point(entry.path):
                stack.append(entry.path)


def iter_files(root):
    """root配下のfileだけを返す。reparse pointのfileも対象に含める。"""
    for path, is_directory in iter_tree(root):
        if not is_directory:
            yield path


def remove_tree(path):
    """pathを取り除く。file、directory、reparse pointのどれでもよい。

    read-onlyのfileも対象にする。Gitのobjectはread-onlyで作られるため、素の
    `shutil.rmtree`では消せない。PowerShellの`Remove-Item -Recurse -Force`が
    形と属性を問わず消せていたのに合わせる。

    reparse pointは中身を辿らずlinkだけを外す。target側を消さないためである。
    消せたかどうかをboolで返す。呼び出し側が握りつぶすか報告するかを決める。
    """
    if not os.path.exists(path) and not os.path.islink(path):
        return True

    if is_reparse_point(path):
        try:
            os.remove(path)
        except OSError:
            try:
                os.rmdir(path)
            except OSError:
                return False
        return True

    if not os.path.isdir(path):
        try:
            os.remove(path)
        except OSError:
            return False
        return True

    for current, directories, files in os.walk(path):
        for name in directories + files:
            target = os.path.join(current, name)
            try:
                os.chmod(target, os.stat(target).st_mode | stat.S_IWUSR)
            except OSError:
                pass
    shutil.rmtree(path, ignore_errors=True)
    return not os.path.exists(path)


def ordinal_sort_key(value):
    """`[StringComparer]::Ordinal`と同じ順序を与えるsort key。

    .NETのOrdinal比較はUTF-16 code unit単位である。Pythonの既定はcode point順で、
    BMP外の文字とU+E000..U+FFFFの相対順が食い違う。digestを環境間で突き合わせる
    以上、並び順の定義まで旧実装と一致させる。
    """
    return value.encode("utf-16-be", errors="surrogatepass")


def sort_unique(items):
    """`Sort-Object -Unique`と同じく、case非依存で重複を除いて並べる。

    診断出力の整形にだけ使う。判定結果には影響しない。
    """
    unique = {}
    for item in items:
        unique.setdefault(item.lower(), item)
    return [unique[key] for key in sorted(unique)]


_DIAGNOSTIC_CATEGORIES = frozenset({"Cc", "Cf", "Zl", "Zp"})


def diagnostic_text(value):
    """生成物由来の値をCI logへそのまま書かないための整形。

    改行、terminal escape、bidi制御等をprintable placeholderへ変え、極端に長い
    attributeでlog容量を占有させない。
    """
    safe = "".join(
        "?" if unicodedata.category(character) in _DIAGNOSTIC_CATEGORIES else character
        for character in value
    )
    if len(safe) > 240:
        return safe[:240] + "..."
    return safe


def file_sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest().upper()
