> Lifecycle closeout: **ARCHIVED / OWNERSHIP RELEASED**. Delivery PR #623 merged as `763c7a7d67fa118f253f214f0219067300e69811`. Any active/checkpoint language below is historical provenance only; live GitHub and protected current state supersede it.

# OTV2-20260915-rustls-advisory-622

```yaml
task_id: OTV2-20260915-rustls-advisory-622
title: Repair protected-main rustls advisory
mode: REPAIR
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: security/rustls-0.23.45-622
issue: 622
base_sha: 775a09091743af395ecb8f1e440cb9c286bc0dd2
owner: ChatGPT sole writer
created_at: 2026-09-15T08:31:00Z
updated_at: 2026-09-15T08:31:00Z
owned_paths:
  - Cargo.lock
  - docs/agents/tasks/active/OTV2-20260915-rustls-advisory-622.md
```

## Outcome

Remove `RUSTSEC-2026-0285` from the protected-main resolved dependency graph by updating the registry-resolved `rustls` package from 0.23.43 to the fixed 0.23.45 release, without changing runtime source, workflows, protection, or the canonical WP3/#356 branch.

## Proven scope

- Protected `main@775a09091743af395ecb8f1e440cb9c286bc0dd2` has no root `[patch.crates-io]` entry for rustls; its failing supply-chain graph resolves registry rustls 0.23.43 from `Cargo.lock`.
- `RUSTSEC-2026-0285` reports 0.23.13 through 0.23.44 affected and 0.23.45 fixed.
- crates.io index metadata for rustls 0.23.45 has checksum `0d41d731c7d2f962d1ccc364cec258de3c0e93b38c2fb3ba97ac74513048d634`; its normal dependency requirements are compatible with the already resolved protected-main graph, including aws-lc-rs >=1.18 and rustls-webpki ^0.103.14.
- Draft PR #356 is a separate WP3 lineage and currently patches rustls to `vendor/rustls-0.23.43`; this repair must not mutate that branch or vendor tree.

## Acceptance

- [ ] Generate the lockfile update using Cargo, not manual dependency invention.
- [ ] Final material diff changes only the rustls registry resolution needed for 0.23.45, plus this task record.
- [ ] `cargo deny check advisories licenses bans sources` passes on the exact candidate.
- [ ] Locked metadata, Linux workspace, Windows client, CodeQL and repository governance remain green on the exact candidate.
- [ ] No `vendor/**`, root `Cargo.toml`, workflow, runtime, architecture, ruleset, protection or PR #356 mutation.
- [ ] Independent review has no unresolved material findings.

## Integration boundary

Protected integration remains governed by the repository's bound Merge Queue policy. This session does not expose the required native `merge-async` operation or a trusted capability-observer decision proving the delegated route, so no direct merge, generic auto-merge, GraphQL enqueue, bypass or protection change is authorized. Preserve a qualified candidate if integration capability remains unavailable.

Refs #622 #621 #356 #351.
