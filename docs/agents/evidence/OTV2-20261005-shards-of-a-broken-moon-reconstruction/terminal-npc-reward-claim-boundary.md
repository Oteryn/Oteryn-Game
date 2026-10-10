# Shards s16 — NPC RewardClaim execution boundary

Status: **EVIDENCE / EXISTING ARCHITECTURE NOT YET IMPLEMENTED**

This closes a false design branch: Shards s16 does **not** need a new quest reward dispatcher.

## Accepted contract already defines the execution order

QUEST-GATE-0 / NPC-QUEST-0 §5.2–§5.4 already says an NPC dialogue node may have:

```text
at most one transition { key }
and at most one claim { reward_claim }
```

For a node with both:

```text
claim MINT first
  -> item + achievement commit
  -> claim writes quest obligation for the declared transition
  -> exact quest transition commits afterward
```

A full backpack/refusal writes neither reward nor transition.

This is the correct Shards s16 shape.

## Current implementation gap is chest-only, not durability

The durable writer already accepts all required semantic data:

`apps/game-server/src/durability/reward_claim_mint.rs`

`RewardClaimMintRequest` already carries:

- claim identity;
- item definition/count;
- Achievement catalogue lookup;
- optional `quest_transition`;
- idempotent once-per-character claim semantics.

The same transaction already grants the Achievement under the Achievement owner and creates the Quest obligation when `quest_transition` is present.

What is still chest-specific:

1. `RewardClaimMintRequest.source_placement: String` is mandatory and validated as the chest source.
2. `ReferenceRewardClaimDefinition` in `content/reference_playable.rs` requires at least one `RewardClaimPlacement`.
3. The reward payload lives inside that placement.
4. `content/world_reward_claims.rs` binds RewardClaims exclusively through map placements.
5. `interaction/chest_use.rs` is the only production adapter constructing the request.

This is exactly the seam NPC-QUEST-1 is already assigned to amend: the accepted architecture says the dialogue occurrence becomes a new D40 MINT source beside the USE child.

## Do not fake an NPC reward chest

For s16, do not:

- invent a hidden/synthetic chest coordinate;
- add a fake RewardClaimPlacement at Saraki/Nilavarna;
- mint i54638 directly from dialogue code;
- grant Amati's Echo directly from dialogue code;
- commit transition first and rewards afterward.

Those all violate the accepted transaction ordering/ownership.

## Proposed Shards content binding

One semantic claim:

```text
claim:
  key: oteryn:reward-claim.authored.shards_of_a_broken_moon_quest.terminal
  per: character
  repeat: once

payload:
  item:
    oteryn:item.tibia.i54638
    count: 1
  achievement:
    oteryn:achievement/amati_s_echo@1

quest obligation:
  oteryn:quest-transition/authored/shards_of_a_broken_moon_quest/s16
```

Both terminal NPC routes point to the **same claim identity** so choosing Saraki vs Nilavarna cannot duplicate the reward.

Dialogue route:

```text
Saraki completion branch
OR
Nilavarna completion branch
  [prison_escaped]
  -> claim terminal RewardClaim
  -> same s16 transition
```

## Exact implementation owner

This belongs inside the already accepted `NPC-QUEST-1` / `NPC-QUEST-CONTENT-1` implementation, not a new Shards runtime.

Expected code surfaces once allocated:

```text
apps/game-server/src/content/reference_playable.rs
  # allow/represent non-spatial dialogue RewardClaim payloads without weakening chest validation

apps/game-server/src/durability/reward_claim_mint.rs
  # generalize source identity from chest placement to accepted D40 source kind
  # preserve intent binding/idempotence

apps/game-server/src/interaction/chest_use.rs
  # remains one caller; ordinary chest behavior unchanged

NPC-TALK / NPC-QUEST runtime adapter
  # constructs dialogue-source RewardClaimMintRequest

NPC dialogue content lowering
  # binds both terminal nodes to the same claim + transition
```

The exact final file names for NPC-TALK/NPC-QUEST are allocator-owned because those runtime children are not yet on main.

## Prerequisites

- `oteryn:item.tibia.i54638` must be materializable first;
- Achievement `oteryn:achievement/amati_s_echo@1` already exists canonically;
- s16 transition already exists canonically;
- NPC-TALK-1 / NPC-QUEST-1 must exist.

## Acceptance

- Saraki path gives exactly one fish + achievement + completion;
- Nilavarna alternate path gives the same exact claim, not a second reward;
- taking one route makes the other route idempotently already-claimed/completed;
- full backpack -> no fish, no achievement, no quest completion;
- replay same dialogue occurrence -> no duplicate;
- reconnect/restart preserves claim, achievement and quest completion;
- ordinary chest RewardClaims remain unchanged.
