//! PREY-DIFF production dispatch and durable transition qualification.
use super::*;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_protocol_oteryn::prey as wire;
use oteryn_protocol_oteryn::CommandStatus;
use sqlx::Row;

#[derive(Clone, Copy)]
enum Case { Tier, Daily, PickAgain, Checkpoint, LapsedCheckpoint, History }

fn run_case(case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("PREY_DIFF POSTGRES_CASE_COMPLETED route_less");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20)
        .spawn(move || scenario(case).map_err(|e| e.to_string()))?
        .join().map_err(|_| "prey difficulty case panicked")??;
    eprintln!("PREY_DIFF POSTGRES_CASE_COMPLETED");
    Ok(())
}
#[test] fn prey_diff_set_tier_preserves_list_and_refuses_locked_or_above_level() -> TestResult { run_case(Case::Tier) }
#[test] fn prey_diff_offers_survive_reload_exclude_three_slots_and_reset_without_charging_exhaustion() -> TestResult { run_case(Case::Daily) }
#[test] fn prey_diff_pick_again_debits_once_and_refuses_unoffered_occupied_locked_or_unfunded() -> TestResult { run_case(Case::PickAgain) }
#[test] fn prey_diff_chosen_race_checkpoint_is_independent_per_slot() -> TestResult { run_case(Case::Checkpoint) }
#[test] fn prey_diff_checkpoint_preserves_admitted_activity_after_premium_lapses() -> TestResult { run_case(Case::LapsedCheckpoint) }
#[test] fn prey_diff_end_history_commits_measured_counters_with_transition() -> TestResult { run_case(Case::History) }

