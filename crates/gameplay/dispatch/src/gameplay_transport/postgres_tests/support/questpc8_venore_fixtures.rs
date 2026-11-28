use super::fixture::{Harness, debug, id};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

use super::questpc8_authored_programmes_fixtures::{object_target, step};

use super::{AdmittedSession, ComposedFreshAdmission, TestResult};

use crate::map::overlay::TilePos;

use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};

use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use oteryn_protocol_oteryn::world_spatial::StepDirection;

use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

pub(super) const PREFIX: &str = "oteryn:quest-progress/chosen/venore-daily/";

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

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, _) = super::death_ml_loss_composed_fixtures::production_generation_with_pin(
        world_id,
        "content-1",
    )?;
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
            (33027, 32139, 33031, 32143, -7),
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
    owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[3, 4, 6, 11, 15, 18],
        )
        .ok_or("NPC capability")?;
    let [norman, scott, frank] = install_npcs(owner, admitted).await?;
    let mut command = 1;
    let foreman = TilePos {
        x: 33016,
        y: 32100,
        floor: -6,
    };
    say(owner, admitted, norman, &mut command, &["hi", "bonus"]).await?;
    say(
        owner,
        admitted,
        norman,
        &mut command,
        &["hi", "pathfinder", "yes"],
    )
    .await?;
    assert_eq!(progress(h, "pathfinder/stage").await?, 1);
    assert_eq!(progress(h, "maintenance/stage").await?, -1);
    assert_eq!(progress(h, "sabotage/stage").await?, -1);
    assert_eq!(quantity(h, 17513).await?, 0);
    assert_eq!(quantity(h, 17514).await?, 1);
    assert_eq!(quantity(h, 17617).await?, 1);

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
        &["hi", "pathfinder kit", "yes"],
    )
    .await?;
    assert_eq!(quantity(h, 17514).await?, 0);
    assert_eq!(quantity(h, 17617).await?, 1);
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
    say(
        owner,
        admitted,
        scott,
        &mut command,
        &["hi", "grind stone", "yes"],
    )
    .await?;
    assert_eq!(quantity(h, 17617).await?, 0);

    arrive_near(owner, admitted, foreman).await?;
    report(owner, admitted, h, norman, &mut command, "pathfinder").await?;
    assert_eq!(progress(h, "maintenance/stage").await?, -1);
    assert_eq!(progress(h, "sabotage/stage").await?, -1);
    say_refused(owner, admitted, norman, &mut command, "maintenance").await?;
    assert_eq!(
        progress(h, "maintenance/stage").await?,
        -1,
        "held rope and shovel required"
    );
    carry(owner, admitted, 3003, 710).await?;
    say_refused(owner, admitted, norman, &mut command, "maintenance").await?;
    assert_eq!(
        progress(h, "maintenance/stage").await?,
        -1,
        "shovel still required"
    );
    carry(owner, admitted, 3457, 711).await?;
    say(
        owner,
        admitted,
        norman,
        &mut command,
        &["hi", "maintenance", "yes"],
    )
    .await?;
    assert_eq!(quantity(h, 17513).await?, 1);
    assert_eq!(progress(h, "sabotage/stage").await?, -1);

    let (x, y, ordinal) = MACHINES[0];
    let first = TilePos { x, y, floor: -6 };
    arrive_in_rings(owner, admitted, first, 3..=3).await?;
    refresh(owner, admitted).await;
    let target = object_target(owner, first, ordinal)?;
    let (result, _) = oil_machine(owner, admitted, command, target).await?;
    assert_eq!(result, UseDisposition::TooFar);
    command += 1;
    assert_eq!(progress(h, "machine-1").await?, 0);
    for (index, (x, y, ordinal)) in MACHINES.into_iter().enumerate().rev() {
        let at = TilePos { x, y, floor: -6 };
        arrive_with_range(owner, admitted, at, 2).await?;
        refresh(owner, admitted).await;
        let target = object_target(owner, at, ordinal)?;
        let (result, packet) = oil_machine(owner, admitted, command, target).await?;
        assert_eq!(result, UseDisposition::Committed);
        assert_eq!(progress(h, &format!("machine-{}", index + 1)).await?, 1);
        let (end, replay, _) =
            super::rust_lottery_fixtures::exchange(owner, *admitted, command, &packet, None, None)
                .await?;
        *admitted = end;
        assert_eq!(replay, UseDisposition::Committed);
        command += 1;
        refresh(owner, admitted).await;
        let target = object_target(owner, at, ordinal)?;
        let (result, _) = oil_machine(owner, admitted, command, target).await?;
        assert_eq!(result, UseDisposition::Rejected);
        command += 1;
        assert_eq!(quantity(h, 17513).await?, 1, "reusable can stays carried");
    }
    arrive_near(owner, admitted, foreman).await?;
    report(owner, admitted, h, norman, &mut command, "maintenance").await?;
    say(
        owner,
        admitted,
        norman,
        &mut command,
        &["hi", "sabotage", "yes"],
    )
    .await?;
    assert_eq!(progress(h, "sabotage/stage").await?, 1);

    super::questpc8_mystery_geometry_fixtures::arrive(
        owner,
        admitted,
        TilePos {
            x: 33029,
            y: 32140,
            floor: -7,
        },
    )
    .await?;
    refresh(owner, admitted).await;
    step(owner, admitted, command, StepDirection::South).await?;
    command += 1;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    assert_eq!(
        (at.x, at.y, at.floor),
        (33029, 32141, 8),
        "Corym hideout entry"
    );
    super::questpc8_mystery_geometry_fixtures::arrive(
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
    assert_eq!(coryms.len(), 5);
    for (index, corym) in coryms.into_iter().enumerate() {
        let before_native_experience = experience(h).await?;
        super::grizzly_npc_fixtures::questpc8_campaign::lethal(
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
            "ordinary death XP"
        );
    }
    arrive_near(owner, admitted, foreman).await?;
    report(owner, admitted, h, norman, &mut command, "sabotage").await?;
    let before_xp = experience(h).await?;
    let before_platinum = quantity(h, 3035).await?;
    let before_gold = quantity(h, 3031).await?;
    say(
        owner,
        admitted,
        norman,
        &mut command,
        &["hi", "bonus", "yes"],
    )
    .await?;
    assert_eq!(experience(h).await? - before_xp, 250);
    assert_eq!(quantity(h, 3035).await? - before_platinum, 2);
    assert_eq!(quantity(h, 3031).await? - before_gold, 50);
    let completed: bool = sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.authored.venore_daily_tasks_quest'")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed);
    owner
        .states
        .quest_sessions
        .lock()
        .map_err(|_| "quest cache")?
        .remove(&admitted.game_session_id);
    owner.admit_quest_session(admitted).await;
    say(owner, admitted, norman, &mut command, &["hi", "bonus"]).await?;
    assert_eq!(
        experience(h).await?,
        before_xp + 250,
        "cold replay cannot repeat bonus"
    );

    let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let catalogue = loaded.catalogue();
    for job in ["maintenance", "pathfinder", "sabotage"] {
        assert_eq!(progress(h, &format!("{job}/stage")).await?, 3);
        let current = progress(h, &format!("{job}/last-report")).await?;
        assert!(current > 0);
        say_refused(owner, admitted, norman, &mut command, job).await?;
        assert_eq!(progress(h, &format!("{job}/stage")).await?, 3);
        assert_eq!(progress(h, &format!("{job}/last-report")).await?, current);
        let restart = catalogue
            .transition(&format!(
                "oteryn:quest-reward/chosen/venore-daily/{job}/restart"
            ))
            .ok_or("job restart")?;
        let values = std::collections::BTreeMap::from([
            (format!("{PREFIX}{job}/stage"), 3),
            (format!("{PREFIX}{job}/last-report"), current),
        ]);
        for (elapsed, expected) in [(71999, false), (72000, true), (72001, true)] {
            assert_eq!(
                catalogue
                    .evaluate(restart, &values, current + elapsed)
                    .is_ok(),
                expected,
                "{job} cooldown at {elapsed}",
            );
        }
    }
    for item in [3003, 3457, 17513] {
        assert_eq!(quantity(h, item).await?, 1);
    }
    let timers: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_timed_states t JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key IN('oteryn:item.tibia.i17513','oteryn:item.tibia.i17514','oteryn:item.tibia.i17617')")
        .fetch_one(&h.pool).await?;
    assert_eq!(timers, 0);
    Ok(())
}

