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
        root = os.environ.get("CLAUDE_PROJECT_DIR") or payload.get("cwd")
        work = os.environ.get("DESKCAT_REVIEW_WORK", "")
        if not root or not work:
            raise review_session.Stopped("set DESKCAT_REVIEW_WORK to the Issue number and initialize/restore its record before review")
        entry = review_session.begin(
            root, work, os.environ.get("DESKCAT_REVIEW_BASE", "origin/develop"),
            os.environ.get("DESKCAT_REVIEW_SCOPE", "Issue scope"),
        )
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse",
            "additionalContext": f"Review round {entry['number']} reserved for Issue #{work}. Parent must record the result with review_gate.py session finish; do not reset the count."}}))
        return 0
    except (OSError, ValueError, TypeError, KeyError, AttributeError, subprocess.CalledProcessError) as exc:
        print(f"Review dispatch stopped: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
