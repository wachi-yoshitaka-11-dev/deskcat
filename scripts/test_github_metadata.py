#!/usr/bin/env python3
"""Metadata policy and transport/recovery tests; no live objects created."""
import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import github_metadata as m


def record(kind="issue", number=466):
    return {"id": number, "node_id": "NODE", "number": number, "kind": kind,
            "labels": [{"name": "type:maintenance"}, {"name": "priority:high"}, {"name": "area:docs"}],
            "assignees": [{"login": "owner"}], "milestone": {"number": 1, "title": "M0 Foundation"} if kind == "issue" else None,
            "state": "open", "base": "develop" if kind == "pr" else None,
            "created_at": "2026-09-25T15:00:00Z", "closed_at": None, "merged_at": None}


def item():
    return {"id": "ITEM", "content": {"number": 466, "repository": {"nameWithOwner": m.REPO}},
            "status": {"name": "In Progress"}, "start": {"date": "2026-09-26"}, "target": {"date": "2099-09-30"}}


class PolicyTests(unittest.TestCase):
    def findings(self, r=None, i=None, **kwargs):
        return m.validate(r or record(), i or item(), m.vocabulary(), **kwargs)

    def test_valid_issue_and_pr(self):
        self.assertEqual([], self.findings())
        self.assertEqual([], self.findings(record("pr")))

    def test_cardinality_differs_for_promotion_pr(self):
        r = record("pr")
        r["labels"].append({"name": "type:bug"})
        self.assertEqual([], self.findings(r))
        r["kind"] = "issue"
        self.assertTrue(any("exactly one" in str(f) for f in self.findings(r)))

    def test_unknown_and_missing_labels_assignee_milestone(self):
        r = record()
        r.update(labels=[{"name": "priority:medium"}], assignees=[], milestone=None)
        findings = self.findings(r)
        self.assertEqual({"labels", "assignees", "milestone"}, {f["field"] for f in findings})
        self.assertTrue(any("unknown label" in f["reason"] for f in findings))

    def test_pr_milestone_forbidden_and_area_required(self):
        r = record("pr")
        r["milestone"] = {"number": 1}
        r["labels"].pop()
        self.assertEqual({"milestone", "labels"}, {f["field"] for f in self.findings(r)})

    def test_conditional_labels_use_evidence(self):
        context = {"areas": ["area:docs"], "blocked": True, "needs": ["needs:decision"], "source": "Issue scope"}
        findings = self.findings(context=context)
        self.assertEqual(1, len(findings))
        self.assertIn("status:blocked", findings[0]["expected"])
        context["blocked"] = False
        context["needs"] = []
        self.assertEqual([], self.findings(context=context))
        with self.assertRaises(m.Failure):
            self.findings(context={})

    def test_missing_project_and_dates(self):
        fields = {f["field"] for f in m.validate(record("pr"), None, m.vocabulary())}
        self.assertEqual({"project", "Status", "Start date", "Target date"}, fields)

    def test_jst_boundary_and_latest_close(self):
        self.assertEqual("2026-09-25", m.jst("2026-09-25T14:59:59Z"))
        self.assertEqual("2026-09-26", m.jst("2026-09-25T15:00:00Z"))
        r = record("pr")
        r.update(state="closed", closed_at="2026-09-26T15:00:00Z")
        i = item()
        i.update(status={"name": "Done"}, target={"date": "2026-09-26"})
        self.assertEqual("2026-09-27", self.findings(r, i)[0]["expected"])
        r["merged_at"] = "2026-09-26T14:59:59Z"
        self.assertEqual([], self.findings(r, i))

    def test_issue_start_not_inferred_from_creation(self):
        i = item()
        i["start"]["date"] = "2026-08-01"
        self.assertEqual([], self.findings(i=i))

    def test_past_forecast_is_notice_but_reopen_done_is_invalid(self):
        i = item()
        i["target"]["date"] = "2000-01-01"
        self.assertEqual("notice", self.findings(i=i)[0]["level"])
        i["status"]["name"] = "Done"
        self.assertTrue(any(f["field"] == "Status" and f["level"] == "violation" for f in self.findings(i=i)))

    def test_repository_only_is_explicit_and_skips_project(self):
        self.assertEqual([], m.validate(record(), None, m.vocabulary(), repository_only=True))


