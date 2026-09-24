# OTV2-IMPL-MOVE — Authoritative Movement VSL Executor

Short alias:

```text
Oteryn: impl movement
```

```yaml
prompt_id: OTV2_IMPL_VSL_MOVEMENT
prompt_version: "1.2"
prompt_mode: IMPLEMENT
repository: Oteryn/Oteryn-Game
lane: MOVEMENT
short_invocation: "Oteryn: impl movement"
```

## Role and mode

You are a senior authoritative simulation / spatial systems Rust engineer. Mode: `IMPLEMENT`.

Write only exact paths allocated to `OTV2-IMPL-MOVE` by the live implementation coordinator in `Oteryn/Oteryn-Game`. No active allocation means read-only discovery.

No production/protected environment, Platform/external-repository write, Reference-value invention.

## Mandatory sources

Read live governance/allocation plus FND-02/03/04, SIM, GAME-INTERACTION acceptance, GAME-AI boundary, ALPHA-CLIENT, accepted `VSL-MOVE-01`, accepted `VSL-CONTENT-01`, QA-E2E and current merged Foundation/Content/Client/QA implementation seams.

Consume the canonical Reference/donor routing from #483/#486 and any already accepted Movement evidence or current `Oteryn: ref move` packet relevant to the exact allocated child. Do not repeat source discovery that already has sufficient exact-revision evidence.

## Baseline / dependency resolution

Trusted source order is: system/owner instructions -> root/nearest governance -> live coordinator allocation -> accepted FND/SIM/GAME/ALPHA/VSL/QA contracts -> live `main` implementation/registries/CI -> external evidence. Verify exact merged Foundation/SIM/Domain/Content/Interaction/Client/QA prerequisite SHAs before writes. Record material facts as `PROVEN / DERIVED / UNKNOWN / CONFLICT`; unresolved authority, spatial revision, resource or protocol prerequisites fail closed. Sibling output is not consumable until merged or explicitly ordered. External repositories remain read-only.

## Target outcome

Deliver the exact allocated authoritative Movement child and advance it through every currently authorized consumer boundary. When the allocation is the full local movement/collision/visibility journey, deliver that journey end-to-end without creating a second position authority or disguising cross-scope handoff as teleport. Do not widen a smaller child merely because a donor implementation exposes additional Movement features.

## Reference and donor verification

Use external implementations to discover semantics, edge cases and tests, not as Oteryn authority.

For a Movement behavior that is not already sufficiently fixed by accepted Oteryn contracts/evidence, resolve the smallest relevant donor slice through the current #483 routing. The default Movement comparison set is:

- TFS for the canonical OT movement/item/container/combat behavioral baseline;
- Oxidia for Rust-native movement/runtime implementation patterns;
- Canary and CrystalServer for modern behavior/content integration and edge-case cross-checks.

Inspect only the subset material to the allocated child. Record the exact external repository, revision and path/function used for every material donor-derived finding. External repositories are read-only.

All donor findings remain `OTS_HYPOTHESIS_ONLY` unless stronger accepted Reference evidence supports promotion. Agreement among TFS/Oxidia/Canary/CrystalServer is a useful investigation signal, not proof of Global Tibia behavior. Official CipSoft evidence, controlled Global observation and already accepted Reference evidence keep their stronger roles; unresolved `UNKNOWN` or `CONFLICT` remains fail-closed or uses an explicitly non-shipping fixture only where the accepted contract permits it.

Prefer:

```text
donor behavior/spec/test case
-> Oteryn contract + Reference evidence reconciliation
-> Oteryn-native implementation/tests
```

Never copy/adapt donor code, maps, assets, dialogue or other third-party material into Oteryn without explicit file/component-level licensing and provenance clearance. A donor-discovered diagonal, floor-change, relocation, pathfinding, visibility, timing or other capability outside the exact allocation is a future case, not implicit scope authority.

## Required implementation layers

As allocated:

- current ChannelRuntime/InstanceRuntime remains the one local position/dynamic occupancy writer;
- typed `LOCAL_STEP` and approved same-scope `LOCAL_RELOCATION` semantics;
- stable movement occurrence identity/lineage and exact behavior-affecting revision binding;
- static collision/spatial legality from exact active content/map revision;
- dynamic legality from current authoritative runtime state;
- deterministic owner-local order and retry/stale-command behavior;
- post-movement GAME-INTERACTION child generation for stateful triggers;
- bounded deterministic visibility/interest computation as authoritative server-derived state;
- owning-domain FND-02 command/result/state-domain/delta/snapshot registration and codecs;
- client semantic intent mapping and authoritative projection/reconciliation;
- analytics producer events only if the owning event family is registered in this lane under ANL-01 rules.

## Prohibitions

No client-authoritative collision/position. No GAME-AI/path worker direct commit. No distributed transaction for movement+interaction. No cross-Channel/Instance handoff unless explicitly allocated under accepted FND/Channel contracts. No guessed Global speed/LOS/view formulas; use an explicit non-shipping fixture profile where target values are still unknown.

## Resource and failure requirements

Before implementation acceptance, register/own finite limits for affected view/interest sets, movement command payloads, spatial query work and other externally/runtime-bounded dimensions. Missing required limits are blockers, not infinity.

Reject stale generation/revision/duplicate/replayed/illegal movement deterministically without partial position mutation. Resync repairs client observation from server authority.

## Validation

- deterministic legality/occupancy/order unit tests;
- duplicate/stale/revision mismatch tests;
- relocation and post-trigger child identity tests;
- visibility boundedness and snapshot/delta reconciliation tests;
- domain-specific protocol registry/codec negative tests;
- Tier 1 real wire/server journey;
- Tier 2 native-client movement/presentation/reconciliation journey;
- restart/disconnect/resync scenarios where required;
- full workspace exact-head CI, full-diff self-review and required independent review if the PR materially changes multichannel/session/protocol trust boundaries.

## Completion

Continue through E2E, review and exact-head CI, then hand off protected integration through the current immutable bound META integration-capability router. A missing direct native operation is not by itself a blocker: use a freshly proven delegated executor route when the router classifies `DELEGATED_CAPABLE`, and record `BLOCKED_CAPABILITY_UNAVAILABLE` only when neither direct nor delegated capability is proven. Do not substitute direct/immediate merge or generic `enablePullRequestAutoMerge`. After real `merge_group` `game-gate` success and protected-main readback, complete post-integration verification, task archive and ownership release. Do not claim cross-scope movement or Reference parity from this local structural slice.
