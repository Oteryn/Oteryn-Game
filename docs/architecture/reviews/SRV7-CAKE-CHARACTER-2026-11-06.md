# SRV7 W05d Cake character errand and Brutus rewards

Source pin: `zimbadev/crystalserver@201189f871d23597b82caf88fd611454a2ddefb8`.
This is the per-character part of W05d. World stages, golem extraction, Cake bites,
substances, scheduling and seasonal entity lifecycle belong to the sibling World owner.

## Source rules

- `data-global/scripts/lib/a_piece_of_cake_config.lua:74–96`: errand state is reset by
  the annual run identifier; it stores started/armed, distinct visited cities and
  two independent once-per-run reward flags.
- `data-global/scripts/quests/a_piece_of_cake/movements_cake_base_isle.lua:5–33`:
  stepping on a real Cake Base grants Cake Conqueror; in stage 3 it grants I Did
  My Part and arms the next extraction visit. Starting Brutus dialogue is not a
  prerequisite. Completed visits do not rearm.
- `data-global/scripts/quests/a_piece_of_cake/movements_extraction_spot.lua:51–75,95–97`:
  stage 3 visits require the Isle mark. A repeated city preserves the mark; a new
  city consumes it. Five distinct cities complete the errand.
- `data-global/scripts/lib/a_piece_of_cake_config.lua:193–200`: city action IDs
  43910 Carlin, 43911 Thais, 43912 Edron, 43913 Darashia, 43914 Liberty Bay.
- `data-global/npc/brutus_the_baker.lua:56–74`: Help starts an incomplete errand;
  completed Help reports completion and preserves its existing started flag.
- `data-global/npc/brutus_the_baker.lua:81–133`: choose one item and independently
  one cupcake per run, only after five distinct visits. Item and cupcake choices
  do not consume one another's entitlement.
- `data-global/scripts/lib/a_piece_of_cake_config.lua:500–506`: cabinet construction
  kit14756, Cake Backpack20347, tapestry20350; blueberry28484, strawberry28485,
  lemon28486. All quantities are one. Native admission retains Backpack capacity20.
- `data-global/scripts/lib/a_piece_of_cake_config.lua:64,474–483`: source end-of-event
  cleanup on February26→27 removes Brutus and the bridge; the separate seven-day
  World bonus does not extend the character reward NPC lifetime.
- `data-global/npc/brutus_the_baker.lua:139–141`: greeting and farewell are literal
  source dialogue. The greeting templates the admitted player name.

## Native owners and durable boundary

`gameplay_transport::seasonal_cake::cake_character_step` runs after committed paced
movement. It qualifies the actual composed source tile, current controlled actor,
session and movement snapshot. Its occurrence includes actor identity, session,
reset epoch and committed movement revision. It calls the durability character
owner for Isle or the source city action.

`npc_command::seasonal_cake` is reached through the normal native NPC conversation
adapter. It retains current NPC generation/range/conversation, current actor
control, revision sequencing, active native generation and independent Character
item fence. Brutus uses the same native NPC protocol/window as the other NPCs.

The character owner locks the current World run and current character predecessor.
Its Type2 transaction combines the source transition, achievements and immutable
occurrence receipt. Replaying an old Isle receipt returns its historical result
and cannot rearm after a later committed extraction. Native reward replay resolves
its original claim before fresh eligibility and never mints another item.

The private character arm uses actual Node possession secret, backend PID and XID.
It verifies the existing private World context and independently verifies current
session/connection/lease/account presence and Channel readiness. No writable GUC
is authority. The private context records the predecessor and one source action.
Deferred SQL guards require the exact resulting five fields, plus a same-XID
source receipt, or for a reward flag a same-XID mature native mint receipt. Source
receipts retain their stamped Node/fence/action proof and cannot be changed,
deleted or truncated by runtime SQL.

`reward_claim_mint::seasonal` composes source eligibility with the existing carried
backpack mint owner in one Type2 transaction. It preserves native reservation,
weight pin, capacity, native identity allocation, full container contents, typed
child audits and replay. A refused destination rolls back both native claim and
annual flag. Existing inbox prohibitions on container rewards remain unchanged.
This also provides the narrow complete-container helper for Santa's source gift.

No capability, wire message or command number was added.

## Verification record

Observed regression/removal RED:

- `srv7_cake_five_city_marks_receipts_stale_fence_and_two_annual_rewards` initially
  exercised the implemented owner and passed in58.846s (`/tmp/srv7-acquisition-cake-red.log:122`).
  A deliberately removed source-event entrypoint then produced the required
  regression RED in3.845s (`/tmp/srv7-runtime-red.log:164–181`, active Cake character).
  The removal was restored immediately. This was a removal/mutation RED, not a
  claim that an absent public API compiled on the original base.
