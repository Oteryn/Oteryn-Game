# OTV2-20261031-newhaven — complete Newhaven starting island

- Repository: Oteryn/Oteryn-Game
- Branch: `chatgpt/newhaven`
- Status: AUTHORING; draft only, no merge or review request
- Base: `a52549e3b9230c7486a24df66f4e8ec59d1f96cb`
- Primary donor: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8`
- Policy binding: Oteryn/Oteryn `1bfb5ff98c8aa156e73669a14e083a1d464c29fb`, policy 3.1.0
- Scope: C19b starter equipment, Newhaven portal effects, level-eight ferry door, NPC services and tasks, source-backed spawns and movement profiles, and a composed real-World scenario.
- Reserved numbers: migrations 0656–0659, capabilities 238–239, commands 456–459, mechanic ordinals/offer orders 596–599. Existing owners and identifiers are preferred.

## Authority and ownership

The owner supplied the Newhaven implementation prompt and explicitly allocated this branch and one draft pull request. One lead serializes commits and publication. Disjoint workers author starter equipment, NPC services/quests, and World content; the lead owns portal/door integration and final validation.

Local Git authoring is available, but its configured network proxy is unavailable. Publication therefore selects the bound policy's connector-compatible Git Data new-candidate route on this exact single-writer branch: fresh predecessor, one complete successor commit, one non-force ref update, immediate readback. Validation is tied to the resulting exact candidate.

## Existing owners to extend

- Starter grant: `crates/gameplay/items/src/starter_grant.rs`, `crates/gameplay/durability/src/durability/starter_grant.rs`, composed admission.
- Tutorial: `crates/gameplay/quests/src/tutorial/`, character tutorial durability and dispatch session/movement/Corruptor modules.
- NPC: existing dialogue, bank, trade, teaching, travel and quest owners.
- World: existing authored spawn, Item profile and bundle generation tools.

Dawnport files, merged migrations, the shared re-audit, vendor, workflows and repository-policy tooling are excluded. Runtime writes remain session-generation fenced and replay-safe.

## Questions

Questions are posted individually on this draft PR with `QUESTION:`; coordinator replies beginning `ANSWER:` are read before each new work step. Missing donor semantics are not invented. Open items remain under the PR's `Left out` section.

## Validation plan

Focused rule/refusal tests; mounted real-dispatch PostgreSQL cases with completion markers and explicit route-less skips; real World start-to-ferry scenario with replay/reconnect/restart checks; changed-package formatting, clippy and tests; authoring validators; regenerated manifests and world pins; final diff review. Only actually executed results will be reported.