class TransportTests(unittest.TestCase):
    def test_api_failure_malformed_json_and_graphql_partial_data(self):
        for response in (subprocess.CompletedProcess([], 1, "", "secret"),
                         subprocess.CompletedProcess([], 0, "not JSON", ""),
                         subprocess.CompletedProcess([], 0, '{"data":{"partial":true},"errors":[{}]}', "")):
            with self.subTest(response=response), patch.object(m.subprocess, "run", return_value=response):
                with self.assertRaises(m.Failure) as caught:
                    m.GitHub().api("graphql")
                self.assertNotIn("secret", str(caught.exception))

    def test_rest_pages_deduplicate_and_fail_on_later_page(self):
        api = m.GitHub()
        batch = [{"id": n} for n in range(100)]
        with patch.object(api, "rest", side_effect=[batch, [{"id": 99}, {"id": 100}]]):
            self.assertEqual(101, len(api.pages("issues?state=all")))
        with patch.object(api, "rest", side_effect=[batch, m.Failure("403")]):
            with self.assertRaises(m.Failure):
                api.pages("issues?state=all")

    def test_graphql_pages_counts_and_cursor(self):
        api = m.GitHub()
        first = {"nodes": [{"id": "a"}], "totalCount": 2, "pageInfo": {"hasNextPage": True, "endCursor": "A"}}
        second = {"nodes": [{"id": "b"}], "totalCount": 2, "pageInfo": {"hasNextPage": False, "endCursor": "B"}}
        with patch.object(api, "graphql", side_effect=[first, second]):
            self.assertEqual(2, len(api.connection("query", lambda d: d)))
        second["totalCount"] = 3
        with patch.object(api, "graphql", side_effect=[first, second]):
            with self.assertRaises(m.Failure):
                api.connection("query", lambda d: d)
        first["pageInfo"]["endCursor"] = None
        with patch.object(api, "graphql", return_value=first):
            with self.assertRaises(m.Failure):
                api.connection("query", lambda d: d)

    def test_inaccessible_or_duplicate_project_content(self):
        with self.assertRaises(m.Failure):
            m.board_index([{"content": None}])
        with self.assertRaises(m.Failure):
            m.board_index([item(), item()])

    def test_project_identity_fields_and_option_types(self):
        api = m.GitHub()
        fields = [{"id": "STATUS", "name": "Status", "dataType": "SINGLE_SELECT", "options": [{"id": str(n), "name": v} for n, v in enumerate(("Todo", "In Progress", "Done"))]},
                  {"id": "START", "name": "Start date", "dataType": "DATE"},
                  {"id": "TARGET", "name": "Target date", "dataType": "DATE"}]
        project = {"user": {"projectV2": {"id": "PROJECT", "title": "deskcat", "number": 5}}}
        with patch.object(api, "graphql", return_value=project), patch.object(api, "connection", return_value=fields):
            self.assertEqual("TARGET", api.project()["fields"]["Target date"]["id"])
            fields[-1]["dataType"] = "TEXT"
            with self.assertRaises(m.Failure):
                api.project()
            fields[-1]["dataType"] = "DATE"
            project["user"]["projectV2"]["title"] = "other"
            with self.assertRaises(m.Failure):
                api.project()

    def test_live_subprocess_boundary_uses_json_stdin_without_shell(self):
        with tempfile.TemporaryDirectory() as directory:
            fake = Path(directory) / "gh-fixture"
            fake.write_text("#!/usr/bin/env python3\nimport json,sys\np=json.load(sys.stdin)\nassert sys.argv[1:]==['api','graphql','--input','-']\nprint(json.dumps({'data':p['variables']}))\n", encoding="utf-8")
            fake.chmod(0o755)
            with patch.dict(m.os.environ, {"DESKCAT_GH": str(fake)}):
                value = m.GitHub().graphql("mutation($v:String!){x}", v="$(must-not-execute); 日本語", field=None)
            self.assertEqual({"v": "$(must-not-execute); 日本語", "field": None}, value)


