#!/usr/bin/env python3
"""Synthetic fixtures for the GitHub and commit publication boundaries."""

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

from check_publication import _without_verified_verbose_diff, staged_text
from lib.publication_guard import (
    CONFIG_NAME, Configuration, ScanError, load_config, read_text, scan_text,
)


ROOT = Path(__file__).resolve().parent.parent
CLI = ROOT / "scripts" / "check_publication.py"
HOOK = ROOT / "scripts" / "hooks" / "publication_guard_hook.py"
EMPTY = Configuration(False, ())


def private_address():
    return ".".join(("192", "168", "12", "34"))


def mailbox(local):
    return local + "@" + "laboratory.tld"


class PublicationPatterns(unittest.TestCase):
    def categories(self, value, config=EMPTY):
        return {item.category for item in scan_text(value, config)}

    def test_private_networks_and_documentation_ranges(self):
        for octets in (("10", "2", "3", "4"), ("172", "20", "1", "2"),
                       ("192", "168", "4", "5")):
            with self.subTest(octets=octets):
                self.assertIn("private IP", self.categories(".".join(octets)))
        for octets in (("192", "0", "2", "1"), ("198", "51", "100", "7"),
                       ("203", "0", "113", "9")):
            with self.subTest(octets=octets):
                self.assertNotIn("private IP", self.categories(".".join(octets)))
        self.assertEqual(self.categories("999.999.999.999"), set())

    def test_mac_secret_certificate_and_credentials(self):
        mac = ":".join(("02", "00", "00", "00", "00", "01"))
        self.assertIn("MAC/BSSID", self.categories(mac))
        self.assertIn("MAC/BSSID", self.categories(".".join(("0200", "0000", "0001"))))
        self.assertIn("secret or certificate", self.categories("ghp_" + "x" * 24))
        self.assertIn("secret or certificate", self.categories("-----BEGIN " + "CERTIFICATE-----"))
        self.assertIn("secret or certificate", self.categories("id=user " + "password" + "=synthetic-value"))
        self.assertIn("secret or certificate", self.categories("Authorization: Bearer " + "a" * 20))
        self.assertNotIn("secret or certificate", self.categories("password" + "=<placeholder>"))
        self.assertIn("secret or certificate", self.categories("password" + "=<synthetic-secret-value>"))
        reference = "password" + '=os.getenv("SYNTHETIC")'
        self.assertNotIn("secret or certificate", self.categories(reference))
        self.assertIn("secret or certificate", self.categories(reference + " + literal"))

    def test_email_examples_bot_and_trailer(self):
        self.assertIn("email", self.categories(mailbox("person")))
        self.assertNotIn("email", self.categories(mailbox("noreply")))
        self.assertNotIn("email", self.categories("Co-Authored-By: Helper <" + mailbox("service-bot") + ">"))
        self.assertIn("email", self.categories(mailbox("service-bot")))
        self.assertNotIn("email", self.categories("writer@" + "example.com"))
        self.assertNotIn("email", self.categories("writer@" + "example.invalid"))

    def test_optional_local_entries_and_missing_configuration(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.assertFalse(load_config(root).present)
            self.assertIn("private IP", self.categories(private_address(), load_config(root)))
            (root / CONFIG_NAME).write_text(json.dumps({
                "names": ["Invented Person"], "emails": [mailbox("local-person")],
                "ssids": ["Fictional Wireless"], "identifiers": ["Synthetic Label"],
            }))
            config = load_config(root)
            self.assertTrue(config.present)
            self.assertIn("local names", self.categories("invented person", config))
            self.assertIn("local ssids", self.categories("fictional wireless", config))
            self.assertIn("local emails", self.categories(mailbox("local-person"), config))
            self.assertIn("local identifiers", self.categories("synthetic label", config))
            self.assertIn("local ssids", self.categories("Ｆｉｃｔｉｏｎａｌ Ｗｉｒｅｌｅｓｓ", config))

    def test_invalid_configuration_and_unreadable_input_stop(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / CONFIG_NAME
            path.write_text('{"names": ["invented"]}')
            with self.assertRaises(ScanError):
                load_config(root)
            path.write_text(json.dumps({"names": [""], "emails": [], "ssids": [], "identifiers": []}))
            with self.assertRaises(ScanError):
                load_config(root)
            with self.assertRaises(ScanError):
                read_text(root / "missing.txt")
            with self.assertRaises(ScanError):
                scan_text(chr(0xD800), EMPTY)
            path.write_text(json.dumps({"names": ["item" + str(i) for i in range(257)],
                                        "emails": [], "ssids": [], "identifiers": []}))
            with self.assertRaises(ScanError):
                load_config(root)


class PublicationEntrypoints(unittest.TestCase):
    def test_manual_command_reports_partial_coverage_without_values(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "body.txt"
            value = private_address()
            path.write_text(value)
            run = subprocess.run([sys.executable, str(CLI), "--file", str(path)],
                                 capture_output=True, text=True, check=False)
            self.assertEqual(run.returncode, 1)
            self.assertIn("NOT checked", run.stdout)
            self.assertIn("private IP", run.stderr)
            self.assertNotIn(value, run.stdout + run.stderr)
            missing = subprocess.run([sys.executable, str(CLI), "--file", str(path) + ".missing"],
                                     capture_output=True, text=True, check=False)
            self.assertEqual(missing.returncode, 2)

    def test_staged_additions_and_binary_failure_without_a_commit(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            path = root / "note.txt"
            path.write_text("safe\n" + private_address() + "\n")
            subprocess.run(["git", "add", "note.txt"], cwd=root, check=True)
            self.assertIn("private IP", {item.category for item in scan_text(staged_text(root), EMPTY)})
            (root / "binary.bin").write_bytes(b"\x00\xff\x00")
            subprocess.run(["git", "add", "binary.bin"], cwd=root, check=True)
            with self.assertRaises(ScanError):
                staged_text(root)

    def test_git_hooks_call_shared_scan_before_commit(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            (root / "scripts" / "lib").mkdir(parents=True)
            (root / ".githooks").mkdir()
            for relative in ("scripts/check_publication.py", "scripts/lib/publication_guard.py",
                             "scripts/lib/publish_guards.py", ".githooks/pre-commit",
                             ".githooks/commit-msg"):
                shutil.copy2(ROOT / relative, root / relative)
            (root / "note.txt").write_text(private_address())
            subprocess.run(["git", "add", "note.txt"], cwd=root, check=True)
            pre = subprocess.run([str(root / ".githooks" / "pre-commit")], cwd=root,
                                 capture_output=True, text=True, check=False)
            self.assertEqual(pre.returncode, 1)
            self.assertNotIn(private_address(), pre.stdout + pre.stderr)
            (root / "note.txt").write_text("safe")
            subprocess.run(["git", "add", "note.txt"], cwd=root, check=True)
            self.assertEqual(subprocess.run([str(root / ".githooks" / "pre-commit")], cwd=root,
                                            capture_output=True, check=False).returncode, 0)
            message = root / "message.txt"
            message.write_text(mailbox("person"))
            commit_msg = subprocess.run([str(root / ".githooks" / "commit-msg"), str(message)],
                                        cwd=root, capture_output=True, text=True, check=False)
            self.assertEqual(commit_msg.returncode, 1)
            self.assertNotIn(mailbox("person"), commit_msg.stdout + commit_msg.stderr)

    def test_verbose_commit_ignores_only_git_generated_diff(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            (root / "scripts" / "lib").mkdir(parents=True)
            (root / ".githooks").mkdir()
            for relative in ("scripts/check_publication.py", "scripts/lib/publication_guard.py",
                             "scripts/lib/publish_guards.py", ".githooks/pre-commit",
                             ".githooks/commit-msg"):
                shutil.copy2(ROOT / relative, root / relative)
            for key, value in (("user.name", "Example User"),
                               ("user.email", "writer@example.invalid")):
                subprocess.run(["git", "config", key, value], cwd=root, check=True)
            note = root / "note.txt"
            note.write_text(private_address() + "\n")
            subprocess.run(["git", "add", "note.txt"], cwd=root, check=True)
            subprocess.run(["git", "-c", "core.hooksPath=/dev/null", "commit", "-qm", "initial"],
                           cwd=root, check=True)
            note.write_text("safe text\n")
            subprocess.run(["git", "add", "note.txt"], cwd=root, check=True)
            editor = root / "editor.sh"
            editor.write_text('#!/bin/sh\ncp "$1" "$PWD/verbose-template.txt"\n'
                              'sed -i "1i safe subject" "$1"\n')
            editor.chmod(0o755)
            run = subprocess.run(["git", "-c", "core.hooksPath=.githooks", "commit", "-v"],
                                 cwd=root, env={**os.environ, "GIT_EDITOR": str(editor)},
                                 capture_output=True, text=True, check=False)
            self.assertEqual(run.returncode, 0, run.stderr)
            self.assertIn(private_address(), (root / "verbose-template.txt").read_text())
            committed = subprocess.run(["git", "log", "-1", "--format=%B"], cwd=root,
                                       capture_output=True, text=True, check=True).stdout
            self.assertEqual(committed.strip(), "safe subject")
            message = root / "message.txt"
            message.write_text("safe subject\n# ------------------------ >8 ------------------------\n"
                               + private_address() + "\n")
            rejected = subprocess.run([str(root / ".githooks" / "commit-msg"), str(message)],
                                      cwd=root, capture_output=True, text=True, check=False)
            self.assertEqual(rejected.returncode, 1)
            self.assertNotIn(private_address(), rejected.stdout + rejected.stderr)
            note.write_text("next safe text\n")
            subprocess.run(["git", "add", "note.txt"], cwd=root, check=True)
            patch = subprocess.run(
                ["git", "diff", "--cached", "--no-ext-diff", "--no-textconv", "--no-color"],
                cwd=root, capture_output=True, text=True, check=True,
            ).stdout
            other_marker = ("X ------------------------ >8 ------------------------\n"
                            "X Do not modify or remove the line above.\n"
                            "X Everything below it will be ignored.\n")
            self.assertEqual(_without_verified_verbose_diff("safe subject\n" + other_marker + patch),
                             "safe subject\n" + other_marker + patch)
            editor.write_text('#!/bin/sh\n'
                              + f'sed -i "1i # {private_address()}" "$1"\n'
                              + 'sed -i "1i safe subject" "$1"\n')
            retained = subprocess.run(
                ["git", "-c", "core.hooksPath=.githooks", "commit", "-v",
                 "--cleanup=verbatim"], cwd=root,
                env={**os.environ, "GIT_EDITOR": str(editor)},
                capture_output=True, text=True, check=False,
            )
            self.assertEqual(retained.returncode, 1)
            self.assertNotIn(private_address(), retained.stdout + retained.stderr)
            message.write_text("safe subject\n# Please enter the commit message for your changes. Lines starting\n"
                               "# with '#' will be ignored, and an empty message aborts the commit.\n"
                               "# " + private_address() + "\n")
            forged = subprocess.run([str(root / ".githooks" / "commit-msg"), str(message)],
                                    cwd=root, capture_output=True, text=True, check=False)
            self.assertEqual(forged.returncode, 1)

    def _hook(self, command):
        return subprocess.run([sys.executable, str(HOOK)],
                              input=json.dumps({"tool_input": {"command": command}}),
                              capture_output=True, text=True, check=False)

    def test_claude_hook_checks_comments_and_review_replies(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "body.txt"
            value = private_address()
            path.write_text(value)
            blocked = self._hook(f"gh issue comment 1 --body-file {path}")
            self.assertIn('"permissionDecision": "deny"', blocked.stdout)
            self.assertNotIn(value, blocked.stdout + blocked.stderr)
            review = self._hook('gh pr review 1 --body "' + mailbox("person") + '"')
            self.assertIn('"permissionDecision": "deny"', review.stdout)
            safe = self._hook('gh pr review 1 --body "' + mailbox("noreply") + '"')
            self.assertEqual(safe.stdout, "")
            self.assertIn("NOT checked", safe.stderr)
            unreadable = self._hook("gh issue comment 1 --body-file /missing/body.txt")
            self.assertIn('"permissionDecision": "deny"', unreadable.stdout)

    def test_claude_hook_checks_titles_and_api_fields(self):
        value = private_address()
        title = self._hook('gh issue create --title "' + value + '" --body "safe"')
        self.assertIn('"permissionDecision": "deny"', title.stdout)
        api = self._hook('gh api -X POST repos/owner/repo/issues/1/comments -f body="' + value + '"')
        self.assertIn('"permissionDecision": "deny"', api.stdout)
        raw_at = self._hook("gh api -X POST repos/owner/repo/issues/1/comments -f body=@missing-file")
        self.assertEqual(raw_at.stdout, "")
        typed_at = self._hook("gh api -X POST repos/owner/repo/issues/1/comments -F body=@missing-file")
        self.assertIn('"permissionDecision": "deny"', typed_at.stdout)
        graphql = self._hook("gh api graphql -f 'query=query($id: ID!){node(id: $id){id}}'")
        self.assertEqual(graphql.stdout, "")
        missing = self._hook("gh api -X POST repos/owner/repo/issues/1/comments")
        self.assertIn('"permissionDecision": "deny"', missing.stdout)
        metadata = self._hook("gh issue edit 1 --add-label type:maintenance")
        self.assertEqual(metadata.stdout, "")
        approval = self._hook("gh pr review 1 --approve")
        self.assertEqual(approval.stdout, "")
        expanded = self._hook('gh issue comment 1 --body "$BODY"')
        self.assertIn('"permissionDecision": "deny"', expanded.stdout)


if __name__ == "__main__":
    unittest.main()
