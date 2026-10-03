# NPC source audit R4 candidate

This packet publishes the owner's completed offline NPC audit for review. It contains actual native DTO candidates and source branches, rather than a completion percentage. It is not loaded by the content materializer or runtime.

The schema/tooling fixes are based on PR #1358 at `25d79fb52104f1d2ce006f080f4bec665cc346ee`. The frozen data was constructed from its earlier `c1b6d3ce43eab12455df3509c860554a60921da0` context. Later world, Item and creature changes are not overwritten. Applying these candidates to current active content requires a separately qualified admission and regeneration.

## Contents

| File | Review scope |
|---|---|
| `npc-candidates.json` | 1,112 native NPC candidates, their dialogue references and source qualification fields |
| `dialogue-candidates.json` | 902 nonempty source programs and 39 empty source holds needed by the candidate references |
| `source-index.json` | Source URLs, revisions, raw digests, status, branch hashes and unresolved-group counts for 1,112 identities |
| `root-proofs.json` | Complete-root semantic digests and captured-source locators for all 20,872 roots |
| `service-corrections.json` | 71 proposed Service corrections, including eight base travel-price changes and variant/access holds |
| `manifest.json` | Input custody hashes, output hashes, counts and qualification limits |

There are 20,872 accepted source roots and 2,505 held root groups. Of the 902 nonempty programs, 773 use a primary transcript marked VERIFIED, 65 UNVERIFIED, 58 unknown, and six wiki observations. These statuses are preserved source claims, not independent Game qualification. The other 210 NPC have no unambiguous nonempty source program.

Each source program preserves the selected complete source branches and reply order. Conflicting branches stay held; the builder does not merge conflicting children into a guessed conversation. Player transcript prompts do not prove the Game keyword matcher, handler precedence, quest guards, focus/reset behavior or Service actions. The empty holds do not mean that their NPC are silent.

The Service candidates preserve 124 variant holds, 34 postal quote contexts, typed currency and quantity facts. Conditional Postman prices do not become unconditional base prices. Legacy OT fluid subtype values are not mapped automatically into the different native fluid enum. Two invalid presentation references remain unresolved rather than being replaced by a guessed outfit.

Three source-readiness records narrow the audit label `REMOVED_CURRENT_GLOBAL` to `TWO_WIKI_DEPRECATED_OR_INACCESSIBLE`. Their evidence establishes two-wiki deprecated status; Fandom's category can mean removed or currently inaccessible. It must not be used as proof of removal or to delete those NPC.

Five unallocated identity proposals, their two dialogues, the loose observation corpus, raw wiki pages, original client assets and map placements are excluded. Native presentation/behavior/Item dependencies are inherited from the frozen source context; this partial packet is not a standalone WorldProject.

## Reproduce and validate

From the repository root:

```text
python tools/content-schema/npc-authoring/validate_source_audit.py docs/agents/evidence/OTV2-20261001-npc-source-audit-r4
python -m unittest discover -s tools/content-schema/npc-authoring -p "test*.py"
python tools/content-schema/npc-authoring/validate_npc.py tools/content-schema/npc-authoring/samples/bundles
python tools/content-schema/npc-authoring/validate_promotion.py tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json
```

`export_source_audit.py --frozen-root <offline-audit-root> --out <directory>` verifies the three exact input hashes before deterministic UTF-8/LF export. It compares every candidate keyword tree with the frozen selected program, preserving trigger semantics, children and reply order. The original capture artifacts are retained outside the repository and are not downloadable from this packet. `capture-artifact:` labels replace local paths; accompanying digests and source URLs remain available. Custody/serialization checks do not replace source recapture or native qualification.

The older full R4 WorldProject passed native construction, parse, canonical roundtrip and reference-link checks. Those historical checks are not rerun candidate-specific CI. Its Reference playable compiler failed because 56,480 definitions exceeded an Item-only limit of 38,157. No limit is raised here and no playable or Global-completeness claim is made.

## Sources and attribution

NPC text is recorded as third-party reference data under `LICENSE-ASSETS.md`, outside the software MPL license. Original Tibia NPC text remains attributable to CipSoft; no redistribution license is inferred from public access.

Primary transcript source: [s2ward/tibia](https://github.com/s2ward/tibia/tree/8824eb38872a1174b0f0c923e08719e981f32ecc), including its [verified transcript report](https://github.com/s2ward/tibia/blob/8824eb38872a1174b0f0c923e08719e981f32ecc/docs/npc_trees/verified_transcripts.md). Supplemental observations: [TibiaSecrets](https://tibiasecrets.com/), [TibiaWiki](https://tibia.fandom.com/) and [TibiaWiki BR](https://www.tibiawiki.com.br/). Per-NPC source locators and revisions are in `source-index.json`. Derivative transcripts are not counted as independent confirmation. Wiki prose and original media are not included.

Canary and Crystal remain hypotheses/reference sources. The corrected Canary pin is `47dfd51f45280a59a1d3e50ba7edd573d7234446`; Crystal references are pinned in retained candidate provenance. Client appearance data proves the appearance's existence and pixels, not NPC colors, movement, prices or quest conditions.

The owner explicitly requested this draft on 2026-10-01 after the offline audit. Root is the only writer of `codex/npc-source-audit-r4`. This publication grants no merge, queue, review-trigger, runtime or map authority. Programme mapping is KAN-16; Jira synchronization is left to the existing control plane.
