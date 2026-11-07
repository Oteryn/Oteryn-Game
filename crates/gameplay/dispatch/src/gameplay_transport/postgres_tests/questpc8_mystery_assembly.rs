//! Mandatory NPC introduction, native shrine bindings and atomic assembly.
use super::fixture::{Harness, debug, id};
use super::questpc8_authored_programmes::{object_target, use_target};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::foundation::{CommandId, CommandRef};
use crate::map::overlay::TilePos;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::UseDisposition;

const TRACK: &str = "oteryn:quest-progress/chosen/mystery-of-the-valley/stage";
const QUEST: &str = "oteryn:quest.authored.mystery_of_the_valley_quest";

#[test]
fn questpc8_mystery_assembly_consumes_all_three_or_none() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("questpc8-mystery-assembly", || {
        super::quest_world_callbacks::scenario(
            "authored-mystery-assembly",
            Position {
                x: 32571,
                y: 32506,
                floor: -7,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_mystery_assembly_consumes_all_three_or_none duration={:?}",
        started.elapsed()
    );
    Ok(())
}

async fn quantity(h: &Harness, item: u16) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key=$1 AND i.lifecycle=1 AND e.character_id=encode($2,'hex')::uuid")
        .bind(format!("oteryn:item.tibia.i{item}")).bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
async fn stage(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(id(41).as_slice()).bind(TRACK).fetch_one(&h.pool).await?)
}
async fn move_third(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
    pickup: bool,
) -> TestResult {
    use crate::durability::item_loop_move::{LoopDestination, LoopGround, LoopMoveRequest};
    use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
    let item:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(item_instance_id) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i50338' AND lifecycle=1").fetch_one(&h.pool).await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let permit = owner.spell_lane_permit().await.ok_or("lane")?;
    let (digest, frame) = {
        let r = owner.runtime.lock().await;
        (
            r.content_pin().server_artifact_digest(),
            r.content_pin().frame_binding_digest(),
        )
    };
    let cell = crate::durability::map_item_mint_audit::map_item_spatial_position(33081, 32704, -5);
    let destination = if pickup {
        LoopDestination::MainBackpack
    } else {
        LoopDestination::Ground(LoopGround {
            spatial_position: cell.clone(),
            map_revision: crate::map::overlay::map_revision(
                owner.bundle_map().ok_or("map")?.overlay.base(),
            ),
            content_revision: format!(
                "sha256:{}",
                digest
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            ),
            placement_context: frame.to_vec(),
        })
    };
    let result = owner
        .root
        .commit_item_loop_move(
            &permit,
            owner.character,
            owner.holder,
            &fence.item_fence(),
            CommandRef::new(
                admitted.game_session_id,
                CommandId::new(if pickup { 8021 } else { 8020 }).map_err(debug)?,
            ),
            &NativeItemWeights(native),
            LoopMoveRequest {
                item: item.try_into().map_err(|_| "item id")?,
                count: None,
                pickup_cell: pickup.then_some(cell),
                destination,
            },
        )
        .await
        .map_err(debug)?;
    assert!(result.is_ok(), "third-fragment custody move {result:?}");
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    eprintln!(
        "POSTGRES_CASE_STARTED questpc8_mystery_assembly after PostgreSQL fixture initialization"
    );
    // Mandatory introduction and prerequisite events use their existing real
    // receipt owners. No Mystery progress or quest item is inserted by SQL.
    super::questpc8_mystery::exercise_intro(owner, admitted, h, authority).await?;
    for (stage_before, at, fragment) in [
        (
            1,
            TilePos {
                x: 32459,
                y: 31093,
                floor: -6,
            },
            50336,
        ),
        (
            2,
            TilePos {
                x: 33198,
                y: 31184,
                floor: -6,
            },
            50337,
        ),
        (
            3,
            TilePos {
                x: 33834,
                y: 31697,
                floor: -6,
            },
            50338,
        ),
    ] {
        super::questpc8_mystery_geometry::arrive(owner, admitted, at).await?;
        let target = object_target(owner, at, 0)?;
        let command = 1 + u64::try_from(stage_before - 1)? * 2;
        assert_eq!(stage(h).await?, stage_before);
        assert_eq!(
            use_target(owner, admitted, command, &target).await?,
            UseDisposition::Committed,
            "shrine {stage_before}"
        );
        assert_eq!(stage(h).await?, stage_before + 1);
        assert_eq!(quantity(h, fragment).await?, 1);
        assert_eq!(
            use_target(owner, admitted, command, &target).await?,
            UseDisposition::Committed
        );
        let current_target = object_target(owner, at, 0)?;
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        assert_eq!(
            use_target(owner, admitted, command + 1, &current_target).await?,
            UseDisposition::Rejected
        );
        assert_eq!(quantity(h, fragment).await?, 1);
    }
    super::questpc8_mystery_geometry::arrive(
        owner,
        admitted,
        TilePos {
            x: 33081,
            y: 32704,
            floor: -5,
        },
    )
    .await?;
    let target = object_target(
        owner,
        TilePos {
            x: 33080,
            y: 32704,
            floor: -5,
        },
        0,
    )?;
    assert_eq!(
        use_target(owner, admitted, 7, &target).await?,
        UseDisposition::Rejected,
        "exploration is mandatory"
    );
    super::questpc8_authored_programmes::step(
        owner,
        admitted,
        8,
        oteryn_protocol_oteryn::world_spatial::StepDirection::West,
    )
    .await?;
    assert_eq!(stage(h).await?, 5);
    let target = object_target(
        owner,
        TilePos {
            x: 33080,
            y: 32704,
            floor: -5,
        },
        0,
    )?;
    move_third(owner, admitted, h, false).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(
        use_target(owner, admitted, 9, &target).await?,
        UseDisposition::Rejected
    );
    assert_eq!(stage(h).await?, 5);
    assert_eq!(quantity(h, 50336).await?, 1);
    assert_eq!(quantity(h, 50337).await?, 1);
    assert_eq!(quantity(h, 50338).await?, 0);
    assert_eq!(quantity(h, 50335).await?, 0);
    move_third(owner, admitted, h, true).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let premium = owner.premium_coordinator.take();
    assert_eq!(
        use_target(owner, admitted, 10, &target).await?,
        UseDisposition::Rejected
    );
    owner.premium_coordinator = premium;
    assert_eq!(stage(h).await?, 5);
    for item in 50336..=50338 {
        assert_eq!(quantity(h, item).await?, 1);
    }
    assert_eq!(
        use_target(owner, admitted, 11, &target).await?,
        UseDisposition::Committed
    );
    assert_eq!(stage(h).await?, 6);
    for item in 50336..=50338 {
        assert_eq!(quantity(h, item).await?, 0);
    }
    assert_eq!(quantity(h, 50335).await?, 1);
    assert_eq!(
        use_target(owner, admitted, 11, &target).await?,
        UseDisposition::Committed
    );
    owner
        .states
        .quest_sessions
        .lock()
        .map_err(|_| "quest cache")?
        .remove(&admitted.game_session_id);
    owner.admit_quest_session(admitted).await;
    let current_target = object_target(
        owner,
        TilePos {
            x: 33080,
            y: 32704,
            floor: -5,
        },
        0,
    )?;
    assert_eq!(
        use_target(owner, admitted, 12, &current_target).await?,
        UseDisposition::Rejected
    );
    assert_eq!(quantity(h, 50335).await?, 1);
    let complete:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2").bind(id(41).as_slice()).bind(QUEST).fetch_one(&h.pool).await?;
    assert!(complete);
    Ok(())
}
