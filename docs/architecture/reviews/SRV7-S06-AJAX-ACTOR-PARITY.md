# SRV7 S06 source actor effects and fresh callback integration

Pinned Crystal201189f871d23597b82caf88fd611454a2ddefb8 `data-global/npc/ajax.lua:149–156` creates Fire, DELAYED=true, ten2000ms/-10 ticks, applies it to the Player, sets BarbarianAddon2/WaitTimer(now+3600), and closes focus. `ajax.lua:53–62` is a registered CALLBACK_GREET: only a genuinely new engagement after the source hour advances2→3. The callback takes no text argument; Say keyword matching remains unchanged.

The native command7 SourceRoot owns the dialogue, exact session/Character lease/Node/activeContent, costs and one Character revision. The source-only actor stager clones the actual PlayerSpellState and applies the existing Foundation kernel without immediate HP changes. The source parent, immutable actor child and exact general condition snapshot commit in the same Type2 transaction. Runtime/PlayerSpellState Tokio guards remain held through commit and the serialized exact child receipt probe. No std Mutex/RwLock crosses await. Unknown results retain one bounded original before/after memo per actor; resume and actor-end reconcile the receipt before publication or saving. Retained command replay never restages Fire.

ActorEffect validation independently replays the fixed Fire condition on frozen before data and compares the entire after body. New sequence metadata is checked to be strictly later than existing instances (and below exhaustion); persistence does not retain expired allocator history. The captured base speed is bounded to the actual field type. No public effects or arbitrary private properties are discarded. A before/after non-Fire NoOp, changed total/interval/remaining or unrelated condition change cannot satisfy the child. Legitimate stronger Fire KeptCurrent follows the same native kernel.

The fresh callback uses a private non-Clone witness from the actual TalkBook/ChatRuntime owner. An Ok talk whose real joined generation changed, with the exact visible named NPC and current range/book context, creates the witness. Repeated hi does not. It is consumed into retained internal data, then exact NPC identity/generation, book, conversation and movement are rechecked after await under the actual actor guard. SQL consumes the source occurrence once. No client boolean or greeting text manufactures this authority. Current executable callback population is Ajax only; Vescu condition eligibility and other newly audited registered callbacks are separate qualified input work. Undefined/nil and unregistered Lua helpers are not enabled.

The Royal payment preparation augments the same retained effects object, preserving actor/greet. Its currency and bank owners are independent of actor conditions.

Validation observed:

- Genuine seven native command7 utterances on one connection, paced at the unchanged four-utterance source window: valid Fire acquisition RED98.140s, actual stage1 versus required2. A prior two-connection4+3 attempt was only a setup failure because reconnect drops focus/topic; it is not gameplay-gap evidence.
- Fresh native hello/hi, coherent expired-hour predecessor: valid Greet RED160.048s, actual2 versus required3. Repeated-hi remains a single same-connection negative. Current31 GREEN101.775s proves stage3, exactly one callback parent and unchanged timer.
- Fire first implementation reached a genuine existing quest-track revision guard. The fixture had future-stamped its controlled predecessor at2 while Root was1. The correction uses the mature sequenced XP owner to advance Root1→2 with1XP, then stamps current2; the tested source parent advances3. No rowguard is relaxed or Root receipt forged.
- Two focused actor-kernel/child-validator unit tests passed in the current30 owner run. Exact native Fire acquisition/stage/hour/same-TX snapshot/retained replay then GREEN102.615s in current31.
- Cake actual six committed food/bite receipts plus real stager/current actor removal exposed missing pending cleanup RED113.254s. Only normal bounded pending_cake_food pruning was added; snapshots/receipts are unchanged. The complete real Food+prune case GREEN114.640s in current31.

Exact filters:

- `srv7_npc_ajax_real_source_fire_stage_timer_and_replay_are_atomic`
- `srv7_npc_ajax_actual_fresh_greet_after_source_hour_advances_once`
- `srv7_npc_ajax_real_periodic_ten_ticks_logout_and_fresh_lease_restore_remaining_fire`
- `srv7_ajax_source_fire_is_ten_delayed_two_second_ticks_and_preserves_before`
- `srv7_ajax_actor_child_accepts_only_exact_native_fire_successor`
- `srv7_fresh_greet_is_the_actual_join_once_not_the_word_hi`
- `srv7_stale_or_unengaged_talk_cannot_witness_a_source_greet`
- `srv7_cake_current_tibia_four_immediate_food_bites_then_full_and_one_minute_hunger`

Exact shared actor_spell.rs integration is limited to the pending NPC actor-effect vector/initializer, existing pending guard arm, bounded pending cleanup and a test-only normal-housekeeping entry. The Cake vector gets its missing normal retain. No casting enum, spell/cast implementation or casting policy is changed. Production actor-end condition persistence resolves this closed source pending owner before snapshotting.

Fire logout policy is existing, not a new offline rule. Crystal201 `src/lua/functions/creatures/combat/condition_functions.cpp:52–64` defaults Condition(FIRE) to CONDITIONID_COMBAT; condition.cpp1712–1738 makes finite ten2000ms damage intervals;356–370 makes finite COMBAT persistent;167–175 writes remaining ticks and1680–1694 writes unspent intervals. `Condition::startCondition`345–350 rebases the stored remaining ticks to current OTSYS_TIME. Canary47 `src/creatures/players/player.cpp:12231–12234` re-adds stored conditions; its12293–12298 offline subtraction is explicitly only getMuteConditions, not Fire. Existing Foundation condition_persistence.rs freezes remaining microseconds/rebases a fresh process clock. A draft actual lifecycle case will drive five true native periodic ticks, genuine logout/fresh admission with actual newer lease, then remaining five ticks and normal final save. No lease SQL or fabricated ServerAccepted payload is used. The actual new lifecycle test is mounted after first native Fire GREEN; execution remains pending.
