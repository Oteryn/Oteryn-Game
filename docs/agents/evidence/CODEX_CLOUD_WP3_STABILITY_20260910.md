# Codex Cloud WP3 stability evidence — 2026-09-10

Classification: operational evidence; not task/merge authority.

## Verified observations

- Canonical WP3 PR #356 was Draft/Open at `agent/sqlx-driver-budget-351@933ccef1d37b2f0f31b3d88dd3576b5d804b284a` when the mitigation was prepared.
- Repeated same-line Codex returns used the service message `Codex couldn't complete this request. Try again later.` and preserved the canonical branch when classified as infrastructure-only failures.
- A captured Codex Cloud startup log showed repository-wide automatic dependency discovery traversing many unrelated Rust manifests before the material worker phase, including experiments and multiple vendored/workspace crates.
- The repository root contains many independent Rust manifests and large vendored Tokio/rustls/SQLx trees, so generic repository-wide dependency discovery performs work unrelated to a single WP3 material cell.

## Mitigation

- `tools/codex/cloud-wp3-setup.sh` limits setup-time Cargo fetching to the four WP3 dependency manifests and the Linux target.
- `tools/codex/cloud-wp3-maintenance.sh` avoids dependency refetch/build work on cached continuations.
- `docs/agents/CODEX_CLOUD_WP3_STABILITY_PROFILE.md` changes execution granularity only: one recoverable material unit per Codex Cloud invocation, with full qualification retained for a frozen candidate.

## Limits

This evidence does not prove that repository size, dependency setup, prompt length, quota, or any other single factor caused the Codex service failures. The exact infrastructure root cause remains unknown. The mitigation removes controllable startup/context amplification without weakening required repository validation.