fn scenario(case: Case) -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "prey_difficulty", true, false).await?;
        prepare_current_session(&h).await?;
        super::daily_reward::epoch(&h, 1).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (generation, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let room = pin.qualified_room().clone();
        let (channel_pin, _, _, _) = pin.into_channel_parts();
        let deps = Dependencies::with_content(
            channel_pin,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = generation.active();
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let current = crate::gameplay_transport::character_gameplay_fence::current(
            &owner,
            deps.admitted.game_session_id,
            false,
        )
        .await
        .ok_or("fence")?;
        let formula = owner
            .active_generation
            .and_then(|a| a.native_gameplay())
            .and_then(|n| n.training_formula())
            .ok_or("formula")?;
        let before = crate::durability::character_build::DurableBuildState::default();
        let choice = crate::gameplay_transport::vocation_choice::vocation_choice_request(
            crate::durability::character_build::BuildOccurrence::from_bytes(id(94))
                .map_err(debug)?,
            before,
            "monk",
            formula,
            None,
        )
        .map_err(debug)?;
        owner
            .revision_sequencer
            .acquire(current.character_id)
            .await
            .commit_build(
                owner.root,
                owner.character,
                owner.holder,
                current,
                choice,
                formula,
                None,
            )
            .await
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.item_fence = Some(current.item_fence());
        admitted.first_entry = FirstEntryOutcome::Positioned;
        assert!(owner.prey_available());
        assert!(owner.daily_reward_available());
        let base = crate::gameplay_transport::daily_reward::offered(
            crate::gameplay_transport::capabilities::PRODUCTION_OFFERED_CAPABILITIES,
            true,
        );
        let offered = crate::gameplay_transport::prey::offered(&base, true);
        assert!(offered.iter().any(|c| c.id == 140));
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offered, &[1, 6, 182, 140]).ok_or("prey capability")?;
        sqlx::query("UPDATE game_prey_decision_root SET secret=decode($1,'hex')")
            .bind("a9f248eab2b6a601f753a75c7741a6c8be1e428c3923b13995e3f3ef64b0ac2a")
            .execute(&h.pool)
            .await?;
        assert!(owner.admit_prey(admitted).await);

        let initial = send(&owner, &mut admitted, &wire::Request::Query).await?;
        let native = owner.active_generation.and_then(|g| g.native_gameplay()).ok_or("native")?;
        let catalogue = crate::durability::prey::PreyCatalogue::try_from(native).map_err(debug)?;
        let first = initial.slots[0].creatures[0].clone();
        match case {
            Case::Tier => {
                let tier = initial.slots[0].allowed_tier;
                let tier_command = admitted.continuity.next_command_id;
                let changed = send(&owner, &mut admitted, &wire::Request::SetTier { slot: 1, tier }).await?;
                let tier_bytes = wire::encode_request(&wire::Request::SetTier { slot: 1, tier }).map_err(debug)?;
                let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_prey_receipts").fetch_one(&h.pool).await?;
                let replay = owner.prey_command(admitted, tier_command, &tier_bytes).await;
                let crate::gameplay_transport::guild_intent::GuildReply::Payload(replayed) = replay else { return Err("tier replay refused".into()); };
                assert_eq!(wire::decode_reply(&replayed).map_err(debug)?, changed);
                assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_prey_receipts").fetch_one(&h.pool).await?, count);
                let mut stale = admitted;
                stale.item_fence.as_mut().ok_or("fence")?.character_lease_generation += 1;
                assert!(matches!(owner.prey_command(stale, tier_command, &tier_bytes).await, crate::gameplay_transport::guild_intent::GuildReply::Rejected));
                assert_eq!(changed.slots[0].chosen_tier, tier);
                assert_eq!(changed.slots[0].creatures, initial.slots[0].creatures);
                assert_eq!(changed.slots[0].free_reroll_at, initial.slots[0].free_reroll_at);
                let saved: i16 = sqlx::query_scalar("SELECT chosen_tier FROM game_character_prey_slots WHERE slot=1").fetch_one(&h.pool).await?;
                assert_eq!(saved, i16::from(tier));
                if tier < 5 {
                    let refused = send(&owner, &mut admitted, &wire::Request::SetTier { slot: 1, tier: tier + 1 }).await?;
                    assert_eq!(refused.status, wire::STATUS_INVALID_TIER);
                    assert_eq!(refused.slots[0], changed.slots[0]);
                }
                rejected(&owner, &mut admitted, &wire::Request::SetTier { slot: 3, tier: 1 }).await?;
                let redrawn = send(&owner, &mut admitted, &wire::Request::ListReroll { slot: 1 }).await?;
                assert!(redrawn.slots[0].creatures.iter().all(|key| catalogue.rules().creatures.get(key).is_some_and(|value| value.tier == tier)));
            }
            Case::Daily => {
                unlock_all(&h).await?;
                assert!(owner.admit_prey(admitted).await);
                let all = send(&owner, &mut admitted, &wire::Request::Query).await?;
                let original: std::collections::BTreeSet<_> = all.slots.iter().flat_map(|s| s.creatures.iter().cloned()).collect();
                assert_eq!(original.len(), usize::from(all.list_size) * 3);
                seed_coins(&h, native).await?;
                let mut seen = original;
                for _ in 0..4 {
                    let next = send(&owner, &mut admitted, &wire::Request::ListReroll { slot: 1 }).await?;
                    assert_eq!(next.status, 0);
                    for key in &next.slots[0].creatures { assert!(seen.insert(key.clone()), "offered twice: {key}"); }
                }
                let durable: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_prey_offered WHERE reset_epoch=1").fetch_one(&h.pool).await?;
                assert_eq!(durable as usize, seen.len());
                let before = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_fee_burns WHERE cause_kind=12").fetch_one(&h.pool).await?;
                let keys: Vec<_> = catalogue.rules().creatures.keys().cloned().collect();
                sqlx::query("INSERT INTO game_character_prey_offered(character_id,creature_key,reset_epoch,draw_occurrence) SELECT encode($1,'hex')::uuid,k,1,(SELECT occurrence FROM game_character_prey_receipts WHERE character_id=encode($1,'hex')::uuid LIMIT 1) FROM unnest($2::text[]) k ON CONFLICT(character_id,creature_key) DO NOTHING")
                    .bind(id(41).as_slice()).bind(keys).execute(&h.pool).await?;
                let exhausted_command = admitted.continuity.next_command_id;
                let exhausted = send(&owner, &mut admitted, &wire::Request::ListReroll { slot: 1 }).await?;
                assert_eq!(exhausted.status, wire::STATUS_NO_CANDIDATES);
                assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_fee_burns WHERE cause_kind=12").fetch_one(&h.pool).await?, before);
                // The Channel retains this GameSession's command history across owner
                // reconstruction (SESSION-INGRESS-1); counters alone cannot restore it.
                // All spell/Prey state stays empty and admission must reconstruct the
                // retained lists and daily exclusion from PostgreSQL.
                let commands = std::mem::take(
                    &mut std::sync::Arc::get_mut(&mut owner.states)
                        .ok_or("isolated previous session state")?
                        .command_ingress,
                );
                let mut reloaded = deps.owner(&h, &authority);
                reloaded.active_generation = generation.active();
                std::sync::Arc::get_mut(&mut reloaded.states)
                    .ok_or("isolated reloaded session state")?
                    .command_ingress = commands;
                deps.initialize(&reloaded).await;
                assert!(reloaded.admit_prey(admitted).await);
                let after_reload = send(&reloaded, &mut admitted, &wire::Request::Query).await?;
                assert_eq!(after_reload.slots, exhausted.slots);
                assert_eq!(send(&reloaded, &mut admitted, &wire::Request::ListReroll { slot: 1 }).await?.status, wire::STATUS_NO_CANDIDATES);
                super::daily_reward::epoch(&h, 2).await?;
                let same_draw = wire::encode_request(&wire::Request::ListReroll { slot: 1 }).map_err(debug)?;
                let replay = reloaded.prey_command(admitted, exhausted_command, &same_draw).await;
                let crate::gameplay_transport::guild_intent::GuildReply::Payload(replayed) = replay else { return Err("depleted draw replay refused".into()); };
                assert_eq!(wire::decode_reply(&replayed).map_err(debug)?.status, wire::STATUS_NO_CANDIDATES);
                let next_day = send(&reloaded, &mut admitted, &wire::Request::ListReroll { slot: 1 }).await?;
                assert_eq!(next_day.status, 0);
                assert_eq!(next_day.slots[0].creatures.len(), usize::from(next_day.list_size));
                assert!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_prey_offered WHERE reset_epoch=2").fetch_one(&h.pool).await? > 0);
            }
            Case::PickAgain => {
                let page = send(&owner, &mut admitted, &wire::Request::OfferedToday { slot: 1, after: String::new() }).await?;
                let offered = page.offered_today.iter().find(|r| r.creature == first).ok_or("offered price")?;
                let pick = wire::Request::PickAgain { slot: 1, creature: first.clone(), expected_price_gold: offered.price_gold };
                rejected(&owner, &mut admitted, &pick).await?;
                rejected(&owner, &mut admitted, &wire::Request::PickAgain { slot: 3, creature: first.clone(), expected_price_gold: offered.price_gold }).await?;
                let absent = catalogue.rules().creatures.keys().find(|key| !initial.slots[0].creatures.contains(key)).ok_or("unoffered")?.clone();
                assert_eq!(send(&owner, &mut admitted, &wire::Request::PickAgain { slot: 1, creature: absent, expected_price_gold: 0 }).await?.status, wire::STATUS_NOT_OFFERED);
                seed_coins(&h, native).await?;
                rejected(&owner, &mut admitted, &wire::Request::PickAgain { slot: 1, creature: first.clone(), expected_price_gold: offered.price_gold + 1 }).await?;
                assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_fee_burns WHERE cause_kind=12").fetch_one(&h.pool).await?,0);
                assert_paid_slot_binding(&owner, &h, admitted, &catalogue, &pick).await?;
                let bytes = wire::encode_request(&pick).map_err(debug)?;
                let command = admitted.continuity.next_command_id;
                let result = super::prey::exchange(&owner, &mut admitted, &[(command, &bytes)]).await?;
                assert_eq!(result[0].0, CommandStatus::Accepted);
                let selected = wire::decode_reply(&result[0].1).map_err(debug)?;
                let started = selected_at(&h, 1).await?;
                assert_eq!(selected.slots[0].selected, first);
                assert_eq!(selected.slots[0].state, 2);
                let fee: i64 = sqlx::query_scalar("SELECT fee_gold_units FROM game_item_fee_burns WHERE purchase_service_key='oteryn:prey.pick_again.1'").fetch_one(&h.pool).await?;
                assert_eq!(fee as u64, offered.price_gold);
                assert_eq!(sqlx::query_scalar::<_, i16>("SELECT fee_slot FROM game_character_prey_receipts WHERE fee_kind='PICK_AGAIN' AND fee_gold_units>0").fetch_one(&h.pool).await?, 1);
                let replay = owner.prey_command(admitted, command, &bytes).await;
                let crate::gameplay_transport::guild_intent::GuildReply::Payload(replayed) = replay else { return Err("pick replay refused".into()); };
                assert_eq!(replayed, result[0].1);
                assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_fee_burns WHERE purchase_service_key='oteryn:prey.pick_again.1'").fetch_one(&h.pool).await?, 1);
                unlock_all(&h).await?; assert!(owner.admit_prey(admitted).await);
                let all = send(&owner, &mut admitted, &wire::Request::Query).await?;
                let occupied = all.slots[1].creatures[0].clone();
                assert_eq!(send(&owner, &mut admitted, &wire::Request::PickAgain { slot: 1, creature: occupied, expected_price_gold: 0 }).await?.status, wire::STATUS_OCCUPIED);
                assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_fee_burns WHERE purchase_service_key='oteryn:prey.pick_again.1'").fetch_one(&h.pool).await?, 1);
                let ended = send(&owner, &mut admitted, &wire::Request::ListReroll { slot: 1 }).await?;
                assert_eq!(ended.slots[0].state, 1);
                assert_history(&h, &selected.slots[0], &catalogue, started, "CANCELLED",
                    HistoryCounters { pick_gold: offered.price_gold as i64, ..HistoryCounters::default() }).await?;
            }
            Case::Checkpoint => {
                unlock_all(&h).await?; assert!(owner.admit_prey(admitted).await);
                send(&owner, &mut admitted, &wire::Request::Select { slot: 1, index: 1 }).await?;
                let active = send(&owner, &mut admitted, &wire::Request::Select { slot: 2, index: 1 }).await?;
                for now in [0, 60_000_000] { activity(&owner, deps.actor, admitted.game_session_id, "oteryn:creature.not_chosen", now).await; }
                assert!(owner.flush_prey(admitted.game_session_id).await);
                let idle = send(&owner, &mut admitted, &wire::Request::Query).await?;
                assert_eq!(idle.slots[0].hunting_seconds, 7200); assert_eq!(idle.slots[1].hunting_seconds, 7200);
                for now in [0, 60_000_000] { activity(&owner, deps.actor, admitted.game_session_id, &active.slots[0].selected, now).await; }
                assert!(owner.flush_prey(admitted.game_session_id).await);
                let first_used = send(&owner, &mut admitted, &wire::Request::Query).await?;
                assert_eq!(first_used.slots[0].hunting_seconds, 7140); assert_eq!(first_used.slots[1].hunting_seconds, 7200);
                for now in [0, 60_000_000] { activity(&owner, deps.actor, admitted.game_session_id, &active.slots[1].selected, now).await; }
                assert!(owner.flush_prey(admitted.game_session_id).await);
                let independent = send(&owner, &mut admitted, &wire::Request::Query).await?;
                assert_eq!(independent.slots[0].hunting_seconds, 7140); assert_eq!(independent.slots[1].hunting_seconds, 7140);
            }
            Case::LapsedCheckpoint => {
                use crate::durability::prey::{PreyCheckpoint, PreyMutation};
                trusted(&owner, admitted, id(196), PreyMutation::Initialize, &catalogue, true).await?;
                let active = trusted(&owner, admitted, id(197),
                    PreyMutation::Command(wire::Request::Select { slot: 2, index: 1 }), &catalogue, true).await?;
                assert_eq!(active.slots[1].state, 2);
                let mutation = PreyMutation::Checkpoint(vec![PreyCheckpoint {
                    slot: 2, creature: active.slots[1].selected.clone(),
                    selection_epoch: active.slots[1].selection_epoch, seconds: 60, kills: 1, loot_gold: 42,
                }]);
                // The frozen event was admitted before Premium lapsed. Its slot and
                // selection token still match, so persisting it must not discard it.
                let lapsed = trusted(&owner, admitted, id(198), mutation.clone(), &catalogue, false).await?;
                assert_eq!(lapsed.slots[1].state, 0);
                assert_eq!(lapsed.slots[1].hunting_seconds, 7140);
                assert_eq!(lapsed.slots[1].selection_epoch, active.slots[1].selection_epoch);
                assert_eq!(trusted(&owner, admitted, id(198), mutation, &catalogue, false).await?, lapsed);
                let saved: (String, i32, i64, i64, i64) = sqlx::query_as(
                    "SELECT state,hunting_s_left,consumed_combat_seconds,chosen_kills,chosen_loot_gold FROM game_character_prey_slots WHERE slot=2"
                ).fetch_one(&h.pool).await?;
                assert_eq!(saved, ("ACTIVE".into(), 7140, 60, 1, 42));
                rejected(&owner, &mut admitted, &wire::Request::ListReroll { slot: 2 }).await?;
            }
            Case::History => {
                let active = send(&owner, &mut admitted, &wire::Request::Select { slot: 1, index: 1 }).await?;
                let started = selected_at(&h, 1).await?;
                // Even an administrative transition cannot end the selected lifetime
                // without its same-transaction measurement. The existing runtime-only
                // generic receipt guard is deliberately not what this assertion tests.
                let mut invalid = h.pool.begin().await?;
                sqlx::query("UPDATE game_character_prey_slots SET selection_epoch=selection_epoch+1 WHERE slot=1")
                    .execute(&mut *invalid).await?;
                let failure = invalid.commit().await.err().ok_or("selection ended without history")?;
                assert_eq!(failure.as_database_error().and_then(|error| error.code()).as_deref(), Some("23514"));
                assert_eq!(sqlx::query_scalar::<_, i64>("SELECT selection_epoch FROM game_character_prey_slots WHERE slot=1")
                    .fetch_one(&h.pool).await?, active.slots[0].selection_epoch as i64);
                let mut revision = owner.revision_sequencer.acquire(current.character_id).await;
                let fence = crate::gameplay_transport::character_gameplay_fence::current(&owner, admitted.game_session_id, true).await.ok_or("current fence")?;
                let delta = crate::durability::prey::PreyCheckpoint { slot: 1, creature: active.slots[0].selected.clone(), selection_epoch: active.slots[0].selection_epoch, seconds: 60, kills: 2, loot_gold: 123 };
                let mutation = crate::durability::prey::PreyMutation::Checkpoint(vec![delta]);
                owner.root.commit_prey_command(owner.character, owner.holder, fence, id(195), mutation, &catalogue, None, &NoPremium).await.map_err(debug)?;
                revision.observe_loop_training(); drop(revision);
                assert_runtime_history_protection(&h).await?;
                let ended = send(&owner, &mut admitted, &wire::Request::ListReroll { slot: 1 }).await?;
                assert_eq!(ended.slots[0].state, 1);
                let row: (i64,i64,i64,String) = sqlx::query_as("SELECT consumed_combat_seconds,chosen_kills,chosen_loot_gold,end_reason FROM game_character_prey_history WHERE slot=1").fetch_one(&h.pool).await?;
                assert_eq!(row, (60,2,123,"CANCELLED".into()));
                assert_history(&h, &active.slots[0], &catalogue, started, "CANCELLED",
                    HistoryCounters { consumed: 60, kills: 2, loot_gold: 123, ..HistoryCounters::default() }).await?;
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL ROLE oteryn_game_control").execute(&mut *tx).await?;
                assert_eq!(sqlx::query_scalar::<_, i64>("SELECT selections FROM game_prey_balance_by_tier LIMIT 1").fetch_one(&mut *tx).await?,1);
                tx.rollback().await?;
            }
        }
        drop(owner); drop(authority); drop(seal); h.cleanup().await?; Ok(())
    })
}

