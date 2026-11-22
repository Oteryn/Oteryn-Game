use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::durability::activity_handoff::{ActivityEnterOutcome, ActivityHandoffOwner};
use crate::durability::boss_encounter::reward_chest::{
    BossDeathAtomicPlan, BossOutfit, BossOutfitDeath, BossOutfitRecipient, BossRewardChestPlan,
    BossRewardContributor, BossRewardDeath,
};
use crate::foundation::{CharacterId, CharacterLease, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::kill_reward::DrainOutcome;
use oteryn_gameplay_encounters::bosses::reward_chest::RewardParticipation;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

const ARBAZILOTH: &str = "oteryn:creature.arbaziloth";
const PALE_WORM: &str = "oteryn:creature.the_pale_worm";

#[test]
fn boss_outfits_ratmiral_grants_rascoohan_once_on_native_death() -> TestResult {
    exercise("oteryn:creature.ratmiral_blackwhiskers", "rascoohan", true)
}

#[test]
fn boss_outfits_lord_retro_grants_formal_dress_once_on_native_death() -> TestResult {
    exercise("oteryn:creature.lord_retro", "formal_dress", true)
}

#[test]
fn boss_outfits_pale_worm_does_not_grant_poltergeist_outside_its_rectangle() -> TestResult {
    exercise(PALE_WORM, "poltergeist", false)
}

#[test]
fn boss_outfits_arbaziloth_does_not_grant_fiend_slayer_outside_its_room() -> TestResult {
    exercise(ARBAZILOTH, "fiend_slayer", false)
}

#[test]
fn boss_outfits_pale_worm_grants_poltergeist_at_spectator_rectangle_edge() -> TestResult {
    exercise(PALE_WORM, "poltergeist", true)
}

#[test]
fn boss_outfits_arbaziloth_grants_fiend_slayer_once_from_accepted_room_victory() -> TestResult {
    on_postgres(arbaziloth_room)
}

fn exercise(creature: &'static str, outfit: &'static str, earns: bool) -> TestResult {
    on_postgres(move || scenario(creature, outfit, earns))
}

fn on_postgres(case: impl FnOnce() -> TestResult + Send + 'static) -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || case().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "boss outfit PostgreSQL scenario panicked")??;
    eprintln!(
        "BOSS_OUTFITS POSTGRES_CASE_COMPLETED duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn scenario(creature: &str, outfit: &str, earns: bool) -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "boss_outfits", true, false).await?;
        prepare_current_session(&h).await?;
        let world = super::fixture::fence(1)?.runtime_scope.world_id();
        let (generation, pin) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                world,
                "content-1",
            )?;
        let active = generation.active().ok_or("active generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        let room = pin.qualified_room().clone();
        let runtime_pin = pin.into_channel_pin();
        let deps = Dependencies::with_content_and_capacity(
            runtime_pin,
            room,
            native.spell_book().clone(),
            4,
        )?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(admitted.game_session_id)
                .await?
                .0;
        let fence =
            crate::gameplay_transport::item_fence_of(current).ok_or("current item fence")?;
        admitted.item_fence = Some(fence);
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    admitted.game_session_id,
                    CharacterLease::new(
                        CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            if creature == PALE_WORM {
                // Crystal's donor rectangle includes the east edge and excludes the next tile.
                runtime
                    .initialize_pinned_test_position(
                        deps.actor,
                        MovementLocalPosition {
                            x: if earns { 33816 } else { 33817 },
                            y: 31504,
                            floor: 14,
                        },
                    )
                    .map_err(debug)?;
            } else {
                runtime
                    .initialize_first_entry_position(deps.actor)
                    .map_err(debug)?;
            }
            native
                .install_companion_policies(&mut runtime)
                .map_err(debug)?;
        }
        owner.admit_progression_session(&mut admitted).await;
        assert_eq!(admitted.first_entry, FirstEntryOutcome::Positioned);
        for command in [1, 2] {
            let target = super::qengine_kill_stories_fixtures::queue_death(
                &owner, &admitted, creature, command,
            )
            .await?;
            assert_eq!(
                owner.drain_kill_rewards(admitted.game_session_id).await,
                DrainOutcome::Drained
            );
            assert_grants(&h, outfit, earns).await?;
            super::qengine_kill_stories_fixtures::drain_deaths(&owner, &admitted, &[target])
                .await?;
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn assert_grants(h: &Harness, outfit: &str, earns: bool) -> TestResult {
    let grants: Vec<String> = sqlx::query_scalar(
        "SELECT r.unlock_key FROM game_account_appearance_unlock_receipts r
         JOIN game_account_appearance_unlocks u USING (grant_occurrence_id, account_id, unlock_key)
         JOIN game_character_roots c ON c.character_id=r.granting_character_id AND c.account_id=r.account_id
         JOIN game_boss_death_settlements d ON position(uuid_send(d.death_id) IN r.source_event_id) > 0
         WHERE r.source_kind='quest' ORDER BY r.unlock_key",
    ).fetch_all(&h.pool).await?;
    let expected = earns.then(|| format!("oteryn:outfit.{outfit}/quest-grant/0"));
    assert_eq!(grants, expected.into_iter().collect::<Vec<_>>());
    let receipt_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlock_receipts")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(receipt_count, i64::from(earns));
    Ok(())
}

fn arbaziloth_room() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "boss_outfit_room", true, false).await?;
        let (evidence, mut request) =
            super::activity_runtime_fixtures::prepare_activity_fixture(&h).await?;
        request.activity_key = "oteryn:boss.arbaziloth".into();
        let world = request.destination_scope.world_id();
        let (generation, _) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                world,
                "content-1",
            )?;
        let native = generation
            .active()
            .and_then(|g| g.native_gameplay())
            .ok_or("active gameplay")?;
        let pin = request.destination_content;
        // Keep the existing generic activity fixture geometry and real Arbaziloth key.
        let cell = |x, y| serde_json::json!({"x":x,"y":y,"floor":7});
        sqlx::query("INSERT INTO game_boss_room_catalogue(boss_key,content_revision,boss_creature_key,minimum_players,maximum_players,minimum_level,cooldown_seconds,time_limit_seconds,lever_positions,room_template_key,reward_key,enabled,template_revision,entry_positions,exit_position,boss_position,after_kill_seconds) VALUES($1,'content-1',$2,1,1,1,20,60,$3,'oteryn:room.arbaziloth','oteryn:reward.arbaziloth',true,$4,$5,$6,$7,60)")
            .bind(&request.activity_key).bind(ARBAZILOTH).bind(serde_json::json!([cell(100,101)]))
            .bind(&request.template_revision).bind(serde_json::json!([cell(200,201)]))
            .bind(cell(102,103)).bind(cell(201,201)).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,'arbaziloth-outfit',$2,$3,$4,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),'2026-10')")
            .bind(world.as_bytes().as_slice()).bind(pin.server_artifact_digest.as_slice())
            .bind([0x44_u8;32].as_slice()).bind(pin.coordinate_frame_digest.as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
            .bind(world.as_bytes().as_slice())
            .execute(&h.pool)
            .await?;
        let account: String =
            sqlx::query_scalar("SELECT account_id::text FROM game_character_roots")
                .fetch_one(&h.pool)
                .await?;
        evidence
            .evidence
            .refresh_account(&h.root, &h.node, &account)
            .await
            .map_err(debug)?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let activities = ActivityHandoffOwner::new(h.root.clone());
        let outcome = activities
            .enter_boss_room(&authority, &h.node, request.clone())
            .await?;
        assert!(
            matches!(outcome, ActivityEnterOutcome::Entered(_)),
            "{outcome:?}"
        );
        let session = request.participants[0].destination_game_session_id;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(session)
                .await?
                .0;
        let fence =
            crate::gameplay_transport::item_fence_of(current).ok_or("accepted room lease")?;
        let recipient = BossOutfitRecipient {
            character: fence.character_id,
            session,
            lease_generation: fence.character_lease_generation,
        };
        let minted: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                .fetch_one(&h.pool)
                .await?;
        let death = super::fixture::id(91);
        let contribution = RewardParticipation {
            damage_dealt: 1,
            damage_received: 0,
            healing_others: 0,
        };
        let rewards = BossRewardDeath {
            death,
            boss_key: ARBAZILOTH.into(),
            minted_at_unix_ms: u64::try_from(minted)?,
            contributors: vec![BossRewardContributor {
                character_id: recipient.character,
                session_id: session,
                lease_generation: recipient.lease_generation,
                contribution,
            }],
            chests: vec![BossRewardChestPlan {
                character_id: recipient.character,
                session_id: session,
                lease_generation: recipient.lease_generation,
                contribution,
                items: Vec::new(),
            }],
        };
        let mut plan = BossDeathAtomicPlan::from(rewards);
        plan.outfit = Some(
            BossOutfitDeath::admit(BossOutfit::FiendSlayer, vec![recipient], native)
                .ok_or("admitted Fiend Slayer base grant")?,
        );
        assert_grants(&h, "fiend_slayer", false).await?;
        let mut tx = h.pool.begin().await?;
        let stale = crate::foundation::ScopeOwnershipGeneration::new(
            request.destination_generation.get() + 1,
        )
        .map_err(debug)?;
        assert!(
            !h.root
                .settle_boss_death_atomic(
                    &mut tx,
                    &authority,
                    &h.node,
                    request.destination_scope,
                    stale,
                    plan.clone(),
                )
                .await?
        );
        tx.rollback().await?;
        assert_grants(&h, "fiend_slayer", false).await?;

        // Refuse the appearance receipt after the native victory, then verify the
        // same room/death/appearance transaction rolled back and can be retried.
        sqlx::raw_sql(
            "CREATE SEQUENCE boss_outfit_fault_hit;
             CREATE FUNCTION boss_outfit_fault() RETURNS trigger LANGUAGE plpgsql AS $$
             BEGIN
               IF EXISTS (SELECT 1 FROM game_boss_room_activities WHERE state=2) THEN
                 PERFORM nextval('boss_outfit_fault_hit');
               END IF;
               RAISE EXCEPTION 'injected boss outfit receipt failure' USING ERRCODE='23514';
             END $$;
             CREATE TRIGGER boss_outfit_fault BEFORE INSERT ON game_account_appearance_unlock_receipts
               FOR EACH ROW EXECUTE FUNCTION boss_outfit_fault()",
        ).execute(&h.pool).await?;
        let mut tx = h.pool.begin().await?;
        assert!(
            h.root
                .settle_boss_death_atomic(
                    &mut tx,
                    &authority,
                    &h.node,
                    request.destination_scope,
                    request.destination_generation,
                    plan.clone(),
                )
                .await
                .is_err()
        );
        tx.rollback().await?;
        let storage: (bool, i16, i64) = sqlx::query_as(
            "SELECT (SELECT is_called FROM boss_outfit_fault_hit),
                    (SELECT state FROM game_boss_room_activities),
                    (SELECT count(*) FROM game_boss_death_settlements)",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(storage, (true, 1, 0));
        assert_grants(&h, "fiend_slayer", false).await?;
        sqlx::query("DROP TRIGGER boss_outfit_fault ON game_account_appearance_unlock_receipts")
            .execute(&h.pool)
            .await?;
        for _ in 0..2 {
            let mut tx = h.pool.begin().await?;
            assert!(
                h.root
                    .settle_boss_death_atomic(
                        &mut tx,
                        &authority,
                        &h.node,
                        request.destination_scope,
                        request.destination_generation,
                        plan.clone(),
                    )
                    .await?
            );
            tx.commit().await?;
            assert_grants(&h, "fiend_slayer", true).await?;
        }
        let recorded: Vec<u8> = sqlx::query_scalar(
            "SELECT uuid_send(defeat_id) FROM game_boss_room_activities WHERE state=2",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(recorded, death);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
