//! New command220 actions use the real stream. Losing that stream refuses
//! resumed input, while old native receipts preserve their original answers.
use super::{Harness, ReconciledCharacterAuthority, RewardClaimInboxMintOutput,
    TestResult, TypedDefinitionRef, HIRELING_LAMP_KEY, debug, id};
use crate::foundation::ExactActorRef;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{AdmittedSession, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::{CommandStatus, hireling as wire};

pub(super) struct Replay {
    initial: AdmittedSession,
    records: Vec<Recorded>,
    hireling: [u8; 16],
    actor: ExactActorRef,
}

struct Recorded {
    admitted: AdmittedSession,
    request: wire::Request,
    answer: bool,
}

pub(super) async fn scenario(h: &Harness, authority: &ReconciledCharacterAuthority<'_, '_>,
    owner: &ComposedFreshAdmission<'_, '_, '_>, mut admitted: AdmittedSession,
    actor: ExactActorRef) -> TestResult<Replay> {
    let current = admitted.item_fence.ok_or("legacy wire House fence")?;
    let hireling = id(141);
    let lamp = id(142);
    let definition = {
        let runtime = owner.runtime.lock().await;
        let index = crate::gameplay_transport::item_ref_admission::pinned_index(&runtime,
            owner.active_generation).ok_or("native item index")?;
        TypedDefinitionRef { family: "Item".into(), production_key: HIRELING_LAMP_KEY.into(),
            revision_ref: index.revision(HIRELING_LAMP_KEY).ok_or("canonical lamp")?.into() }
    };
    let native = owner.active_generation.and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or("native gameplay")?;
    let item = crate::gameplay_transport::item_ref_admission::generation_inbox_item_facts(native, &definition)
        .map_err(debug)?;
    let source = super::inbox_source(current, admitted.channel_id, 101, 143, vec![lamp])?;
    let mut mint = super::mint_request(vec![RewardClaimInboxMintOutput {
        source_line: 0, item, quantity: 1, key_number: None,
    }]);
    mint.claim = definition;
    super::native_mint(h, authority, current, &source, mint).await?;
    super::seed_identity(h, hireling, lamp).await?;
    let state = owner.runtime.lock().await.house_actor_state(actor, admitted.game_session_id).map_err(debug)?;
    let summon = wire::Request::Summon { x: u16::try_from(state.position.x)?,
        y: u16::try_from(state.position.y)?, floor: -i8::try_from(state.position.floor)? };
    let initial = admitted;
    let mut records = Vec::new();
    for (request, expected) in original_requests(summon) {
        let original = admitted;
        let answer = command(owner, &mut admitted, request).await
            .map_err(|error| format!("legacy initial sequence: {error}"))?;
        assert_eq!(answer, expected, "native command220 action {request:?}");
        records.push(Recorded { admitted: original, request, answer });
    }
    assert_state(h, owner, &admitted, actor, hireling, false, 1, 1, 2).await?;
    // Every fresh transition, including resummoning, is admitted before the
    // owner loses its real SessionCommands. No new direct command follows loss.
    for (request, expected) in [(wire::Request::Query, false), (summon, true), (wire::Request::Query, true)] {
        let original = admitted;
        let answer = command(owner, &mut admitted, request).await?;
        assert_eq!(answer, expected);
        records.push(Recorded { admitted: original, request, answer });
    }
    assert_state(h, owner, &admitted, actor, hireling, true, 2, 0, 3).await?;
    Ok(Replay { initial, records, hireling, actor })
}

/// A new owner cannot admit the started stream from its counters. Retry only
/// the already-executed native commands, each with its exact original identity.
pub(super) async fn replay_after_ingress_loss(h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>, saved: Replay) -> TestResult<Replay> {
    retry_recorded(h, owner, &saved).await?;
    Ok(saved)
}

/// The old Dismiss answer must not return the lamp placed by a later command.
pub(super) async fn replay_on_second_owner(h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>, saved: Replay) -> TestResult {
    retry_recorded(h, owner, &saved).await
}

async fn retry_recorded(h: &Harness, owner: &ComposedFreshAdmission<'_, '_, '_>, saved: &Replay) -> TestResult {
    super::wire::assert_started_ingress_lost(owner, saved.initial,
        wire::COMMAND_TYPE_HIRELING, &wire::Request::Query.encode()).await?;
    assert_state(h, owner, &saved.initial, saved.actor, saved.hireling, true, 2, 0, 3).await?;
    for recorded in &saved.records {
        let actual = owner.hireling(recorded.admitted,
            recorded.admitted.continuity.next_command_id, recorded.request).await
            .ok_or("original command220 native retry")?;
        assert_eq!(actual, recorded.answer,
            "original {request:?} keeps its recorded answer after later resummoning", request = recorded.request);
        assert_state(h, owner, &saved.initial, saved.actor, saved.hireling, true, 2, 0, 3).await?;
    }
    Ok(())
}

fn original_requests(summon: wire::Request) -> [(wire::Request, bool); 5] {
    [(wire::Request::Query, false), (summon, true), (wire::Request::Query, true),
        (wire::Request::Dismiss, false), (wire::Request::Query, false)]
}

async fn command(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    request: wire::Request) -> TestResult<bool> {
    let (status, answer) = super::wire::exchange(owner, admitted,
        wire::COMMAND_TYPE_HIRELING, &request.encode()).await?;
    assert_eq!(status, CommandStatus::Accepted);
    match answer.as_slice() { [0] => Ok(false), [1] => Ok(true), _ => Err("bounded command220 answer".into()) }
}

#[allow(clippy::too_many_arguments)]
async fn assert_state(h: &Harness, owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession, actor: ExactActorRef, hireling: [u8; 16],
    active: bool, uses: i64, inbox: i64, transitions: i64) -> TestResult {
    assert_eq!(super::active(h, hireling).await?, active);
    assert_eq!(super::uses(h).await?, uses, "old Summon never burns another lamp");
    assert_eq!(super::inbox_count(h).await?, inbox, "old Dismiss never mints another lamp");
    let actual: i64 = sqlx::query_scalar("SELECT count(*) FROM game_hireling_lamp_transitions WHERE hireling_id=encode($1,'hex')::uuid")
        .bind(hireling.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(actual, transitions);
    let entities = owner.runtime.lock().await.house_hireling_entities(actor, admitted.game_session_id).map_err(debug)?;
    assert_eq!(entities.len(), usize::from(active), "NPC projection follows current custody, not replayed booleans");
    Ok(())
}