async fn assert_paid_slot_binding(
    owner: &ComposedFreshAdmission<'_, '_, '_>, h: &Harness, admitted: AdmittedSession,
    catalogue: &crate::durability::prey::PreyCatalogue, request: &wire::Request,
) -> TestResult {
    // Corrupt only the typed slot on the real owner's new receipt. Every production
    // constraint remains enabled; the fee, native coin burn and action are otherwise real.
    sqlx::query("CREATE FUNCTION prey_test_misbind_fee_slot() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.fee_slot:=2; RETURN NEW; END $$")
        .execute(&h.pool).await?;
    sqlx::query("CREATE TRIGGER prey_test_misbind_fee_slot BEFORE INSERT ON game_character_prey_receipts FOR EACH ROW WHEN(NEW.fee_kind='PICK_AGAIN' AND NEW.fee_gold_units>0) EXECUTE FUNCTION prey_test_misbind_fee_slot()")
        .execute(&h.pool).await?;
    let before = super::revision(h).await?;
    let character = admitted.item_fence.ok_or("item fence")?.character_id;
    let mut revision = owner.revision_sequencer.acquire(character).await;
    let mut fence = crate::gameplay_transport::character_gameplay_fence::current(
        owner, admitted.game_session_id, true,
    ).await.ok_or("current fee fence")?;
    fence.expected_character_revision = revision.cursor(owner.root, owner.character).await.map_err(debug)?;
    let coins = {
        let runtime = owner.runtime.lock().await;
        crate::gameplay_transport::charm_progression::fee_change(owner, &runtime, None, [id(193), id(194)])
    };
    let error = owner.root.commit_prey_command(
        owner.character, owner.holder, fence, id(199),
        crate::durability::prey::PreyMutation::Command(request.clone()), catalogue, coins, &NoPremium,
    ).await.err().ok_or("a different slot's receipt proved the fee")?;
    revision.observe_loop_training();
    drop(revision);
    // The durability boundary intentionally retains only the typed commit outcome,
    // never the database's raw error text. The typed slot is the sole corrupted fact.
    assert!(matches!(&error, crate::durability::character_progression::CharacterProgressionError::Unavailable(
        crate::durability::DurabilityError::CommitRejected
    )), "the action/slot mismatch must be a definite commit refusal: {error:?}");
    assert_eq!(super::revision(h).await?, before);
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_fee_burns WHERE cause_kind=12").fetch_one(&h.pool).await?, 0);
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_prey_receipts WHERE occurrence=encode($1,'hex')::uuid")
        .bind(id(199).as_slice()).fetch_one(&h.pool).await?, 0);
    sqlx::query("DROP TRIGGER prey_test_misbind_fee_slot ON game_character_prey_receipts").execute(&h.pool).await?;
    sqlx::query("DROP FUNCTION prey_test_misbind_fee_slot()").execute(&h.pool).await?;
    Ok(())
}

