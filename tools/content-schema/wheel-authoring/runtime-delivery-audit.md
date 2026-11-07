# Wheel / Gem authoring and native delivery boundary

Audited main: `e225b3f76e152d195f75cb757b3d639a3ff5577a`, 2026-10-01.
Source evidence is separate from runtime admission. Owner-directed follow-up:
[#162](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5936424268).

## Verdict and comparison

**PROVEN: Oteryn does not yet provide native Wheel or Gem Atelier gameplay at the
audited head.** Both progression indexes declare `READY_UNPOPULATED`; the candidate
has `runtime_admitted:false`. Server `spell/authoring.rs:89-91` refuses
`wheel_unlock:true` because Wheel has no owner. The negative test proves refusal.

| Boundary | Oteryn at audited main | Crystal / Canary inspected source |
|---|---|---|
| Activation | Reference candidate; no native loader/admission | Loaders and enabled configuration |
| 36 slots, reset, budget | Authoring graph; no Character state/writer | Allocation/parser/recalculation |
| Persistence | Generic Character/Item primitives | SQL Wheel points; KV Gem/grade/vessel rows |
| Dedication, Conviction, Revelation | Typed reference effects | Executable callbacks/combat paths, with version differences |
| Reveal/destroy/rotate/lock/upgrade | Policy declarations | Real handlers; atomicity/parity not qualified |
| Vessels, grades, resonance | Catalogue/reference rules | Placement, grades and bonus execution |
| Item materialization | All 18 Gem/fragment/crusher definitions false | Item identities used by handlers |
| Protocol/client | No native Wheel/Gem capability/routes/views/UI | Server packets; external client not audited |

Inventory included native Rust, protocol/session and 30 SQL migrations (last 0031,
0027 absent). Relevant boundaries: server `durability/mod.rs:7-37`,
`durability/character_build.rs:85`, `spell/mod.rs:147,331`; protocol
`src/lib.rs:171,2235-2243`; client `src/lib.rs:4-7`. Generic fees, mint/burn,
CharacterRevision, Harmony and augment fields are foundations, not Wheel integration.

## Findings and dispositions

- **PROVEN / HARDENING, fixed:** `missing_health_step` previously allowed zero.
  The schema now requires a positive number, still bounded by 100. Full-validator
  cases reject zero/negative at every Combat Mastery stage; positive fractional
  tuning and permitted zero bonuses remain valid. Current 14/12/10 are unchanged.
  Canary divides by healthStep; this was not a shipped Oteryn runtime crash.
- **CONFLICT / EVIDENCE_GAP, recorded:** Basic position 2 includes ID 2 Death
  Resistance and excludes ID 30 Mitigation in Oteryn/TibiaPal; four upstream
  variants reverse those two IDs. First-position and Supreme lists agree.
  `samples/basic-mod-position-2-conflict.json` records revisions/hashes/anchors,
  categorical wiki corroboration and the exact-list admission boundary.
- **PROVEN / EVIDENCE_GAP, fixed:** vendor evidence cited an obsolete trade shard.
  Evidence now follows the trade index and reproduces311 offers across 18 keys.
  Declarations do not prove vendor/runtime admission.
- **PROVEN / DELIVERY_GAP, GEM-CONTENT-1:** all 18 keys/appearances exist, but each
  native item has `materializable:false`, `stack_class:Unknown`. The reproducible
  metadata verifier records known/unknown semantics and client flags without
  granting materialization. Upstream XML differs in five observed weights;
  accepted native values are preserved. See `samples/item-delivery-reference.json`
  and `samples/item-source-semantics-observations.json`.

## Coverage, versions and limits

Coverage: 36 slots × 5 vocations, 20 Revelation assignments, 9 unique Conviction types,
46 Basic / 94 Supreme mods and four grades. Independently compared 1,580 Basic values
and 54 spell-augment evidence rows against selected captures. These are reference
checks, not native combat/economy qualification.

Icons: 205 crops / 520 bindings; 189 pixel-equal fallbacks and 16 differing fallbacks
blocked. Item appearances do not map perk UI icons. Client admission/packaging
remain separate. No proprietary PNG/source bytes are added to this batch.

`samples/upstream-variation-summary.json` contains 34 source anchors from 142 verified
Git blobs. Normal HTTPS compared these exact revisions:

- Crystal summer: `00ce02a57ca5a12e48f32a3476e37471167e4c3f`.
- Crystal main: `96d13eff5a1afef17b11b9abc7d574020381cc53`.
- Canary pinned: `99902524e052f37574194466c2949c576e4ab269`.
- Canary main: `04b83b512114bfd888000d6e1433ed8ecaec7c5b`.

No upstream build/external-client E2E. Ten inspected Crystal core files match;
7/36 broader files differ. Canary differs in 17/35. Current Canary is not
automatically a better Global source: Focus Mastery, Healing Link, Guiding and
mitigation differ from selected official/pinned values. Crystal Shield Slam has
an inert-field TODO; Drain Body returns 0 by an OTS decision. Atomicity, temple
enforcement, loot and destroy-equipped-Gem need independent native contract tests.

Wiki HTTP failures used research-only Chrome/CDP. Methods/hashes and unavailable
article content are in `samples/browser-source-audit.json`. English Basic_Mod
supports Death in position 2 and Mitigation in position 1, but its revision and full
positional ID list were not established. Its old mitigation table cannot override
released official values. No new target-date/current-Global certificate is made.
Remote Desktop controlled/read only a browser; project work ran in the cloud.

## Remaining owners

Architect: merged #1472 selects cap225 / Supreme III12.5M, now applied in reference authoring; exact positional catalogue
qualification before GEM-R admission; Guiding+33 arithmetic/rounding/self-copy under
FORMULA. Former candidate250 /12M is superseded in reference authoring. Historical Mystic II40 selection used English
r1206174; Task Shop 50 / Grade IV 69 remain OUT_OF_SCOPE. No new owner value gate is inferred.

Coordinator/workers: catalogue admission; fenced allocation/reset/reconciliation;
durable Gem/grade/vessel/initial-grant state and paid-row migration; atomic Atelier;
stat/spell effects; Item materialization/crusher/loot; protocol/session; client
UI/icons; PostgreSQL/reload/replay/race and effect/operation E2E. Recording gaps
does not implement them or allocate runtime owners.

## Superseding official-perk repair

The owner-directed Oct1 repair selects Battle Healing shield multiplier2 from
official8872 and Mystic RepulseII60% from the current official Tibia.com planner.
The former3/40 selections are obsolete. Selected descriptions now use cooldown−4s,
Guiding party+33%, Focus group cooldown, Flurry affected area and LordII22.5%.
The original source capture preserves contradictory text. A separate independent
official-perk reference qualifies typed values for all five vocations; new negative
regressions cover the prior failures. Runtime delivery and control-plane ownership
remain separate from this authoring correction.

## Final audit disposition and repair

Return AUTHORING from07466202 under direct owner instruction; notice5940467174.
The full final sweep compared current main with Canary04b83b51, pinned99902524,
Crystal main96d13eff and summer00ce02a5. All384 saved drop/reveal source hashes
were requalified and public upstream heads freshly confirmed via normal HTTPS.
EnglishWiki r1151969 and current official manual were reread via research-only
Chrome/CDP after HTTP402/403; browser closed. Wiki confirms225/12.5M but uses
older fragment yields; the manual supports the selected smaller yields and
locked/in-vessel/last-domain dismantle refusals. Do not copy missing OTS guards.

PROVEN EVIDENCE_GAP repaired: independent_trials misdescribed the upstream
stop-on-first-failure loop. Candidate/schema/semantic checks now use maximum_trials
and mandatory stop_on_first_failure; independent selected-reference qualification
also checks the full drop table and225/12.5M grade/cap facts. No native drop
implementation existed to change. Absolute Global rates remain unproved; OTS
weights are the owner's retained reference, not a new request for a decision.

Item evidence requalification passes all18 current definitions. Its
native_definition_sha256 hashes the canonical definition, not the entire shard;
shard hashes are in input_digests. An audit comparison of those distinct hashes
was incorrect and is withdrawn; no stale Item evidence was found. All18 still
have materializable:false/Unknown stack class, so Item delivery remains open.

Unchanged worker scope: native W-R/GEM-R admission, fenced durable Wheel/Gem/grade/
vessel/initial-grant state, atomic fees/item writes and receipt/replay, effects,
crusher/drop/trade, protocol, client UI/icons and PostgreSQL/restart/race/E2E.
Guiding+33 arithmetic remains architect scope. Native reveal main-backpack-direct
is a declared contract difference; upstream searches nested inventory+StoreInbox
without stash/depot/postalInbox. No source proves Global's exact search scope.
AmberCrusher, presets and vocation change are outside the admitted V1 scope.

## Superseding owner cost correction (2026-10-01)

The owner explicitly corrects Supreme Grade II -> III to **12,000,000 gold +
15 Greater Fragments**. The selected candidate and independent qualification
now use12M/15, while cap225 is retained. Wiki r1151969 and merged#1472 list12.5M;
that disagreement is preserved as reference evidence, not selected over the
owner's instruction. The preceding12.5M selections in this document are historical
and superseded by this section. Coordinator/architect must synchronize the
owning decision before native admission; this authoring repair admits no runtime.

The owner additionally confirms this12M/15 value from Global. Classification:
OWNER_CONFIRMED_GLOBAL_VALUE; no independent worker observation of the transaction
is claimed. This confirms the one price row, not all Global RNG/runtime behavior.
