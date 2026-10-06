"""Local, value-safe checks for text before a GitHub post or commit.

Only categories and line numbers leave this module. The optional local dictionary
is deliberately kept outside Git: individual names and SSIDs are not source data.
"""

from dataclasses import dataclass
import ipaddress
import json
from pathlib import Path
import re
import unicodedata

from .publish_guards import SECRET_RE


CONFIG_NAME = ".publication.local.json"
CONFIG_LIMIT = 64 * 1024
MAX_LOCAL_ENTRIES = 256
INPUT_LIMIT = 4 * 1024 * 1024
CONFIG_FIELDS = ("names", "emails", "ssids", "identifiers")

IP_RE = re.compile(r"(?<![\w.])(?:\d{1,3}\.){3}\d{1,3}(?![\w.])")
PRIVATE_NETWORKS = tuple(ipaddress.ip_network((address, prefix)) for address, prefix in (
    (0x0A000000, 8), (0xAC100000, 12), (0xC0A80000, 16)
))
MAC_RE = re.compile(r"(?<![\da-fA-F])(?:[\da-fA-F]{2}([:-]))(?:[\da-fA-F]{2}\1){4}[\da-fA-F]{2}(?![\da-fA-F])")
MAC_DOTTED_RE = re.compile(r"(?<![\da-fA-F])[\da-fA-F]{4}(?:\.[\da-fA-F]{4}){2}(?![\da-fA-F])")
EMAIL_RE = re.compile(r"(?<![\w.+-])[\w.+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}(?![\w.-])")
CERT_RE = re.compile("-----BEGIN " + "CERTIFICATE-----")
PASSWORD_RE = re.compile(r"(?i)\b(?:password|passwd|api[_-]?key|access[_-]?token|secret)\s*[:=]\s*([^\s,;]+)")
BEARER_RE = re.compile(r"(?i)\bBearer\s+[A-Za-z0-9._~+/=-]{12,}")
PLACEHOLDERS = frozenset({"example", "sample", "dummy", "placeholder", "redacted", "tbd", "changeme"})
DOC_DOMAINS = frozenset({"example.com", "example.net", "example.org"})
TRAILER_RE = re.compile(r"(?i)^\s*(?:co-authored-by|signed-off-by|reviewed-by):")
ENV_REFERENCE_RE = re.compile(
    r'''(?:os\.getenv\(["'][A-Za-z_][A-Za-z0-9_]*["']\)|'''
    r'''os\.environ\[["'][A-Za-z_][A-Za-z0-9_]*["']\]|'''
    r'''\$\{[A-Za-z_][A-Za-z0-9_]*\}|process\.env\.[A-Za-z_][A-Za-z0-9_]*)'''
)


class ScanError(Exception):
    """The scan did not run to completion. Never include input data in the message."""


@dataclass(frozen=True)
class Configuration:
    present: bool
    entries: tuple[tuple[str, str], ...]


@dataclass(frozen=True)
class Finding:
    category: str
    line: int


def load_config(root: Path) -> Configuration:
    path = root / CONFIG_NAME
    try:
        with path.open("rb") as stream:
            raw = stream.read(CONFIG_LIMIT + 1)
    except FileNotFoundError:
        if path.is_symlink():
            raise ScanError("local configuration cannot be read")
        return Configuration(False, ())
    except OSError as exc:
        raise ScanError("local configuration cannot be read") from exc
    if len(raw) > CONFIG_LIMIT:
        raise ScanError("local configuration is too large")
    try:
        data = json.loads(raw.decode("utf-8"))
    except (UnicodeError, json.JSONDecodeError) as exc:
        raise ScanError("local configuration is invalid") from exc
    if not isinstance(data, dict) or set(data) != set(CONFIG_FIELDS):
        raise ScanError("local configuration must contain names, emails, ssids and identifiers")
    entries = []
    for key in CONFIG_FIELDS:
        values = data[key]
        if not isinstance(values, list):
            raise ScanError("local configuration has an invalid list")
        if any(not isinstance(value, str) or not value.strip() or value != value.strip()
               or any(ord(char) < 32 for char in value) for value in values):
            raise ScanError("local configuration has an invalid entry")
        if len(entries) + len(values) > MAX_LOCAL_ENTRIES:
            raise ScanError("local configuration has too many entries")
        normalized = [unicodedata.normalize("NFKC", value).casefold() for value in values]
        if len(set(normalized)) != len(values):
            raise ScanError("local configuration has duplicate entries")
        entries.extend((key, value) for value in normalized)
    return Configuration(True, tuple(entries))


def read_text(path: Path) -> str:
    try:
        with path.open("rb") as stream:
            raw = stream.read(INPUT_LIMIT + 1)
    except OSError as exc:
        raise ScanError("input cannot be read") from exc
    if len(raw) > INPUT_LIMIT:
        raise ScanError("input is too large")
    try:
        return raw.decode("utf-8")
    except UnicodeError as exc:
        raise ScanError("input is not UTF-8 text") from exc


def _is_bot_mailbox(address: str, line: str) -> bool:
    local = address.split("@", 1)[0].casefold()
    if local in {"noreply", "no-reply", "bot"}:
        return True
    return bool(TRAILER_RE.match(line)) and local.endswith(("-bot", ".bot", "_bot"))


def _is_documentation_mailbox(address: str) -> bool:
    domain = address.rsplit("@", 1)[1].casefold()
    return domain in DOC_DOMAINS or domain.endswith((".example", ".invalid", ".test"))


def _is_placeholder(value: str) -> bool:
    value = value.strip('"\'`')
    if not value:
        return True
    if value.startswith("<") and value.endswith(">"):
        value = value[1:-1]
    return value.casefold() in PLACEHOLDERS


def _is_example_assignment(value: str, line: str) -> bool:
    if _is_placeholder(value):
        return True
    return bool(ENV_REFERENCE_RE.fullmatch(value) and PASSWORD_RE.fullmatch(line.strip()))


def scan_text(text: str, config: Configuration) -> list[Finding]:
    try:
        size = len(text.encode("utf-8"))
    except UnicodeError as exc:
        raise ScanError("input is not valid UTF-8 text") from exc
    if size > INPUT_LIMIT:
        raise ScanError("input is too large")
    findings = set()
    for number, line in enumerate(text.splitlines(), 1):
        for match in IP_RE.finditer(line):
            try:
                address = ipaddress.ip_address(match.group())
            except ValueError:
                continue
            if any(address in network for network in PRIVATE_NETWORKS):
                findings.add(Finding("private IP", number))
        if MAC_RE.search(line) or MAC_DOTTED_RE.search(line):
            findings.add(Finding("MAC/BSSID", number))
        if CERT_RE.search(line) or BEARER_RE.search(line):
            findings.add(Finding("secret or certificate", number))
        for match in SECRET_RE.finditer(line):
            assignment = PASSWORD_RE.search(match.group())
            if not (assignment and _is_example_assignment(assignment.group(1), line)):
                findings.add(Finding("secret or certificate", number))
        for match in PASSWORD_RE.finditer(line):
            if not _is_example_assignment(match.group(1), line):
                findings.add(Finding("secret or certificate", number))
        for match in EMAIL_RE.finditer(line):
            address = match.group()
            if not (_is_documentation_mailbox(address) or _is_bot_mailbox(address, line)):
                findings.add(Finding("email", number))
        folded = unicodedata.normalize("NFKC", line).casefold()
        for key, value in config.entries:
            if value in folded:
                findings.add(Finding("local " + key, number))
    return sorted(findings, key=lambda item: (item.line, item.category))
