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
| `apps/game-server/src/quest/mod.rs`, `src/quest/loader.rs` | QUEST-CAT-BOOT-1: one read-only count accessor (§1.1) | CHEST-QUEST-BIND-1: none |
| `apps/game-server/src/gameplay_transport/quest_catalogue_boot_tests.rs` | QUEST-CAT-BOOT-1 (new) | CHEST-QUEST-BIND-1 adds its served-path test (§1.5) here, so it needs no `mod.rs` line |

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
- One event line reports the quest, transition and not-supported counts. Those counts need a
  read-only accessor on `QuestStateCatalogue` (`quest/mod.rs`) or `LoweredQuestState`
  (`quest/loader.rs`). The accessor is the only change allowed in those files.
- The catalogue reaches gameplay as an `Arc<QuestStateCatalogue>` through a new
  `GameplaySeamOwners` field. It replaces the hardcoded `quest_catalogue: None`.

### 1.2 The catalogue revision is the node's served content revision

- `durability/quest_state.rs` refuses a Character whose progression `content_revision` differs from
  the catalogue's (`ProgressionContextMismatch`). The catalogue is therefore loaded with the content
  revision the node bootstraps and admits Characters with. The expected value is
  `content::accepted::REVISIONS[0]` (`oteryn:content/entry-r1`). The chest `USE` path already
  binds to it (`entry_chest::CONTENT_REVISION`, asserted equal in `gameplay_transport/mod.rs`).
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
- **Key normalization before the exact comparison.** `ots_chests.py` `marker()` returns the
  storage path without a namespace: the slugged parts of the storage expression after its root,
  joined by `/`, for example `quest/u8_4/...`. A lowered track `source_key` is
  `canary:quest-progress/<path>`. A marker matches a track only when
  `"canary:quest-progress/" + marker == source_key`, byte for byte.
  - Only the Canary namespace is joined. A `crystalserver:quest-progress/...` track and a
    `kv/...` marker (a KV quest name, not a storage key) never match.
  - No other case, slug or separator rewriting happens on either side.
- **Test vectors**, taken from the committed content of `origin/main` 69f171fc:
  - claim `oteryn:reward-claim.quest.u8_4.the_hidden_city_of_beregar.firewalker_boots` has the
    marker `quest/u8_4/the_hidden_city_of_beregar/firewalker_boots`. It matches the source key
    `canary:quest-progress/quest/u8_4/the_hidden_city_of_beregar/firewalker_boots`, whose track
    is `oteryn:quest-progress/quest/u8_4/the_hidden_city_of_beregar/firewalker_boots`.
  - claim `oteryn:reward-claim.quest.u7_8.the_shattered_isles.dragahs_spellbook` matches
    `canary:quest-progress/quest/u7_8/the_shattered_isles/dragahs_spellbook`.
  - Of the 231 plain claims, exactly these two match today. A golden pins that count.
  - Negative vectors: the marker `quest/u8_6/afathers_burden/cloth` without the namespace
    prefix, compared directly with the source key, does not match. A `kv/` marker never matches.
- The transition sets the value the source chest script writes to that storage key. A chest
  whose written value is not a literal gets no binding.
- No inference: a near match, a curated link with no exact key, or more than one candidate gets no
  binding.
- Boot refuses with `ContentActivation` when a bound transition is not in the loaded catalogue,
  like the reward-claim achievement check.

### 1.5 What the node serves, and where the binding is proven

- The node serves exactly one chest. `serve.rs` builds the gameplay content with
  `with_entry_chest(&door_content)` (`serve.rs`:1182-1187). `native_entry.rs`
  (`qualify_native_entry_room_with_gameplay`) loads no global catalogue, so no imported
  reward-claim placement from `content/interactions/reward_claims/` reaches a running node.
- The entry chest has no source quest, so binding it would be inference (§1.4). It stays unbound.
- CHEST-QUEST-BIND-1 therefore proves the binding on the **served path**:
  - the boot check runs over the same content `serve.rs` serves (`chest` above);
  - the acceptance test builds that served content with the production composition
    (`with_entry_chest` over the activated room) plus one generated bound placement, the
    Beregar vector from §1.4, injected the same way `with_entry_chest` injects the entry chest.
    It drives `USE` through `gameplay_transport` with the catalogue from QUEST-CAT-BOOT-1.
  - No served chest carries a binding when this packet merges. A player sees a chest advance a
    quest only once imported placements are served.
