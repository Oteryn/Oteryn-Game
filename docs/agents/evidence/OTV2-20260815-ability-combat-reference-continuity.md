# OTV2-20260815 — ABILITY_COMBAT Reference continuity and provenance research

- Issue: `#259`
- Task: `OTV2-20260815-ability-combat-reference-continuity`
- Worker lane: `docs/arch-a-reference-continuity`
- Trusted worker base: `088b46638ac014cd7928d6b0b75cee44902fe22c`
- Research session date: `2026-08-15`
- Reference target: `global-tibia-observable-2026-07-28-post-server-save`
- Scope: exactly the four `ABILITY_COMBAT` cases already registered in manifest revision 3
- Runtime/client/protocol/DDL/Platform/production authority: **NONE**
- Result: **0 of 4 cases promoted; all existing `UNKNOWN / PENDING` classifications remain unchanged**

## 1. Decision summary

The bounded research did not obtain admissible evidence that both:

1. directly captures or establishes continuity from the accepted 2026-07-28 post-server-save target boundary for the four registered cases; and
2. clears the source/case provenance and legal-review blockers required by the accepted Reference evidence contract.

Official Tibia search-indexed Library content continues to support the same present/near-present hypotheses recorded by PR #255, but direct retrieval of the exact Library pages from this research environment still returns HTTP 403. Search results expose only relative crawl-age signals such as "2 weeks ago" or "3 weeks ago", not an exact crawl timestamp or a time-locked target-boundary capture.

Official July 27/28 news establishes the maintenance/change boundary and describes selected July 28 production changes. It does not provide affirmative evidence for the four spell cases. The accepted contract explicitly forbids treating patch-note/search silence as continuity proof, so omission of Light Healing or Ice Strike from those notes is not evidence that their behavior was unchanged.

A locator-integrity repair on 2026-08-15 rechecked E1, E2 and E6 separately from their substantive use. The prior E1/E2 news query strings were broad/malformed search-result URLs and do not identify an exact article. Exact E1/E2 article IDs were not independently verified, so those records are now explicitly classified as official News Archive/search-index surfaces rather than exact article identities. E6 has a separately verified exact official News Ticker locator. This locator repair changes no target-evidence, provenance, legal-review, implementation or parity classification.

Accordingly:

```text
ability_combat.light_healing.cast_metadata.v1                 -> UNKNOWN / PENDING
ability_combat.light_healing.self_heal_semantics.v1           -> UNKNOWN / PENDING
ability_combat.ice_strike.cast_metadata.v1                    -> UNKNOWN / PENDING
ability_combat.ice_strike.targeted_ice_damage_semantics.v1    -> UNKNOWN / PENDING
```

No manifest revision is warranted. No mirror update to the first evidence/fixture package is warranted. No owning contract clarification is warranted: the accepted fail-closed continuity/provenance rules handled the evidence gap as intended.

## 2. Preflight case state

| Case | Target evidence | Source/case provenance | Legal review | Oteryn implementation | Parity |
|---|---|---|---|---|---|
| `ability_combat.light_healing.cast_metadata.v1` | `UNKNOWN` | `PENDING` | `PENDING` | `NOT_STARTED` | `PARITY_PENDING_EVIDENCE` |
| `ability_combat.light_healing.self_heal_semantics.v1` | `UNKNOWN` | `PENDING` | `PENDING` | `NOT_STARTED` | `PARITY_PENDING_EVIDENCE` |
| `ability_combat.ice_strike.cast_metadata.v1` | `UNKNOWN` | `PENDING` | `PENDING` | `NOT_STARTED` | `PARITY_PENDING_EVIDENCE` |
| `ability_combat.ice_strike.targeted_ice_damage_semantics.v1` | `UNKNOWN` | `PENDING` | `PENDING` | `NOT_STARTED` | `PARITY_PENDING_EVIDENCE` |

Source: accepted manifest revision 3 on the trusted worker base, delivered by PR #255.

## 3. Admission rule used for this research

The accepted Reference contract is applied literally:

- a post-target observation may support the immutable 2026-07-28 target only when continuity from that boundary is evidenced;
- patch-note/search absence is not continuity proof;
- OTS code is hypothesis/inventory input only and cannot independently prove Global behavior;
- provenance other than `CLEARED` blocks evidence promotion;
- missing sufficient proof remains `UNKNOWN` rather than being filled from implementation convenience;
- classification promotion must be evidence-backed and append history rather than rewrite uncertainty away.

This package therefore distinguishes **source identity**, **retrieved/indexed content**, **time relation to the target**, and **admissibility for target promotion**. Knowing an exact official locator does not make an approximate search-index snapshot a target-time capture. Conversely, official-domain search-index content does not make a malformed or broad result URL an exact article identity; where exact identity is not independently verified, the locator is classified only at the broad archive/search-surface level.

## 4. Bounded source matrix

### E1 — official July 27 maintenance announcement

- Source type: `OFFICIAL_PUBLIC`
- Exact article locator: **NOT VERIFIED** in this repair; no article ID is asserted.
- Discovery surface: official Tibia News Archive/search indexing. Verified broad archive surface: `https://www.tibia.com/news/?id=&subtopic=newsarchive`.
- Locator integrity: the previously recorded `https://www.tibia.com/news/?id3530=&subtopicnewsarchive=` string is malformed as article identity and is retained here only as the historical faulty value being rejected, not as a source locator.
- Published statement surfaced by official-domain search indexing: `2026-07-27 15:00`; July 28 server save expected to take about 45 minutes because of maintenance, with worlds expected back around 10:45 CEST.
- Retrieval mode: official-domain search-indexed content from the broad News surface; exact article identity/direct article retrieval is not claimed.
- Target relation: **boundary context only**; it helps identify the maintenance window immediately preceding the accepted post-server-save cut.
- Admitted use: corroborate that July 28 had an announced maintenance/server-save boundary.
- Rejected use: it says nothing affirmative about Light Healing or Ice Strike behavior and cannot establish continuity for any of the four cases.

### E2 — official July 28 "Balancing, Fixes and Changes"

- Source type: `OFFICIAL_PUBLIC`
- Exact article locator: **NOT VERIFIED** in this repair; no article ID is asserted.
- Discovery surface: official Tibia News Archive/search indexing. Verified broad archive surface: `https://www.tibia.com/news/?id=&subtopic=newsarchive`.
- Locator integrity: the previously recorded `https://www.tibia.com/news/?id968=&subtopicnewsarchive=` string is malformed as article identity and is retained here only as the historical faulty value being rejected, not as a source locator.
- Published statement surfaced by official-domain search indexing: `2026-07-28`; with that day's server save CipSoft reported adjustments to new bosses and hunting grounds introduced with the Summer Update 2026.
- Retrieval mode: official-domain search-indexed content from the broad News surface; exact article identity/direct article retrieval is not claimed.
- Target relation: **same-date change record**, but only for explicitly documented subjects.
- Admitted use: corroborates that production changes were publicly described at the accepted boundary and records only the subjects actually surfaced by the official-domain index.
- Rejected use: omission of Light Healing and Ice Strike cannot establish that those mechanics did not change. The accepted baseline specifically rejects publication/search silence as continuity proof.

### E3 — exact official Light Healing Library locator

- Source type: `OFFICIAL_PUBLIC`
- Locator: `https://www.tibia.com/library/?spell=lighthealing&subtopic=spells`
- Search-indexed content surfaced on 2026-08-15: Light Healing; formula `exura`; Druid/Monk/Paladin/Sorcerer; Healing group; Instant; Healing magic type; cooldown 1s and group cooldown 1s; level 8; mana 20; Premium no; qualitative text says the spell heals the caster's own wounds/injuries.
- Search-index freshness signal: approximately **2 weeks since crawl**; no exact crawl timestamp exposed.
- Direct retrieval: **HTTP 403** from the research environment.
- Target relation: **indeterminate**. A relative two-week crawl age at research time is not a precise timestamped capture tied to the July 28 post-server-save state.
- Admitted use: corroborates the existing PR #255 discovery hypothesis and exact official locator identity.
- Rejected use: cannot independently promote either Light Healing target case because target continuity is not established and direct-source/case provenance remains uncleared.

