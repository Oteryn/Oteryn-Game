//! Exact shared threshold gates after real physical switch USE commands.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
pub(super) fn spec(unique: u16) -> TestResult<(u16, u16, Position)> {
    Ok(match unique {
        50135 => (
            12121,
            8,
            Position {
                x: 33367,
                y: 32805,
                floor: -14,
            },
        ),
        50137 => (
            12122,
            7,
            Position {
                x: 33083,
                y: 32568,
                floor: -14,
            },
        ),
        50138 => (
            12123,
            5,
            Position {
                x: 33198,
                y: 32885,
                floor: -11,
            },
        ),
        _ => return Err("unknown source gate".into()),
    })
}
#[test]
fn l6_shared_gate_closed_50135_uses_exact_threshold() -> TestResult {
    run("l6-tomb-gate-closed-50135")
}
#[test]
fn l6_shared_gate_open_50135_uses_exact_threshold() -> TestResult {
    run("l6-tomb-gate-open-50135")
}
#[test]
fn l6_shared_gate_closed_50137_uses_exact_threshold() -> TestResult {
    run("l6-tomb-gate-closed-50137")
}
#[test]
fn l6_shared_gate_open_50137_uses_exact_threshold() -> TestResult {
    run("l6-tomb-gate-open-50137")
}
#[test]
fn l6_shared_gate_closed_50138_uses_exact_threshold() -> TestResult {
    run("l6-tomb-gate-closed-50138")
}
#[test]
fn l6_shared_gate_open_50138_uses_exact_threshold() -> TestResult {
    run("l6-tomb-gate-open-50138")
}
fn run(name: &'static str) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    eprintln!("POSTGRES_CASE_STARTED {name}");
    let result = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            super::quest_world_callbacks::scenario(
                name,
                Position {
                    x: 7,
                    y: 1,
                    floor: -7,
                },
                false,
            )
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "shared gate fixture panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
async fn step(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    ordinal: &mut u64,
    direction: StepDirection,
) -> TestResult {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(1))
        .ok_or("pace gate STEP")?;
    let reply = super::quest_world_callbacks::wire(
        owner,
        admitted,
        *ordinal,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(direction),
    )
    .await?;
    *ordinal += 1;
    assert_eq!(
        decode_step_result(&reply).map_err(debug)?,
        StepDisposition::Moved
    );
    Ok(())
}
#[allow(clippy::too_many_arguments)]
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    actor: crate::foundation::ExactActorRef,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    trigger: Position,
    name: &str,
) -> TestResult {
    let unique = name.rsplit('-').next().ok_or("gate UID")?.parse()?;
    let (_, required, success) = spec(unique)?;
    let open = name.contains("-open-");
    let count = if open { required } else { required - 1 };
    let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
        .fetch_one(&h.pool)
        .await?;
    eprintln!("POSTGRES_DATABASE_READY l6_shared_gate version={version}");
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            actor,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let quests_before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?;
    let mut ordinal = 1;
    step(owner, admitted, &mut ordinal, StepDirection::North).await?;
    for _ in 0..2 {
        step(owner, admitted, &mut ordinal, StepDirection::East).await?;
    }
    step(owner, admitted, &mut ordinal, StepDirection::South).await?;
    for n in 0..count {
        if n > 0 {
            for _ in 0..2 {
                step(owner, admitted, &mut ordinal, StepDirection::East).await?;
            }
        }
        let key = oteryn_world_bundle::bundle::placement_key(
            trigger.floor,
            trigger.x + 1 + 2 * n,
            trigger.y + 1,
            1,
        )
        .ok_or("source lever")?;
        let mut placement = owner
            .bundle_map()
            .ok_or("gate room")?
            .overlay
            .base()
            .digest()
            .to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(1))
            .ok_or("pace gate USE")?;
        let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
            placement,
            expected_revision: 0,
        })
        .map_err(debug)?;
        let reply =
            super::quest_world_callbacks::wire(owner, admitted, ordinal, 2, payload).await?;
        ordinal += 1;
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
            UseDisposition::Committed,
            "every shared contribution comes from real exact USE"
        );
    }
    step(owner, admitted, &mut ordinal, StepDirection::North).await?;
    for _ in 0..(2 + 2 * (count - 1)) {
        step(owner, admitted, &mut ordinal, StepDirection::West).await?;
    }
    step(owner, admitted, &mut ordinal, StepDirection::South).await?;
    let gate_command = ordinal;
    step(owner, admitted, &mut ordinal, StepDirection::East).await?;
    let expected = if open {
        success
    } else {
        Position {
            x: trigger.x - 1,
            ..trigger
        }
    };
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    assert_eq!(
        at,
        super::quest_world_callbacks::local(expected),
        "source threshold {required} with exactly {count} physical contributions"
    );
    let replay = super::quest_world_callbacks::wire(
        owner,
        admitted,
        gate_command,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(
        decode_step_result(&replay).map_err(debug)?,
        StepDisposition::Moved
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        at,
        "original STEP replay cannot move again"
    );
    let effects = owner
        .bundle_map()
        .ok_or("source gate map")?
        .facts
        .movement_effects_since(0);
    assert_eq!(
        effects.len(),
        1,
        "one source teleport cue; no cue duplication on STEP replay"
    );
    assert_eq!(
        (effects[0].1, effects[0].2),
        (
            crate::map::overlay::TilePos {
                x: expected.x,
                y: expected.y,
                floor: expected.floor
            },
            11
        ),
        "source cue at success destination or restored origin"
    );
    let leases: i64 = sqlx::query_scalar("SELECT count(*) FROM game_shared_switch_leases")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(leases, i64::from(count));
    let quests_after: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(
        quests_before, quests_after,
        "counter read/teleport is not personal Quest progress"
    );
    Ok(())
}
