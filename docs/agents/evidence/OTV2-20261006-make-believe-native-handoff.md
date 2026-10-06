# Make Believe Quest — Native/runtime handoff

Date: 2026-10-06  
Repository: `Oteryn/Oteryn-Game`  
Handoff base: `main@f6894e793c162d9d8a43578332f3a6f77d2936a3`

## Purpose

This handoff freezes the current state of the Make Believe Quest work after the post-release DATA reconstruction was merged. It is intentionally documentation-only. The next work is Native/runtime integration through the already accepted Quest/NPC/Encounter owners; do not create a Make-Believe-specific quest engine or duplicate canonical identities.

Before doing any write, fresh-read current `main`, PRs/issues named below and canonical coordination issue `#1622`. Do not trust the SHA values below as current after this handoff.

## Integrated state

- PR **#1865** — `content: refine Make Believe Quest from post-release evidence` — merged. It carries the post-release authored journey and completion corrections.
- PR **#1877** — `fix(quest): normalize quest-tree provenance paths` — merged. It closes the Windows/Linux provenance-path determinism discovered while qualifying #1865.
- Make Believe remains DATA-only:
  - `runtime_enabled=false`;
  - the completion binding plan has **20 stages**;
  - every stage still has `native_dispatch_binding=null`;
  - `native_reward_delivery_binding=null`.
- Historical/pre-release source packets stay preserved as audit evidence. Post-release corrections are modeled through chosen authoring/correction overlays rather than rewriting source history.

## Post-release Make Believe facts already represented in DATA

The authored journey now carries the material post-release differences found during the reconstruction:

- Venore storehouse infiltration uses the cargo-crate disguise and guarded/noisy route semantics.
- Gong puzzle is the **five-colour** sequence, not the older three-device approximation.
- Mimar Haffar uses repeated Moonsilver Crystal / Energy Cannon shield cycles.
- Maior Domus uses the charge/platform vulnerability cycle.
- The fanatic puzzle requires **eight Elite Fanatics simultaneously Petrified**.
- Phosphorus begins with vocation Rivals in the order:
  `Knight -> Paladin -> Sorcerer -> Druid -> Monk`.
- The later sequence includes the Maior Domus and Mimar Haffar rematches and the final Phosphorus phase with persistent hazards.
- Final progression includes Fate Forge access and boss shortcuts.
- Salgadora invasion credit is **damage participation against at least one Moonspawn Juggernaut**. It must not require last-hit ownership.
- The current authored schema has no damage-participation event kind, so that stage remains a DATA approximation until the proper encounter producer is wired.

## Canonical identities — do not duplicate

Exact canonical Creature identities already exist for the main Make Believe encounter actors:

- `oteryn:creature.moonspawn_juggernaut`
- `oteryn:creature.cult_initiate`
- `oteryn:creature.moonstone_miner`
- `oteryn:creature.trapped_soul`
- `oteryn:creature.elite_fanatic`
- `oteryn:creature.petrified_fanatic`
- `oteryn:creature.knight_rival`
- `oteryn:creature.paladin_rival`
- `oteryn:creature.sorcerer_rival`
- `oteryn:creature.druid_rival`
- `oteryn:creature.monk_rival`
- `oteryn:creature.mimar_haffar`
- `oteryn:creature.mimar_haffar_final`
- `oteryn:creature.maior_domus`
- `oteryn:creature.maior_domus_final`
- `oteryn:creature.phosphorus`
- `oteryn:creature.phosphorus_final`

Known interaction/content identities also include the two Energy Cannon creature records. Moonsilver Crystals are backed by donor Item id **54267** and carry the short-lived/decay behavior in source evidence.

## NPC state

Exact NPC + Dialogue identities already exist for:

- `oteryn:npc.wayland_smythers`
- `oteryn:dialogue.npc.wayland_smythers`
- `oteryn:npc.mayor_pocaro`
- `oteryn:dialogue.npc.mayor_pocaro`

The real identity conflict is **Doctor Marrow**:

- current canonical content has `oteryn:creature.doctor_marrow`, sourced from the older Canary boss classification;
- post-release Make Believe requires Doctor Marrow as an **NPC** with prison-cell dialogue;
- there is no admitted `oteryn:npc.doctor_marrow` at this handoff.

Do not coerce the Creature identity into NPC quest dialogue. Allocate a bounded NPC identity + Dialogue reconciliation through the NPC content owner.

The existing Wayland/Mayor Dialogue records provide branch candidates, but the final Wayland completion path is not yet a typed quest-completion branch. Selecting a generic keyword such as an outfit branch is not sufficient.

## Runtime owner sequence

The accepted architecture already defines the missing seams. Keep this sequence unless current `main`/coordinator has superseded it:

1. **QUEST-GATE-1**
   - dependency readback during this work found QUEST-PRED-1, MAP-LOAD-1, MAP-WIRE-2 and QUEST-CONTENT-2 merged;
   - evaluate accepted quest predicates at gated door/tile/teleport runtime;
   - use existing WorldInteraction/placement/overlay/relocation owners;
   - no durable write in the gate check.

2. **QUEST-TRIGGER-1**
   - follows QUEST-GATE-1;
   - turn qualified `USE`, `ON_ENTER`, `ON_LEAVE` roots into the already existing named `QuestTransitionRequest` path;
   - do not invent an alternate quest writer.

3. **NPC-TALK-1 -> NPC-QUEST-1**
   - typed quest conditions/outcomes in NPC dialogue;
   - confirmation-bound occurrences;
   - transition/reward/exchange through accepted writers;
   - Make Believe additionally needs Doctor Marrow NPC/Dialogue reconciliation.