async fn assert_runtime_history_protection(h: &Harness) -> TestResult {
    // Force this one deferred guard before rolling back. The older generic receipt
    // guard remains enabled, but cannot mask a missing lifetime/counter invariant.
    for assignment in ["creature_key='oteryn:creature.guard_rewrite'", "chosen_kills=0"] {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
        sqlx::query(sqlx::AssertSqlSafe(format!("UPDATE game_character_prey_slots SET {assignment} WHERE character_id=encode($1,'hex')::uuid AND slot=1")))
            .bind(id(41).as_slice()).execute(&mut *tx).await?;
        let error = sqlx::query("SET CONSTRAINTS game_prey_ended_selection_consistent IMMEDIATE")
            .execute(&mut *tx).await.err().ok_or("runtime rewrote an active selection without ending it")?;
        assert_eq!(error.as_database_error().and_then(|error| error.code()).as_deref(), Some("23514"));
        assert_eq!(error.as_database_error().map(|error| error.message()),
            Some("Prey lifetime identity and cumulative counters cannot be rewritten"));
        tx.rollback().await?;
    }
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_prey_history(character_id,slot,selection_epoch,creature_key,tier,bonus_type,grade,selected_at_unix_ms,ended_at_unix_ms,consumed_combat_seconds,wildcards_used,list_rerolls_free,list_rerolls_gold,chosen_kills,chosen_loot_gold,pick_again_gold,end_reason,end_occurrence) SELECT s.character_id,s.slot,s.selection_epoch,s.creature_key,s.selected_tier,CASE s.bonus_type WHEN 'DAMAGE' THEN 1 WHEN 'REDUCTION' THEN 2 WHEN 'XP' THEN 3 WHEN 'LOOT' THEN 4 END,s.grade,s.selected_at_unix_ms,floor(extract(epoch FROM clock_timestamp())*1000)::bigint,s.consumed_combat_seconds,s.wildcards_used,s.list_rerolls_free,s.list_rerolls_gold,0,s.chosen_loot_gold,s.pick_again_gold,'CANCELLED',(SELECT occurrence FROM game_character_prey_receipts WHERE character_id=s.character_id LIMIT 1) FROM game_character_prey_slots s WHERE s.character_id=encode($1,'hex')::uuid AND s.slot=1")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_character_prey_slots SET state='SELECTION',creature_key=NULL,hunting_s_left=0 WHERE character_id=encode($1,'hex')::uuid AND slot=1")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    let error = sqlx::query("SET CONSTRAINTS game_prey_ended_selection_consistent IMMEDIATE")
        .execute(&mut *tx).await.err().ok_or("ending history discarded already measured kills")?;
    assert_eq!(error.as_database_error().and_then(|error| error.code()).as_deref(), Some("23514"));
    assert_eq!(error.as_database_error().map(|error| error.message()),
        Some("Ending a Prey selection requires its atomic measurement row"));
    tx.rollback().await?;
    Ok(())
}

