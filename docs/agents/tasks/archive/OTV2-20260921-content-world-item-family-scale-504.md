> Lifecycle closeout: **COMPLETED / ARCHIVED / OWNERSHIP RELEASED**. Canonical delivery PR #737 merged through protected main on 2026-09-22 as `9daf3522efbf799c5d9ffe9817215895d4fa8af0`. Any validating/current checkpoint prose below is retained historical provenance only; live GitHub and protected current state supersede it.

# OTV2-20260921 — Content World full-family Item identity/resource successor

```yaml
task_id: OTV2-20260921-content-world-item-family-scale-504
title: Full-family Item identity registry and artifact v3
mode: MIGRATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-world-item-family-scale-504
issue: 162
architecture_gate: 504
decision_comment: 5767823480
allocation_comment: 5767897229
base_sha: 0bbaa898e5d5a33f844786054e8c7e82148b6180
expected_final_authoring_head: null
pr: 737
owner: Oteryn: work coordinator / content
execution_route: api_native_authoring
publication_route: frozen_api_authored_head
production_authority: NONE
```

## Objective

Close the finite B1 Item identity family in one generation: 38,157 catalogued source identities -> 38,157 stable Oteryn Item identities, while preserving the existing 64 semantic bindings and keeping unsupported gameplay semantics unresolved.

## Owned paths

- `apps/game-server/src/content/cw2_b1_import.rs`
- `apps/game-server/tests/content_world_cw2_b1_import.rs`
- `apps/game-server/src/content/reference_playable.rs`
- `apps/game-server/src/content/project.rs`
- `apps/game-server/src/content/reference_artifact.rs`
- `docs/agents/evidence/OTV2-20260921-content-world-item-family-scale-registry.json`
- this task record

The existing source catalogue tool remains source-census authority and is not required to mint native identities.

## Accepted identity policy

Existing 64 protected keys remain unchanged. The remaining 38,093 identities use the owner-accepted opaque Content Registry allocation epoch. Numeric IDs, display names, paths, hashes, appearances and OTS gameplay observations are provenance only. The exact protected B1 source generation plus allocator epoch freezes the mapping; future reimports cannot renumber or remap it.

## Semantic boundary

Identity-only Items use explicit `Unknown` physical/stack semantics, are non-materializable and have no destination capability. This is not a default gameplay value. Existing 64 keep their already accepted minimal authored semantics. Attack/armor/weight/charges/container/equipment/imbuement/proficiency and other unsupported fields remain provenance-only.

## Resource profile

- v1 one-Item profile: unchanged.
- v2 bounded 2..=64 profile: unchanged.
- v3 family-scale profile: 65..=38,157.
- 38,158: fail closed.
- encoded/decoded limits remain finite and derive from the existing codec constants plus exact 38,157 count.

## Validation before freeze

- full family count = 38,157;
- preserved semantic bindings = 64;
- opaque identity-only bindings = 38,093;
- source/native uniqueness and closure;
- existing 64 provenance revalidated against protected B1 evidence;
- deterministic allocation digest;
- full-family strict JSON resource profile is bound to the exact measured canonical requirement:
  - `imports/candidates.json` decoded fields = `2,098,651`;
  - `imports/candidates.json` decoded string bytes = `42,332,603`;
  - exact limits must pass and each `exact-1` limit must fail closed;
- allocation digest = `ee9219ccf9d8b2350911abca321507ff924ccd4cb83196efd08b91fbdf098966`;
- project validation uses indexed Item lookup (no O(n^2) scan);
- canonical project round-trip;
- v3 compile deterministic;
- max+1 rejection;
- existing v1/v2 regression suite unchanged;
- hosted fmt, strict Clippy, workspace tests, PostgreSQL, Windows/Linux, supply-chain, CodeQL and terminal game-gate.

No runtime/client UI/protocol/persistence/value/production mutation is authorized.
