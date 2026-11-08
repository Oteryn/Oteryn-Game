//! A source refusal must return the STEP origin and emit exactly one message.
//!
//! Crystal 201189f871d23597b82caf88fd611454a2ddefb8:
//! data-global/scripts/quests/lions_rock/movements_lions_rock.lua:31-51.
//! The existing synthetic map exercises the real callback owner; this test does
//! not claim to qualify production map collision. No quest state is seeded.
use super::*;
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

const ENTRY: &str = "oteryn:quest-callback/lions-rock-inner-sanctum-entry";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u10_70/lions_rock/questline";
const REFUSAL: &str = "You have to pass the Lion's Tests to enter the inner sanctum!";

#[test]
fn lions_rock_postgres_entry_refusal_has_one_message_and_no_state_change() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::super::kill_reward_e2e::scenario_thread::run("lions-rock-entry-refusal", || {
        super::scenario(
            "lions-rock-inner-sanctum-entry",
            Position {
                x: 33128,
                y: 32308,
                floor: -8,
            },
            false,
        )
    })
}

/// Called by the shared scenario after the normal quest-session admission.
#[allow(clippy::too_many_arguments)]
pub(super) async fn exercise(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    fence: CurrentCharacterGameplayFence,
    callback: &Callback,
    origin: MovementLocalPosition,
) -> TestResult {
    assert_eq!(callback.key, ENTRY);
    assert_eq!(callback.failure_message.as_deref(), Some(REFUSAL));
    assert!(callback.return_on_failure);
    let initial = owner
        .quest_catalogue
        .as_ref()
        .and_then(|catalogue| catalogue.track(QUESTLINE))
        .ok_or("Lions Questline track")?
        .initial;
    assert_eq!(initial, 0);
    let before = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(
        before.tracks().get(QUESTLINE).copied().unwrap_or(initial),
        0
    );
    assert_eq!(counts(h).await?, (0, 0));
    let actor = admitted.runtime_actor.ok_or("Lions actor")?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        origin
    );
    owner
        .states
        .chat
        .admit(admitted.game_session_id, "Lion", owner.owner_now());
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;

    let reply = wire(
        owner,
        admitted,
        1,
        1,
        oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::East),
    )
    .await?;
    assert_eq!(
        oteryn_protocol_oteryn::world_spatial::decode_step_result(&reply).map_err(debug)?,
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
        origin,
        "source refusal must return fromPosition"
    );
    let after = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(after.tracks(), before.tracks());
    assert_eq!(counts(h).await?, (0, 0));
    let messages = owner.states.chat.drain(admitted.game_session_id);
    assert_eq!(
        messages
            .iter()
            .filter(|line| matches!(
                line,
                oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. }
                    if text == REFUSAL
            ))
            .count(),
        1,
        "one refused source STEP must emit one refusal, not one per internal owner"
    );
    eprintln!(
        "LIONS_ROCK POSTGRES_CASE_COMPLETED source_entry_refusal single_feedback origin_restored state_unchanged no_receipt no_consumption"
    );
    Ok(())
}

async fn counts(h: &Harness) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_character_quest_receipts), \
         (SELECT count(*) FROM game_quest_callback_consumptions)",
    )
    .fetch_one(&h.pool)
    .await?)
}
