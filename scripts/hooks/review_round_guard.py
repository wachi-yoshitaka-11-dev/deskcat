#!/usr/bin/env python3
"""Reserve a round before Claude dispatches a registered review agent.

Structured Agent/Task input only; no shell parsing. Each invocation is one
round. The parent records the result via review_gate session finish before
dispatching another reviewer. Other agent types are outside this guard.
"""

import json
import os
from pathlib import Path
import sys
import subprocess

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import review_session  # noqa: E402


def main():
    try:
        payload = json.load(sys.stdin)
        if payload.get("tool_name") not in ("Agent", "Task"):
            return 0
        agent = payload.get("tool_input", {}).get("subagent_type")
        if agent not in ("fresh-context-reviewer", "consistency-inspector"):
            return 0
        root = payload.get("cwd") or os.environ.get("CLAUDE_PROJECT_DIR")
        if not root:
            raise review_session.Stopped("CLAUDE_PROJECT_DIR or hook cwd is required to locate the review worktree")
        choice = review_session.selected(root)
        work, base, scope = choice["work"], choice["base"], choice["scope"]
        try:
            entry = review_session.begin(root, work, base, scope)
        except FileNotFoundError as exc:
            raise review_session.Stopped(
                f"Issue #{work} has no review record; run session init or restore its saved record before review"
            ) from exc
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse",
            "additionalContext": f"Review round {entry['review_round']} of review {entry['review']} reserved for Issue #{work} (Issue total {entry['number']}). Parent must record the result with review_gate.py session finish; do not reset the count."}}))
        return 0
    except (OSError, ValueError, TypeError, KeyError, AttributeError, subprocess.CalledProcessError) as exc:
        print(f"Review dispatch stopped: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
