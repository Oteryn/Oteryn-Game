//! P08 perk modification through the production command owner (commands 379, 380, 392-395)
//! with the Global shaping values and migration 0575's activation: MODIFY spends 250 dust and
//! lands on rank 0, ORB_RANK burns one carried Lunar Ascension Orb with its line, a replay burns
//! nothing, RANK_UP stays not admitted, CLEAR frees the slot and ORB_RANK without an orb is
//! `NO_ORB`.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::character_proficiency::ProficiencyDefinitions;
use crate::foundation::MovementLocalPosition;
use crate::map::boot::tests::{floor_items, floor_pins, teleport_bundle};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::weapon_proficiency::ProductionProficiencyCatalogue;
use oteryn_gameplay_domain::domain::weapon_proficiency::PROFICIENCY_FIRST_MODIFICATION_LEVEL;
use oteryn_protocol_oteryn::weapon_proficiency::{
    ModificationOperation, ModificationReply, ModificationRequest, ModificationResult,
    SelectPerkRequest, SelectPerkResult,
};
use std::sync::Arc;

const SWORD: &str = "oteryn:item.tibia.i3264";
const ORB: &str = "oteryn:item.tibia.i53695";
const BAG: u8 = 170;
const ORBS: u8 = 172;

#[test]
fn proficiency_modification_spends_dust_burns_an_orb_and_clears() -> TestResult {
    std::thread::Builder::new()
        .name("proficiency_modification".into())
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "proficiency modification scenario panicked")?
        .map_err(Into::into)
}

