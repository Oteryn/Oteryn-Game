//! M47: literal USE of the native barrel reaches the durable random-outcome owner.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::quest_state::{
    CommittedQuestTransition, QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
};
use crate::foundation::{CommandId, CommandRef};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, barbarian_mead as mead,
    world_callbacks::{Callback, Position},
};
use oteryn_protocol_oteryn::{
    chat::ChatLine,
    world_object::{UseDisposition, WorldObjectTarget},
};
use std::sync::Arc;

const AT: Position = Position {
    x: 32201,
    y: 31154,
    floor: -7,
};

#[test]
fn m47_native_use_commits_random_outcome_feedback_blackout_and_replays() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("m47-native-mead", || {
        super::quest_world_callbacks::scenario("barbarian-test-mead-native", AT, false)
    })
}

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_crop};
    native_crop(
        native,
        NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z07-x125-y121.b3"
            )),
            region: (7, 125, 121),
            bounds: (32199, 31152, 32203, 31156),
            start: (32202, 31154),
            expected: vec![ExpectedTile {
                position: (32201, 31154),
                ids: vec![408, 2321, 7142],
                counted_entry: None,
            }],
        },
    )
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    fence: CurrentCharacterGameplayFence,
    callback: &Callback,
) -> TestResult {
    let base = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    // Arrange Sven's external accepted state with one sip left and the winning streak at 8.
    let catalogue = Arc::new(
        QuestStateCatalogue::new(
            base.content_revision(),
            mead::NATIVE_TRACKS
                .into_iter()
                .map(|key| base.track(key).cloned().ok_or("track"))
                .collect::<Result<Vec<_>, _>>()?,
            vec![base.transition(mead::KEY).ok_or("transition")?.clone()],
        )
        .map_err(debug)?,
    );
    owner.quest_catalogue = Some(catalogue.clone());
    let prerequisites = mead::TRACKS
        .into_iter()
        .zip([2, 2, 8, 20])
        .map(|(key, value)| (key.to_owned(), value))
        .collect::<Vec<_>>();
    super::qengine_outfit_stories::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &prerequisites,
        125,
    )
    .await?;
    let key =
        oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, 2).ok_or("barrel key")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    owner.states.chat.admit(session, "Mead", owner.owner_now());
    let mut failed = false;
    let mut won = false;
    // Randomness stays server-owned. Exercise both branches with a bounded number of real uses.
    for attempt in 0..128 {
        if attempt > 0 {
            super::qm47_mead::seed_sven_state(h, owner, admitted, [2, 2, 8, 20]).await?;
        }
        let command = attempt + 1;
        let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
            placement: placement.clone(),
            expected_revision: owner.bundle_map().ok_or("map")?.facts.object_revision(key),
        })
        .map_err(debug)?;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let result =
            super::quest_world_callbacks::wire(owner, admitted, command, 2, payload.clone())
                .await?;
        let feedback = owner.states.chat.drain(session);
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
            UseDisposition::Committed
        );
        let permit = owner.spell_lane_permit().await.ok_or("permit")?;
        let context = context(owner, admitted).await?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let saved = slot
            .commit_world_callback(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: mead::KEY.into(),
                    cause: QuestCause::Use(CommandRef::new(
                        session,
                        CommandId::new(command).map_err(debug)?,
                    )),
                },
                catalogue.clone(),
                &permit,
                context,
                None,
            )
            .await?;
        let QuestTransitionOutcome::AlreadyCommitted(receipt) = saved else {
            return Err(format!("native USE did not create receipt: {saved:?}").into());
        };
        drop(slot);
        drop(permit);
        let draw = receipt
            .changes
            .iter()
            .find(|c| c.track == mead::DRAW)
            .ok_or("draw")?
            .after;
        assert!((1..=5).contains(&draw));
        let values: Vec<i64> = mead::TRACKS
            .iter()
            .map(|key| {
                receipt
                    .changes
                    .iter()
                    .find(|c| &c.track == key)
                    .map(|c| c.after)
                    .ok_or("source track")
            })
            .collect::<Result<_, _>>()?;
        assert_eq!(
            values,
            if draw == 1 {
                vec![2, 2, 0, 21]
            } else {
                vec![3, 3, 9, 20]
            }
        );
        let expected: Vec<_> = if draw == 1 {
            vec!["The mead was too strong. You passed out for a moment."]
        } else {
            vec![
                "The world seems to spin but you manage to stay on your feet.",
                "10 sips in a row. Yeah!",
            ]
        }
        .into_iter()
        .map(|text| ChatLine::Private {
            speaker_name: "Game".into(),
            text: text.into(),
        })
        .collect();
        assert_eq!(feedback, expected);
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let condition = states
            .get(&runtime, actor, session)
            .ok_or("state")?
            .owned_conditions()
            .get(oteryn_gameplay_domain::domain::condition::ConflictKey::Outfit)
            .cloned();
        if draw == 1 {
            let blackout = condition
                .as_ref()
                .ok_or("blackout from native failed USE")?;
            assert_eq!(
                blackout.remaining_duration_ms(blackout.started_at()),
                Some(1000)
            );
            assert_eq!(
                blackout
                    .definition()
                    .source_item_appearance()
                    .ok_or("look")?
                    .definition_key(),
                "oteryn:item.tibia.i111"
            );
            failed = true;
        } else {
            won = true;
        }
        drop(states);
        drop(runtime);
        let replay =
            super::quest_world_callbacks::wire(owner, admitted, command, 2, payload.clone())
                .await?;
        assert_eq!(replay, result);
        assert!(owner.states.chat.drain(session).is_empty());
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        assert_eq!(
            states
                .get(&runtime, actor, session)
                .ok_or("state")?
                .owned_conditions()
                .get(oteryn_gameplay_domain::domain::condition::ConflictKey::Outfit),
            condition.as_ref()
        );
        drop(states);
        drop(runtime);
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
        )
        .bind(mead::KEY)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(count, i64::try_from(attempt + 1)?);
        assert_eq!(
            callback.committed_message(&receipt.changes),
            expected
                .last()
                .and_then(|line| if let ChatLine::Private { text, .. } = line {
                    Some(text.as_str())
                } else {
                    None
                })
        );
        if failed && won {
            round_states(owner, admitted, h, fence, receipt).await?;
            eprintln!(
                "M47_NATIVE_POSTGRES_EXECUTED failed={failed} won={won} uses={}",
                attempt + 1
            );
            return Ok(());
        }
    }
    Err("native mead did not exercise both outcomes in 128 uses".into())
}

