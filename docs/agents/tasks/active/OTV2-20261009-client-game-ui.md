# OTV2-20261009 — complete native game client

```yaml
task_id: OTV2-20261009-client-game-ui
issue: 1927
pr: 1942
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
branch: feat/client-completion-20261009
owner: root
```

## Current state — 2026-10-16 / AUTHORING

Canonical predecessor6d1a7b1fc0a06eb3b54d89b1651f0de97c061795.
Draft PR1942 explicitly returned to AUTHORING before this bounded consumer batch.
Root is sole canonical writer/publisher. Owner requested maximum parallel workers:
five isolated client proposal worktrees plus one read-only server trace/reviewer,
with root-only Rust/native build lease. Routing validated ALLOW.
Temporary worker branches are never independently published or merged.

Current scope: apps/client, crates/renderer identity observation, this task and
the owned evidence checkpoint. No protocol, dependencies, server, database,
production trust, Platform account or deployment mutation. Preserve
protocol-oteryn, distinct WorldId/ChannelId and session-generation fences.

## Batch and remaining validation

1. Exclude own actor by identity in real Session-to-HUD conversion; retain a
   co-located other player. Test actual public Session::admit and unpolled handoff.
2. Align sidebar and Battle panel visibility with current admitted actors,
   floor, viewport and map occlusion. Visibility does not confer attack authority.
3. Distinguish unavailable equipment, absent/empty/populated main backpack and
   empty open container; disable inert paper-doll buttons while preserving real
   item move/use and stale-generation protections.
4. Display actual held renderer backend/adapter, refresh/clear across native
   login/HUD/audit redraws; no selectable fake engine or persisted device identity.
5. Add compiled release identity to Help Info without changing settings.

Local source validation: client239/230/5 and renderer41 pass; strict all-target
Clippy and workspace-boundaries pass; governance and its124 tests pass. The
actual Session regression fails on the former iter() handoff (own identity present)
and passes with others(). Disabled-hover tests simulate actual post-click pointer
movement, not forced-visible tooltips; both battle consumers are covered.
Full workspace fmt also passes. Next: one coherent guarded publication, exact successor
freeze and fresh Windows qualification. Historical binary proof never transfers.
No known World INPUT_PATHS are edited by this batch; preserve accepted World
7429f13b and inputs5ff3f898. Never force/reset/rebase/amend or retrigger CI with no-op.

## Last proven native checkpoint (predecessor only)

6d1a7b1f: native MSVC1.95 client236/227/5 + Session88, Clippy/fmt/release exit0.
Executable SHA256 3bfb7422a0370d4f73d9fa47ad1a2bdebf4d78f28091c5f8ed1df1c3358ccbbb.
DX12 audit exit0,20pages/78 rehashed captures/0invalid.
Normal QA proved system-browser PKCE, both character rows, explicit selection,
ServerAccepted map, received Local chat, physical WASD D/A movement with real map
shift/return, profile-resolved Ctrl+L committed logout and welcome/TCP0.
Owner confirmed the earlier unbound Enter click; old failures are not this result.
Before successor build, exact old test24116 was observed with one active7172
connection. Input/closing were paused, owner was asked and explicitly approved
normal closure after the new build is prepared. Preserve that approval; never
kill unrelated clients or replace a binary under an active process.
Vitals absent while inventory/container/chat/entities present; equipment12 absent.
No full inventory-population or 1:1 parity claim. Private audit/acceptance hashes
and the preserved complete earlier task are in the checkpoint linked below.

## Server-owner handoff / blockers

HP/MP: source shows no-vocation may omit PlayerSpellState and therefore domain3.
Live QA vocation/progression UNKNOWN. Existing accepted content includes none
maxima; never manufacture Knight facts, level1 or UI pools. First-entry facts
load BEFORE the normal fenced progression initializer, so readiness ordering is
also material. CP must allocate a server owner to separate real pool readiness
from class-specific cast readiness while preserving one authoritative pool,
damage/death, capture/restore/reconnect and all session-generation fences.
Exact source chain, accepted contracts, bounded files and RED/GREEN cases are in
the durable checkpoint. This client task has NO server write allocation.

## Open completion gaps / next batches

- Closed-room chat submission must preserve draft/history across every send
  route; ChatRoomSet is membership, not private/relay capability. Server still
  refuses non-Say; do not enable fake private/room behavior.
- Real spell/object assignment sources, picker compositions, equipment commands,
  most General actions and many server-backed panel interiors are missing.
- Physical native clipboard/picker, trackpad and exact original visual acceptance
  remain open where not explicitly qualified.
- Graphics engine switching, full lighting/effects, all sound playback/device
  enumeration, conditions, Misc/Gameplay consumers and automatic screenshot
  triggers remain unavailable. Saved controls do not count as features.
- Action-bar production assignment source absent; pending controls stay disabled.
- Secure remembered sign-in stays OFF; browser PKCE remains the real login path.
- No overall client100%, 1:1, deployed successor or MQ-ready claim.

## Evidence / preserved history

[Current parallel batch and verbatim preceding task](../../evidence/OTV2-20261016-client-parallel-checkpoint.md).
[Earlier task history](../../evidence/OTV2-20261016-client-task-history.md).
Reference authority: apps/client/UI-REFERENCE-COVERAGE.md, UI-FIDELITY.md,
UI-REFERENCE-OBSERVATIONS.md and UI-REFERENCE-SETTINGS.md.
Original materials remain private comparison evidence, not redistributable assets.
