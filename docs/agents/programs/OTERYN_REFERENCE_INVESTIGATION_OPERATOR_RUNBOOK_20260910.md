# Oteryn Reference Investigation — Operator Runbook

- Programme: #486
- Control plane: #162 / current `OTV2_WORK_DELIVERY_COORDINATOR`
- Date: 2026-09-10
- Canonical investigator prompt: `docs/agents/prompts/OTV2_REFERENCE_INVESTIGATOR.md`
- Source policy: `OTERYN_REFERENCE_INVESTIGATION_SOURCE_REGISTRY_20260910.md`
- Runtime/write authority granted: **NONE**

## 1. Goal

Run deep Reference investigation in parallel without creating seven independent control planes or seven duplicated long prompts.

Use one parameterized reusable prompt with short aliases. Research lanes are read-only; canonical implementation workers remain separately allocated and owned.

## 2. Control plane — exactly one

Keep the existing programme coordinator:

```text
Oteryn: work coordinator
```

Recommended current execution profile:

```yaml
surface: ChatGPT Work
model: GPT-5.6 Sol or current highest compatible Sol-class model
effort: Medium
role: single #162 control plane
```

Do **not** launch `Oteryn: terra game coordinator` as a second mutating coordinator for the same #162 lifecycle. It remains recovery/read-only unless a protected control-plane transfer explicitly changes the active profile.

The coordinator owns:

- GitHub LIVE reconciliation;
- allocations/ownership/DAG;
- deduplication;
- deciding when research is sufficient for an implementation allocation;
- integration lifecycle.

It should not personally perform every deep investigation.

## 3. Short aliases

All aliases resolve the single registered `OTV2_REFERENCE_INVESTIGATOR` prompt.

| Alias | Investigation scope | Recommended effort |
|---|---|---:|
| `Oteryn: ref world` | World/map + world-facing Content | High |
| `Oteryn: ref combat` | Ability + Combat + creatures + AI/spawns | Extra High |
| `Oteryn: ref char` | Character + items + progression | High |
| `Oteryn: ref npc` | NPC + quest + services | High |
| `Oteryn: ref move` | Movement + Interaction + actor-runtime evidence | Extra High |
| `Oteryn: ref durability` | loot/pickup/item value durability | Extra High |
| `Oteryn: ref evidence` | cross-domain evidence normalization/gap detection | High |
| `Oteryn: ref qa` | Reference journey/QA synthesis | High |

Long equivalent when debugging alias resolution:

```text
Oteryn: reference investigator <lane>
```

## 4. Protected adoption canary — run first

The prompt is new reusable instruction infrastructure. The bound `PROMPT_EVAL_STANDARD` requires actual adoption/delivery and representative behavior to be distinguished from static contract checks.

After this prompt PR is protected-integrated, **do not launch the full Wave 1 immediately**. First open one separate chat and run exactly:

```text
Oteryn: ref world
```

with GPT-5.6 Sol / **High**.

The canary must prove all of the following from protected `main`:

```text
CANONICAL_PROMPT_RESOLVED = OTV2_REFERENCE_INVESTIGATOR
LANE = world
SOURCE_REGISTRY_LOADED = true
PROJECT_TRUTH = GitHub LIVE
TRACKED_FILE_WRITE_AUTHORITY = NONE_BY_ALIAS
SECOND_CONTROL_PLANE_CREATED = false
OTS_PROMOTED_TO_REFERENCE_TRUTH = false
OUTPUT_PACKET_SHAPE = Reference investigator lane packet
```

It should perform one bounded representative world/content investigation using the existing #486/#483 context and return a normal lane packet. Compare its behavior to the intent of the previous R0/R1/R2 Reference launch lines: the new alias may reduce repeated instruction text, but it must preserve target/source/authority/anti-duplication rules.

Classify the canary:

```text
ADOPTION_PASS
ADOPTION_FAIL
NOT_EVALUATED
```

Only `ADOPTION_PASS` releases the remaining Wave-1 investigator aliases for broad parallel use. If alias resolution or source delivery fails, use the long equivalent `Oteryn: reference investigator world` only to diagnose the delivery issue; do not claim the reusable alias adopted successfully until the canonical protected prompt is actually consumed.

The canary is read-only and does not consume a mutating writer slot.

## 5. Wave 1 — launch after canary PASS

Recommended active set after `ADOPTION_PASS`:

```text
CONTROL PLANE
  Oteryn: work coordinator       — Medium

SEPARATE RESEARCH CHATS
  Oteryn: ref world              — High        # canary chat may simply continue
  Oteryn: ref combat             — Extra High
  Oteryn: ref char               — High
  Oteryn: ref npc                — High
```

This produces broad content coverage quickly:

```text
WORLD/MAP/CONTENT
+ ABILITY/COMBAT/CREATURE/AI
+ CHARACTER/ITEMS/PROGRESSION
+ NPC/QUEST/SERVICES
```

All four investigator chats are read-only and therefore do not compete for implementation write ownership.

### Why not launch every lane at once?

The repository scheduler allows broad read-only work, but practical context/review throughput is better with four primary investigators at once. Movement and Durability depend heavily on findings from the first wave and live infrastructure state, so they are more useful as Wave 2.

## 6. Evidence consolidation

After the first useful A-D packets exist, launch:

```text
Oteryn: ref evidence             — High
```

Its job is to:

