#!/usr/bin/env python3
"""Validate canonical PR PostgreSQL and simulation qualification contracts."""
from __future__ import annotations

import hashlib
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MERGE_GATE = ROOT / ".github/workflows/merge-gate.yml"
DURABILITY_TARGET = "apps/game-server/tests/durability_postgres.rs"
POSTGRES_IMAGE = (
    "postgres:17.6-bookworm@"
    "sha256:f3bd19c606e442c3d7bdfa8002e03fe260a1023351e0ea4598032022b68dd6e3"
)
# Like the canonical scope/aggregate pins, these bind execution semantics, not just text fragments.
EXPECTED_EVIDENCE_JOB_SHA256 = {
    "rust_linux": "15670ea5a0209d692c41a705b3de58017871f5b90c8931b803657bfc24c52ab9",
    "rust_windows": "de7eca96c0dd87d6a7c6d119a619f3ab67211bbb0982b5a1f1125d8ea7a5ca08",
}


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


def step_block(block: str | None, step_name: str) -> str | None:
    if block is None:
        return None

    lines = block.replace("\r\n", "\n").splitlines(keepends=True)
    marker = re.compile(rf"^      - name: {re.escape(step_name)}\s*(?:#.*)?\n$")
    starts = [index for index, line in enumerate(lines) if marker.fullmatch(line)]
    if len(starts) != 1:
        return None

    start = starts[0]
    end = len(lines)
    sibling_step = re.compile(r"^      - ")
    for index in range(start + 1, len(lines)):
        if sibling_step.match(lines[index]):
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
    if re.search(r"^\s*(?:continue-on-error|['\"]continue-on-error['\"])\s*:", block, re.MULTILINE):
        errors.append(f"merge gate job {job} must not permit continue-on-error")
    return errors


