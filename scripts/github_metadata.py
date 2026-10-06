#!/usr/bin/env python3
"""Read back GitHub metadata; explicit, resumable local writes. Standard library only.

Policy: CONTRIBUTING metadata sections. See docs/runbooks/github-metadata.md.
No shell hooks, token storage, merge, settings changes or background writer.
"""

import argparse
from contextlib import contextmanager
from datetime import date, datetime, timedelta, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import uuid

sys.path.insert(0, str(Path(__file__).resolve().parent / "hooks"))
import gh_metadata_guard  # noqa: E402 - reuse #348's template selection/heading rules

REPO = "wachi-yoshitaka-11-dev/deskcat"
OWNER = "wachi-yoshitaka-11-dev"
ROOT = Path(__file__).resolve().parent.parent
JST = timezone(timedelta(hours=9))
PROJECT_FIELDS = ("Status", "Start date", "Target date")
REST_FIELDS = ("labels", "assignees", "milestone", "state")


class Failure(ValueError):
    """Unknown/failed is never a successful empty result."""


def read(path):
    return json.loads(Path(path).read_text(encoding="utf-8-sig"))


def save(path, value):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    os.replace(temporary, path)


@contextmanager
def lock(path):
    path = Path(str(path) + ".lock")
    path.parent.mkdir(parents=True, exist_ok=True)
    try:
        fd = os.open(path, os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
    except FileExistsError as exc:
        raise Failure("journal locked; inspect the interrupted writer before recovery") from exc
    os.close(fd)
    try:
        yield
    finally:
        path.unlink()


def jst(timestamp):
    value = datetime.fromisoformat(timestamp.replace("Z", "+00:00"))
    if value.tzinfo is None:
        raise Failure("API timestamp has no timezone")
    return value.astimezone(JST).date().isoformat()


def vocabulary():
    return set(re.findall(r"^- name: (.+)$", (ROOT / ".github/labels.yml").read_text(encoding="utf-8"), re.M))


def iso_date(value):
    if not isinstance(value, str) or not re.fullmatch(r"\d{4}-\d{2}-\d{2}", value):
        raise Failure("date must be YYYY-MM-DD")
    return date.fromisoformat(value)


def conditional_labels(context, known):
    if not isinstance(context, dict) or not {"areas", "blocked", "needs", "source"} <= context.keys():
        raise Failure("context requires areas, blocked, needs, evidence source")
    if not isinstance(context["source"], str) or not context["source"] or type(context["blocked"]) is not bool:
        raise Failure("context requires source text and boolean blocked")
    for field, prefix in (("areas", "area:"), ("needs", "needs:")):
        values = context[field]
        if not isinstance(values, list) or not all(isinstance(v, str) and v.startswith(prefix) and v in known for v in values) or len(set(values)) != len(values):
            raise Failure("invalid conditional " + field)
    related = context.get("related_issues", [])
    if not isinstance(related, list) or not all(type(n) is int and n > 0 for n in related) or len(set(related)) != len(related):
        raise Failure("related_issues must be unique positive numbers")
    return sorted(context["areas"] + context["needs"] + (["status:blocked"] if context["blocked"] else []))


class GitHub:
    def __init__(self, repo=REPO):
        if repo != REPO:
            raise Failure("this policy is scoped to " + REPO)
        self.repo = repo

    def api(self, endpoint, payload=None, method=None):
        argv = [os.environ.get("DESKCAT_GH", "gh"), "api", endpoint]
        if method:
            argv += ["--method", method]
        if payload is not None:
            argv += ["--input", "-"]
        try:
            result = subprocess.run(argv, input=json.dumps(payload) if payload is not None else None,
                                    capture_output=True, text=True, encoding="utf-8", timeout=60)
        except (OSError, subprocess.TimeoutExpired) as exc:
            raise Failure("API transport failed; read back before retrying writes") from exc
        if result.returncode:
            # Do not persist stderr: authentication diagnostics can include sensitive data.
            raise Failure(f"API failed ({result.returncode}): {endpoint.split('?')[0]}")
        try:
            value = json.loads(result.stdout)
        except ValueError as exc:
            raise Failure("API returned invalid JSON") from exc
        if isinstance(value, dict) and value.get("errors"):
            raise Failure("GraphQL errors (including partial data); result rejected")
        return value

    def rest(self, path, payload=None, method=None):
        return self.api(f"repos/{self.repo}/{path}", payload, method)

    def pages(self, path):
        values = {}
        for page in range(1, 10001):
            batch = self.rest(path + ("&" if "?" in path else "?") + f"per_page=100&page={page}")
            if not isinstance(batch, list):
                raise Failure("REST pagination returned a non-list")
            for item in batch:
                key = item["id"]
                if key in values and values[key] != item:
                    raise Failure("REST item changed during pagination; rerun snapshot")
                values[key] = item
            if len(batch) < 100:
                return list(values.values())
        raise Failure("REST pagination limit exceeded")

    def graphql(self, query, **variables):
        value = self.api("graphql", {"query": query, "variables": variables})
        if not value.get("data"):
            raise Failure("GraphQL data unavailable")
        return value["data"]

    def connection(self, query, extract, **variables):
        cursor, seen, values, total = None, set(), {}, None
        while True:
            result = extract(self.graphql(query, cursor=cursor, **variables))
            if not result or "nodes" not in result or "pageInfo" not in result:
                raise Failure("GraphQL connection unavailable")
            if total is not None and result["totalCount"] != total:
                raise Failure("GraphQL total changed; rerun snapshot")
            total = result["totalCount"]
            for node in result["nodes"]:
                if not node or not node.get("id"):
                    raise Failure("inaccessible node in connection")
                if node["id"] in values:
                    raise Failure("duplicate GraphQL node; unstable pagination")
                values[node["id"]] = node
            page = result["pageInfo"]
            if not page["hasNextPage"]:
                if len(values) != total:
                    raise Failure("GraphQL count mismatch")
                return list(values.values())
            cursor = page["endCursor"]
            if not cursor or cursor in seen:
                raise Failure("GraphQL cursor did not advance")
            seen.add(cursor)

    def project(self):
        project = self.graphql('query($owner:String!){user(login:$owner){projectV2(number:5){id title number}}}', owner=OWNER)["user"]["projectV2"]
        if not project or project["title"] != "deskcat" or project["number"] != 5:
            raise Failure("expected owner/project number/title not found")
        fields = self.connection('''query($id:ID!,$cursor:String){node(id:$id){... on ProjectV2{
          fields(first:100,after:$cursor){totalCount pageInfo{hasNextPage endCursor} nodes{
            ... on ProjectV2FieldCommon{id name dataType}
            ... on ProjectV2SingleSelectField{options{id name}}
          }}}}}''', lambda d: d["node"]["fields"], id=project["id"])
        project["fields"] = {}
        for name in PROJECT_FIELDS:
            matches = [f for f in fields if f["name"] == name]
            expected_type = "SINGLE_SELECT" if name == "Status" else "DATE"
            if len(matches) != 1 or matches[0]["dataType"] != expected_type:
                raise Failure("project field missing/ambiguous/wrong type: " + name)
            project["fields"][name] = matches[0]
        options = project["fields"]["Status"]["options"]
        if len({o["name"] for o in options}) != len(options) or not {"Todo", "In Progress", "Done"} <= {o["name"] for o in options}:
            raise Failure("project Status options unavailable/ambiguous")
        return project

    def items(self, project):
        # Singular field lookup avoids silently truncating nested fieldValues connections.
        return self.connection('''query($id:ID!,$cursor:String){node(id:$id){... on ProjectV2{
          items(first:100,after:$cursor){totalCount pageInfo{hasNextPage endCursor} nodes{
            id isArchived content{__typename ... on Issue{number repository{nameWithOwner}}
              ... on PullRequest{number repository{nameWithOwner}}}
            status:fieldValueByName(name:"Status"){... on ProjectV2ItemFieldSingleSelectValue{name optionId}}
            start:fieldValueByName(name:"Start date"){... on ProjectV2ItemFieldDateValue{date}}
            target:fieldValueByName(name:"Target date"){... on ProjectV2ItemFieldDateValue{date}}
          }}}}}''', lambda d: d["node"]["items"], id=project["id"])

    def records(self):
        issues = self.pages("issues?state=all&sort=created&direction=asc")
        pulls = {p["number"]: p for p in self.pages("pulls?state=all&sort=created&direction=asc")}
        if {i["number"] for i in issues if "pull_request" in i} != set(pulls):
            raise Failure("Issue/PR inventories disagree; rerun snapshot")
        for item in issues:
            item["kind"] = "pr" if "pull_request" in item else "issue"
            if item["kind"] == "pr":
                item["merged_at"] = pulls[item["number"]]["merged_at"]
                item["base"] = pulls[item["number"]]["base"]["ref"]
        return issues

    def project_item(self, project, item_id):
        item = self.graphql('''query($id:ID!){node(id:$id){... on ProjectV2Item{
          id project{id} isArchived content{__typename ... on Issue{number repository{nameWithOwner}}
            ... on PullRequest{number repository{nameWithOwner}}}
          status:fieldValueByName(name:"Status"){... on ProjectV2ItemFieldSingleSelectValue{name optionId}}
          start:fieldValueByName(name:"Start date"){... on ProjectV2ItemFieldDateValue{date}}
          target:fieldValueByName(name:"Target date"){... on ProjectV2ItemFieldDateValue{date}}
        }}}''', id=item_id)["node"]
        if not item or item["project"]["id"] != project["id"]:
            raise Failure("project item disappeared or changed membership")
        board_index([item])  # Reject redacted content, including during read-back.
        return item

    def record(self, number):
        item = self.rest(f"issues/{number}")
        item["kind"] = "pr" if "pull_request" in item else "issue"
        if item["kind"] == "pr":
            pr = self.rest(f"pulls/{number}")
            item.update(merged_at=pr["merged_at"], base=pr["base"]["ref"])
        return item

    def set_project(self, project, item_id, name, value):
        field = project["fields"][name]
        if name == "Status":
            options = [o["id"] for o in field["options"] if o["name"] == value]
            if len(options) != 1:
                raise Failure("unknown Status option")
            value = {"singleSelectOptionId": options[0]}
        else:
            iso_date(value)
            value = {"date": value}
        return self.graphql('''mutation($p:ID!,$i:ID!,$f:ID!,$v:ProjectV2FieldValue!){
          updateProjectV2ItemFieldValue(input:{projectId:$p,itemId:$i,fieldId:$f,value:$v}){projectV2Item{id}}}''',
                            p=project["id"], i=item_id, f=field["id"], v=value)


def board_index(items, repo=REPO):
    result = {}
    for item in items:
        content = item["content"]
        if content is None:
            raise Failure("inaccessible project content; cannot prove complete membership")
        if content.get("repository", {}).get("nameWithOwner") != repo:
            continue
        number = content["number"]
        if number in result:
            raise Failure(f"duplicate project content #{number}")
        result[number] = item
    return result


def actual(record, item):
    return {"labels": sorted(x["name"] for x in record["labels"]),
            "assignees": sorted(x["login"] for x in record["assignees"]),
            "milestone": record["milestone"]["number"] if record["milestone"] else None,
            "state": record["state"], "base": record.get("base"), "project": bool(item),
            "Status": (item.get("status") or {}).get("name") if item else None,
            "Start date": (item.get("start") or {}).get("date") if item else None,
            "Target date": (item.get("target") or {}).get("date") if item else None}


def resolved(expected, record):
    result = dict(expected)
    for name, value in result.items():
        if value == "@created":
            if name != "Start date" or record["kind"] != "pr":
                raise Failure("@created is only valid for PR Start date")
            result[name] = jst(record["created_at"])
        elif value == "@closed":
            timestamp = record.get("merged_at") or record["closed_at"]
            if name != "Target date" or not timestamp or record["state"] != "closed":
                raise Failure("@closed requires a completed close/merge")
            result[name] = jst(timestamp)
        elif name in ("labels", "assignees"):
            result[name] = sorted(value)
    return result


def validate(record, item, known, expected=None, context=None, repository_only=False):
    values, findings = actual(record, item), []
    def add(field, want, reason, level="violation"):
        findings.append({"number": record["number"], "kind": record["kind"], "field": field,
                         "expected": want, "actual": values.get(field), "reason": reason, "level": level,
                         "resume": "retry unchanged plan/journal; changed expectations require a new update plan/journal; check --number " + str(record["number"])})
    labels = values["labels"]
    if set(labels) - known:
        add("labels", "labels.yml vocabulary", "unknown label: " + ", ".join(sorted(set(labels) - known)))
    for prefix in ("type:", "priority:"):
        count = sum(x.startswith(prefix) for x in labels)
        if (record["kind"] == "issue" and count != 1) or (record["kind"] == "pr" and count == 0):
            add("labels", prefix + (" exactly one" if record["kind"] == "issue" else " at least one"), "Issue/PR cardinality differs")
    if record["kind"] == "pr" and not any(x.startswith("area:") for x in labels):
        add("labels", "area:*", "PR label rule requires area")
    if not values["assignees"]:
        add("assignees", "responsible person", "do not infer a historical assignee", "pending")
    if record["kind"] == "pr" and values["milestone"] is not None:
        add("milestone", None, "ADR-0012")
    if record["kind"] == "issue" and not values["milestone"]:
        add("milestone", "M0–M6 milestone", "select from scope evidence", "pending")
    elif record["kind"] == "issue" and not re.match(r"^M[0-6](?:\s|$)", record["milestone"]["title"]):
        add("milestone", "M0–M6 milestone", "milestone outside repository stages")
    if not repository_only:
        if not item:
            add("project", True, "deskcat board membership required")
        if values["state"] == "closed":
            if values["Status"] != "Done":
                add("Status", "Done", "closed/merged item")
        elif values["Status"] not in ("Todo", "In Progress"):
            add("Status", "Todo or In Progress", "open/reopened item requires active status")
        for field in ("Start date", "Target date"):
            try:
                iso_date(values[field])
            except (ValueError, TypeError):
                add(field, "ISO date", "required date missing/invalid", "pending")
        if record["kind"] == "pr" and values["Start date"] != jst(record["created_at"]):
            add("Start date", jst(record["created_at"]), "PR creation date in JST")
        if values["state"] == "closed":
            timestamp = record.get("merged_at") or record["closed_at"]
            if not timestamp:
                raise Failure("closed object without termination timestamp")
            if values["Target date"] != jst(timestamp):
                add("Target date", jst(timestamp), "latest close/merge date in JST")
        elif values["Target date"] and values["Target date"] < datetime.now(JST).date().isoformat():
            add("Target date", "review forecast", "past plan is not a proven violation", "notice")
    if context is not None:
        conditional = conditional_labels(context, known)
        observed = sorted(x for x in labels if x.startswith(("area:", "needs:")) or x == "status:blocked")
        if observed != conditional:
            add("labels", conditional, "conditional labels from " + context["source"])
    for field, original in (expected or {}).items():
        if original == "@closed" and record["state"] != "closed":
            add(field, "JST date after close/merge", "@closed requires closed state; object is now open")
            continue
        value = resolved({field: original}, record)[field]
        if values[field] != value:
            add(field, value, "explicit operation expectation")
    return findings


def snapshot(api, repository_only=False):
    records = api.records()
    project = None if repository_only else api.project()
    items = [] if repository_only else api.items(project)
    board = board_index(items)
    findings = [f for record in records for f in validate(record, board.get(record["number"]), vocabulary(), repository_only=repository_only)]
    return {"fetched_at": datetime.now(timezone.utc).isoformat(), "repo": api.repo,
            "scope": "repository-only" if repository_only else "repository-and-project",
            "counts": {"issues": sum(r["kind"] == "issue" for r in records), "prs": sum(r["kind"] == "pr" for r in records), "project_items": len(items)},
            "deduplication": "REST by id; conflicting duplicates fail; GraphQL ids/count/cursors checked",
            "coverage_pending": ["conditional area/needs/blocked and responsible person require scope evidence",
                                 "historical PR labels/assignees at creation and Issue actual start require evidence",
                                 "reopened forecast requires operator expectation; an old date alone cannot prove intent"],
            "records": records, "project": project, "items": items, "findings": findings}


def validate_plan(plan):
    if plan.get("repo") != REPO or plan.get("kind") not in ("issue", "pr") or not plan.get("source"):
        raise Failure("plan requires repo, kind, evidence source")
    if ("number" in plan) == ("create" in plan):
        raise Failure("plan requires exactly one of number/create")
    expected = plan.get("expected", {})
    if not expected or set(expected) - set(REST_FIELDS + PROJECT_FIELDS):
        raise Failure("unknown/empty expected fields")
    if expected.get("state", "open") not in ("open", "closed"):
        raise Failure("state must be open/closed; merge is a separate approved operation")
    if expected.get("Status") == "Done" and expected.get("state") != "closed":
        raise Failure("Done can auto-close an Issue; explicitly require state=closed")
    if "Status" in expected and expected["Status"] not in ("Todo", "In Progress", "Done"):
        raise Failure("unknown expected Status")
    if "number" in plan and (type(plan["number"]) is not int or plan["number"] < 1):
        raise Failure("number must be a positive integer")
    if "milestone" in expected and expected["milestone"] is not None and (type(expected["milestone"]) is not int or expected["milestone"] < 1):
        raise Failure("milestone must be a positive number or null")
    for field in ("labels", "assignees"):
        if field in expected and (not isinstance(expected[field], list) or not all(isinstance(v, str) and v for v in expected[field]) or len(set(expected[field])) != len(expected[field])):
            raise Failure("labels/assignees must be unique string lists")
    if "labels" in expected and set(expected["labels"]) - vocabulary():
        raise Failure("unknown expected label")
    if "context" in plan:
        conditional_labels(plan["context"], vocabulary())
    if plan["kind"] == "pr" and expected.get("milestone") is not None:
        raise Failure("PR milestone prohibited")
    for name in ("Start date", "Target date"):
        if name in expected and expected[name] not in ("@created", "@closed"):
            iso_date(expected[name])
    if expected.get("Start date") == "@closed" or expected.get("Target date") == "@created" or (plan["kind"] == "issue" and expected.get("Start date") == "@created"):
        raise Failure("date source does not match field semantics")
    if "create" in plan:
        required = {"labels", "assignees", "Status", "Start date", "Target date"}
        if plan["kind"] == "issue":
            required.add("milestone")
        if not required <= expected.keys() or not plan.get("context"):
            raise Failure("create requires complete metadata and conditional context")
        create = plan["create"]
        allowed = {"title", "body"} if plan["kind"] == "issue" else {"title", "body", "base", "head", "draft"}
        if set(create) - allowed:
            raise Failure("create only accepts title/body/base/head/draft; metadata belongs in expected")
        if not create.get("title") or not create.get("body"):
            raise Failure("create requires title/body")
        if plan["kind"] == "pr" and (not create.get("base") or not create.get("head")):
            raise Failure("PR create requires explicit base/head")
        if expected.get("state", "open") != "open" or expected["Status"] == "Done" or expected["Target date"] == "@closed":
            raise Failure("create must have active status and a forecast")
        if not expected["assignees"] or (plan["kind"] == "issue" and not expected["milestone"]):
            raise Failure("create requires assignee and Issue milestone")
        labels = expected["labels"]
        for prefix in ("type:", "priority:"):
            count = sum(label.startswith(prefix) for label in labels)
            if count == 0 or (plan["kind"] == "issue" and count != 1):
                raise Failure("invalid create label cardinality")
        if plan["kind"] == "pr" and not any(label.startswith("area:") for label in labels):
            raise Failure("PR create requires area label")
        template_args = ["--label", ",".join(labels)]
        if plan.get("template"):
            template_args += ["--template", plan["template"]]
        headings, error = gh_metadata_guard._expected_template(
            (plan["kind"] if plan["kind"] == "pr" else "issue", "create"), template_args)
        if headings is None:
            raise Failure(error)
        missing = set(headings) - set(gh_metadata_guard._section_headings(create["body"]))
        if missing:
            raise Failure("create body missing template headings: " + ", ".join(sorted(missing)))
        if sorted(s for s in labels if s.startswith(("area:", "needs:")) or s == "status:blocked") != conditional_labels(plan["context"], vocabulary()):
            raise Failure("create labels do not match conditional context")


def related_expectations(api, plan):
    """Only compare corresponding Issues when explicitly identified by the operator."""
    expected = dict(plan["expected"])
    if plan["kind"] == "pr" and "create" in plan:
        expected["base"] = plan["create"]["base"]
    context = plan.get("context")
    if plan["kind"] == "pr" and context and context.get("related_issues"):
        records = [api.record(n) for n in context["related_issues"]]
        if any(r["kind"] != "issue" for r in records):
            raise Failure("corresponding Issue is a PR")
        for field in ("labels", "assignees"):
            derived = sorted({value for r in records for value in actual(r, None)[field]})
            if field in expected and sorted(expected[field]) != derived:
                raise Failure("PR " + field + " differs from explicitly corresponding Issues; resolve scope first")
            expected[field] = derived
    return expected


def apply(api, plan, journal_path, pause=time.sleep):
    validate_plan(plan)
    expected = related_expectations(api, plan)
    digest = hashlib.sha256(json.dumps(plan, sort_keys=True).encode()).hexdigest()
    with lock(journal_path):
        if Path(journal_path).exists():
            journal = read(journal_path)
            if journal["plan_hash"] != digest:
                raise Failure("plan changed: preserve journal and make a separate update plan")
            if journal["expected"] != expected:
                raise Failure("corresponding Issue metadata changed; review a new plan")
        else:
            journal = {"plan_hash": digest, "expected": expected, "marker": str(uuid.uuid4()), "number": plan.get("number"), "create_attempted": False, "before": None}
            save(journal_path, journal)
        project = api.project()  # Resolve/verify names and owner before any write.
        if not journal["number"]:
            marker = "<!-- deskcat-metadata:" + journal["marker"] + " -->"
            if journal["create_attempted"]:
                matches = [r for r in api.records() if marker in (r.get("body") or "") and r["kind"] == plan["kind"]]
                if len(matches) != 1:
                    raise Failure("creation outcome unknown; do not create again. Inspect GitHub and preserve journal marker " + marker)
                journal["number"] = matches[0]["number"]
            else:
                journal["create_attempted"] = True
                save(journal_path, journal)  # Durable BEFORE the non-idempotent request.
                body = dict(plan["create"])
                body["body"] += "\n\n" + marker
                result = api.rest("issues" if plan["kind"] == "issue" else "pulls", body, "POST")
                journal["number"] = result["number"]
            save(journal_path, journal)
        number = journal["number"]
        record = api.record(number)
        if record["kind"] != plan["kind"]:
            raise Failure("target kind differs from plan")
        board = board_index(api.items(project))
        item = board.get(number)
        current = actual(record, item)
        if journal.get("updates_verified"):
            # A completed operation is a receipt, not an ongoing desired-state bot.
            # Never undo a later edit/reopen merely because it equals the old baseline.
            journal.update(verified_at=datetime.now(timezone.utc).isoformat(), actual=current,
                           findings=validate(record, item, vocabulary(), expected, plan.get("context")))
            save(journal_path, journal)
            return journal
        if journal["before"] is None:
            journal["before"] = {**current, **plan.get("before", {})}
            save(journal_path, journal)
        if expected.get("state") == "open" and record["state"] == "closed":
            if record.get("merged_at"):
                raise Failure("a merged PR cannot be reopened")
            if not {"Status", "Target date"} <= expected.keys() or expected["Target date"] == "@closed" or expected["Status"] == "Done":
                raise Failure("reopen requires active Status and new forecast")
        def conflict(field, target, observed):
            if observed != target and observed != journal["before"].get(field):
                raise Failure(f"#{number} {field} changed concurrently; expected {target!r}, actual {observed!r}; review a new plan")
        for field in expected:
            if expected[field] not in ("@created", "@closed"):
                target = sorted(expected[field]) if field in ("labels", "assignees") else expected[field]
                conflict(field, target, current[field])
        # Reset Done before reopening, so the board auto-close rule cannot undo reopen.
        if item and expected.get("state") == "open" and current["Status"] == "Done":
            api.set_project(project, item["id"], "Status", expected["Status"])
        patch = {field: expected[field] for field in REST_FIELDS if field in expected and current[field] != (sorted(expected[field]) if field in ("labels", "assignees") else expected[field])}
        if patch:
            api.rest(f"issues/{number}", patch, "PATCH")
        for attempt in range(3):
            record = api.record(number)  # Actual timestamps only after close.
            observed = actual(record, None)
            if all(observed[k] == (sorted(v) if k in ("labels", "assignees") else v) for k, v in patch.items()):
                break
            if attempt < 2:
                pause(2)
        else:
            raise Failure(f"#{number} repository update pending/mismatch: expected {patch!r}, actual {observed!r}; rerun same journal")
        targets = resolved(expected, record)
        if not item:
            api.graphql('''mutation($p:ID!,$c:ID!){addProjectV2ItemById(input:{projectId:$p,contentId:$c}){item{id}}}''', p=project["id"], c=record["node_id"])
        for attempt in range(3):
            item = api.project_item(project, item["id"]) if item else board_index(api.items(project)).get(number)
            if item:
                break
            if attempt < 2:
                pause(2)
        if not item:
            raise Failure(f"#{number} project membership pending; rerun same journal")
        current = actual(record, item)
        if not journal["before"]["project"] and not journal.get("project_baseline"):
            # A newly added item can acquire workflow defaults. Record these once,
            # then retain the same baseline across retries and partial success.
            journal["before"].update({f: current[f] for f in PROJECT_FIELDS})
            journal["project_baseline"] = True
            save(journal_path, journal)
        for field in PROJECT_FIELDS:
            if field in targets and targets[field] != current[field]:
                conflict(field, targets[field], current[field])
                api.set_project(project, item["id"], field, targets[field])
        findings = []
        for attempt in range(3):
            record = api.record(number)
            item = api.project_item(project, item["id"])
            if number not in board_index([item]):
                raise Failure("read-back project content identity differs")
            findings = validate(record, item, vocabulary(), expected, plan.get("context"))
            if not any(f["level"] != "notice" for f in findings):
                break
            if attempt < 2:
                pause(2)
        journal.update(verified_at=datetime.now(timezone.utc).isoformat(), actual=actual(record, item), findings=findings,
                       updates_verified=all(actual(record, item)[f] == v for f, v in resolved(expected, record).items()),
                       coverage_pending=[] if plan.get("context") else ["conditional labels and responsibility require scope evidence"],
                       resume=f"apply with the same plan and journal for #{number}")
        save(journal_path, journal)
        return journal


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    audit = sub.add_parser("audit")
    audit.add_argument("--output", required=True)
    audit.add_argument("--repository-only", action="store_true")
    check = sub.add_parser("check")
    check.add_argument("--number", type=int, required=True)
    check.add_argument("--expect", help="plan file for exact values and conditional evidence")
    check.add_argument("--repository-only", action="store_true")
    run = sub.add_parser("apply")
    run.add_argument("--plan", required=True)
    run.add_argument("--journal", required=True)
    args = parser.parse_args(argv)
    api = GitHub()
    try:
        if args.command == "audit":
            report = snapshot(api, args.repository_only)
            save(args.output, report)
            result = {k: v for k, v in report.items() if k not in ("records", "items", "project")}
        elif args.command == "check":
            plan = read(args.expect) if args.expect else {}
            if plan and plan.get("number") != args.number:
                raise Failure("expect plan number mismatch")
            record = api.record(args.number)
            if plan and plan.get("kind") != record["kind"]:
                raise Failure("expect plan kind mismatch")
            board = {} if args.repository_only else board_index(api.items(api.project()))
            result = {"number": args.number, "scope": "repository-only" if args.repository_only else "repository-and-project",
                      "coverage_pending": [] if plan.get("context") else ["conditional labels, responsibility and forecast intent require evidence"],
                      "findings": validate(record, board.get(args.number), vocabulary(), related_expectations(api, plan) if plan else None, plan.get("context"), args.repository_only)}
        else:
            result = apply(api, read(args.plan), args.journal)
        print(json.dumps(result, ensure_ascii=False, indent=2))
        return 1 if any(f["level"] != "notice" for f in result["findings"]) else 0
    except (Failure, KeyError, TypeError, ValueError, OSError) as exc:
        result = {"status": "failed", "error": str(exc), "resume": "inspect failure; rerun same plan/journal; never recreate on uncertain response"}
        if args.command == "audit":
            save(args.output, result)
        print(json.dumps(result, ensure_ascii=False), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