### E4 — exact official Ice Strike Library locator

- Source type: `OFFICIAL_PUBLIC`
- Locator: `https://www.tibia.com/library/?spell=icestrike&subtopic=spells`
- Search-indexed content surfaced on 2026-08-15: Ice Strike; formula `exori frigo`; Druid/Sorcerer; Attack group; Instant; Ice magic type; cooldown 2s and group cooldown 2s; level 8; mana 20; Premium no; qualitative text describes an aimed opponent in close range hit with ice and damage determined by caster magical abilities.
- Search-index freshness signal: approximately **2 weeks since crawl**; no exact crawl timestamp exposed.
- Direct retrieval: **HTTP 403** from the research environment.
- Target relation: **indeterminate** for the same reason as E3.
- Admitted use: corroborates the existing PR #255 discovery hypothesis and exact official locator identity.
- Rejected use: cannot independently promote either Ice Strike target case because target continuity is not established and direct-source/case provenance remains uncleared.

### E5 — official Library aggregate spell-list surface

- Source type: `OFFICIAL_PUBLIC`
- Locator surfaced by official-domain search: `https://www.tibia.com/library/?spell=icicle&subtopic=spells`
- Search-indexed content includes both `Ice Strike (exori frigo) ... 8 ... 20 ... no` and `Light Healing (exura) ... 8 ... 20 ... no` in the spell list.
- Search-index freshness signal: approximately **3 weeks since crawl**; no exact crawl timestamp exposed.
- Direct retrieval: **HTTP 403** from the research environment.
- Target relation: **indeterminate**. The relative crawl-age label is too coarse to establish whether the indexed snapshot is before, at, or after the exact post-server-save target state, and the aggregate listing does not cover every case aspect.
- Admitted use: additional official-domain corroboration for names/formulas/groups/types/level/mana/Premium hypotheses.
- Rejected use: not a provenance-cleared time-locked bridge and insufficient for qualitative self-heal/targeted-damage semantics or exact cooldown/vocation coverage by itself.

### E6 — official evidence that publication can lag production behavior

- Source type: `OFFICIAL_PUBLIC`
- Exact official News Ticker locator: `https://www.tibia.com/news/?id=8901&subtopic=newsarchive`.
- Locator verification: independently surfaced by official-domain search as the July 20, 2026 News Ticker entry containing the delayed Echo Raid disclosure. The previously recorded `https://www.tibia.com/news/?id2788=&subtopicnewsarchive=` string is malformed as article identity and is rejected.
- Relevant official statement: a July 20 ticker says that since the July 16 server save Echo Raids could no longer spawn on Rookgaard and apologises for the delayed information.
- Target relation: methodological evidence, not one of the four spell mechanics.
- Admitted use: reinforces the already-accepted baseline reason that official publication absence is not proof that no production behavior changed.
- Rejected use: not evidence for any Light Healing/Ice Strike value or semantic.

## 5. Time-relation analysis

| Evidence | Time relation to 2026-07-28 post-server-save target | Sufficient continuity bridge? |
|---|---|---|
| E1 July 27 announcement | immediately pre-boundary context; exact article ID unverified | **NO** — no spell behavior asserted |
| E2 July 28 change note | boundary-date publication; exact article ID unverified | **NO** — silence about these spells is not proof |
| E3 Light Healing indexed page | approximate two-week crawl age only | **NO** — exact snapshot time and continuity unknown |
| E4 Ice Strike indexed page | approximate two-week crawl age only | **NO** — exact snapshot time and continuity unknown |
| E5 aggregate spell list | approximate three-week crawl age only | **NO** — timestamp too coarse and aspect coverage incomplete |
| E6 delayed Echo Raid disclosure | historical methodology example; exact ticker locator verified | **NO** — deliberately demonstrates why silence cannot bridge |