async fn oil_machine(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command: u64,
    target: WorldObjectTarget,
) -> TestResult<(UseDisposition, Vec<u8>)> {
    let reference = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", "oteryn:item.tibia.i17513", "definition-r1"))
        .ok_or("can of oil")?;
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("handle")?),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target)),
        use_with_position: None,
    })
    .map_err(debug)?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let (end, result, wire) = super::rust_lottery_fixtures::exchange(
        owner,
        *admitted,
        command,
        &payload,
        Some(reference),
        None,
    )
    .await?;
    *admitted = end;
    Ok((result, wire))
}

async fn report(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    norman: EntityRef,
    command: &mut u64,
    job: &str,
) -> TestResult {
    let before = (
        experience(h).await?,
        quantity(h, 3035).await?,
        quantity(h, 3031).await?,
    );
    say(owner, admitted, norman, command, &["hi", job]).await?;
    assert_eq!(progress(h, &format!("{job}/stage")).await?, 2);
    assert_eq!(experience(h).await?, before.0 + 250);
    assert_eq!(quantity(h, 3035).await?, before.1 + 2);
    assert_eq!(quantity(h, 3031).await?, before.2 + 50);
    let transition = format!("oteryn:quest-reward/chosen/venore-daily/{job}/report");
    let (actual, derived, next): (String, String, String) = sqlx::query_as(
        "SELECT m.claim_production_key,game_quest_reward_mint_claim_key(s.character_id,s.cause_id,s.cause_ordinal,s.transition_key,s.plan),game_quest_reward_mint_claim_key(s.character_id,s.cause_id,s.cause_ordinal+1,s.transition_key,s.plan) FROM game_quest_reward_sources s JOIN game_reward_claim_mint_receipts m ON m.transaction_id=s.mint_transaction_id WHERE s.character_id=encode($1,'hex')::uuid AND s.transition_key=$2",
    ).bind(id(41).as_slice()).bind(&transition).fetch_one(&h.pool).await?;
    assert_eq!(actual, derived);
    assert_ne!(actual, transition);
    assert_ne!(actual, next);
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
            return super::questpc8_mystery_geometry_fixtures::arrive(
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
    Err(format!(
        "no native landing at {at:?} in the requested range, nearby walkable cells: {nearby:?}"
    )
    .into())
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
    super::quest_world_callbacks_fixtures::input_item(
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
    let confirmation = command.checked_add(2).ok_or("command")?;
    super::grizzly_npc_fixtures::wire::talk_refused(
        owner,
        admitted,
        npc,
        &[
            (*command, "hi"),
            (*command + 1, text),
            (confirmation, "yes"),
        ],
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
    super::grizzly_npc_fixtures::wire::talk(owner, admitted, npc, &commands).await?;
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