class FakeAPI:
    def __init__(self):
        self.r, self.i, self.creates, self.writes = record(), item(), 0, []
        self.fail_field = None
        self.lost_create = False

    def project(self):
        return {"id": "PROJECT"}

    def items(self, project):
        return [copy.deepcopy(self.i)] if self.i else []

    def record(self, number):
        return copy.deepcopy(self.r)

    def project_item(self, project, item_id):
        return copy.deepcopy(self.i)

    def records(self):
        return [copy.deepcopy(self.r)]

    def rest(self, path, payload, method):
        self.writes.append((path, copy.deepcopy(payload)))
        if method == "POST":
            self.creates += 1
            self.r["body"] = payload["body"]
            if self.lost_create:
                raise m.Failure("response lost after server commit")
            return self.r
        for key, value in payload.items():
            self.r[key] = ([{"name": v} for v in value] if key == "labels" else
                           [{"login": v} for v in value] if key == "assignees" else
                           {"number": value, "title": "M0 Foundation"} if key == "milestone" and value else value)
            if key == "state":
                self.r["closed_at"] = "2026-09-26T15:00:00Z" if value == "closed" else None
        return self.r

    def graphql(self, query, **variables):
        self.i = item()
        return {}

    def set_project(self, project, item_id, name, value):
        if name == self.fail_field:
            raise m.Failure("project update failed")
        key = {"Status": "status", "Start date": "start", "Target date": "target"}[name]
        self.i[key] = {"name" if name == "Status" else "date": value}
        self.writes.append((name, value))


class RecoveryTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.journal = Path(self.directory.name) / "journal.json"
        self.api = FakeAPI()
        self.plan = {"repo": m.REPO, "kind": "issue", "number": 466, "source": "#466 explicit scope",
                     "expected": {"Target date": "2099-10-01"}}

    def run_plan(self):
        return m.apply(self.api, self.plan, self.journal, pause=lambda _: None)

    def create_plan(self):
        self.plan.pop("number")
        self.plan["create"] = {"title": "fixture only", "body": (m.ROOT / ".github/ISSUE_TEMPLATE/maintenance_task.md").read_text(encoding="utf-8")}
        self.plan["expected"] = m.actual(self.api.r, self.api.i)
        self.plan["expected"].pop("project")
        self.plan["expected"].pop("base")
        self.plan["expected"]["Target date"] = "2099-10-01"
        self.plan["context"] = {"areas": ["area:docs"], "needs": [], "blocked": False, "source": "fixture"}

    def test_partial_success_reuses_number_and_preserves_other_fields(self):
        self.create_plan()
        self.api.fail_field = "Target date"
        with self.assertRaises(m.Failure):
            self.run_plan()
        self.assertEqual(466, m.read(self.journal)["number"])
        self.api.fail_field = None
        self.assertEqual([], self.run_plan()["findings"])
        self.assertEqual(1, self.api.creates)
        self.assertFalse(any(w[0] == "Start date" for w in self.api.writes))

    def test_uncertain_create_recovers_marker_without_duplicate(self):
        self.create_plan()
        self.api.lost_create = True
        with self.assertRaises(m.Failure):
            self.run_plan()
        self.assertEqual([], self.run_plan()["findings"])
        self.assertEqual(1, self.api.creates)

    def test_uncertain_create_with_no_match_stops(self):
        self.create_plan()
        self.api.lost_create = True
        with self.assertRaises(m.Failure):
            self.run_plan()
        self.api.r["body"] = "no marker"
        with self.assertRaisesRegex(m.Failure, "do not create again"):
            self.run_plan()
        self.assertEqual(1, self.api.creates)

    def test_replay_is_noop_and_plan_change_rejected(self):
        self.run_plan()
        writes = len(self.api.writes)
        self.run_plan()
        self.assertEqual(writes, len(self.api.writes))
        self.plan["expected"]["Target date"] = "2099-11-01"
        with self.assertRaisesRegex(m.Failure, "plan changed"):
            self.run_plan()

    def test_stale_before_stops_without_write(self):
        self.plan["before"] = {"Target date": "2000-01-01"}
        with self.assertRaisesRegex(m.Failure, "concurrently"):
            self.run_plan()
        self.assertEqual([], self.api.writes)

    def test_completed_journal_does_not_undo_later_changes(self):
        self.plan["expected"] = {"assignees": ["new-owner"]}
        self.run_plan()
        writes = len(self.api.writes)
        self.api.r["assignees"] = [{"login": "owner"}]
        self.assertTrue(self.run_plan()["findings"])
        self.assertEqual(writes, len(self.api.writes))
        self.assertEqual([{"login": "owner"}], self.api.r["assignees"])

    def test_verified_updates_stay_readonly_with_unrelated_policy_findings(self):
        self.api.r["labels"].append({"name": "unknown"})
        self.plan["expected"] = {"assignees": ["new-owner"]}
        result = self.run_plan()
        self.assertTrue(result["findings"])
        self.assertTrue(result["updates_verified"])
        writes = len(self.api.writes)
        self.api.r["assignees"] = [{"login": "owner"}]
        self.assertTrue(self.run_plan()["findings"])
        self.assertEqual(writes, len(self.api.writes))

    def test_completed_close_does_not_close_an_external_reopen(self):
        self.plan["expected"] = {"state": "closed", "Status": "Done", "Target date": "@closed"}
        self.run_plan()
        writes = len(self.api.writes)
        self.api.r.update(state="open", closed_at=None)
        self.api.i = item()
        result = self.run_plan()
        self.assertTrue(any(f["field"] == "state" and f["number"] == 466 for f in result["findings"]))
        self.assertEqual(writes, len(self.api.writes))
        self.assertEqual("open", self.api.r["state"])

    def test_close_reopen_reclose_uses_new_timestamp(self):
        self.plan["expected"] = {"state": "closed", "Status": "Done", "Target date": "@closed"}
        self.assertEqual("2026-09-27", self.run_plan()["actual"]["Target date"])
        self.journal = self.journal.with_name("reopen.json")
        self.plan["expected"] = {"state": "open", "Status": "In Progress", "Target date": "2099-09-30"}
        self.assertEqual([], self.run_plan()["findings"])
        self.assertEqual("open", self.api.r["state"])
        self.journal = self.journal.with_name("reclose.json")
        self.plan["expected"] = {"state": "closed", "Status": "Done", "Target date": "@closed"}
        self.assertEqual([], self.run_plan()["findings"])

    def test_reopen_requires_new_plan_and_done_requires_close(self):
        self.api.r["state"] = "closed"
        self.plan["expected"] = {"state": "open"}
        with self.assertRaisesRegex(m.Failure, "reopen requires"):
            self.run_plan()
        self.plan["expected"] = {"Status": "Done"}
        with self.assertRaisesRegex(m.Failure, "auto-close"):
            self.run_plan()

    def test_successful_write_with_wrong_readback_does_not_pass(self):
        with patch.object(self.api, "set_project"):
            result = self.run_plan()
        self.assertTrue(result["findings"])
        self.assertEqual("2099-09-30", result["actual"]["Target date"])

    def test_readback_permission_failure_is_not_empty_success(self):
        with patch.object(self.api, "record", side_effect=[record(), m.Failure("403")]):
            with self.assertRaises(m.Failure):
                self.run_plan()

    def test_journal_lock_prevents_concurrent_creation(self):
        with m.lock(self.journal):
            with self.assertRaisesRegex(m.Failure, "locked"):
                self.run_plan()
        self.assertEqual(0, self.api.creates)

    def test_bad_creation_rejected_before_any_write(self):
        self.create_plan()
        self.plan["create"]["body"] = "missing existing template"
        with self.assertRaisesRegex(m.Failure, "template headings"):
            self.run_plan()
        self.assertEqual(0, self.api.creates)

    def test_condition_mismatch_rejected_before_creation(self):
        self.create_plan()
        self.plan["context"]["blocked"] = True
        with self.assertRaisesRegex(m.Failure, "conditional context"):
            self.run_plan()
        self.assertEqual(0, self.api.creates)

    def test_corresponding_issue_metadata_and_frozen_recovery(self):
        self.plan["kind"] = "pr"
        self.api.r = record("pr")
        self.plan["context"] = {"related_issues": [1], "areas": ["area:docs"], "blocked": False, "needs": [], "source": "fixture"}
        issue = record("issue", 1)
        get = self.api.record
        with patch.object(self.api, "record", side_effect=lambda n: copy.deepcopy(issue) if n == 1 else get(n)):
            self.assertEqual([], self.run_plan()["findings"])
            issue["assignees"] = [{"login": "changed"}]
            with self.assertRaisesRegex(m.Failure, "metadata changed"):
                self.run_plan()

    def test_cli_exit_codes_and_error_snapshot(self):
        output = Path(self.directory.name) / "audit.json"
        with patch.object(m, "GitHub", return_value=self.api), patch.object(m, "snapshot", side_effect=m.Failure("denied")), patch("sys.stderr"):
            self.assertEqual(2, m.main(["audit", "--output", str(output)]))
            self.assertEqual("failed", m.read(output)["status"])
        with patch.object(m, "GitHub", return_value=self.api), patch.object(m, "board_index", return_value={466: self.api.i}), patch("sys.stdout"):
            self.assertEqual(0, m.main(["check", "--number", "466"]))
            self.api.r["assignees"] = []
            self.assertEqual(1, m.main(["check", "--number", "466"]))

    def test_new_project_workflow_defaults_and_partial_resume(self):
        self.create_plan()
        self.api.i = None
        def add(query, **variables):
            self.api.i = item()
            self.api.i.update(status={"name": "Todo"}, start=None, target=None)
        self.api.fail_field = "Target date"
        with patch.object(self.api, "graphql", side_effect=add):
            with self.assertRaises(m.Failure):
                self.run_plan()
            self.api.fail_field = None
            self.assertEqual([], self.run_plan()["findings"])
        self.assertEqual(1, self.api.creates)

    def test_invalid_update_context_and_date_do_not_write(self):
        for context in ({}, {"areas": ["priority:high"], "needs": [], "blocked": False, "source": "fixture"}):
            self.plan["context"] = context
            with self.assertRaises(m.Failure):
                self.run_plan()
        self.plan.pop("context")
        self.plan["expected"]["Target date"] = "20991001"
        with self.assertRaises(m.Failure):
            self.run_plan()
        self.assertEqual([], self.api.writes)

    def test_pr_create_base_is_read_back(self):
        self.create_plan()
        self.plan["kind"] = "pr"
        self.plan["expected"].pop("milestone")
        self.plan["create"].update(base="develop", head="codex/test", body=(m.ROOT / ".github/pull_request_template.md").read_text(encoding="utf-8"))
        self.api.r = record("pr")
        self.api.r["base"] = "main"
        self.assertTrue(any(f["field"] == "base" for f in self.run_plan()["findings"]))

    def test_rest_reflection_delay_is_retried(self):
        self.plan["expected"] = {"assignees": ["new-owner"]}
        original = self.api.record
        calls = []
        def delayed(number):
            calls.append(number)
            if len(calls) <= 2:
                return record()
            return original(number)
        with patch.object(self.api, "record", side_effect=delayed):
            self.assertEqual([], self.run_plan()["findings"])
        self.assertGreaterEqual(len(calls), 4)

    def test_merged_pr_reopen_is_rejected(self):
        self.api.r = record("pr")
        self.api.r.update(state="closed", merged_at="2026-09-26T15:00:00Z", closed_at="2026-09-26T15:00:00Z")
        self.plan["kind"] = "pr"
        self.plan["expected"] = {"state": "open", "Status": "In Progress", "Target date": "2099-10-01"}
        with self.assertRaisesRegex(m.Failure, "cannot be reopened"):
            self.run_plan()
        self.assertEqual([], self.api.writes)


if __name__ == "__main__":
    unittest.main()
