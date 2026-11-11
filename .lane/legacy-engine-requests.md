# ENGITEM request ledger

Branch: `claude/happy-davinci-ijz3wy-engitem`. Base: `origin/main`.
Migration reservation: **1640–1659**; merged migrations are immutable.
Rows deduplicate all reports in the packet. DONE means the bounded engine
capability, not whole-quest content qualification. OPEN rows are being revisited
as owner APIs arrive. Ownership/blocker entries identify code and release conditions.

| ID | Engine capability / requesting quests | Status and evidence |
|---|---|---|
| 01 | Deterministic authoritative wait clock — Koshei, Sea of Light, Spike Tasks | DONE: `bbe018482`; `engitem_quest_clock.rs`; real PG marker, 3.60s. |
| 02 | Cake extraction/bite receipt participants — A Piece of Cake | OPEN: SEASON has landed in main `6ef6b0cad`; inspect its extraction/bite receipts and transaction hooks next. |
| 03 | Selected carried-tool USE_WITH_MAP_ITEM, target-bound effects — To Blind the Enemy, Thieves Guild | BLOCKED: B1/B6; main rejects targeted carried quest USE and World context has no selected-tool proof. Message placement/removal also needs the World object lifecycle participant. |
| 04 | Consumed temporary outfit / expiry — Secret Service, In Service of Yalahar | BLOCKED: B1/B5; existing outfit rewards grant permanent entitlement, not an expiring appearance condition/restoration receipt. |
| 05 | Personal donation threshold + once/event item grant — A Piece of Cake | OPEN: SEASON has landed in main `6ef6b0cad`; reassess personal threshold participation, preserving its 200-substance world goal. |
| 06 | Atomic ground reward MINT / ground USE_WITH cart — Shattered Isles, The Beginning | OWNED ELSEWHERE: QENGINE2 `ENGINE-QUEUE.json:ground_item_mint_and_ground_use_with`; shared ground-MINT/cart owner, including reuse for Shattered Isles. |
| 07 | Native diagonal movement — Unnatural Selection | OWNED ELSEWHERE: protocol movement owner, `crates/protocol-oteryn/src/world_spatial.rs`; outside the three allocated engine crates. |
| 08 | Forward preservation of exact charge plans and root/child fluids after 1251 — Hot Cuisine, Annual Autumn Vintage, Demon's Lullaby | BLOCKED: B2; L2's 1219 schema and root-fluid/charge request fields are absent on main. A forward composition test must first have that owner, not a substitute proof function. |
| 09 | Generic eleven/sixteen-track receipt regression after 1349; preserve Father's exception | OWNED ELSEWHERE: ENGNPC `3423ac56e`, migration 1600 and `engnpc_atomic_{tracks,rewards}.rs`; merged with a merge commit. Duplicate ENGITEM 1640 and atomic test removed. Migrations 1600–1639 remain untouched. |
| 10 | Atomic multiple root NPC item rewards, capacity refusal/replay — Botany, Bigfoot | OWNED ELSEWHERE: L2 `96d03398f`, `reward_claim_mint/backpack_outputs.rs`, migration 1213 on `-questl2b`; reuse that implementation. |
| 11 | Exact live-creature capture/sampling + consumed/transformed tool and custody — Goblin Merchant | BLOCKED: B6; need World-owned exact live-creature capture/sampling admission and custody handoff. An actor placement or creature name is not target-generation proof. |
| 12 | Premium account guard at authored admission — Asura Palace | OWNED ELSEWHERE: QENGINE2 authored admission; reuse `RewardCallback.requires_premium`, plus WPKG2 `Callback.premium_required`, rather than another premium projection. |
| 13 | Independently pinned Crystal Summer membership — Rotten Blood | OWNED ELSEWHERE: content identity owner, `tools/content-schema/native-gameplay/build_spell_item_identities.py`; content-only qualification. |
| 14 | Vocation-dependent tile-entry damage — Pits of Inferno | OWNED ELSEWHERE: combat/actor damage and movement owners; damage must use live vocation/health authority, not quest tracks. |
| 15 | Carried-target transform preserving stack quantity — Tainted Souls | BLOCKED: B4; qualified Items conversion emits quantity 1 for transformed operands. Need recipe-scoped whole-target quantity semantics from Items/Content before durability may preserve a donor stack. |
| 16 | Annual-date/elapsed predicates and transaction-clock writes — First Dragon, Threatened Dreams | OWNED ELSEWHERE: QENGINE2 queue `annual_npc_dialogue_calendar_gate` and IN_PROGRESS `world_callback_transaction_clock_operations` (`790996f8e`, `world_callbacks/state_operations.rs`); includes StepIn consumers of that shared predicate language. |
| 17 | Conditional achievement on world MINT — First Dragon / Treasure Hunter | OWNED ELSEWHERE: QENGINE2 queue `conditional_world_reward_achievement`; no duplicate twentieth-chest achievement owner. |
| 18 | Stateless level-range StepIn with failure destination — The Beginning | BLOCKED: B1/B7; requires scoped read-only native admission and a World-owned refusal destination. Main only has `minimum_level` coupled to stored progress. |
| 19 | Atomic NPC reward teleport, destination authority, replay — Paradox Tower | BLOCKED: B1/B7; scoped native NPC admission is owned by QENGINE2, and NPC reward admission carries no current World destination/movement participant. A teleport after the durable reward would not be atomic. |
| 20 | Atomic bitmask guard/update + carried consumption + XP — Rookie Guard | BLOCKED: B1; new bound predicate/effect must enter the owned immutable callback catalogue and reach its physical/input admission. A generic unfenced transition is not the requested composed USE. |
| 21 | Exact world-item relocation preserving identity/attributes — Outlaw Camp | BLOCKED: B6; World/custody owner must provide an exact-instance relocation participant. `ItemCycle`/`ItemPresence` are map projections, not portable Item moves. |
| 22 | Carried-item pair transform and exact self-target USE — Cursed Crystal | BLOCKED: B1/B4; requires two selected-instance proofs and qualified conversion semantics; main carried quest USE rejects every target. |
| 23 | Improper-level weapon penalties (`unproperly=true`) — Outlaw Camp | OWNED ELSEWHERE: equipment/combat and native Item identity owners (`equipment_move.rs`, Items equipment admission); this changes weapon behavior, not a quest transaction. |
| 24 | Main-compatible authored native-binding admission — Paradox, Realm of Dreams, Ancient Tombs, Treasure Hunt | OWNED ELSEWHERE: QENGINE2 `authored_bindings.rs` and native-admission qualification `192de4928` / `614d00506`; `-qengine2` queue marks QUALIFIED. |
| 25 | Shared reset epochs and map-container pickup custody — Skeleton Decoration, Dreamer's Challenge, Lion's Rock, Machinery of War | OWNED ELSEWHERE: World map/reset/pickup custody owner, `gameplay-world/map/overlay/pickup.rs`; quest flags cannot replace shared reset epochs or container custody. |
| 26 | Claimant-attributed wolf capture/pickup quest credit and daily reset — Overhunting | BLOCKED: B6/B8; World trap/pickup owner must issue claimant-attributed event proof and daily phase identity before Quest may credit it. Reuse Benevola's World owner. |
| 27 | Daily restart/completion and cycle-scoped repeated world MINT — Oramond, Overhunting | BLOCKED: B1/B8; scoped callback admission and an admitted quest restart/cycle token are missing on main. L2 paid-command repeats are not daily restart or once-per-cycle world grants. |
| 28 | Exact fenced live-creature relocation with tool/progress — 20 Years a Cook | BLOCKED: B6/B7; current context proves the player placement, not a selected creature generation or an atomic creature movement participant. |
| 29 | Selected nested reward leaf under original once ledger — Ornamented Shield | OWNED ELSEWHERE: Content/QREADY source-claim owner, `content/world_reward_claims/selected_leaf.rs` and `selected_native_leaf.py`; selection must remain under the existing source claim. |
| 30 | Bounded literal readable NPC/World item reward text — Nomads Land | OPEN: ENGNPC's merged `commit_fixture_quest_reward` enables the real reward-transaction fixture. Extend the shared ItemReward/MINT bridge next. |
| 31 | Fenced MapBase-to-Ground movement — Nomads Land | OWNED ELSEWHERE: World/map-pickup and item-loop custody owner; `item_move.rs::loop_move` explicitly excludes Base-to-Ground and there is no admitted source-detachment handoff. |
| 32 | Mutable ground stack/order predicate — Nomads Land | BLOCKED: B6; current `WorldRequirement` admits immutable native map appearances only. Need World-owned current ground stack/order observation and CAS proof. |
| 33 | Quest-guarded object taming + atomic mount/progress grant — Targuna | BLOCKED: B9; `MountTamingRequest` has no quest participant and the taming receipt owns the action. Need accepted mount/Quest receipt composition and a qualified quest-guarded object binding. |
| 34 | Read-only foreign quest prerequisite with local progress — Opticording Sphere | BLOCKED: B1; main's NPC `TrackRequirement` exists, but World callbacks have no foreign-track read participant. Do not relabel a foreign track as locally owned. |
| 35 | Recipe-scoped nondecaying specimen, preserve original timed item — Explorer Society | BLOCKED: B4; need Content/Items-qualified nondecaying recipe binding. L2 `quest_decay::resolve` deliberately rejects omitted decay for a timed native Item; bypassing it would weaken identity/duration admission. |
| 36 | Main-compatible world reward / premium / completion / read-only APIs — Cook, Lighthouse, Serpentine | OWNED ELSEWHERE: WPKG2 `world_callbacks::{Callback.completes,Callback.premium_required,Storage.read_only}` plus QENGINE2 catalogue-scoped world reward admission; reuse the retained implementation. |
| 37 | CallbackEffects / CallbackEffectSet source compatibility — Feaster, Goblin, Hot Cuisine, outfits | DONE: already on main `83acbdac3`, retained on this branch; `effect_wire_forms_preserve_receipt_serialization_and_reject_mixing` is the focused compatibility regression. No persistence change. |
| 38 | Bounded repeated world rewards with exact carried/ground inputs — Feaster of Souls | BLOCKED: B1/B8; main world MINT calls `mint(..., false, ...)`; L2 repeat SQL only permits proved costed exchanges and does not establish the requested world input/cycle composition. |
| 39 | Portable first-party item/outfit constructors — Illuminated Warrior | OWNED ELSEWHERE: content identity/cosmetics owners, `content/native_gameplay/authored_items.rs` and `cosmetics_loader.rs`; no invented source IDs. |
| 40 | Selected carried USE replacement/reward — Colours of Magic | OWNED ELSEWHERE: QENGINE2 qualified slice `cca7c5790`, `RewardCallback.carried_exchange` and `reward_callbacks::load_for`. |
| 41 | Fenced live other-player colour/quest predicate — Colours of Magic | OWNED ELSEWHERE: QENGINE2 queue `carried_use_live_player_quest_predicate`; uses its selected carried exchange owner. |
| 42 | Item-look disguise (`lookTypeEx`) and appearance restoration — Wrath of Emperor | OWNED ELSEWHERE: QENGINE2 queue `item_look_disguise_restore`, through Appearance's condition/restoration owner. |
| 43 | Revision-fenced timed ground transform/reset — Uneasy Alliance, Lion's Rock | BLOCKED: B6; physical `floor_transform` expiry exists, but callback map projections expose no Item identity/revision to that queue. Need World/custody handoff, not a second unfenced timer. |
| 44 | Carried tool: counter + temporary outfit + replay-bound random spawn — In Service of Yalahar | BLOCKED: B1/B5/B6; counter increment/reusable tools exist, but no receipt-bound temporary appearance or random live-spawn participant. Encounter timing alone does not prove a random outcome. |
| 45 | Quest-objective receipt after native tool action, retain transform/decay — Kissing a Pig | BLOCKED: B6; native tool owner must hand its successful action receipt to Quest in the same transaction. A separate quest callback cannot infer success from the transformed item. |
| 46 | Pure read-only StepIn guards and ordered refusal messages — Bigfoot | OWNED ELSEWHERE: retained L2/PC2/WPKG2 `Storage.read_only` / `Callback.is_read_only`, plus QENGINE2 authored World admission; existing branch tests reject mixed mutations. |
| 47 | Player-driven shared object state, NPC guards, refill/toxin/reset — Krailos | OWNED ELSEWHERE: World-change/shared-object state owner; `world_event::changes` owns shared occurrence/reset state, not personal Quest storage. |
| 48 | Public speech + atomic conditional object toggle + delayed reset — Children of Revolution | BLOCKED: B6/B7; needs World-owned toggle/reset CAS and Chat-owned player speech publication tied to the committed occurrence. Existing private feedback/map projection is not that participant. |
| 49 | Replay-bound random outcomes and live condition predicates — Barbarian Test | BLOCKED: B1/B5; NPC `QuestActorConditionProof` already supports Drunk, but World callbacks have no admitted live-condition/random-outcome participant. Source predicates must not be guessed from quest storage. |
| 50 | Retain actual NPC13 definition predecessor — Hidden Threats | OWNED ELSEWHERE: PC2 `e3566291a`; loader calls `questpc_npc13_prefix::retain` before installing current definitions. |

