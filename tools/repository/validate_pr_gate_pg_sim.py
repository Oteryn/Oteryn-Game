#!/usr/bin/env python3
"""Validate canonical PR PostgreSQL and simulation qualification contracts."""
from __future__ import annotations

import hashlib
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MERGE_GATE = ROOT / ".github/workflows/merge-gate.yml"
REGISTERED_POSTGRES_TARGETS = (
    ("durability_postgres", "apps/game-server/tests/durability_postgres.rs"),
    ("character_authority_postgres", "apps/game-server/tests/character_authority_postgres.rs"),
    ("runtime_scope_assignment_postgres", "apps/game-server/tests/runtime_scope_assignment_postgres.rs"),
    ("native_admission_source_postgres", "apps/game-server/tests/native_admission_source_postgres.rs"),
)
POSTGRES_IMAGE = (
    "postgres:17.6-bookworm@"
    "sha256:f3bd19c606e442c3d7bdfa8002e03fe260a1023351e0ea4598032022b68dd6e3"
)
# Like the canonical scope/aggregate pins, these bind execution semantics, not just text fragments.
EXPECTED_EVIDENCE_JOB_SHA256 = {
    "rust_linux": "fa9bea9fb2b4ce7cdbabe9679cf4141bcf520947cfeb737faf88212e45f4651e",
    "rust_windows": "f28b0844ae3779d164cb85f5d8ef5bb4532b78baa2cd55e20cdff9e67c47f1d4",
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
            "          ref: ${{ needs.scope.outputs.target_sha }}\n",
            "          EXPECTED_SHA: ${{ needs.scope.outputs.target_sha }}\n",
            "        run: test \"$(git rev-parse HEAD)\" = \"$EXPECTED_SHA\"\n",
        ),
    )

    classifier_fragments = (
        "        id: pg_target\n",
        "          EXPECTED_HEAD: ${{ needs.scope.outputs.target_sha }}\n",
        "          EXPECTED_BASE: ${{ needs.scope.outputs.base_sha }}\n",
        "          GH_TOKEN: ${{ github.token }}\n",
        "          PULL_NUMBER: ${{ needs.scope.outputs.pr_number }}\n",
        "          expected_base = os.environ['EXPECTED_BASE'].strip().lower()\n",
        "          targets = (\n",
        "          def target_blobs(commit: str) -> dict[str, str | None]:\n",
        "                  commit_payload = api(f'/git/commits/{commit}')\n",
        "                  tree_payload = api(f'/git/trees/{tree_sha}?recursive=1')\n",
        "              for name, target in targets:\n",
        "          base_blobs = target_blobs(expected_base)\n",
        "          head_blobs = target_blobs(expected_head)\n",
        "              checkout_present = os.path.isfile(target)\n",
        "                  ['git', 'hash-object', '--', target],\n",
        "              if base_present and not head_present:\n",
        "          pull_after_target = api(f'/pulls/{number_text}')\n",
        "              output.write(\n",
        "                  f\"present={'true' if head_blobs['durability_postgres'] is not None else 'false'}\\n\"\n",
    ) + tuple(
        f"              ('{name}', '{target}'),\n"
        for name, target in REGISTERED_POSTGRES_TARGETS
    )
    errors.extend(
        require_unconditional_evidence_step(
            linux,
            "rust_linux",
            "Classify registered PostgreSQL targets",
            classifier_fragments,
        )
    )

    evidence_fragments = (
        "        run: |\n",
        "          DURABILITY_TARGET_PRESENT: ${{ steps.pg_target.outputs.present }}\n",
        "          run_registered_target() {\n",
        '              cargo +1.94.0 test --locked -p oteryn-game-server --test "$name"\n',
        '            elif [[ -e "$path" ]]; then\n',
    ) + tuple(
        f"          run_registered_target {name} {target}\n"
        for name, target in REGISTERED_POSTGRES_TARGETS
    )
    errors.extend(
        require_unconditional_evidence_step(
            linux,
            "rust_linux",
            "Run registered PostgreSQL E2E targets when allocated",
            evidence_fragments,
        )
    )

    classifier = step_block(linux, "Classify registered PostgreSQL targets") or ""
    evidence = step_block(linux, "Run registered PostgreSQL E2E targets when allocated") or ""
    for forbidden in ("glob(", "rglob(", "fnmatch", "TARGETS_JSON", "fromJSON(", "postgres-target-manifest"):
        if forbidden in classifier or forbidden in evidence:
            errors.append(
                f"merge gate PostgreSQL routing must use only the fixed protected target family; found {forbidden}"
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
                '$client = ".\\\\target\\\\x86_64-pc-windows-msvc\\\\release\\\\oteryn-client.exe"',
                "Test-Path -LiteralPath $client -PathType Leaf",
                "& $client --smoke",
                "cargo +1.94.0 run --locked -p oteryn-synthetic-client-harness --target x86_64-pc-windows-msvc",
                "cargo +1.94.0 test --locked -p oteryn-input-platform --target x86_64-pc-windows-msvc",
                "cargo +1.94.0 test --locked -p oteryn-simulation-determinism --target x86_64-pc-windows-msvc",
            ),
        )
    )
    errors.extend(
        require_unconditional_evidence_step(
            windows,
            "rust_windows",
            "Test Windows input platform",
            (
                "        shell: pwsh\n",
                "        run: cargo +1.94.0 test --locked -p oteryn-input-platform --target x86_64-pc-windows-msvc\n",
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
