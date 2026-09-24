# Reusable agent prompts

`../PROMPT_LIFECYCLE.json` is the dispatchability source of truth. Resolve one known alias by reading only its matching lifecycle entry and prompt. Only `status: reusable` entries may run. Files under `retired/` are cold provenance and must not be dispatched.

Prompts are task-specific deltas over root/nearest instructions, bound META and accepted contracts. Alias existence grants no allocation, tracked-file write, control-plane, merge, production or cross-repository authority.

## Primary entry points

- `OTV2_WORK_DELIVERY_COORDINATOR.md` — current #162 programme control plane when live state names it. **`Oteryn: work coordinator`.**
- `OTV2_OWNER_EXECUTION_STATUS_ADVISOR.md` — read-only owner placement/status guide. **`Oteryn: owner execution guide`.**
- `OTV2_SOL_SUPERVISING_ARCHITECT.md` — material architecture escalation inside accepted authority. **`Oteryn: sol supervising architect`.**
- `OTV2_INDEPENDENT_PROGRAMME_ARCHITECTURE_AUDIT.md` — broad independent programme audit. **`Oteryn: audyt`.**

The former `Oteryn: terra game coordinator` and `Oteryn: implementation coordinator` profiles are retired provenance. `OTV2_WORK_DELIVERY_COORDINATOR.md` is the sole reusable mutating Game control-plane profile; live allocation still governs whether it may mutate.

## Current WP3-v2 / upstream-first family

- `OTV2_ASTRA_WP3_V2_PROGRAMME_COORDINATOR.md` — **`Oteryn: astra wp3-v2 programme coordinator`.**
- `OTV2_ASTRA_WP3_V2_ARCHITECTURE_LEAD.md` — **`Oteryn: astra wp3-v2 architecture lead`.**
- `OTV2_SOL_WP3_V2_EVIDENCE_AUDITOR.md` — **`Oteryn: sol wp3-v2 evidence auditor`.**
- `OTV2_ASTRA_WP3_V2_IMPLEMENTATION_LEAD.md` — **`Oteryn: astra wp3-v2 implementation lead`.**
- `../programs/OTV2_WP3_V2_AGENT_LAUNCH_RUNBOOK.md` — current launch order and gates.

The old `OTV2_WP3_WRITER`, `OTV2_WP3_TLS_AUDITOR` and `OTV2_WP3_QUALIFICATION_AUDITOR` aliases targeted the broad #356 lineage and are retired. #356 remains read-only research/evidence; do not dispatch those aliases or restore their broad-fork-first authority.

## Specialist families

- Current delivery leads: `OTV2_SOL_DURABILITY_LEAD`, `OTV2_SOL_SERVER_SEAM_LEAD`, `OTV2_SOL_CLIENT_QA_LEAD`, `OTV2_SOL_MOVEMENT_LEAD`, `OTV2_SOL_COMBAT_LEAD` and their explicitly read-only analyst profiles. Their common short form is `Oteryn: sol <lane> lead` where the prompt defines it.
- Native UI: reusable `OTV2_SOL_NATIVE_UI_*` roles; every mutating role still requires exact live allocation and leases.
- Reference investigation: `OTV2_REFERENCE_INVESTIGATOR.md`, parameterized as **`Oteryn: ref <lane>`**.
- Defect Discovery: reusable `OTV2_DEFECT_DISCOVERY_*` supervisor/lead/qualifier/module roles under their live allocation and runbook.
- Direct implementation recovery: `OTV2_IMPL_*`; read-only unless the unique active control plane grants the exact current lane and owned paths.
- Independent audits: `OTV2_CONTENT_WORLD_INDEPENDENT_AUDIT.md` and `OTV2_WORK_DELIVERY_INDEPENDENT_AUDITOR.md` with the authority limits in their prompt bodies.
- Full Tibia content census/crosswalk programme continuation: `OTV2_FULL_CONTENT_CENSUS_PROGRAMME.md` — **`Oteryn: full content census`**. Continue from the first unfinished protected gate; this profile is subordinate to the canonical Work control plane and grants no independent mutation or merge authority.
- Future-wave preparation: `OTV2_SOL_*_PREP` profiles remain read-only until a later merged allocation activates implementation.

For the complete reusable set, exact short alias, owner, version and supersession rule, perform a targeted lookup in `../PROMPT_LIFECYCLE.json`. Do not maintain another full hand-written catalogue here.

## Dispatch and safety

Reusable prompts are not project state. Refresh only the live Issue/task/branch/PR/check facts material to the next decision. A second reusable coordinator alias is not a second writer or scheduler.

Required external review follows root instructions, the bound META policy and `../OWNER_FUNDED_AI_POLICY.md`. The standing authorization survives chat/worker handoffs. A direct worker never emits the owner-funded review trigger; it returns the exact packet to the unique active control plane for live same-head de-duplication.

High-risk protocol/session/admission/persistence/item/loot/value/multichannel/fencing changes retain applicable genuinely independent exact-head review. Repository gates, protection and Merge Queue remain integration authority.

## Retired provenance

Retired prompt bodies normally live under `retired/`; a short compatibility stub may remain at its historical path when moving it would break retained links. Lifecycle status controls dispatchability. Retired programme runbooks live under `../programs/archive/`. Historical coordinates and examples never regain dispatch authority merely because the files remain readable.