#[allow(clippy::too_many_lines)]
fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "proficiency_modification", false, false)
                .await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let generation = controller.active().ok_or("active")?;
        let sword = generation
            .native_gameplay()
            .ok_or("native")?
            .item_index()
            .ok_or("index")?
            .definition_ref("Item", SWORD, "definition-r1")
            .ok_or("item ref")?;
        let room = pin.qualified_room().clone();
        // The production room has no protection zone; the teleport fixture's tile at
        // (5, 1, 8) has the PZ flag (as in `daily_resting`).
        let (bytes, pins) = teleport_bundle();
        let channel = crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?;
        let map = crate::map::boot::boot(&bytes, &floor_pins(pins), world, channel, floor_items)
            .map_err(debug)?;
        let cells = map.movement_cells(room.movement_cells()).map_err(debug)?;
        let frame_digest = pin.frame_binding().digest();
        let deps = Dependencies::with_content(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 5,
                    y: 1,
                    floor: 8,
                },
            )
            .map_err(debug)?;
        let catalogue = ProductionProficiencyCatalogue::embedded().ok_or("catalogue")?;
        let track_shape = catalogue
            .resolve_weapon(SWORD)
            .ok_or("sword definition")?
            .shape
            .track_shape();
        let mut owner = deps.owner(&harness, &authority);
        owner.movement_cells = &cells;
        owner.active_generation = Some(generation);
        owner.proficiency_catalogue = Some(Arc::clone(&catalogue));
        deps.initialize(&owner).await;
        let actor = deps.actor;
        let session = deps.admitted.game_session_id;
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;

        // Training progress that reaches the first modification level, checkpointed through
        // the production saver.
        let progress = (1..=u32::MAX / 2)
            .map(|step| step * 50)
            .find(|progress| track_shape.level(*progress) >= PROFICIENCY_FIRST_MODIFICATION_LEVEL)
            .ok_or("first modification level")?;
        {
            let mut states = owner.states.proficiencies.lock().map_err(|_| "states")?;
            let state = states.entry(session).or_default();
            let mut track = catalogue.empty_track(SWORD).ok_or("sword track")?;
            track.progress = progress;
            state.install_track(SWORD, track);
            state.pending_progress.insert(SWORD.to_owned(), progress);
            state.dirty_since = Some(0);
        }
        assert_eq!(
            owner.save_weapon_proficiency(actor, session, true).await,
            crate::gameplay_transport::actor_spell::TrainingSave::Saved
        );
        let in_zone = {
            let mut runtime = deps.runtime.lock().await;
            crate::gameplay_transport::actor_spell::qualified_protection_zone(
                &mut runtime,
                owner.movement_cells,
                actor,
            ) == Some(true)
        };
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice()).bind(frame_digest.as_slice()).execute(&harness.pool).await?;
        seed_bag(&harness).await?;
        seed_dust(&harness, 1_000).await?;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "fence")?
            .ok_or("fence")?
            .item_fence();
        let command = |command_id| UseCommand {
            game_session_id: session,
            command_id,
            item_fence: Some(fence),
        };
        let selected = FreshAdmissionAuthority::weapon_proficiency_select(
            &owner,
            actor,
            command(1),
            SelectPerkRequest {
                definition: sword,
                level: 0,
                perk: Some(0),
            },
        )
        .await
        .ok_or("select")?;
        assert_eq!(selected.result, SelectPerkResult::Accepted);

        let pool = &harness.pool;
        let track_revision = || async {
            sqlx::query_scalar::<_, String>(
                "SELECT committed_character_revision::text FROM game_character_proficiency \
                 WHERE item_key=$1",
            )
            .bind(SWORD)
            .fetch_one(pool)
            .await
            .map_err(debug)?
            .parse::<u64>()
            .map_err(debug)
        };
        let dust = || async {
            sqlx::query_scalar::<_, i64>("SELECT balance FROM game_character_forge_dust")
                .fetch_one(pool)
                .await
        };
        let burns = || async {
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_item_use_records WHERE cause_kind=495",
            )
            .fetch_one(pool)
            .await
        };
        let orbs = || async {
            sqlx::query_scalar::<_, i64>(
                "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
            )
            .bind(id(ORBS).as_slice())
            .fetch_one(pool)
            .await
        };
        let send = async |command_id, operation, expected_revision| -> TestResult<ModificationReply> {
            // Each step stands in its own second of PROF1B-RL-03 (two occurrences a second).
            owner
                .states
                .proficiencies
                .lock()
                .map_err(|_| "states")?
                .entry(session)
                .or_default()
                .modification_cap = Default::default();
            FreshAdmissionAuthority::weapon_proficiency_modify(
                &owner,
                actor,
                command(command_id),
                ModificationRequest {
                    definition: sword,
                    slot: 1,
                    operation,
                    expected_revision,
                },
            )
            .await
            .ok_or_else(|| "modification reply".into())
        };
        let modify = async |command_id, operation| -> TestResult<ModificationReply> {
            send(command_id, operation, track_revision().await?).await
        };

        // 1B §5 check 4: every modification needs a protection zone.
        assert!(in_zone, "the fixture tile (5, 1, 8) is a protection zone");

        // MODIFY: 250 dust, rank 0 (PROFICIENCY-1C §2, §4).
        let modified = modify(10, ModificationOperation::Modify { level: 0 }).await?;
        assert_eq!(modified.result, ModificationResult::Accepted);
        let slot = modified.slot.ok_or("modified slot")?;
        assert_eq!((slot.level, slot.rank, slot.pending_offer), (0, 0, None));
        assert_eq!(dust().await?, 750);
        // RANK_UP reads unpublished rank 1-9 values: not admitted, nothing spent.
        assert_eq!(
            modify(11, ModificationOperation::RankUp).await?.result,
            ModificationResult::NotAdmitted
        );
        assert_eq!(dust().await?, 750);

        // ORB_RANK burns the carried orb with its line in one transaction (1C §5, 0575).
        let before_orb = track_revision().await?;
        let ranked = send(12, ModificationOperation::OrbRank, before_orb).await?;
        assert_eq!(ranked.result, ModificationResult::Accepted);
        assert_eq!(ranked.slot.map(|slot| slot.rank), Some(10));
        assert_eq!((orbs().await?, burns().await?), (1, 1));
        let paired: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_use_records u \
             JOIN game_character_proficiency_modification_lines l \
               ON l.proficiency_occurrence_id=u.transaction_id \
             WHERE u.cause_kind=495 AND u.shape=1 AND u.source_definition_production_key=$1 \
               AND l.operation='ORB_RANK' AND l.orbs_spent=1 AND l.rank_after=10",
        )
        .bind(ORB)
        .fetch_one(pool)
        .await?;
        assert_eq!(paired, 1);
        // The same command answers from its receipt without a second burn.
        assert_eq!(
            send(12, ModificationOperation::OrbRank, before_orb).await?,
            ranked
        );
        assert_eq!((orbs().await?, burns().await?), (1, 1));
        assert_eq!(
            modify(13, ModificationOperation::OrbRank).await?.result,
            ModificationResult::RankMax
        );

        // CLEAR is free and frees the slot (TibiaWiki Updates/15.30.a30dad).
        let cleared = modify(14, ModificationOperation::Clear).await?;
        assert_eq!((cleared.result, cleared.slot), (ModificationResult::Accepted, None));
        assert_eq!(dust().await?, 750);

        // A second MODIFY, then ORB_RANK burns the last orb; a third finds none.
        assert_eq!(
            modify(15, ModificationOperation::Modify { level: 0 })
                .await?
                .result,
            ModificationResult::Accepted
        );
        assert_eq!(
            modify(16, ModificationOperation::OrbRank).await?.result,
            ModificationResult::Accepted
        );
        assert_eq!(burns().await?, 2);
        assert_eq!(modify(17, ModificationOperation::Clear).await?.result, ModificationResult::Accepted);
        assert_eq!(
            modify(18, ModificationOperation::Modify { level: 0 })
                .await?
                .result,
            ModificationResult::Accepted
        );
        assert_eq!(
            modify(19, ModificationOperation::OrbRank).await?.result,
            ModificationResult::NoOrb
        );
        assert_eq!((dust().await?, burns().await?), (250, 2));
        eprintln!("proficiency_modification: cases executed");
        Ok(())
    })
}

