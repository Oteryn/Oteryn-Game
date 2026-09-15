# OTV2-20260814 — Official Tibia spell-library observations

- Evidence task: `OTV2-20260814-ability-combat-first-evidence-fixtures`
- Reference target: `global-tibia-observable-2026-07-28-post-server-save`
- Retrieval timestamp: `2026-08-14T11:01:22Z`
- Source class: `OFFICIAL_PUBLIC`
- Retrieval mode: search-indexed content for the exact official Tibia Library locators; direct page open from the research environment returned HTTP 403
- Search-index freshness signal: approximately two weeks since crawl; exact crawl timestamp unavailable
- Provenance state: `PENDING` — exact official locator/source identity is verified, but the indexed content could not be directly revalidated against the official live page from this research environment
- Legal review state: `PENDING`
- Usage: paper-only Reference discovery evidence; no proprietary assets/code copied

## Evidence discipline

These are bounded observations of content indexed from the official Tibia Library and surfaced during research on 2026-08-14. The research environment could identify the exact official locators and surfaced their indexed spell content, but a direct page open returned HTTP 403. The search result reported a crawl approximately two weeks earlier; its exact crawl timestamp is unavailable.

This evidence is therefore neither a live-page capture nor a time-locked capture from the accepted 2026-07-28 Reference boundary. It does not establish continuity from the immutable target cut. Because direct content verification against the official page was unavailable, provenance remains `PENDING` for evidence-promotion purposes even though the exact official locator identity is known.

`REFERENCE_EVIDENCE_PARITY_MANIFEST_CONTRACT.md` requires continuity evidence before a post-target observation may support the 2026-07-28 target, and any provenance state other than `CLEARED` independently blocks evidence promotion. Patch-note/search absence is not continuity proof. Therefore manifest cases based only on this capture must remain target-evidence `UNKNOWN`, with the indexed official observations retained as discovery/current-state-near evidence. They must not become `PARITY_CONFIRMED` without target continuity evidence, exact Oteryn implementation/test evidence and cleared legal/provenance gates.

## Light Healing

Official locator:

`https://www.tibia.com/library/?spell=lighthealing&subtopic=spells`

Official Library content surfaced by search at retrieval:

- name: Light Healing;
- formula: `exura`;
- vocations listed: Druid, Monk, Paladin, Sorcerer;
- group: Healing;
- type: Instant;
- magic type: Healing;
- cooldown: 1 second; group cooldown: 1 second;
- experience level: 8;
- mana: 20;
- premium requirement: no.

The surfaced official description presents the spell as healing the caster's own wounds/injuries and describes the effect as limited. This supports discovery of a qualitative self-healing hypothesis for the immutable target, but without continuity evidence and cleared direct-source provenance it does **not** establish that target-cut behavior. It also does not provide an exact healing formula, magnitude range, rounding rule, scaling coefficients, failure/legality precedence or complete target-policy semantics.

## Ice Strike

Official locator:

`https://www.tibia.com/library/?spell=icestrike&subtopic=spells`

Official Library content surfaced by search at retrieval:

- name: Ice Strike;
- formula: `exori frigo`;
- vocations listed: Druid, Sorcerer;
- group: Attack;
- type: Instant;
- magic type: Ice;
- cooldown: 2 seconds; group cooldown: 2 seconds;
- experience level: 8;
- mana: 20;
- premium requirement: no.

The surfaced official description states qualitatively that the spell hits an aimed opponent in close range with ice and that its damage is determined by the caster's magical abilities. This supports discovery of targeted attack + ice-damage semantics for the immutable target, but without continuity evidence and cleared direct-source provenance it does **not** establish that target-cut behavior. It also does not provide an exact tile/range metric, exact target cardinality in every edge case, LoS/floor/PvP/PZ legality, damage formula, scaling coefficients, rounding, resistance interaction, critical/block/dodge behavior or RNG ordering.

## Explicitly unresolved

The following remain `UNKNOWN` for the immutable Reference target and must not be inferred from OTS implementations, current Oteryn convenience or absence of official patch notes:

- continuity from the 2026-07-28 target boundary to the indexed official spell-library content, including every listed metadata field and qualitative description;
- the exact crawl timestamp and whether the indexed content exactly matches the live official page at retrieval time;
- direct-source provenance clearance for the indexed spell content;
- exact Light Healing magnitude/formula/rounding/scaling;
- exact Ice Strike range metric and complete targeting/legality matrix;
- exact Ice Strike damage formula/rounding/scaling/mitigation/resistance;
- exact mana/cooldown commit anchors on failed/interrupted/illegal casts;
- error precedence and client-visible failure behavior;
- any implementation or parity state in Oteryn.

## Evidence needed for promotion

A later package may promote a target case only if it supplies continuity evidence admissible under the manifest contract and clears source/legal provenance, for example a provenance-cleared time-appropriate official artifact/capture or another accepted evidence chain that directly bridges the immutable target boundary. Indexed official-state agreement plus silence about intervening changes is insufficient.