## Concrete blocked dependencies and release conditions

- **B1 — scoped immutable callback admission (QENGINE2).** On this branch,
  `quest_state/writer.rs:216` calls global `reward_callbacks::load()` and
  `writer.rs:286` resolves World callbacks from global `world_callbacks::load()`;
  unknown physical callbacks fail `InvalidInput`. `authored_bindings.rs` does not
  exist on main. QENGINE2 already implements `Bindings::attach`, `load_for`, and
  catalogue-scoped admission validation. Release: integrate that owner's qualified
  engine API. A second mutable/global fixture registry or donor quest-content edits
  are not substitutes for the requested engine-only, source-bound PG fixture.
  ENGNPC's merged test-support `commit_fixture_quest_reward` now permits isolated
  NPC reward transaction fixtures; NPC-only rows are reopened rather than left blocked.
- **B2 — retained L2 fluid/charge owner.** Main contains 1251's child-fluid owner,
  but neither L2 migration 1219 nor the root-fluid/charge request model. L2 owns
  `reward_claim_mint/{fluid,charges}.rs`, `fluid_plan`/`charges_plan`, and their exact
  same-transaction proof functions. Release: land that owner first, then compose
  its guards forward after 1251 using reserved 1641–1659 and real root/child MINT
  cases. Do not edit 1219/1251 or invent a stand-in proof function to make PG pass.