- **Serving imported chests is not decided here.** It needs the imported map and placements
  on a node, under its own accepted world-content decision. This batch does not packet it.

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
  - apps/game-server/src/quest/mod.rs                # the count accessor only
  - apps/game-server/src/quest/loader.rs             # the count accessor only
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
  the quest loader or the lowering generator other than the count accessor.

### 2.2 CHEST-QUEST-BIND-1

```yaml
task_id: OTV2-20261004-chest-quest-bind-1
decision: ARCH-QUEST-WIRING-PACKETS-1 §1.4-§1.5
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
  - apps/game-server/src/gameplay_transport/quest_catalogue_boot_tests.rs  # the §1.5 served-path test
  - docs/agents/tasks/archive/OTV2-20261004-chest-quest-bind-1.md
validation:
  - the generators' own tests, and regeneration with no diff
  - cargo fmt --check
  - cargo clippy -p oteryn-game-server --all-targets -- -D warnings
  - cargo test -p oteryn-game-server
  - git diff --check
```

- **Builds:** the placement field and prefix check, generation from exact matches with the §1.4
  normalization, the reader in `RewardClaimPlacement`, `resolve_chest` reading it, the boot check
  over the served content, and regenerated content.
- **Acceptance:**
  - the §1.4 positive and negative vectors, and the golden match count of two;
  - on the §1.5 served path, the Beregar chest records its transition as a pending obligation
    once, and the session refresh applies it to its track;
  - the served entry chest stays unbound;
  - a binding with a wrong prefix, or one missing from the catalogue, refuses boot;
  - a chest without an exact match has no binding;
  - regenerating the content gives no diff.
- **Not in scope:** serving imported placements on a node (§1.5), binding the entry chest,
  inferred or curated-only bindings, new quests or tracks, non-chest quest triggers.

## 3. Rejected options

- **Lazy load on first quest use.** A bad document would surface in play instead of at boot.
  Owner `2a` chose fail-closed boot.
- **Boot without quests on a load error.** Characters would silently lose progress.
- **One packet for both.** The boot wiring is independent and shorter; binding needs it in place.
- **Binding from `chest_quest_links.json` alone.** Those links name quests, not the exact
  transition and value.

## 4. Decision test

`docs/agents/ARCHITECTURE_DECISION_DISCIPLINE.md`, mandatory decision test:

1. **Must decide now?**
   - QUEST-CAT-BOOT-1: **YES.** Without a catalogue, admission leaves every quest obligation
     pending and refuses every quest transition, so the merged quest state does nothing on a node.
   - CHEST-QUEST-BIND-1: **YES** for the content field, the normalization and the resolver.
     The owner asked for them (`1a`), and they are small and additive. **NO** for serving
     imported chests (§1.5), which stays undecided.
2. **What concrete work is blocked?**
   - Any quest progress on a running node. That includes draining the pending obligations that
     `reward_claim_mint.rs` already records.
   - Chest-driven quest progress, and later quest content that relies on chests.
3. **What becomes harder later?**
   - Fail-closed boot couples readiness to the quest document. A bad regeneration takes a node
     down instead of degrading it. This is the accepted trade-off of owner `2a`, and it matches
     the spell manifest.
   - The catalogue is pinned to the served revision (§1.2). A future revision switch needs a
     progression migration, or Characters on the old revision stay refused.
   - The placement field is additive and optional. Removing it later is a content-schema change,
     with no stored data to migrate.
4. **What would justify superseding it?**
   - Boot-time cost or memory of the catalogue measured as material on a node.
   - A requirement for nodes that serve several content revisions.
   - Evidence that chest markers and tracks need a mapping that exact matching cannot express,
     for example a quest whose chest writes a key its tracks do not lower.
   - A security or durability finding in the obligation drain.
5. **What is deliberately not decided?**
   - Serving imported placements and the imported map on a node (§1.5).
   - Revision migration.
   - Lowering of the 221 `Computed` effects.
   - Quest triggers other than chests, such as doors, NPC dialogue and kills.
   - Curated (`chest_quest_links.json`) or inferred bindings.

**Risks and trade-offs.**
- Fail-closed boot trades availability for never serving lost progress.
- Exact matching binds only two chests today. That gives low coverage but no false bindings.
- Proving the binding with an injected placement on the served path (§1.5) shows the mechanism,
  not a player-visible chest. The owner should know that no served chest advances a quest until
  imported placements are served.

**Proof.** After both packets merge:
- a node with a malformed quest document does not become ready;
- on the served path, the bound chest advances its quest track exactly once;
- every bound transition in the served content exists in the loaded catalogue.
