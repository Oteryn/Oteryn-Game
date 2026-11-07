//! Source exit35011 checks the current alternating four-lever pattern.
use super::{TestResult, fixture::debug};
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_spatial::{
    StepDirection, StepDisposition, decode_step_result, encode_step_intent,
};

pub(super) const TRIGGER: Position = Position {
    x: 32803,
    y: 31586,
    floor: -1,
};
pub(super) fn pattern() -> [(Position, u16); 4] {
    std::array::from_fn(|i| {
        (
            Position {
                x: [32802, 32803, 32804, 32805][i],
                y: 31584,
                floor: -1,
            },
            [2772, 2773, 2772, 2773][i],
        )
    })
}
pub(super) fn mismatch(name: &str) -> Option<usize> {
    if name.ends_with("live") {
        Some(2)
    } else {
        name.rsplit('-').next().and_then(|n| n.parse().ok())
    }
}
pub(super) fn authored() -> TestResult<Callback> {
    let d: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/content-schema/quest-authoring/quests-2/world-callbacks/draconia_quest.json"
    )))?;
    Ok(serde_json::from_value(
        d["callbacks"]
            .as_array()
            .ok_or("callbacks")?
            .iter()
            .find(|c| c["key"] == "oteryn:quest-callback/draconia-exit-35011")
            .ok_or("authored exit")?
            .clone(),
    )?)
}
#[test]
fn draconia_exit_current_pattern_wrong_levers_and_live_recovery() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            for name in [
                "draconia-exit-correct",
                "draconia-exit-wrong-0",
                "draconia-exit-wrong-1",
                "draconia-exit-wrong-2",
                "draconia-exit-wrong-3",
                "draconia-exit-live",
            ] {
                let started = std::time::Instant::now();
                super::quest_world_callbacks::scenario(name, TRIGGER, false)
                    .map_err(|e| e.to_string())?;
                println!(
                    "POSTGRES_CASE_COMPLETED {name} duration={:?}",
                    started.elapsed()
                );
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "Draconia exit scenario panicked")??;
    Ok(())
}

pub(super) fn live_lever(callback: &Callback) -> TestResult<Callback> {
    let mut value = serde_json::to_value(callback)?;
    value
        .as_object_mut()
        .ok_or("callback")?
        .remove("object_conditions");
    value["key"] = "oteryn:quest-callback/fixture/draconia-live-lever".into();
    value["trigger"] = "use".into();
    value["selector"] = serde_json::json!({"position":pattern()[2].0,"appearance":2773});
    value["lever"] = serde_json::json!([2772, 2773]);
    value["reversible"] = true.into();
    value["teleport"] = serde_json::Value::Null;
    value["failure_teleport"] = serde_json::Value::Null;
    value["return_on_failure"] = false.into();
    Ok(serde_json::from_value(value)?)
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &super::fixture::Harness,
    actor: ExactActorRef,
    name: &str,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let before = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    let step = async |admitted: &mut AdmittedSession, id, direction| -> TestResult<Vec<u8>> {
        let reply = super::quest_world_callbacks::wire(
            owner,
            admitted,
            id,
            1,
            encode_step_intent(direction),
        )
        .await?;
        assert_eq!(
            decode_step_result(&reply).map_err(debug)?,
            StepDisposition::Moved
        );
        Ok(reply)
    };
    let first = step(admitted, 1, StepDirection::East).await?;
    let success = Position {
        x: 32701,
        y: 31639,
        floor: -6,
    };
    let failure = Position {
        x: 32803,
        y: 31587,
        floor: -1,
    };
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?;
    assert_eq!(
        at.position(),
        super::quest_world_callbacks::local(if mismatch(name).is_none() {
            success
        } else {
            failure
        })
    );
    assert_eq!(
        super::quest_world_callbacks::wire(
            owner,
            admitted,
            1,
            1,
            encode_step_intent(StepDirection::East)
        )
        .await?,
        first
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?,
        at
    );
    if name.ends_with("live") {
        for (id, dir) in [
            (2, StepDirection::West),
            (3, StepDirection::North),
            (4, StepDirection::North),
            (5, StepDirection::East),
        ] {
            tokio::time::sleep(std::time::Duration::from_millis(210)).await;
            step(admitted, id, dir).await?;
        }
        let map = owner.bundle_map().ok_or("map")?;
        let p = pattern()[2].0;
        let key = oteryn_world_bundle::bundle::placement_key(p.floor, p.x, p.y, 1)
            .ok_or("lever placement")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(
            &oteryn_protocol_oteryn::world_object::WorldObjectTarget {
                placement,
                expected_revision: 0,
            },
        )
        .map_err(debug)?;
        let reply = super::quest_world_callbacks::wire(owner, admitted, 6, 2, payload).await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
            oteryn_protocol_oteryn::world_object::UseDisposition::Committed
        );
        let tile = crate::map::view::compose_tile(
            &map.overlay,
            &map.facts,
            crate::map::overlay::TilePos {
                x: p.x,
                y: p.y,
                floor: p.floor,
            },
        )
        .map_err(debug)?
        .ok_or("lever tile")?;
        assert!(tile.entries.iter().any(|e| e.facts.appearance_id == 2772));
        tokio::time::sleep(std::time::Duration::from_millis(210)).await;
        step(admitted, 7, StepDirection::South).await?;
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            super::quest_world_callbacks::local(success),
            "condition reads changed facts, not the original palette"
        );
    }
    assert_eq!(
        owner
            .root
            .read_character_quest_state(owner.character, fence.character_id)
            .await
            .map_err(debug)?,
        before
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?,
        0
    );
    Ok(())
}