async fn round_states(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    fence: CurrentCharacterGameplayFence,
    late: CommittedQuestTransition,
) -> TestResult {
    let catalogue = owner.quest_catalogue.as_ref().ok_or("catalogue")?.clone();
    let key =
        oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, 2).ok_or("barrel key")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    for (initial, values, message) in [
        (
            [3, 3, 9, 17],
            [3, 3, 9, 17],
            "You already passed the test, no need to torture yourself anymore.",
        ),
        ([2, 2, 4, 21], [1, 1, 4, 21], "Ask Sven for another round."),
    ] {
        let command = admitted.continuity.next_command_id;
        super::qm47_mead::seed_sven_state(h, owner, admitted, initial).await?;
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
            placement: placement.clone(),
            expected_revision: owner.bundle_map().ok_or("map")?.facts.object_revision(key),
        })
        .map_err(debug)?;
        let result =
            super::quest_world_callbacks::wire(owner, admitted, command, 2, payload.clone())
                .await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
            UseDisposition::Committed
        );
        assert_eq!(
            owner.states.chat.drain(admitted.game_session_id),
            vec![ChatLine::Private {
                speaker_name: "Game".into(),
                text: message.into(),
            }]
        );
        let expected: std::collections::BTreeMap<_, _> = mead::NATIVE_TRACKS
            .into_iter()
            .zip(values.into_iter().chain([0]))
            .map(|(key, value)| (key.to_owned(), value))
            .collect();
        assert_eq!(
            tracks(h, fence)
                .await?
                .into_iter()
                .collect::<std::collections::BTreeMap<_, _>>(),
            expected
        );
        let replay =
            super::quest_world_callbacks::wire(owner, admitted, command, 2, payload).await?;
        assert_eq!(replay, result);
        assert!(owner.states.chat.drain(admitted.game_session_id).is_empty());
    }
    let before = tracks(h, fence).await?;
    let receipts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(mead::KEY)
    .fetch_one(&h.pool)
    .await?;
    // The wire retains only its newest terminal result. Older durable receipts must
    // still preserve the original roll after later USEs have changed the round.
    let permit = owner.spell_lane_permit().await.ok_or("permit")?;
    let context = context(owner, admitted).await?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let replay = slot
        .commit_world_callback(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: late.transition_key.clone(),
                cause: late.cause,
            },
            catalogue,
            &permit,
            context,
            None,
        )
        .await?;
    assert_eq!(replay, QuestTransitionOutcome::AlreadyCommitted(late));
    drop(slot);
    drop(permit);
    assert!(owner.states.chat.drain(admitted.game_session_id).is_empty());
    assert_eq!(tracks(h, fence).await?, before);
    let after: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
    )
    .bind(mead::KEY)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(after, receipts);
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    assert!(
        states
            .get(
                &runtime,
                admitted.runtime_actor.ok_or("actor")?,
                admitted.game_session_id
            )
            .ok_or("state")?
            .owned_conditions()
            .displayed_temporary_appearance_at(owner.owner_now().get())
            .is_none()
    );
    eprintln!("M47_NATIVE_ROUNDS_POSTGRES_EXECUTED passed=true exhausted=true late_replay=true");
    Ok(())
}

async fn tracks(
    h: &Harness,
    fence: CurrentCharacterGameplayFence,
) -> TestResult<Vec<(String, i64)>> {
    Ok(sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2::text[]) ORDER BY track_key")
        .bind(fence.character_id.as_bytes().as_slice()).bind(mead::NATIVE_TRACKS.as_slice())
        .fetch_all(&h.pool).await?)
}

async fn context(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult<crate::durability::quest_world_callback::QuestWorldContext> {
    let runtime = owner.runtime.lock().await;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    Ok(crate::durability::quest_world_callback::QuestWorldContext {
        map_revision: crate::map::overlay::map_revision(
            owner.bundle_map().ok_or("map")?.overlay.base(),
        ),
        content_revision: format!(
            "sha256:{}",
            runtime
                .content_pin()
                .server_artifact_digest()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ),
        placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
        reset_epoch: owner.reset_epoch,
        actor_placement: actor.placement_identity(),
        movement_revision: runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .revision(),
        party: Vec::new(),
    })
}