/// One main backpack holding a stack of two Lunar Ascension Orbs.
async fn seed_bag(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for table in ["game_item_container_entries", "game_item_container_slots"] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DELETE FROM {table} WHERE character_id=encode($1,'hex')::uuid"
        )))
        .bind(id(41).as_slice())
        .execute(&mut *tx)
        .await?;
    }
    let item = |seed, key: &str, count| {
        crate::durability::item_fee_burn::test_support::item(seed, key, "definition-r1", count)
    };
    sqlx::raw_sql(sqlx::AssertSqlSafe(item(BAG, "oteryn:item.tibia.i2854", 1)))
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)").bind(id(41).as_slice()).bind(id(BAG).as_slice()).bind(id(42).as_slice()).bind(id(245).as_slice()).execute(&mut *tx).await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(item(ORBS, ORB, 2)))
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($5,'hex')::uuid)").bind(id(41).as_slice()).bind(id(BAG).as_slice()).bind(id(ORBS).as_slice()).bind(id(42).as_slice()).bind(id(246).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

/// Credit the Character `amount` dust (one creature_kill GAIN, the chain's first entry).
/// MODIFY costs 250, above the base limit of 100; a fixture-only chain starts at limit 1000
/// (replica role skips the first-entry trigger, as `prey` seeds its fixtures).
async fn seed_dust(h: &Harness, amount: i64) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for sql in [
        "INSERT INTO game_character_forge_dust (character_id, balance, dust_limit) \
         SELECT character_id, 0, 100 FROM game_character_roots",
        "INSERT INTO game_character_forge_dust_entries (entry_id, character_id, kind, cause, \
            cause_occurrence_id, transaction_id, amount, lost_amount, balance_before, \
            balance_after, dust_limit_before, dust_limit_after) \
         SELECT '01020304-0506-7008-8000-0000000000e1', character_id, 'GAIN', 'creature_kill', \
            '01020304-0506-7008-8000-0000000000e2', '01020304-0506-7008-8000-0000000000e3', \
            $1, 0, 0, $1, 1000, 1000 FROM game_character_roots",
        "UPDATE game_character_forge_dust SET balance = $1, dust_limit = 1000, \
            last_entry_id = '01020304-0506-7008-8000-0000000000e1'",
    ] {
        sqlx::query(sql).bind(amount).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}
