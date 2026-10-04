# Architect batch: quest catalogue at boot and chest quest bindings

- Batch: `ARCH-QUEST-WIRING-PACKETS-1` (owner answers `1a 2a`)
- Status: **ACCEPTED WHEN THIS DECISION MERGES** for the rulings, leases and packets below.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers:
  1. Why the quest catalogue is not loaded by the node and chests bind no quest transition:
     QUEST-LOWER-1 left both to its callers and no packet followed.
  2. Owner `1a`: both packets are written now, in order. QUEST-CAT-BOOT-1 goes first and
     CHEST-QUEST-BIND-1 follows it.
  3. Owner `2a`: a quest catalogue that fails to load refuses boot, as the spell manifest does.
- Runtime, migration, deployment and production authority: NONE. Each packet needs its #162
  allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 0. Leases and order

### 0.1 Shared files

| File | Owner | Others |
|---|---|---|
| `apps/game-server/src/node/serve.rs` | QUEST-CAT-BOOT-1: the quest catalogue load, its event line and its pass to the seam owners. CHEST-QUEST-BIND-1: the bound-transition check next to it | no other packet in this batch |
| `apps/game-server/src/gameplay_transport/mod.rs` | ATTACK-1b | QUEST-CAT-BOOT-1: the `GameplaySeamOwners` quest field, the `quest_catalogue: None` line (:456) and one `#[cfg(test)]` module declaration, merged **before** ATTACK-1b starts. CHEST-QUEST-BIND-1: none |
| `apps/game-server/src/interaction/chest_use.rs` | CHEST-QUEST-BIND-1: `ResolvedChest.quest_transition` from content (:268, :616) | QUEST-CAT-BOOT-1: none |
| `apps/game-server/src/content/reference_playable.rs` | CHEST-QUEST-BIND-1: `RewardClaimPlacement.quest_transition` and its prefix check | QUEST-CAT-BOOT-1: none |

Neither packet needs a migration, a protocol registry row or a wire change. The pending
obligation path in `durability/reward_claim_mint.rs` and the drain in
`gameplay_transport/mod.rs` (`refresh_quest_session`, `schedule_quest_refresh`) already exist.

### 0.2 Order

1. **QUEST-CAT-BOOT-1** now. If ATTACK-1b starts first, QUEST-CAT-BOOT-1 takes its `mod.rs` lines
   by merging after it, not by sharing a branch.
2. **CHEST-QUEST-BIND-1** after QUEST-CAT-BOOT-1 merges. Its boot check needs the loaded catalogue.

## 1. Rulings

### 1.1 The node loads the embedded quest catalogue at boot, fail-closed (owner `2a`)

- `quest::loader::load_embedded_quest_state(content_revision)` already lowers
  `content/quests/missions/quest-state.json` and fails closed on a malformed document.
- `node/serve.rs` calls it next to the charm and achievement catalogues. An error returns
  `BootError::ContentActivation("quest state catalogue")`, so the node never becomes ready. An
  empty or partial catalogue is not a fallback.
- One event line reports the quest, transition and not-supported counts.
- The catalogue reaches gameplay as an `Arc<QuestStateCatalogue>` through a new
  `GameplaySeamOwners` field. It replaces the hardcoded `quest_catalogue: None`.

### 1.2 The catalogue revision is the node's served content revision

- `durability/quest_state.rs` refuses a Character whose progression `content_revision` differs from
  the catalogue's (`ProgressionContextMismatch`). The catalogue is therefore loaded with the content
  revision the node bootstraps and admits Characters with.
- The worker confirms that revision in code. If the node has no single served revision, the worker
  stops with `BLOCKER`; it does not pick one.
- A Character pinned to another revision keeps the existing refusal. This batch adds no migration
  between revisions.

### 1.3 Inexact effects stay refused

The 221 effects that load as `Computed` keep refusing their transitions with `NOT_SUPPORTED`. Boot
does not fail for them; they are counted in the event line.

### 1.4 Chest bindings are generated from exact matches only

- A reward-claim placement gains an optional `quest_transition`, following the `achievement`
  precedent. A value must carry the `oteryn:quest-transition/` prefix, or content activation fails.
