#!/usr/bin/env python3
"""Validate canonical PR PostgreSQL and simulation qualification contracts."""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MERGE_GATE = ROOT / ".github/workflows/merge-gate.yml"
DURABILITY_TARGET = "apps/game-server/tests/durability_postgres.rs"
POSTGRES_IMAGE = (
    "postgres:17.6-bookworm@"
    "sha256:f3bd19c606e442c3d7bdfa8002e03fe260a1023351e0ea4598032022b68dd6e3"
)


def job_block(text: str, key: str) -> str | None:
    lines = text.replace("\r\n", "\n").splitlines(keepends=True)
    marker = re.compile(rf"^  {re.escape(key)}:\s*(?:#.*)?\n$")
    starts = [index for index, line in enumerate(lines) if marker.fullmatch(line)]
    if len(starts) != 1:
        return None

    start = starts[0]
    end = len(lines)
    sibling = re.compile(r"^  [a-z][a-z0-9_-]*:\s*(?:#.*)?\n$")
    for index in range(start + 1, len(lines)):
        if sibling.fullmatch(lines[index]):
            end = index
            break
    return "".join(lines[start:end])


def require_fragments(block: str | None, job: str, fragments: tuple[str, ...]) -> list[str]:
    if block is None:
        return [f"merge gate missing canonical job: {job}"]

    errors: list[str] = []
    for fragment in fragments:
        if fragment not in block:
            errors.append(f"merge gate job {job} missing canonical contract: {fragment.strip()}")
    if re.search(r"^\s*continue-on-error\s*:", block, re.MULTILINE):
        errors.append(f"merge gate job {job} must not permit continue-on-error")
    return errors


def validate() -> list[str]:
    if not MERGE_GATE.is_file():
        return ["missing canonical pull-request merge gate"]

    text = MERGE_GATE.read_text(encoding="utf-8")
    linux = job_block(text, "rust_linux")
    windows = job_block(text, "rust_windows")

    errors = require_fragments(
        linux,
        "rust_linux",
        (
            "    if: needs.scope.outputs.rust == 'true'\n",
            "      pull-requests: read\n",
            "    services:\n",
            "      postgres:\n",
            f"        image: {POSTGRES_IMAGE}\n",
            "          POSTGRES_USER: oteryn_test_admin\n",
            "          POSTGRES_PASSWORD: ci-${{ github.run_id }}-${{ github.run_attempt }}\n",
            "          POSTGRES_DB: postgres\n",
            "          - 5432:5432\n",
            "          --health-cmd \"pg_isready -U oteryn_test_admin -d postgres\"\n",
            "      - name: Classify Durability PostgreSQL target\n",
            "        id: pg_target\n",
            "          EXPECTED_HEAD: ${{ needs.scope.outputs.target_sha }}\n",
            "          GH_TOKEN: ${{ github.token }}\n",
            "          PULL_NUMBER: ${{ needs.scope.outputs.pr_number }}\n",
            f"          target = '{DURABILITY_TARGET}'\n",
            "          previous_filename = item.get('previous_filename')\n",
            "          status = item.get('status')\n",
            "          if (filename == target and status == 'removed') or (\n",
            "              previous_filename == target and filename != target\n",
            "          with open(os.environ['GITHUB_OUTPUT'], 'a', encoding='utf-8') as output:\n",
            "      OTERYN_TEST_POSTGRES_ADMIN_URL: postgresql://oteryn_test_admin:ci-${{ github.run_id }}-${{ github.run_attempt }}@127.0.0.1:5432/postgres\n",
            "          TARGET_REMOVED: ${{ steps.pg_target.outputs.removed }}\n",
            f"          if [[ -f {DURABILITY_TARGET} ]]; then\n",
            "          elif [[ \"$TARGET_REMOVED\" == \"true\" ]]; then\n",
            "            echo \"Durability PostgreSQL test target was removed or renamed by this pull request; failing closed.\" >&2\n",
            "            echo \"NOT_APPLICABLE: Durability PostgreSQL test target is not allocated on this revision.\"\n",
            "          ref: ${{ needs.scope.outputs.target_sha }}\n",
            "          EXPECTED_SHA: ${{ needs.scope.outputs.target_sha }}\n",
            "        run: test \"$(git rev-parse HEAD)\" = \"$EXPECTED_SHA\"\n",
            "cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres",
        ),
    )
    errors.extend(
        require_fragments(
            windows,
            "rust_windows",
            (
                "    if: needs.scope.outputs.rust == 'true'\n",
                "          ref: ${{ needs.scope.outputs.target_sha }}\n",
                "          EXPECTED_SHA: ${{ needs.scope.outputs.target_sha }}\n",
                "if ((git rev-parse HEAD).Trim() -ne \"$env:EXPECTED_SHA\")",
                "cargo +1.94.0 test --locked -p oteryn-simulation-determinism --target x86_64-pc-windows-msvc",
            ),
        )
    )

    return errors


def main() -> int:
    errors = validate()
    if errors:
        print("Canonical PR PG/SIM gate validation failed:")
        for error in errors:
            print(f"- {error}")
        return 1
    print("Canonical PR PostgreSQL and simulation gate contracts passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