4. **Encounter runtime**
   - current QuestState already defines `QuestCause::CreatureDeath`, but readback found no production creature/encounter producer of `QuestTransitionRequest`;
   - accepted Encounter architecture assigns runtime/outcome delivery through **ENC-RT-1 -> ENC-OUTCOME-1**;
   - do not add a quest-only creature-death bypass;
   - Salgadora needs a damage-participation outcome, not last-hit;
   - boss/rematch stages need qualified encounter outcome delivery.

5. **Reward/completion delivery**
   - use the existing QuestState transition/reward obligation seams;
   - do not make Make Believe a special reward path.

## Stage-owner decomposition

Use this as a starting map; fresh-read current generated content before implementation:

- `s1` — talk: NPC-QUEST-1; Wayland candidate exists.
- `s2` — Venore stealth/use: QUEST-TRIGGER-1 + WorldInteraction/placement/overlay.
- `s3` — talk: NPC-QUEST-1; Mayor Pocaro candidate exists.
- `s4` — Salgadora invasion: Encounter **damage participation** producer.
- `s5` — talk: NPC-QUEST-1; Mayor Pocaro.
- `s6` — encounter/kill progression: Encounter outcome producer.
- `s7` — world/use progression: QUEST-TRIGGER-1.
- `s8` — five-colour Gong: QUEST-TRIGGER-1 + world/encounter ephemeral state.
- `s9` — twenty short-lived Moonsilver Crystals: inventory/encounter item-count seam; item lifetime is not durable QuestState.
- `s10` — Energy Cannons: QUEST-TRIGGER-1 + encounter/world state.
- `s11` — Mimar Haffar: Encounter outcome producer.
- `s12` — Doctor Marrow talk: NPC identity/dialogue reconciliation + NPC-QUEST-1.
- `s13` — Maior Domus: Encounter outcome producer.
- `s14` — eight simultaneous Petrified Elite Fanatics on floors: encounter/creature-state + placement predicate, emit one qualified outcome when the simultaneous condition is true.
- `s15` — ordered vocation Rivals / Phosphorus phase: Encounter outcome producer.
- `s16` — Maior rematch: Encounter outcome producer.
- `s17` — Mimar rematch: Encounter outcome producer.
- `s18` — final Phosphorus: Encounter outcome producer.
- `s19` — final Wayland talk/unlocks: NPC-QUEST-1 + reward/access delivery.
- `s20` — completion: existing chosen transition `oteryn:quest-transition/authored/make_believe_quest/s20`; Native reducer/delivery binding still missing.

## Current coordination / adjacent work

At the time of this handoff:

- `#1622` remains the canonical work-control continuation and contains the MBQ-NATIVE-1 blocker/readback comments.
- `#1875` and `#1876` are adjacent Quest completion/lowering work. Fresh-read before touching generated Quest files; do not overwrite their newer state.
- Soul War work has also touched overlapping generated Quest outputs. Always regenerate compact/generated JSON from fresh `main`; do not hand-merge one-line JSON.
- The portability issue discovered during #1865 is already fixed by merged #1877. Do not reimplement it.

## Qualification already established

During the Make Believe reconstruction and the quest-tree portability follow-up:

- authored/tree/rollout/binding/V2/state/completion checkers passed at the qualified heads;
- Quest schema validation passed **269/269**;
- focused quest-tree/rollout/binding/completion tests passed;
- the final portability fixed point passed **46/46** targeted tests;
- donor replay/check covered the pinned corpus and passed after Linux-normalized generation;
- generated repository-relative path values were reduced from Windows backslashes to POSIX form.

Always re-run exact-head CI for any new runtime PR. These results are evidence, not permission to skip current qualification.

## Hard constraints

- No Make-Believe-specific QuestState store or alternate transition writer.
- No duplicate Creature records for actors already listed above.
- No coercion of `oteryn:creature.doctor_marrow` into an NPC.
- No last-hit approximation for Salgadora damage-participation credit.
- No durable QuestState fields for ephemeral Gong/cannon/boss-room mechanics unless architecture explicitly changes.
- No manual merge of generated compact JSON; regenerate from fresh accepted inputs.
- No runtime write before the relevant #1622 allocation/lease. If another worker owns the path, work on a disjoint task or escalate a real blocker only.

## Continuation checklist

1. Fresh-read `main`, `#1622`, open Quest/NPC/Encounter PRs, and this handoff.
2. Verify whether QUEST-GATE-1 / QUEST-TRIGGER-1 / NPC-TALK-1 / NPC-QUEST-1 / ENC-RT-1 / ENC-OUTCOME-1 have since been allocated or merged.
3. If no runtime lease exists, do **not** start overlapping code. Report the exact missing allocation to the coordinator.
4. If QUEST-GATE-1 is allocated and still unimplemented, implement only that accepted child first and qualify it independently.
5. Then serialize QUEST-TRIGGER-1.
6. Keep NPC stages behind NPC-TALK-1/NPC-QUEST-1 and Doctor Marrow NPC reconciliation.
7. Keep kill/boss/damage stages behind Encounter runtime/outcome ownership.
8. After each accepted child lands, fresh-read and re-evaluate which Make Believe stage can become the first true `native_dispatch_binding`.
9. Final acceptance is not “DATA complete”: require player start -> progress -> boss/interaction -> completion/reward -> relog/restart gameplay qualification.