- normalize field-level evidence;
- identify duplicate claims;
- identify contradictions between investigators;
- identify fields based only on OTS;
- separate target-date continuity gaps from ordinary missing data;
- rank the smallest official/black-box checks needed next.

This is **not** a formal independent audit. Use:

```text
Oteryn: work auditor
```

for a genuinely independent control-plane/programme audit when needed.

## 7. Wave 2 — infrastructure-sensitive investigation

Rotate available research slots to:

```text
Oteryn: ref move                 — Extra High
Oteryn: ref durability           — Extra High
```

### Movement/runtime lane

Use after World/Combat investigators have identified the exact spatial/targeting cases that matter. This lane closes evidence/readiness around:

- movement/collision/floor transitions;
- Interaction;
- range/LoS inputs;
- authoritative actor identity/current-owner requirements;
- #508 physical runtime gaps.

It must not create an Ability-owned actor registry.

### Durability/value lane

Use alongside the live WP3/WP4 dependency chain to prepare:

```text
loot materialization
-> durable ItemInstance
-> corpse custody
-> pickup
-> inventory/equipment transfer
-> later one NPC trade
```

It consumes #513/#506/#507 and must not create a second transaction/persistence owner.

## 8. Wave 3 — QA synthesis

When the first investigation set is materially populated, run:

```text
Oteryn: ref qa                   — High
```

It builds the cross-domain Reference journey matrix:

```text
world entry
-> movement
-> interaction
-> attack + heal
-> creature kill
-> XP
-> corpse
-> loot/pickup
-> inventory/equipment
-> death/re-entry
-> AI/spawn
-> NPC/depot/trade
-> reconnect/restart
-> native client presentation
```

It does not fake missing E2E. Each stage stays parity-confirmed/pending/conflict/difference/out-of-scope according to real evidence.

## 9. How to launch in ChatGPT

### Separate-chat mode

For each investigator:

1. open a new normal ChatGPT text chat;
2. select GPT-5.6 Sol;
3. select the effort from the table;
4. paste **only the alias**, for example:

```text
Oteryn: ref combat
```

The agent must resolve the canonical prompt from protected `main`; do not paste an old cached copy of the long prompt unless alias resolution itself is being diagnosed.

When a separate chat returns its final lane packet, pass that packet to the existing `Oteryn: work coordinator` chat. Cross-chat memory is convenience only, never authority.

### Work/subagent mode

The existing Work coordinator may dispatch the same investigator lanes as read-only subagents when that capability is available. The same lane/source contracts apply. The coordinator still remains the only control plane and must verify returned claims against GitHub before acting.

## 10. Existing implementation workers remain separate

Investigation aliases do not replace currently canonical implementation lineages.

Examples:

- #511 / canonical #525 remains its existing world-census implementation lineage;
- #507 material calculator work uses its exact protected allocation/canonical branch when activated;
- WP3/#356 and WP4/#335 stay under their existing owners;
- Server Seam and later Movement/Combat mutators remain under their dedicated leads/allocations.

If an investigator discovers that implementation already exists or is active, it should analyze/read it and return gaps, not start another branch.

## 11. Source-use shorthand

Every investigator follows:

```text
PROJECT STATE:
GitHub LIVE -> protected accepted Oteryn evidence/contracts

REFERENCE DATA:
CipSoft official
  + controlled Global observation
  + Tibia Wiki as first-class structured bulk data
  + other structured Tibia databases for cross-check
  + community corroboration
  + historical Oteryn migration evidence
  + Canary/Crystal/OTS as OTS_HYPOTHESIS_ONLY
```

Tibia Wiki is intentionally used heavily for items, monsters, NPCs, spells, quests and other bulk content. Expensive official/black-box work should concentrate on conflicts, target-date changes and subtle runtime mechanics.

## 12. Recommended concurrency

Normal operator target **after the canary passes**:

```text
1 coordinator
+ 4 active investigation chats
+ existing canonical implementation workers that already have separate write authority
```

Read-only investigation does not grant or consume tracked-file ownership. Do not increase the number of mutating workers merely because research is parallel.

## 13. Recommended model/effort rationale

### Medium

Use for the coordinator because most work is deterministic GitHub/DAG/ownership reconciliation rather than deep mechanic inference.

### High

Use for World, Character, NPC, Evidence and QA because they require broad source comparison and large structured datasets but normally have fewer authority-sensitive algorithmic decisions.

### Extra High

Use for Combat and Movement/Durability because subtle errors in formulas, event ordering, actor identity, retry/recovery or transaction semantics can invalidate downstream implementation.

Do not use Light for final evidence classification. Light/fast agents may be used only as subordinate URL/locator/data-collection helpers whose output is rechecked by the owning High/Extra-High investigator.

## 14. Completion of investigation programme

Research is sufficiently complete for Reference implementation acceleration when:

- major selected entities/mechanics have field-level source records;
- bulk static data is extracted/cross-checked;
- OTS-only assumptions are visibly separated;
- subtle mechanics have official/controlled evidence or explicit `UNKNOWN/CONFLICT`;
- target-cut continuity is recorded;
- cross-domain conflicts are identified;
- each unresolved field has one bounded next evidence action;
- #162 can translate results into small exact allocations without another broad research sweep.

This is still not the terminal #486 Reference readiness checkpoint. Implementation, composition and real QA remain separate programme stages.