- **B3 — SEASON cake transaction owner.** Released by main `6ef6b0cad`; the
  extraction/bite and donation rows are reopened. Reuse the canonical event
  receipts and preserve the 200-substance world goal; do not create a second counter.
- **B4 — qualified Items/Content semantics.** `gameplay/items/src/item_craft.rs`
  requires operand count 1 and emits transformed operand quantity 1. A new
  whole-stack mode must be authored/admitted by that owner, not multiplied by a
  durability caller. Similarly, native timed Item policy cannot be silently
  stripped to make a permanent specimen. Release: qualified recipe/identity
  semantics from Items/Content; those crates/tools are outside ENGITEM allocation.
- **B5 — Appearance/condition and random outcome participants.** Main
  `CallbackEffects.outfit_reward` earns permanent cosmetics; there is no temporary
  disguise/restoration participant. Reward callbacks have `QuestActorConditionProof`,
  but World callbacks lack that admission and a frozen server random outcome.
  Release: receipt-bound live condition/appearance participant and QENGINE2 B1;
  preserve source duration, actor generation, expiry and replay identity.
- **B6 — World/custody action proof.** `QuestWorldContext` contains map/content/reset
  and player placement/movement revisions, not a selected tool, target live-creature
  generation, mutable ground-stack CAS, capture custody or native-tool receipt.
  World input consumption currently selects the first eligible carried instance
  (`quest_world_callback.rs:272`), which is insufficient for selected-source USE.
  Release: the corresponding World/Items owner exposes its sealed live observation
  and atomic transaction/custody participant. Do not convert map projections into
  portable Items or credit success from a later observation.
