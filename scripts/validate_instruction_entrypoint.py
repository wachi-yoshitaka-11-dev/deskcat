#!/usr/bin/env python3
"""`CLAUDE.md`がGit index上で通常fileであり、内容が`@AGENTS.md`のimport stubと
一致することを検査する。

Windowsの`core.symlinks=false`でcheckoutすると、mode 120000のentryはlink先のpathを
内容とする通常fileになる。`CLAUDE.md`が`AGENTS.md`へのsymlinkだった間、その環境では
内容がlink先のpath文字列だけになり、`@AGENTS.md`のimport行がfileに存在しなかった。
その状態でtoolが何を読むかはここでは主張しない。**import行を持たない記録を通さない**
ことだけを検査する。

検査するのはworking treeではなくGit indexである。indexのmodeとblobは実行環境に依らず、
working treeの実体はcheckout環境で変わる。working treeを読むと、mode 120000が記録された
ままでも、symlinkを解決する環境では成功してしまう。

内容はMarkdownとして解釈せず、期待するbyte列とそのまま突き合わせる。改行を含めて
一致を要求するため、CRLFへの変化もここで失敗する。
"""

import argparse
import subprocess
import sys
from pathlib import Path

ENTRYPOINT = "CLAUDE.md"
EXPECTED_MODE = "100644"
EXPECTED_BLOB = b"# DeskCat Claude Code Instructions\n\n@AGENTS.md\n"


def _git(root, arguments):
    """gitをbinaryで実行する。失敗はそのまま診断へ載せる。"""
    return subprocess.run(["git", "-C", root, *arguments], capture_output=True)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository-root", default="")
    options = parser.parse_args(argv)
    root = options.repository_root.strip() or str(
        Path(__file__).resolve().parent.parent
    )

    problems = []
    listing = _git(root, ["ls-files", "-s", "--", ENTRYPOINT])
    entries = listing.stdout.decode("utf-8", "replace").splitlines()
    if listing.returncode != 0 or len(entries) != 1:
        problems.append(
            f"Cannot read a single index entry for {ENTRYPOINT}."
            " Untracked, or several stages from an unresolved merge:"
            f" {listing.stderr.decode('utf-8', 'replace').strip()} {entries}"
        )
    else:
        mode = entries[0].split()[0]
        if mode != EXPECTED_MODE:
            problems.append(
                f"{ENTRYPOINT} is recorded with mode {mode}, expected"
                f" {EXPECTED_MODE}. Run `git rm --cached {ENTRYPOINT}` before"
                " adding it as a regular file; `git add` alone keeps the old mode."
            )
        blob = _git(root, ["cat-file", "blob", f":{ENTRYPOINT}"])
        if blob.returncode != 0 or blob.stdout != EXPECTED_BLOB:
            problems.append(
                f"{ENTRYPOINT} content does not match the expected import stub."
                f" Recorded {blob.stdout!r}, expected {EXPECTED_BLOB!r}."
            )

    if problems:
        for problem in problems:
            print(problem, file=sys.stderr)
        return 1

    print(f"ENTRYPOINT={ENTRYPOINT} MODE={EXPECTED_MODE} CONTENT=MATCHED")
    return 0


if __name__ == "__main__":
    sys.exit(main())
