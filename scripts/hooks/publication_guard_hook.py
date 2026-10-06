#!/usr/bin/env python3
"""Claude Code PreToolUse guard for GitHub text supplied to the gh CLI.

The hook inspects literal CLI arguments and readable body files. Shell expansion,
interactive editors, browser posts, and other clients need the manual command.
"""

import json
from pathlib import Path
import re
import sys

import command_line

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from lib.publication_guard import ScanError, load_config, read_text, scan_text  # noqa: E402


ROOT = Path(__file__).resolve().parent.parent.parent
POST_COMMANDS = {
    ("issue", "create"), ("pr", "create"),
    ("issue", "comment"), ("pr", "comment"), ("pr", "review"),
    ("issue", "edit"), ("pr", "edit"),
}


def deny(reason: str) -> None:
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse",
        "permissionDecision": "deny", "permissionDecisionReason": reason}}))
    raise SystemExit(0)


def _option(args: list[str], long: str, short: str | None = None) -> str | None:
    value = None
    for index, arg in enumerate(args):
        if arg == long or (short and arg == short):
            value = args[index + 1] if index + 1 < len(args) else None
        elif arg.startswith(long + "="):
            value = arg[len(long) + 1:]
        elif short and arg.startswith(short) and arg != short:
            value = arg[len(short):]
    return value


def _literal(value: str) -> str:
    if "$(" in value or re.search(r"\$[A-Za-z_{]", value):
        raise ScanError("expanded text cannot be inspected by the hook")
    return value


def _body(args: list[str]) -> str | None:
    path = _option(args, "--body-file", "-F")
    if path is not None:
        if path == "-":
            raise ScanError("stdin body cannot be read by the hook")
        return read_text(Path(path))
    value = _option(args, "--body", "-b")
    return _literal(value) if value is not None else None


def _api_payloads(args: list[str]) -> list[str]:
    parts = []
    path = _option(args, "--input")
    if path is not None:
        if path == "-":
            raise ScanError("stdin API input cannot be read by the hook")
        parts.append(read_text(Path(path)))
    for index, arg in enumerate(args):
        if arg in ("-f", "-F", "--field", "--raw-field"):
            if index + 1 >= len(args):
                raise ScanError("API field value is missing")
            value = args[index + 1]
            typed = arg in ("-F", "--field")
        elif any(arg.startswith(prefix) for prefix in ("--field=", "--raw-field=", "-f", "-F")):
            value = arg.split("=", 1)[1] if arg.startswith("--") else arg[2:]
            typed = arg.startswith(("--field=", "-F"))
        else:
            continue
        if "=" not in value:
            raise ScanError("API field cannot be inspected")
        field_name, field_value = value.split("=", 1)
        if typed and field_value.startswith("@"):
            parts.append(read_text(Path(field_value[1:])))
        else:
            # GraphQL variables use '$' in a literal query. shlex has already
            # removed shell quotes, so treating them as expansion would reject
            # normal read-only and write queries alike.
            parts.append(field_value if field_name == "query" else _literal(field_value))
    return parts


def _texts(args: list[str]) -> list[str] | None:
    args = list(command_line.skip_global_options(args, "gh"))
    head = tuple(args[:2])
    if head in POST_COMMANDS:
        if any(value in ("-h", "--help") for value in args[2:]):
            return None
        body = _body(args[2:])
        title = _option(args[2:], "--title", "-t") if head[1] in ("create", "edit") else None
        if title is not None:
            title = _literal(title)
        if head == ("pr", "review") and body is None and "--approve" in args[2:]:
            return None
        if body is None and (head[1] != "edit" or title is None):
            if head[1] == "edit":
                return None  # Metadata-only edit; no text is supplied.
            raise ScanError("GitHub text was not supplied in readable arguments")
        return [value for value in (title, body) if value is not None]
    if args[:1] == ["api"]:
        payloads = _api_payloads(args[1:])
        if payloads:
            return payloads
        method = _option(args[1:], "--method", "-X")
        if method and method.upper() in ("POST", "PUT", "PATCH"):
            raise ScanError("API write payload cannot be inspected")
    return None


def main() -> int:
    try:
        payload = json.load(sys.stdin)
    except (json.JSONDecodeError, UnicodeDecodeError):
        deny("Publication scan hook input cannot be read")
    command = command_line.command_from(payload)
    if command is None:
        deny("Publication scan hook command cannot be read")
    if "gh" not in command:
        return 0
    try:
        for args in command_line.invocations(command, "gh"):
            texts = _texts(args)
            if texts is None:
                continue
            config = load_config(ROOT)
            print("Publication scan: generic patterns checked; " +
                  (f"{len(config.entries)} local entries checked."
                   if config.present else "local names, emails, SSIDs and identifiers NOT checked (local configuration absent)."),
                  file=sys.stderr)
            for value in texts:
                findings = scan_text(value, config)
                if findings:
                    deny("Publication scan blocked GitHub text: " + findings[0].category +
                         ". The detected value is hidden; run scripts/check_publication.py on the body.")
    except ScanError as exc:
        deny("Publication scan could not inspect GitHub text: " + str(exc))
    return 0


if __name__ == "__main__":
    sys.exit(main())