- **B7 — World movement/publication authority.** NPC reward admission carries no
  destination/actor movement participant. Public speech also belongs to Chat, not
  private quest feedback. Release: owning World/Chat APIs tie their exact CAS or
  publication to the same committed occurrence, including replay/refusal handling.
- **B8 — restart/cycle and repeat proof.** Main world reward MINT passes `false`
  for repeatability (`quest_state/writer.rs:833`). L2's 1211 only permits proved
  costed item exchanges; a command nonce is not an authored daily/quest-cycle token.
  Release: an admitted Quest restart/phase token and once-per-cycle claim identity,
  then exact carried/ground input proof composition. Do not delete completion rows,
  reopen progress with raw SQL, or turn every command into a free world grant.
- **B9 — mount/Quest receipt composition.** `MountTamingRequest` includes item,
  exact target identity/generation, draw, cosmetics and achievements, but no Quest
  participant. Release: an accepted mount-taming/Quest receipt composition and
  Content-qualified object guard. A second post-taming progress write is not atomic.

## Validation

The initial two commits are pushed. Both PG cases reran together on `bbe018482`
with execution markers (3.57s / 3.60s); check/clippy/fmt, generated packets,
dialogue links, completion matrix, pin-check and content-tree validation passed.
Reconciliation: **50 deduplicated rows: 2 DONE, 22 OWNED ELSEWHERE,
23 BLOCKED, 3 OPEN**. Capacity belongs to ENGNPC; ENGITEM retains its quest-clock
work. Existing main compatibility is reused, not counted as new implementation.
No code is claimed for blocked rows. Upstream merges reopen the corresponding
rows. No owner-funded review or external message was sent.

Final continuation validation is recorded in the ledger commit message after
merging current `origin/main` and rerunning the focused checks and generators.