def require_unconditional_evidence_step(
    block: str | None,
    job: str,
    step_name: str,
    fragments: tuple[str, ...],
) -> list[str]:
    step = step_block(block, step_name)
    if step is None:
        return [f"merge gate job {job} missing canonical evidence step: {step_name}"]

    errors: list[str] = []
    for fragment in fragments:
        if fragment not in step:
            errors.append(
                f"merge gate job {job} evidence step {step_name!r} missing canonical contract: "
                f"{fragment.strip()}"
            )

    if re.search(r"^        (?:if|['\"]if['\"])\s*:", step, re.MULTILINE):
        errors.append(
            f"merge gate job {job} evidence step {step_name!r} must not define if; "
            "applicable PG/SIM evidence is unconditional inside the required Rust job"
        )
    if re.search(r"^        (?:continue-on-error|['\"]continue-on-error['\"])\s*:", step, re.MULTILINE):
        errors.append(
            f"merge gate job {job} evidence step {step_name!r} must not permit continue-on-error"
        )
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
            "    if: needs.lanes.outputs.rust == 'true'\n",
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
            "          EXPECTED_BASE: ${{ needs.scope.outputs.base_sha }}\n",
            "          GH_TOKEN: ${{ github.token }}\n",
            "          PULL_NUMBER: ${{ needs.scope.outputs.pr_number }}\n",
            "          expected_base = os.environ['EXPECTED_BASE'].strip().lower()\n",
            "          if re.fullmatch(r'[0-9a-f]{40}', expected_base) is None:\n",
            "              raise SystemExit('expected pull request base SHA is invalid')\n",
            f"          target = '{DURABILITY_TARGET}'\n",
            "          if pull.get('base', {}).get('sha', '').lower() != expected_base:\n",
            "              raise SystemExit('pull request base moved after exact target resolution')\n",
            "          def target_present(commit: str) -> bool:\n",
            "              path = f'/contents/{target}?ref={commit}'\n",
            "              except urllib.error.HTTPError as error:\n",
            "                  if error.code != 404:\n",
            "                  if not isinstance(missing, dict) or missing.get('message') != 'Not Found':\n",
            "              if not isinstance(payload, dict):\n",
            "              if payload.get('path') != target:\n",
            "              if payload.get('type') != 'file':\n",
            "          if isinstance(changed_files, bool) or not isinstance(changed_files, int) or changed_files < 0:\n",
            "          base_present = target_present(expected_base)\n",
            "          head_present = target_present(expected_head)\n",
            "          checkout_present = os.path.isfile(target)\n",
            "          if checkout_present != head_present:\n",
            "          if base_present and not head_present:\n",
            "          pull_after_target = api(f'/pulls/{number_text}')\n",
            "          if pull_after_target.get('state') != 'open':\n",
            "              raise SystemExit('pull request closed during Durability PostgreSQL target classification')\n",
            "          if pull_after_target.get('head', {}).get('sha', '').lower() != expected_head:\n",
            "              raise SystemExit('pull request head moved during Durability PostgreSQL target classification')\n",
            "          if pull_after_target.get('head', {}).get('repo', {}).get('full_name') != repository:\n",
            "              raise SystemExit('pull request repository changed during Durability PostgreSQL target classification')\n",
            "          if pull_after_target.get('base', {}).get('sha', '').lower() != expected_base:\n",
            "              raise SystemExit('pull request base moved during Durability PostgreSQL target classification')\n",
            "          if pull_after_target.get('changed_files') != changed_files:\n",
            "              raise SystemExit('pull request changed-files count moved during Durability PostgreSQL target classification')\n",
            "          with open(os.environ['GITHUB_OUTPUT'], 'a', encoding='utf-8') as output:\n",
            "      OTERYN_TEST_POSTGRES_ADMIN_URL: postgresql://oteryn_test_admin:ci-${{ github.run_id }}-${{ github.run_attempt }}@127.0.0.1:5432/postgres\n",
            "          TARGET_PRESENT: ${{ steps.pg_target.outputs.present }}\n",
            f"          if [[ \"$TARGET_PRESENT\" == \"true\" ]] && [[ -f {DURABILITY_TARGET} ]]; then\n",
            f"          elif [[ \"$TARGET_PRESENT\" == \"false\" ]] && [[ ! -e {DURABILITY_TARGET} ]]; then\n",
            "            echo \"NOT_APPLICABLE: Durability PostgreSQL test target is not allocated on this revision.\"\n",
            "            echo \"Durability PostgreSQL target state changed after classification; failing closed.\" >&2\n",
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
                "    if: needs.lanes.outputs.windows == 'true'\n",
                "          ref: ${{ needs.scope.outputs.target_sha }}\n",
                "          EXPECTED_SHA: ${{ needs.scope.outputs.target_sha }}\n",
                "if ((git rev-parse HEAD).Trim() -ne \"$env:EXPECTED_SHA\")",
                "cargo +1.94.0 test --locked -p oteryn-simulation-determinism --target x86_64-pc-windows-msvc",
            ),
        )
    )

    errors.extend(
        require_unconditional_evidence_step(
            linux,
            "rust_linux",
            "Run Durability PostgreSQL E2E when allocated",
            (
                "        run: |\n",
                "          TARGET_PRESENT: ${{ steps.pg_target.outputs.present }}\n",
                f"          if [[ \"$TARGET_PRESENT\" == \"true\" ]] && [[ -f {DURABILITY_TARGET} ]]; then\n",
                "cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres",
            ),
        )
    )
    errors.extend(
        require_unconditional_evidence_step(
            windows,
            "rust_windows",
            "Verify deterministic simulation golden fixtures",
            (
                "        shell: pwsh\n",
                "        run: cargo +1.94.0 test --locked -p oteryn-simulation-determinism --target x86_64-pc-windows-msvc\n",
            ),
        )
    )

    for job, block in (("rust_linux", linux), ("rust_windows", windows)):
        if (
            block is not None
            and hashlib.sha256(block.encode("utf-8")).hexdigest() != EXPECTED_EVIDENCE_JOB_SHA256[job]
        ):
            errors.append(f"merge gate job {job} must exactly match the canonical evidence job")

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