- `srv7_brutus_native_npc_help_container_and_cupcake_replay` observed actual
  `NoConversation` RED in96.147s before the source-qualified Brutus greeting was
  admitted. The greeting fix belongs to the sibling seven-NPC seasonal source
  overlay and uses the original native talk book.

- The raw-runtime security regression then failed in4.362s (inner4.34s) at
  `seasonal_cake_character.rs:51`, exactly `raw runtime cannot forge five city
  marks`, in `/tmp/srv7-cost-security-rotate-red.log:142–160`. The real source
  owner had already created an initial mark; the unarmed runtime SQL update
  committed before the new guards. This is the direct security RED for them.

SQL syntax validation is separate from behavioral proof: all303 migrations plus
character and closed-cost guard candidates applied in an isolated PostgreSQL
scratch database in4.072s on the final candidate, with the Cake XP receipt column confirmed and
the scratch database then dropped. No source or World data was left behind.

Focused behavioral filters:

- `srv7_cake_five_city_marks_receipts_stale_fence_and_two_annual_rewards`
- `srv7_brutus_native_npc_help_container_and_cupcake_replay`

The first covers all five cities, absent mark, repeated city preserving mark,
old-Isle replay, stale connection, conflicting occurrence, source Help after
completion, raw runtime mark/flag forgery, immutable receipts, pre-save refusal,
missing backpack rollback, empty native Cake Backpack capacity, independent
cupcake, duplicate annual categories and historical replay after the source
six-day event end. The second calls real native NPC hi/help/item/cupcake choices
under current authority and checks native inventory consequences and stale NPC
rejection. Final GREEN timings are recorded by the task coordinator after the
source-frozen combined validation.

## Earned World bonus

Crystal `201189f871d23597b82caf88fd611454a2ddefb8`
`data-global/scripts/quests/a_piece_of_cake/world_bonus.lua:5–9` adds
`floor(exp*50/100)` only for the individual Premium recipient. Lines14–48 grant
an independent regeneration condition to every vocation: each positive native
HP/MP gain adds `max(1,floor(gain*25/100))` on its unchanged native tick period.
This condition does not depend on the food clock. Config lines116–118 fix the
seven-day duration and the25/50 percentages. The retained bonus deadline survives
World stage cleanup; pending next-save and exact expiry remain separate states.

The production party XP owner applies this separate factor after the existing
hunting/stamina calculation per recipient and stores it in the original receipt.
Its0-percent request binding remains byte-compatible. Optional bonus reads use a
savepoint and fail to0; missing optional data cannot abort a kill. The existing
vitals lane reads the earned deadline and enriches its native HP/MP successor
with independent Cake clocks. Food clocks, equipment clocks, existing conditions
and spell cast mutation behavior remain under their original owners.

Observed real production-call regression: removing the bonus gave12/8 instead
of18/8 in116.540s (`/tmp/srv7-coherent-callers-2.log:168–196`). Restoring it passed
`srv7_cake_real_party_kill_premium_and_free_after_event_cleanup_and_replay` in
115.742s (`/tmp/srv7-real-callers-red-4.log:205`). The native party recipients are
Premium and Free respectively, after the six-day event cleanup; replay at a later
clock keeps the original receipt amounts.

Additional focused filters:

- `srv7_cake_real_party_kill_bonus_expires_at_its_own_deadline`
- `cake_bonus_has_independent_hungry_free_native_clocks_and_exact_expiry`
- `cake_bonus_reuses_pz_streak_gates_and_does_not_advance_food_clocks`
- `source_cake_bonus_is_premium_only_separate_and_checked`

Native observation filter
`world_event_postgres_real_dispatch_reset_restart_and_fences` passed223.278s
(`/tmp/srv7-coherent-callers-2.log:63`). It decodes cap371 Cake progress for a
source-valid historical completed run after stage0 cleanup, verifies retained
bonus deadline and server observation clock, legacy absence, restart and exact
receipt replay, and a fresh observation after the calendar closes.

Latest focused PostgreSQL verification also passed
`srv7_cake_five_city_marks_receipts_stale_fence_and_two_annual_rewards` in
62.733s and `srv7_cake_real_party_kill_bonus_expires_at_its_own_deadline` in
124.916s (`/tmp/srv7-native-callers-red-8.log`). The Brutus native conversation
case remains pending the shared canonical reference identity ordering repair;
its latest failure is not recorded as a successful play test.