The bounded searches also attempted to surface a provenance-cleared historical archive or exact timestamped capture for the two official Library locators. None was obtained in this research session. This statement is limited to the performed research; it is not a claim that no such artifact exists anywhere. The locator repair also did not independently verify exact article IDs for E1 or E2; that absence is recorded as an unresolved source-identity limitation, not converted into a guessed URL.

## 6. Four-case decisions

### 6.1 `ability_combat.light_healing.cast_metadata.v1`

- Prior classification: `UNKNOWN`.
- Candidate hypothesis corroborated: current/near-current indexed official content matches the PR #255 metadata hypothesis.
- Promotion decision: **REJECTED**.
- Result: retain target `UNKNOWN`, source/case provenance `PENDING`, legal review `PENDING`, parity `PARITY_PENDING_EVIDENCE`.
- Reason: no exact target-time capture or accepted continuity chain; E3/E5 are search-indexed with approximate crawl age and direct HTTP 403; E2 silence is inadmissible for continuity.

### 6.2 `ability_combat.light_healing.self_heal_semantics.v1`

- Prior classification: `UNKNOWN`.
- Candidate hypothesis corroborated: E3 indexed official description remains consistent with qualitative self-healing.
- Promotion decision: **REJECTED**.
- Result: retain target `UNKNOWN`, source/case provenance `PENDING`, legal review `PENDING`, parity `PARITY_PENDING_EVIDENCE`.
- Reason: E3 is not a provenance-cleared target-boundary capture; E5 does not state the qualitative semantics; publication silence cannot fill the gap.

### 6.3 `ability_combat.ice_strike.cast_metadata.v1`

- Prior classification: `UNKNOWN`.
- Candidate hypothesis corroborated: current/near-current indexed official content matches the PR #255 metadata hypothesis.
- Promotion decision: **REJECTED**.
- Result: retain target `UNKNOWN`, source/case provenance `PENDING`, legal review `PENDING`, parity `PARITY_PENDING_EVIDENCE`.
- Reason: no exact target-time capture or accepted continuity chain; E4/E5 are search-indexed with approximate crawl age and direct HTTP 403; E2 silence is inadmissible for continuity.

### 6.4 `ability_combat.ice_strike.targeted_ice_damage_semantics.v1`

- Prior classification: `UNKNOWN`.
- Candidate hypothesis corroborated: E4 indexed official description remains consistent with an aimed close-range opponent, ice damage and magical-ability dependence.
- Promotion decision: **REJECTED**.
- Result: retain target `UNKNOWN`, source/case provenance `PENDING`, legal review `PENDING`, parity `PARITY_PENDING_EVIDENCE`.
- Reason: E4 is not a provenance-cleared target-boundary capture; E5 does not state the qualitative semantics; publication silence cannot fill the gap.

## 7. Conflict and uncertainty handling

No target case is changed to `CONFLICT` by this research. The admissible official-indexed observations are consistent with the existing hypotheses, but they lack the time/provenance qualities required for promotion.

Older/stale community pages surfaced during discovery with values that conflict with some present official-indexed fields, for example historical level/Premium/vocation descriptions. These were not promoted into the case source set because they are community-tier, lack adequate target-time provenance for the immutable cut and do not constitute sufficiently qualified incompatible target evidence. They are a warning against treating undated/current community documentation as canonical target truth, not a reason to assert a target `CONFLICT`.

Material uncertainty remains:

- exact article IDs/source identities for E1 and E2 beyond the verified official News Archive/search surface;
- exact crawl timestamps for E3-E5;
- whether each indexed snapshot is exactly the live official page state at retrieval;
- an affirmative chain proving the relevant fields/semantics across the post-server-save target boundary;
- independent provenance/legal clearance for use in evidence promotion;
- all mechanic aspects deliberately excluded by the four existing case scopes.

## 8. Provenance and legal boundary

