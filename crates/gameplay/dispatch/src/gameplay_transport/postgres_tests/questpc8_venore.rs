//! Native daily construction circuit: held tools, exact machines, deliveries and captured deaths.
use super::fixture::{Harness, debug, id};
use super::questpc8_authored_programmes::{object_target, use_target};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::overlay::TilePos;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const PREFIX: &str = "oteryn:quest-progress/chosen/venore-daily/";
const MACHINES: [(u16, u16, u8); 9] = [
    (33015, 32022, 1),
    (33016, 32025, 2),
    (33017, 32041, 1),
    (33030, 32099, 2),
    (33016, 32097, 1),
    (33013, 32095, 2),
    (33016, 32142, 2),
    (33015, 32138, 1),
    (33022, 32147, 3),
];
#[test]
fn questpc8_venore_native_daily_circuit_and_exact_repeat_boundary() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("questpc8-venore", || {
        super::quest_world_callbacks::scenario(
            "authored-venore",
            Position {
                x: 33016,
                y: 32100,
                floor: -6,
            },
            false,
        )
    });
    eprintln!(
        "POSTGRES_CASE_FINISHED questpc8_venore_native_daily_circuit duration={:?} result={result:?}",
        started.elapsed()
    );
    result
}
pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) =
        super::death_ml_loss_composed::production_generation_with_pin(world_id, "content-1")?;
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    super::questpc8_native_slice::world(
        native,
        &[
            (33010, 32018, 33034, 32151, -6),
            (33035, 31997, 33043, 32005, -7),
            (33042, 32133, 33050, 32142, -7),
            (33005, 32130, 33035, 32160, -8),
        ],
        &[],
        TilePos {
            x: 33015,
            y: 32100,
            floor: -6,
        },
    )
}
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    eprintln!("POSTGRES_CASE_STARTED questpc8_venore after PostgreSQL fixture initialization");
    owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[3],
        )
        .ok_or("NPC capability")?;
    let [norman, scott, frank] = install_npcs(owner, admitted).await?;
    let chamber = (33005..=33035)
        .flat_map(|x| (32130..=32160).map(move |y| (x, y)))
        .filter(|(x, y)| {
            owner.movement_cells().index().lookup(
                owner.movement_cells().scope(),
                crate::content::LogicalCell { x: *x, y: *y, z: 8 },
            ) == Ok(crate::content::CollisionClass::Walkable)
        })
        .take(8)
        .collect::<Vec<_>>();
    eprintln!("VENORE_NATIVE_CHAMBER_WALKABLE {chamber:?}");
    let mut command = 1;
    say_refused(owner, admitted, norman, &mut command, "construction jobs").await?;
    assert_eq!(
        progress(h, "stage").await?,
        -1,
        "held rope and shovel required"
    );
    carry(owner, admitted, 3003, 710).await?;
    say_refused(owner, admitted, norman, &mut command, "construction jobs").await?;
    assert_eq!(progress(h, "stage").await?, -1, "shovel still required");
    carry(owner, admitted, 3457, 711).await?;
    say(
        owner,
        admitted,
        norman,
        &mut command,
        &["hi", "construction jobs"],
    )
    .await?;
    assert_eq!(progress(h, "stage").await?, 1);
    for item in [17513, 17514, 17617] {
        assert_eq!(quantity(h, item).await?, 1);
    }
    let first = TilePos {
        x: 33015,
        y: 32022,
        floor: -6,
    };
    arrive_in_rings(owner, admitted, first, 3..=3).await?;
    refresh(owner, admitted).await;
    let far_target = object_target(owner, first, 1)?;
    assert_eq!(
        use_target(owner, admitted, command, &far_target).await?,
        UseDisposition::TooFar,
        "declared large-machine reach remains bounded at two cells"
    );
    command += 1;
    assert_eq!(progress(h, "stage").await?, 1);
    assert_eq!(
        quantity(h, 17513).await?,
        1,
        "out-of-reach use cannot consume the tool"
    );
    arrive_near(
        owner,
        admitted,
        TilePos {
            x: 33016,
            y: 32025,
            floor: -6,
        },
    )
    .await?;
    refresh(owner, admitted).await;
    let wrong = object_target(
        owner,
        TilePos {
            x: 33016,
            y: 32025,
            floor: -6,
        },
        2,
    )?;
    assert_eq!(
        use_target(owner, admitted, command, &wrong).await?,
        UseDisposition::Rejected,
        "an out-of-order native machine cannot progress or charge the tool"
    );
    command += 1;
    assert_eq!(progress(h, "stage").await?, 1);
    assert_eq!(quantity(h, 17513).await?, 1);
    for (index, (x, y, ordinal)) in MACHINES.into_iter().enumerate() {
        let at = TilePos { x, y, floor: -6 };
        arrive_with_range(owner, admitted, at, 2).await?;
        refresh(owner, admitted).await;
        let target = object_target(owner, at, ordinal)?;
        assert_eq!(
            use_target(owner, admitted, command, &target).await?,
            UseDisposition::Committed,
            "machine {index}"
        );
        assert_eq!(progress(h, "stage").await?, i64::try_from(index)? + 2);
        assert_eq!(
            use_target(owner, admitted, command, &target).await?,
            UseDisposition::Committed,
            "exact replay"
        );
        command += 1;
        refresh(owner, admitted).await;
        let target = object_target(owner, at, ordinal)?;
        assert_eq!(
            use_target(owner, admitted, command, &target).await?,
            UseDisposition::Rejected,
            "fresh duplicate cannot oil another machine"
        );
        command += 1;
        assert_eq!(
            quantity(h, 17513).await?,
            1,
            "reusable can is never charged or consumed"
        );
    }
    arrive_near(
        owner,
        admitted,
        TilePos {
            x: 33039,
            y: 32001,
            floor: -7,
        },
    )
    .await?;
    say(owner, admitted, scott, &mut command, &["hi", "grind stone"]).await?;
    assert_eq!(progress(h, "stage").await?, 11);
    assert_eq!(quantity(h, 17617).await?, 0);
    arrive_near(
        owner,
        admitted,
        TilePos {
            x: 33046,
            y: 32137,
            floor: -7,
        },
    )
    .await?;
    say(
        owner,
        admitted,
        frank,
        &mut command,
        &["hi", "pathfinder kit"],
    )
    .await?;
    assert_eq!(progress(h, "stage").await?, 12);
    assert_eq!(quantity(h, 17514).await?, 0);
    super::questpc8_mystery_geometry::arrive(
        owner,
        admitted,
        TilePos {
            x: 33019,
            y: 32146,
            floor: -8,
        },
    )
    .await?;
    refresh(owner, admitted).await;
    let coryms = owner
        .quest_guardians_for_test("oteryn:quest-death/authored-venore-coryms")
        .await;
    assert_eq!(coryms.len(), 5, "five native coryms beneath southern site");
    for (index, corym) in coryms.into_iter().enumerate() {
        let before_native_experience = experience(h).await?;
        super::grizzly_npc::questpc8_campaign::lethal(
            owner,
            admitted,
            corym,
            "oteryn:creature.little_corym_charlatan",
            command,
            u16::try_from(index + 1)?,
        )
        .await?;
        assert_eq!(progress(h, "kills").await?, i64::try_from(index + 1)?);
        assert!(
            experience(h).await? > before_native_experience,
            "each native corym death must also commit its ordinary experience; corpse admission cannot be skipped"
        );
    }
    arrive_near(
        owner,
        admitted,
        TilePos {
            x: 33016,
            y: 32100,
            floor: -6,
        },
    )
    .await?;
    assert_eq!(progress(h, "stage").await?, 12, "native corym deaths preserve the completed circuit stage");
    let before_xp = experience(h).await?;
    let gold = quantity(h, 3035).await?;
    say(
        owner,
        admitted,
        norman,
        &mut command,
        &["hi", "construction report"],
    )
    .await?;
    assert_eq!(progress(h, "stage").await?, 13);
    assert_eq!(experience(h).await? - before_xp, 1000);
    let (actual_key,derived_key,next_nonce_key,callback_key):(String,String,String,String)=sqlx::query_as("SELECT m.claim_production_key,game_quest_reward_mint_claim_key(s.character_id,s.cause_id,s.cause_ordinal,s.transition_key,s.plan),game_quest_reward_mint_claim_key(s.character_id,s.cause_id,s.cause_ordinal+1,s.transition_key,s.plan),s.transition_key FROM game_quest_reward_sources s JOIN game_reward_claim_mint_receipts m ON m.transaction_id=s.mint_transaction_id WHERE s.character_id=encode($1,'hex')::uuid AND s.transition_key='oteryn:quest-reward/chosen/venore-daily/report'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        actual_key, derived_key,
        "native MINT uses the shared authoritative claim-key owner"
    );
    assert_ne!(
        actual_key, callback_key,
        "daily report must not reuse a Character-wide once claim"
    );
    assert_ne!(
        actual_key, next_nonce_key,
        "fresh valid occurrences need distinct bounded command nonces"
    );

    assert_eq!(
        quantity(h, 3035).await? - gold,
        10,
        "1000gold in native platinum denomination"
    );
    let report_stamp = progress(h, "last-report").await?;
    let report_receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_sources WHERE character_id=encode($1,'hex')::uuid AND transition_key='oteryn:quest-reward/chosen/venore-daily/report'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    let quest_revision:String=sqlx::query_scalar("SELECT committed_character_revision::text FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.authored.venore_daily_tasks_quest'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    // The exact exchange keyword stays owned after completion. The locked writer
    // rejects it and produces no new quest/value receipt or state mutation.
    say_refused(owner, admitted, norman, &mut command, "construction report").await?;
    say_refused(owner, admitted, norman, &mut command, "construction jobs").await?;
    assert_eq!(
        progress(h, "stage").await?,
        13,
        "completed conversation and immediate restart cannot reopen the circuit"
    );
    assert_eq!(quantity(h, 3035).await?, gold + 10);
    owner
        .states
        .quest_sessions
        .lock()
        .map_err(|_| "quest cache")?
        .remove(&admitted.game_session_id);
    owner.admit_quest_session(admitted).await;
    say_refused(owner, admitted, norman, &mut command, "construction jobs").await?;
    assert_eq!(
        progress(h, "stage").await?,
        13,
        "cold reload retains cooldown"
    );
    assert_eq!(
        progress(h, "last-report").await?,
        report_stamp,
        "fresh/cold attempts preserve the cooldown stamp"
    );
    let after_receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_sources WHERE character_id=encode($1,'hex')::uuid AND transition_key='oteryn:quest-reward/chosen/venore-daily/report'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        after_receipts, report_receipts,
        "completed conversation cannot create a value occurrence"
    );
    let after_revision:String=sqlx::query_scalar("SELECT committed_character_revision::text FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.authored.venore_daily_tasks_quest'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        after_revision, quest_revision,
        "no quest transition on repeated/cold conversation"
    );
    assert_eq!(
        experience(h).await?,
        before_xp + 1000,
        "fresh and cold replay never repeat experience"
    );
    for item in [3003, 3457, 17513] {
        assert_eq!(quantity(h, item).await?, 1, "tools remain carried");
    }
    let completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.authored.venore_daily_tasks_quest'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed, "daily report records completion");
    let timers:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_timed_states t JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key IN('oteryn:item.tibia.i17513','oteryn:item.tibia.i17514','oteryn:item.tibia.i17617')").fetch_one(&h.pool).await?;
    assert_eq!(timers, 0, "no fabricated oil charges or supply timers");
    let accept = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .find(|r| r.key.ends_with("/venore-daily/accept"))
        .ok_or("accept")?;
    let mut occurrence = serde_json::to_value(accept)?;
    occurrence["per_occurrence_mint"] = true.into();
    let effect_keys = vec![format!("{PREFIX}stage"), format!("{PREFIX}kills")];
    let before = vec![-1_i64, -1];
    let after = vec![1_i64, 0];
    let valid: bool =
        sqlx::query_scalar("SELECT game_quest_reward_occurrence_effects_valid($1,$2,$3,$4)")
            .bind(&occurrence)
            .bind(&effect_keys)
            .bind(&before)
            .bind(&after)
            .fetch_one(&h.pool)
            .await?;
    assert!(
        valid,
        "bounded portable occurrence has exact changed effects"
    );
    let mut noop = occurrence.clone();
    for storage in noop["storage"].as_array_mut().ok_or("storage")? {
        storage["preserve"] = true.into();
    }
    let ordinary: bool = sqlx::query_scalar("SELECT game_quest_reward_effects_valid($1,$2,$3,$3)")
        .bind(&noop)
        .bind(&effect_keys)
        .bind(&before)
        .fetch_one(&h.pool)
        .await?;
    assert!(
        ordinary,
        "no-op fixture remains otherwise valid legacy effects"
    );
    let repeated_noop: bool =
        sqlx::query_scalar("SELECT game_quest_reward_occurrence_effects_valid($1,$2,$3,$3)")
            .bind(&noop)
            .bind(&effect_keys)
            .bind(&before)
            .fetch_one(&h.pool)
            .await?;
    assert!(
        !repeated_noop,
        "an occurrence cannot mint from unchanged quest state"
    );
    for field in ["give", "per_occurrence_mint"] {
        let mut invalid = occurrence.clone();
        invalid[field] = serde_json::Value::Null;
        let valid: bool =
            sqlx::query_scalar("SELECT game_quest_reward_occurrence_effects_valid($1,$2,$3,$4)")
                .bind(&invalid)
                .bind(&effect_keys)
                .bind(&before)
                .bind(&after)
                .fetch_one(&h.pool)
                .await?;
        assert!(!valid, "occurrence rejects invalid {field}");
    }
    eprintln!("POSTGRES_OCCURRENCE_CHANGED_STATE_AND_SHAPE_GUARDS_PASS");
    let callback = oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .find(|r| r.key.ends_with("/venore-daily/restart"))
        .ok_or("restart callback")?;
    let plan = serde_json::to_value(&callback)?;
    let stamp: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM transaction_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let keys = vec![
        format!("{PREFIX}stage"),
        format!("{PREFIX}kills"),
        format!("{PREFIX}last-report"),
    ];
    // Use the exact production plan and authoritative SQL clock, without seeding any durable quest state.
    for (elapsed, expected) in [(71999_i64, false), (72000, true), (72001, true)] {
        let valid:bool=sqlx::query_scalar("SELECT game_quest_reward_effects_valid($1,$2,ARRAY[13::bigint,5::bigint,floor(extract(epoch FROM transaction_timestamp()))::bigint-$3],ARRAY[1::bigint,0::bigint,floor(extract(epoch FROM transaction_timestamp()))::bigint-$3])").bind(&plan).bind(&keys).bind(elapsed).fetch_one(&h.pool).await?;
        assert_eq!(
            valid, expected,
            "exact20-hour boundary at {elapsed}, observed stamp {stamp}"
        );
    }
    Ok(())
}
async fn progress(h: &Harness, suffix: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),-1)").bind(id(41).as_slice()).bind(format!("{PREFIX}{suffix}")).fetch_one(&h.pool).await?)
}
async fn quantity(h: &Harness, item: u16) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key=$1 AND i.lifecycle=1 AND e.character_id=encode($2,'hex')::uuid").bind(format!("oteryn:item.tibia.i{item}")).bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
async fn experience(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
async fn arrive_near(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    at: TilePos,
) -> TestResult {
    arrive_with_range(owner, admitted, at, 1).await
}
async fn arrive_with_range(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    at: TilePos,
    range: i32,
) -> TestResult {
    arrive_in_rings(owner, admitted, at, 1..=range).await
}
async fn arrive_in_rings(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    at: TilePos,
    rings: std::ops::RangeInclusive<i32>,
) -> TestResult {
    use crate::content::{CollisionClass, LogicalCell};
    let cells = &owner.movement_cells;
    let offsets = rings.flat_map(|radius| {
        (-radius..=radius).flat_map(move |dx| {
            (-radius..=radius)
                .filter_map(move |dy| (dx.abs().max(dy.abs()) == radius).then_some((dx, dy)))
        })
    });
    for (dx, dy) in offsets {
        let x = u16::try_from(i32::from(at.x) + dx)?;
        let y = u16::try_from(i32::from(at.y) + dy)?;
        if cells.index().lookup(
            cells.scope(),
            LogicalCell {
                x: i32::from(x),
                y: i32::from(y),
                z: i32::from(-at.floor),
            },
        ) == Ok(CollisionClass::Walkable)
        {
            return super::questpc8_mystery_geometry::arrive(
                owner,
                admitted,
                TilePos {
                    x,
                    y,
                    floor: at.floor,
                },
            )
            .await;
        }
    }
    let nearby = (-3..=3)
        .flat_map(|dx| (-3..=3).map(move |dy| (dx, dy)))
        .filter_map(|(dx, dy)| {
            let x = u16::try_from(i32::from(at.x) + dx).ok()?;
            let y = u16::try_from(i32::from(at.y) + dy).ok()?;
            (cells.index().lookup(
                cells.scope(),
                LogicalCell {
                    x: i32::from(x),
                    y: i32::from(y),
                    z: i32::from(-at.floor),
                },
            ) == Ok(CollisionClass::Walkable))
            .then_some((x, y))
        })
        .collect::<Vec<_>>();
    eprintln!("VENORE_NATIVE_LANDING_NEAR {at:?} walkable={nearby:?}");
    Err(format!("no native adjacent landing at {at:?}").into())
}
async fn carry(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: u16,
    occurrence: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: Position {
                x: 33015,
                y: 32100,
                floor: -6,
            },
            item: format!("oteryn:item.tibia.i{item}"),
            count: 1,
        },
        occurrence,
        true,
    )
    .await
}

