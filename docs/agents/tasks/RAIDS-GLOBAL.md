# RAIDS-GLOBAL

Status: AUTHORING
Branch: `chatgpt/raids`
Base: `38641fecd4558a4a14e6b15b3874110ffa2dff19`
Owner decision: 2026-11-01 attached PROMPT_22_raids.md.

## Outcome

Generate the complete evidenced raid catalogue and connect the existing channel scheduler,
durable raid runner, live monster owner and player message owner. Unknown source fields or
unbound areas stay explicit inactive records. Capability 149 remains unoffered.

## Owned paths

- `tools/content-migration/` raid generator and focused Python tests.
- `content/encounters/raids/` generated catalogue and provenance.
- Existing `crates/gameplay/encounter-rules/src/raid/` and live entity owner.
- Existing `crates/gameplay/durability/src/durability/raid_*` owners.
- `apps/game-server/src/node/raid_scheduler*` and its supervision registration.
- Existing auto-mounted raid PostgreSQL cases; migrations 0740-0743 only after collision check.

## Implementation sequence

A. Verify staged apply-plan SHA-256 values, reconcile Crystal 201189f, wiki evidence and map binding.
B. Realize durable wave identities through the existing live monster owner.
C. Deliver ordered announcements through the existing connected-player message path.
D. Complete, clean up and fence old-generation runs; preserve sourced end rules.
E. Drive active catalogue schedules and deterministic source parity tests.

## Evidence and decisions

Primary donor: Crystal `201189f871d23597b82caf88fd611454a2ddefb8`.
Accepted BOSS-RAID-0 remains the source for native durability and corrected interval probability.
The staged raid-content-1 generic schedules were withdrawn and must not be reinstated.
Only this task lead publishes the allocated branch. Subagents own disjoint file sets.
Questions and answers belong on this task's draft PR.

## Validation

Focused formatting, per-crate clippy and raid tests, generator unittest, content validation and
diff checks only. PostgreSQL execution, full validation, final regeneration and CI belong to the
coordinator, as required by the task. No checks have run on implementation yet.

## Remaining

Catalogue reconciliation and all production phases are in progress. No completion is claimed.