Exact public CipSoft/Tibia locator identity is known for E3, E4 and the repaired E6. E1 and E2 are retained only as statements surfaced from official Tibia News Archive/search indexing; their exact article IDs were not verified in this repair, and the former malformed query strings are not treated as source identity. Only bounded textual facts are paraphrased here; no proprietary code, client/server binaries, restricted material or proprietary assets were copied. That is sufficient for bounded public research recording, but it does **not** make the manifest's provenance/legal gates automatically `CLEARED`.

Direct content retrieval of the relevant Library pages remains blocked by HTTP 403 in this environment, the indexed snapshots do not expose exact crawl timestamps, and no separately authorized legal/provenance reviewer has cleared a target-time artifact. The existing case-level `PENDING` provenance and legal-review states therefore remain appropriate.

## 9. Rejected promotion shortcuts

The following were explicitly considered and rejected:

- **Malformed/broad News URLs as exact source identity:** rejected; E1/E2 are explicitly broad archive/search-surface evidence unless an exact article ID is independently verified, while E6 uses the separately verified exact ticker locator.
- **Current/near-current official indexed agreement:** insufficient without target continuity and cleared provenance.
- **Approximate "2 weeks" / "3 weeks" crawl age:** insufficient to create an exact target-time capture or server-save ordering.
- **Official July 28 patch-note silence about the two spells:** forbidden as continuity proof by the accepted Reference contract.
- **Historical official statements about spell availability/levels:** useful history but insufficient to prove no intervening mutation through the July 28 target boundary.
- **Community documentation agreement:** corroborative only and time/provenance limited.
- **OTS/source-code similarity:** not used; explicitly forbidden as independent Global proof.
- **Implementation convenience:** not evidence and not used.

## 10. Repository consequence

This research produces an evidence result, not a mechanic-classification change.

Intentionally unchanged:

- `docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json` — remains accepted revision 3 with all four cases fail closed;
- `docs/architecture/GAME-ABILITY-01_FIRST_REFERENCE_EVIDENCE_FIXTURE_PACKAGE.md` — no classification change exists to mirror;
- `docs/architecture/REFERENCE_EVIDENCE_PARITY_MANIFEST_CONTRACT.md` — no contract defect was found and this worker does not own a redesign of the accepted rule;
- all coordinator-owned current-status/register/horizon overlays;
- all runtime/client/server/protocol/persistence/content/DDL/Platform/production paths.

## 11. Evidence that could justify a later promotion

A later package could reconsider one or more cases only with admissible evidence such as:

- an exact timestamped, provenance-cleared official or owner-primary capture that establishes the relevant case at the accepted post-server-save boundary;
- a provenance-cleared archival chain with timestamps and content sufficient to establish the behavior on both sides needed for continuity, rather than inferring continuity from silence;
- a preserved controlled black-box observation with recorded context/timestamp that is target-relevant and an accepted continuity chain where the observation is post-target;
- another owner-accepted evidence chain that affirmatively proves the relevant behavior persisted through the target boundary.

Even then, target-evidence promotion would not imply Oteryn implementation or `PARITY_CONFIRMED`; exact Oteryn revision and passing fixture/test evidence remain separate requirements.

## 12. Worker handover verdict

**Evidence outcome:** `INSUFFICIENT_FOR_PROMOTION`.

**Case outcome:** `4/4 RETAIN UNKNOWN`; `4/4 RETAIN PROVENANCE PENDING`; `4/4 RETAIN LEGAL REVIEW PENDING`; `4/4 RETAIN PARITY_PENDING_EVIDENCE`.

**Locator-integrity outcome:** E1/E2 exact article IDs remain **NOT VERIFIED** and are represented only as official News Archive/search-surface evidence; E6 exact locator is `https://www.tibia.com/news/?id=8901&subtopic=newsarchive`.

**Architecture outcome:** existing continuity/provenance fail-closed rules are adequate for this representative path; no schema/contract/catalogue-tooling freeze or new stable gate is justified.

**Merge authority:** `ARCHITECTURE_COORDINATOR_ONLY`.