#[derive(Default)]
pub(super) struct HistoryCounters {
    pub(super) consumed: i64,
    pub(super) wildcards: i64,
    pub(super) free_rerolls: i64,
    pub(super) gold_rerolls: i64,
    pub(super) kills: i64,
    pub(super) loot_gold: i64,
    pub(super) pick_gold: i64,
}

pub(super) async fn selected_at(h: &Harness, slot: u8) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT selected_at_unix_ms FROM game_character_prey_slots WHERE character_id=encode($1,'hex')::uuid AND slot=$2")
        .bind(id(41).as_slice()).bind(i16::from(slot)).fetch_one(&h.pool).await?)
}

pub(super) async fn assert_history(
    h: &Harness, slot: &wire::Slot, catalogue: &crate::durability::prey::PreyCatalogue,
    started: i64, reason: &str, counters: HistoryCounters,
) -> TestResult {
    let row = sqlx::query("SELECT h.*,uuid_send(h.character_id) AS character_bytes,h.created_xact_id=r.created_xact_id AS atomic_receipt FROM game_character_prey_history h JOIN game_character_prey_receipts r ON r.character_id=h.character_id AND r.occurrence=h.end_occurrence WHERE h.character_id=encode($1,'hex')::uuid AND h.slot=$2 AND h.selection_epoch=$3")
        .bind(id(41).as_slice()).bind(i16::from(slot.slot)).bind(slot.selection_epoch as i64)
        .fetch_one(&h.pool).await?;
    assert_eq!(row.try_get::<Vec<u8>,_>("character_bytes")?, id(41));
    assert_eq!(row.try_get::<i16,_>("slot")?, i16::from(slot.slot));
    assert_eq!(row.try_get::<i64,_>("selection_epoch")?, slot.selection_epoch as i64);
    assert_eq!(row.try_get::<String,_>("creature_key")?, slot.selected);
    assert_eq!(row.try_get::<i16,_>("tier")?, i16::from(catalogue.rules().creatures.get(&slot.selected).ok_or("selected tier")?.tier));
    assert_eq!(row.try_get::<i16,_>("bonus_type")?, i16::from(slot.bonus));
    assert_eq!(row.try_get::<i16,_>("grade")?, i16::from(slot.grade));
    assert_eq!(row.try_get::<Option<i64>,_>("selected_at_unix_ms")?, Some(started));
    let ended: i64 = row.try_get("ended_at_unix_ms")?;
    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
        .fetch_one(&h.pool).await?;
    assert!((started..=now).contains(&ended));
    assert_eq!(row.try_get::<String,_>("end_reason")?, reason);
    assert!(row.try_get::<bool,_>("atomic_receipt")?);
    for (field, expected) in [
        ("consumed_combat_seconds", counters.consumed),
        ("wildcards_used", counters.wildcards),
        ("list_rerolls_free", counters.free_rerolls),
        ("list_rerolls_gold", counters.gold_rerolls),
        ("chosen_kills", counters.kills),
        ("chosen_loot_gold", counters.loot_gold),
        ("pick_again_gold", counters.pick_gold),
    ] { assert_eq!(row.try_get::<i64,_>(field)?, expected, "{field} for selection {}", slot.selection_epoch); }
    Ok(())
}

