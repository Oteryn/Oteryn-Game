"""Regenerate the exact workflow pin for the owner-approved CATALOG CI addition."""
from __future__ import annotations

import argparse
import hashlib
import re
from pathlib import Path

PREVIOUS_WORKFLOW_SHA256 = "3ad10ed77185ae06bf0b1a2addaf5e94d52723ffc934c0944d028ad69182fff1"
APPROVED_STEPS = '''      - name: Produce public Game catalogue
        run: python3 tools/game-platform-catalog/build.py "$RUNNER_TEMP/game-platform-catalog.json"

      - name: Verify public Game catalogue
        run: python3 tools/game-platform-catalog/producer.py verify "$RUNNER_TEMP/game-platform-catalog.json"

      - name: Upload public Game catalogue
        uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a # v7.0.1
        with:
          name: game-platform-catalog
          path: |
            ${{ runner.temp }}/game-platform-catalog.json
            ${{ runner.temp }}/game-platform-catalog.json.sha256
          if-no-files-found: error
          retention-days: 14

'''


def workflow_digest(workflow: str) -> str:
    if workflow.count(APPROVED_STEPS) != 1:
        raise ValueError("workflow must contain exactly the approved catalogue steps")
    previous = workflow.replace(APPROVED_STEPS, "", 1)
    if hashlib.sha256(previous.encode()).hexdigest() != PREVIOUS_WORKFLOW_SHA256:
        raise ValueError("workflow changed outside the approved catalogue steps")
    return hashlib.sha256(workflow.encode()).hexdigest()


def refresh(root: Path, check: bool = False) -> None:
    digest = workflow_digest((root / ".github/workflows/rust.yml").read_text())
    pin = root / "tools/repository/validate_repository_policy_core.py"
    text = pin.read_text()
    pattern = r'^EXPECTED_POST_MERGE_RUST_SHA256 = "([0-9a-f]{64})"$'
    matches = list(re.finditer(pattern, text, re.MULTILINE))
    if len(matches) != 1 or matches[0].group(1) not in {PREVIOUS_WORKFLOW_SHA256, digest}:
        raise ValueError("unexpected existing workflow pin")
    if check:
        if matches[0].group(1) != digest:
            raise ValueError("stale catalogue workflow pin")
    else:
        pin.write_text(re.sub(pattern, f'EXPECTED_POST_MERGE_RUST_SHA256 = "{digest}"', text, flags=re.MULTILINE))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    refresh(Path(__file__).resolve().parents[2], args.check)
