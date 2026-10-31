//! E08 read-side proof using possessions stored by the existing custody writers.
use super::*;
use oteryn_protocol_oteryn::depot_search::{self as wire, Outcome, Request};
use oteryn_protocol_oteryn::item_view::ItemHandle;
use oteryn_protocol_oteryn::stash::{StashAction, StashIntent, StashOutcome};

async fn dispatch(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    id: u64,
    request: Request,
) -> TestResult<wire::Reply> {
    let payload = wire::encode_request(request).map_err(debug)?;
    let frame = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id: id,
            command_type: wire::COMMAND_TYPE_DEPOT_SEARCH,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    let mut input = u32::try_from(frame.len())?.to_be_bytes().to_vec();
    input.extend(frame);
    let (_, output) =
        super::super::weather_wire::fixture_access::serve_input(owner, admitted, &input).await?;
    let mut cursor = output.as_slice();
    while !cursor.is_empty() {
        let len = u32::from_be_bytes(cursor.get(..4).ok_or("length")?.try_into()?) as usize;
        let frame = cursor.get(4..4 + len).ok_or("frame")?;
        cursor = &cursor[4 + len..];
        let envelope = oteryn_protocol_oteryn::decode_wire_envelope(frame).map_err(debug)?;
        if envelope.message_type() == oteryn_protocol_oteryn::MessageType::CommandResult {
            let result =
                oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(result.command_id, id);
            assert_eq!(
                result.status,
                oteryn_protocol_oteryn::CommandStatus::Accepted
            );
            return wire::decode_reply(result.payload).map_err(debug);
        }
    }
    Err("missing depot list reply".into())
}

pub(super) async fn prepare(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    extras: &[(ItemHandle, [u8; 16])],
) -> TestResult {
    let empty = dispatch(owner, admitted, 701, Request::Open { after: 0 }).await?;
    assert_eq!(empty.outcome, Outcome::Listed);
    assert!(empty.results.is_empty());
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let command = |command_id| UseCommand {
        game_session_id: admitted.game_session_id,
        command_id,
        item_fence: admitted.item_fence,
    };
    let (handle, item) = *extras.first().ok_or("depot stack")?;
    let moved = owner
        .depot(
            actor,
            command(702),
            DepotIntent {
                town: Some(0),
                box_number: 17,
                page: 0,
                action: DepotAction::Deposit(handle),
            },
            Some(ItemKey::Instance(item)),
        )
        .await
        .ok_or("deposit")?;
    assert_eq!(moved.reply.outcome, DepotOutcome::Moved);
    let (handle, item) = *extras.get(1).ok_or("stash stack")?;
    let stowed = owner
        .stash(
            actor,
            command(703),
            StashIntent {
                page: 0,
                action: StashAction::Stow(handle),
            },
            Some(ItemKey::Instance(item)),
            false,
        )
        .await
        .ok_or("stow")?;
    assert_eq!(stowed.outcome, StashOutcome::Committed);
    Ok(())
}

pub(super) async fn exercise(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult {
    let reply = dispatch(owner, admitted, 704, Request::Open { after: 0 }).await?;
    assert_eq!(reply.outcome, Outcome::Listed);
    assert_eq!(
        reply
            .results
            .iter()
            .map(|r| (r.client_id, r.depot, r.stash, r.inbox))
            .collect::<Vec<_>>(),
        [
            (3028, 100, 100, 0),
            (3504, 0, 0, 1),
            (3506, 0, 0, 1),
            (3507, 0, 0, 1)
        ]
    );
    assert_eq!(
        reply.results[0].tiers,
        [wire::TierCount {
            tier: 0,
            count: 200
        }]
    );
    assert_eq!(
        dispatch(owner, admitted, 704, Request::Open { after: 0 }).await?,
        reply
    );
    let search = dispatch(owner, admitted, 705, Request::Search { client_id: 3028 }).await?;
    let detail = dispatch(
        owner,
        admitted,
        706,
        Request::Detail {
            client_id: 3028,
            tier: 0,
        },
    )
    .await?;
    assert_eq!(search.outcome, Outcome::Found);
    assert_eq!(detail.outcome, Outcome::Detailed);
    assert_eq!(detail.results, search.results);
    assert_ne!(
        wire::encode_reply(&search).map_err(debug)?,
        wire::encode_reply(&detail).map_err(debug)?
    );
    let absent = dispatch(
        owner,
        admitted,
        707,
        Request::Detail {
            client_id: 3028,
            tier: 1,
        },
    )
    .await?;
    assert_eq!(absent.outcome, Outcome::Detailed);
    assert_eq!(
        (
            absent.results[0].depot,
            absent.results[0].stash,
            absent.results[0].inbox
        ),
        (0, 0, 0)
    );
    assert_eq!(
        absent.results[0].tiers,
        [wire::TierCount { tier: 1, count: 0 }]
    );
    assert_eq!(
        dispatch(owner, admitted, 708, Request::Open { after: 3506 })
            .await?
            .results,
        reply.results[3..]
    );
    // A live actor away from an authored locker cannot open, search or request detail.
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let (step, duration) = owner
        .paced_step(
            actor,
            admitted.game_session_id,
            oteryn_protocol_oteryn::world_spatial::StepDirection::East,
        )
        .await;
    assert_eq!(
        step.disposition,
        oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
    );
    for (id, request) in [
        (709, Request::Open { after: 0 }),
        (710, Request::Search { client_id: 3028 }),
        (
            711,
            Request::Detail {
                client_id: 3028,
                tier: 0,
            },
        ),
    ] {
        assert_eq!(
            dispatch(owner, admitted, id, request).await?.outcome,
            Outcome::Rejected
        );
    }
    if let Some(duration) = duration {
        tokio::time::sleep(duration).await;
    }
    let (step, _) = owner
        .paced_step(
            actor,
            admitted.game_session_id,
            oteryn_protocol_oteryn::world_spatial::StepDirection::West,
        )
        .await;
    assert_eq!(
        step.disposition,
        oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
    );
    eprintln!(
        "DEPOTLIST_EXECUTED: real dispatch; depot + stash + nested Inbox; detail; outside-depot refusals"
    );
    Ok(())
}