async fn send(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession, request: &wire::Request) -> TestResult<wire::Reply> {
    let bytes = wire::encode_request(request).map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let results = super::prey::exchange(owner, admitted, &[(command, &bytes)]).await?;
    assert_eq!(results[0].0, CommandStatus::Accepted);
    wire::decode_reply(&results[0].1).map_err(debug)
}
async fn rejected(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession, request: &wire::Request) -> TestResult {
    let bytes = wire::encode_request(request).map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    assert_eq!(super::prey::exchange(owner, admitted, &[(command, &bytes)]).await?[0].0, CommandStatus::Rejected); Ok(())
}
async fn unlock_all(h: &Harness) -> TestResult {
    for slot in [2_i16,3] {
        sqlx::query("INSERT INTO game_account_prey_unlocks(account_id,world_id,unlock_kind,target,source,claiming_character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'PREY_SLOT_PERMANENT',$4,'STORE_CLAIM',encode($3,'hex')::uuid) ON CONFLICT DO NOTHING").bind(id(40).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(slot).execute(&h.pool).await?;
    }
    Ok(())
}
async fn activity(owner: &ComposedFreshAdmission<'_, '_, '_>, actor: crate::foundation::ExactActorRef, session: crate::foundation::GameSessionId, creature: &str, now: u64) {
    let runtime = owner.runtime.lock().await;
    owner.states.spell_states.lock().await.prey_activity(&runtime, actor, session, crate::gameplay_transport::prey::PreyActivity {
        creature, kind: oteryn_gameplay_progression::prey::PreyActivityKind::DealtDamage, now,
    });
}
struct NoPremium;
impl crate::durability::character_wheel::WheelPremiumGate for NoPremium {
    async fn allocation_permitted(&self, _: &mut sqlx::Transaction<'_,sqlx::Postgres>, _: [u8;16]) -> Result<bool,crate::durability::DurabilityError> { Ok(false) }
}

struct AssertedPremium(bool);
impl crate::durability::character_wheel::WheelPremiumGate for AssertedPremium {
    async fn allocation_permitted(&self, _: &mut sqlx::Transaction<'_,sqlx::Postgres>, _: [u8;16]) -> Result<bool,crate::durability::DurabilityError> { Ok(self.0) }
}

async fn trusted(
    owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: AdmittedSession, occurrence: [u8;16],
    mutation: crate::durability::prey::PreyMutation,
    catalogue: &crate::durability::prey::PreyCatalogue, premium: bool,
) -> TestResult<wire::Reply> {
    let character = admitted.item_fence.ok_or("item fence")?.character_id;
    let mut revision = owner.revision_sequencer.acquire(character).await;
    let mut fence = crate::gameplay_transport::character_gameplay_fence::current(
        owner, admitted.game_session_id, true,
    ).await.ok_or("current fence")?;
    fence.expected_character_revision = revision.cursor(owner.root, owner.character).await.map_err(debug)?;
    let result = owner.root.commit_prey_command(owner.character, owner.holder, fence, occurrence,
        mutation, catalogue, None, &AssertedPremium(premium)).await.map_err(debug)?;
    revision.observe_loop_training();
    Ok(result.0)
}

async fn seed_coins(h: &Harness, native: &oteryn_gameplay_content::content::native_gameplay::NativeGameplayState) -> TestResult {
            let index = native.item_index().ok_or("items")?;
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            for (seed, key, quantity) in [
                (210, "oteryn:item.tibia.i2854", 1),
                (212, "oteryn:item.tibia.i3043", 100),
            ] {
                sqlx::raw_sql(sqlx::AssertSqlSafe(
                    crate::durability::item_fee_burn::test_support::item(
                        seed,
                        key,
                        index.revision(key).ok_or("revision")?,
                        quantity,
                    ),
                ))
                .execute(&mut *tx)
                .await?;
            }
            sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)").bind(id(41).as_slice()).bind(id(210).as_slice()).bind(id(42).as_slice()).bind(id(214).as_slice()).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)").bind(id(41).as_slice()).bind(id(210).as_slice()).bind(id(212).as_slice()).bind(id(42).as_slice()).bind(id(215).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;

    Ok(())
}