async fn say_refused(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    command: &mut u64,
    text: &str,
) -> TestResult {
    refresh(owner, admitted).await;
    let confirmation = command.checked_add(1).ok_or("command")?;
    super::grizzly_npc::wire::talk_refused(
        owner,
        admitted,
        npc,
        &[(*command, "hi"), (confirmation, text)],
    )
    .await?;
    *command = confirmation.checked_add(1).ok_or("command")?;
    Ok(())
}

async fn say(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    command: &mut u64,
    text: &[&str],
) -> TestResult {
    refresh(owner, admitted).await;
    let commands = text
        .iter()
        .enumerate()
        .map(|(offset, text)| {
            Ok((
                command
                    .checked_add(u64::try_from(offset)?)
                    .ok_or("command")?,
                *text,
            ))
        })
        .collect::<TestResult<Vec<_>>>()?;
    super::grizzly_npc::wire::talk(owner, admitted, npc, &commands).await?;
    *command = command
        .checked_add(u64::try_from(text.len())?)
        .ok_or("command")?;
    Ok(())
}

// The live owner cadence reloads scheduled quest state. Explicitly perform that same
// durable read between fixture commands so cross-owner selection never reads an old copy.
async fn refresh(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &AdmittedSession) {
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
}

async fn install_npcs(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<[EntityRef; 3]> {
    use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
    use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
    use crate::gameplay_transport::npc_talk::NpcTalkBook;
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    let keys = [
        "oteryn:npc.norman_the_foreman",
        "oteryn:npc.scott_the_scout",
        "oteryn:npc.frank_the_plank",
    ];
    owner.states.chat.install_npc_talk(std::sync::Arc::new(
        NpcTalkBook::from_model_with_native_quests(&model, &keys),
    ));
    owner.states.chat.admit(
        admitted.game_session_id,
        "Construction worker",
        owner.owner_now(),
    );
    owner.states.chat.set_npc_service(admitted.game_session_id);
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut runtime = owner.runtime.lock().await;
    runtime
        .bind_attacker_lease(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let mut npcs = keys
        .into_iter()
        .zip([(33016, 32100, -6), (33039, 32001, -7), (33046, 32137, -7)])
        .map(|(key, (x, y, floor))| NpcPlacements {
            key: key.into(),
            placements: vec![NpcPlacement {
                x,
                y,
                floor,
                direction: MovementFacing::North,
            }],
        })
        .collect::<Vec<_>>();
    npcs.sort_by(|a, b| a.key.cmp(&b.key));
    runtime
        .place_npcs(&NpcPlacementTable { npcs }, |_| true, true)
        .map_err(debug)?;
    let refs = keys
        .into_iter()
        .map(|key| {
            runtime
                .visible_entities()
                .npcs
                .into_iter()
                .find_map(|npc| {
                    (runtime.npc_key(npc.actor) == Some(key)).then_some(EntityRef {
                        identity: npc.actor.placement_identity(),
                        generation: npc.generation,
                    })
                })
                .ok_or_else(|| "source NPC".into())
        })
        .collect::<TestResult<Vec<_>>>()?;
    refs.try_into().map_err(|_| "exact NPC refs".into())
}
