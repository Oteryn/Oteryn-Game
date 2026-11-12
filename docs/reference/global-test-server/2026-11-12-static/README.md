# Global Test Server: static data capture

Task: `OTV2-20261112-global-test-server-static`

This research workspace collects owner-authorized observations from the visible
Global Tibia Test Server client and compares them with Oteryn. The owner selected
the new Winter Update as the target on 2026-11-12; the observed build is
15.33.ad0955. [PLAN.md](PLAN.md) records the decision, predecessor targets,
coverage and later phases. Individual observations do not certify runtime parity
or authorize automatic content changes.

## Scope and collection order

1. Identify the client build, server and character context.
2. Read character statistics and visible equipment/item descriptions.
3. Read available Cyclopedia item entries and spell definitions.
4. Read visible Bestiary/Bosstiary entries, charms, imbuements and proficiencies.

Only data readable from the current location is in scope. Combat, movement
experiments, formula inference, timing, loot-rate sampling, purchases and
destructive actions belong to later work. Locked entries and hidden fields stay
unknown. One local CLI operator controls the desktop; comparison work consumes
saved records and does not compete for keyboard or pointer focus.

Start with 10–15 item entries to check transcription, evidence retention and
identity matching before expanding collection. Work in coherent batches rather
than pushing each observation separately.

## Files

- `observations.json`: session metadata, evidence references and field-level
  transcriptions. Pilot batch 001 contains 114 field observations for 10 items.
- `comparisons.csv`: field-level comparisons against exact Oteryn revisions.
  Pilot batch 001 contains 114 comparison rows. It is not a completion or parity manifest.

Canonical records live in these files, not in PR comments. Comments summarize
batch results and questions. This PR collects evidence only; confirmed runtime
or content corrections should use separate bounded changes after triage.

## Session and evidence

Before admitting observations, fill in the exact displayed client build and
server identity, observation timestamp with UTC offset, client language and
relevant character context. Record missing metadata as `null`; never infer a
build from a prior chat, screenshot title or observation date. The character
context includes level, vocation, skills, equipment and active modifiers when
they affect the displayed field. Reference it per observation if it changes.

Each evidence entry has `evidence_id`, `captured_at`, `artifact_uri`, `sha256`
and `retention_status` (`LOCAL_ONLY` or `RETAINED`). A laptop path may locate a
working capture but is not a portable retained artifact. Before finalizing a
batch, retain its relevant images in an authorized, access-controlled artifact
location and verify their hashes. Do not invent an attachment URL. Keep large
images, video and archives out of Git. Crop or redact unrelated conversations,
account details and personal information before sharing images.

## Observation fields

Each observation records:

- `observation_id`, `observed_at`, `domain` and `panel`;
- `entity_name`, `entity_variant` and `displayed_id` (null if not displayed);
- `field`, `raw_text`, `value`, `unit` and `availability`;
- `context` and `evidence_ids`;
- `verification` (`TRANSCRIBED` or `VISUALLY_RECHECKED`).

Use domains `CHARACTER`, `ITEM`, `SPELL`, `CREATURE`, `BOSS`, `CHARM`,
`IMBUEMENT` or `PROFICIENCY`. Availability is `VISIBLE`, `LOCKED`,
`NOT_DISPLAYED` or `UNREADABLE`. Non-visible values are `null`, never zero.
Retain exact UI text alongside normalized values, including ranges and units.
Separate item variants, equipped forms, tiers and context-dependent bonuses.
Do not manufacture internal Tibia IDs or match entities by name alone.

## Comparison fields and decisions

Each CSV row references one observation and includes the exact Oteryn commit,
stable entity key, file/record locator, field value and relevant ruleset/context.
`oteryn_value_json` is a JSON-encoded value inside the CSV cell. An empty value
means not yet compared, not a missing Oteryn field; record absence explicitly
in `notes` and use `NOT_FOUND`. Quote CSV cells containing commas or newlines.

Use one comparison status:

- `PENDING`: no completed comparison;
- `MATCH`: visible field agrees under the recorded conditions;
- `DIFFERENCE`: visible values differ;
- `UNKNOWN`: ambiguous identity, missing context or unreadable evidence;
- `NOT_FOUND`: no matching entity/field found in the searched Oteryn revision;
- `DECLARED_DIFFERENCE`: linked accepted Oteryn decision explains the difference.

Every row also records `baseline_relevance`: `UNKNOWN`,
`TEST_SERVER_CHANGE_CANDIDATE`, `CONTINUITY_EVIDENCED` or
`ACCEPTED_DIFFERENCE`, with supporting references in `notes`. The owner-selected
current build additionally uses `TARGET_BUILD_OBSERVATION`: direct evidence from
15.33.ad0955, without a claim about historical continuity or production release.
`MATCH` is a field-level observation, not `PARITY_CONFIRMED`. A current Test
Server value does not establish production or historical-baseline behavior.
Review differences against the owner-selected target and source hierarchy before
proposing a correction. The explicit target decision is recorded in PLAN.md;
historical observations and old target documents are preserved as history, and
repository-wide target-reference reconciliation remains separate follow-up work.

## Batch acceptance

- Session/build metadata and relevant context are recorded.
- Every captured value has readable evidence and has been visually rechecked.
- Entity matching, units and base-versus-modified values are explicit.
- Comparison rows point to the exact code/data revision and locator.
- Unknowns and Test Server changes remain explicit; no completion claims are
  inferred from an empty register or an OTS implementation.
- Relevant evidence is retained before the batch is finalized.

## Existing authority

- [Reference source registry](../../../agents/programs/OTERYN_REFERENCE_INVESTIGATION_SOURCE_REGISTRY_20260910.md)
- [Reference parity precedence](../../../architecture/GAME-VISION-01_REFERENCE_PARITY_PRECEDENCE_OWNER_BASELINE.md)
- [Reference evidence manifest](../../../contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json)

This capture format is local to this investigation; it does not amend those
documents or promote evidence automatically.

## Pilot batch 001 — 2026-11-12

See [BATCH-001.md](BATCH-001.md) and [batch-summary.json](batch-summary.json).
Ten items, 114 observations/comparisons: 81 MATCH, one DIFFERENCE, 18 UNKNOWN,
and 14 PENDING. Twenty retained screenshots include 18 item panels, Info and
Character context. The specific world name remains unknown; this is a recorded
pilot with partial session metadata, not full dataset acceptance. No runtime,
content, pins or production state were changed. Field matches do not establish
admission: all ten inspected family definitions have materializable=false.