- A binding is generated only where the chest's claim-marker storage key
  (`tools/content-schema/quest-authoring/ots_chests.py`, `marker()`) equals a lowered track
  `source_key`. The transition sets the value the source script writes.
- No inference: a near match, a curated link with no exact key, or more than one candidate gets no
  binding.
- Boot refuses with `ContentActivation` when a bound transition is not in the loaded catalogue,
  like the reward-claim achievement check.

## 2. Packets

### 2.1 QUEST-CAT-BOOT-1

```yaml
task_id: OTV2-20261004-quest-cat-boot-1
decision: ARCH-QUEST-WIRING-PACKETS-1 §1.1-§1.3
worker: oteryn-impl-worker
review: persistence (Codex), on the frozen head
branch: agent/quest-cat-boot-1-20261004
base: main
owned_paths:
  - apps/game-server/src/node/serve.rs
  - apps/game-server/src/gameplay_transport/mod.rs   # §0.1 lines only
  - apps/game-server/src/gameplay_transport/quest_catalogue_boot_tests.rs
  - docs/agents/tasks/archive/OTV2-20261004-quest-cat-boot-1.md
validation:
  - cargo fmt --check
  - cargo clippy -p oteryn-game-server --all-targets -- -D warnings
  - cargo test -p oteryn-game-server
  - git diff --check
```

- **Builds:** the boot load and refusal (§1.1), the event line, the seam-owner field, the revision
  confirmation (§1.2).
- **Acceptance:**
  - a malformed quest document refuses boot with `ContentActivation("quest state catalogue")`;
  - with the catalogue present, a quest transition applies and a pending obligation is drained
    once;
  - a Character on another content revision is refused with `ProgressionContextMismatch`;
  - `Computed` transitions still refuse with `NOT_SUPPORTED`.
- **Not in scope:** chest bindings (§2.2), new quest content, a revision migration, any change to
  the quest loader or the lowering generator.

### 2.2 CHEST-QUEST-BIND-1

```yaml
task_id: OTV2-20261004-chest-quest-bind-1
decision: ARCH-QUEST-WIRING-PACKETS-1 §1.4
depends_on: [OTV2-20261004-quest-cat-boot-1]
worker: oteryn-impl-worker
review: persistence (Codex), on the frozen head
branch: agent/chest-quest-bind-1-20261004
base: main, after QUEST-CAT-BOOT-1 merges
owned_paths:
  - tools/content-schema/quest-authoring/ots_chests.py
  - tools/content-schema/quest-authoring/quest_state_lowering.py
  - tools/content-schema/reward-claim-authoring/
  - content/interactions/reward_claims/
  - content/quests/missions/quest-state.json
  - apps/game-server/src/content/reference_playable.rs   # §0.1 lines only
  - apps/game-server/src/interaction/chest_use.rs
  - apps/game-server/src/node/serve.rs                   # §0.1 check only
  - docs/agents/tasks/archive/OTV2-20261004-chest-quest-bind-1.md
validation:
  - the generators' own tests, and regeneration with no diff
  - cargo fmt --check
  - cargo clippy -p oteryn-game-server --all-targets -- -D warnings
  - cargo test -p oteryn-game-server
  - git diff --check
```

- **Builds:** the placement field and prefix check, generation from exact matches, the reader in
  `RewardClaimPlacement`, `resolve_chest` reading it, the boot check, and regenerated content.
- **Acceptance:**
  - a bound chest records its transition as a pending obligation once, and the session refresh
    applies it;
  - a binding with a wrong prefix, or one missing from the catalogue, refuses boot;
  - a chest without an exact match has no binding;
  - regenerating the content gives no diff.
- **Not in scope:** inferred or curated-only bindings, new quests or tracks, non-chest quest
  triggers.

## 3. Rejected options

- **Lazy load on first quest use.** A bad document would surface in play instead of at boot.
  Owner `2a` chose fail-closed boot.
- **Boot without quests on a load error.** Characters would silently lose progress.
- **One packet for both.** The boot wiring is independent and shorter; binding needs it in place.
- **Binding from `chest_quest_links.json` alone.** Those links name quests, not the exact
  transition and value.

## 4. Decision test

The batch holds when, after both packets merge:
- a node with a malformed quest document does not become ready;
- opening a bound chest advances its quest track exactly once, across a reconnect;
- every bound transition exists in the served catalogue.
