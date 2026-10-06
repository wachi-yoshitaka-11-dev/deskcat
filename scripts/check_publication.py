#!/usr/bin/env python3
"""Scan a GitHub body, commit message, or staged additions before publishing.

Examples: python3 scripts/check_publication.py --file BODY
          python3 scripts/check_publication.py --staged
          python3 scripts/check_publication.py --stdin
          python3 scripts/check_publication.py --commit-message MESSAGE
"""

import argparse
from pathlib import Path
import re
import subprocess
import sys
import tempfile

from lib.publication_guard import (
    CONFIG_NAME, INPUT_LIMIT, ScanError, load_config, read_text, scan_text,
)


ROOT = Path(__file__).resolve().parent.parent
VERBOSE_HEADER_RE = re.compile(
    r"(?m)^# ------------------------ >8 ------------------------\r?\n"
    r"# Do not modify or remove the line above\.\r?\n"
    r"# Everything below it will be ignored\.\r?\n"
)


def _without_verified_verbose_diff(message: str) -> str:
    """Omit Git's generated diff only when it matches the staged patch exactly."""
    headers = list(VERBOSE_HEADER_RE.finditer(message))
    if not headers:
        return message
    try:
        with tempfile.TemporaryFile() as output:
            result = subprocess.run(
                ["git", "diff", "--cached", "--no-ext-diff", "--no-textconv",
                 "--no-color"], cwd=ROOT, stdout=output, stderr=subprocess.DEVNULL,
                timeout=30, check=False,
            )
            output.seek(0)
            raw_patch = output.read(INPUT_LIMIT + 1)
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise ScanError("commit verbose diff cannot be verified") from exc
    if result.returncode:
        raise ScanError("commit verbose diff cannot be verified")
    if len(raw_patch) > INPUT_LIMIT:
        raise ScanError("commit verbose diff is too large")
    try:
        patch = raw_patch.decode("utf-8")
    except UnicodeError as exc:
        raise ScanError("commit verbose diff is not UTF-8 text") from exc
    for header in reversed(headers):
        if patch and message[header.end():] == patch:
            return message[:header.end()]
    return message


def commit_message_text(path: Path) -> str:
    """Scan all possible message content except Git's verified verbose diff.

    The hook cannot see a `git commit --cleanup` command-line override, so
    comment lines must stay in the scan even when one mode would remove them.
    """
    return _without_verified_verbose_diff(read_text(path))


def staged_text(root: Path = ROOT) -> str:
    def git(*args: str) -> bytes:
        try:
            result = subprocess.run(
                ["git", *args], cwd=root, capture_output=True, timeout=30, check=False,
            )
        except (OSError, subprocess.TimeoutExpired) as exc:
            raise ScanError("staged input cannot be obtained") from exc
        if result.returncode:
            raise ScanError("staged input cannot be obtained")
        return result.stdout

    names = git("diff", "--cached", "--name-only", "-z", "--diff-filter=ACMR").split(b"\0")
    if any(Path(name.decode("utf-8", errors="surrogateescape")).name == CONFIG_NAME
           for name in names if name):
        raise ScanError("local configuration was staged")
    raw = git("diff", "--cached", "--patch", "--unified=0", "--binary",
              "--no-ext-diff", "--no-textconv", "--no-color")
    if len(raw) > INPUT_LIMIT:
        raise ScanError("staged input is too large")
    try:
        diff = raw.decode("utf-8")
    except UnicodeError as exc:
        raise ScanError("staged input is not UTF-8 text") from exc
    if "\nGIT binary patch\n" in diff or "\nBinary files " in diff:
        raise ScanError("staged binary content cannot be scanned")
    # Only additions can newly publish a value. Scanning whole files would reject
    # unchanged historical examples whenever an unrelated line is edited.
    additions = []
    in_hunk = False
    for line in diff.splitlines():
        if line.startswith("diff --git "):
            in_hunk = False
        elif line.startswith("@@ "):
            in_hunk = True
        elif in_hunk and line.startswith("+"):
            additions.append(line[1:])
    return "\n".join(additions)


def stdin_text() -> str:
    raw = sys.stdin.buffer.read(INPUT_LIMIT + 1)
    if len(raw) > INPUT_LIMIT:
        raise ScanError("input is too large")
    try:
        return raw.decode("utf-8")
    except UnicodeError as exc:
        raise ScanError("input is not UTF-8 text") from exc


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--file", type=Path)
    source.add_argument("--stdin", action="store_true")
    source.add_argument("--staged", action="store_true")
    source.add_argument("--commit-message", type=Path)
    args = parser.parse_args(argv)
    try:
        config = load_config(ROOT)
        if args.file is not None:
            text = read_text(args.file)
        elif args.commit_message is not None:
            text = commit_message_text(args.commit_message)
        elif args.stdin:
            text = stdin_text()
        else:
            text = staged_text()
        findings = scan_text(text, config)
    except ScanError as exc:
        print(f"Publication scan stopped: {exc}", file=sys.stderr)
        return 2
    if config.present:
        print(f"Publication scan coverage: generic patterns and {len(config.entries)} local entries checked.")
    else:
        print("Publication scan coverage: generic patterns checked; local names, emails, SSIDs and identifiers NOT checked (local configuration absent).")
    for finding in findings:
        print(f"Publication scan blocked: {finding.category} on input line {finding.line}.", file=sys.stderr)
    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main())
