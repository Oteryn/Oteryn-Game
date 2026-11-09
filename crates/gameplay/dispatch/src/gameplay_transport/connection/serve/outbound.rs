//! Terminalize and send the result before ordered domain updates and relocation.

use super::*;

pub(super) fn movement_magic_effect_event(
    ordinal: u32,
    position: crate::map::overlay::TilePos,
    effect_id: u16,
) -> Option<oteryn_protocol_oteryn::presentation::PresentationEvent> {
    Some(oteryn_protocol_oteryn::presentation::PresentationEvent {
        decision_ordinal: 1,
        emission_ordinal: ordinal,
        kind: oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect {
            position: oteryn_protocol_oteryn::world_spatial::ActorPosition {
                x: i32::from(position.x),
                y: i32::from(position.y),
                floor: i16::from(position.floor).checked_neg()?,
            },
            effect_id: u32::from(effect_id),
            source: oteryn_protocol_oteryn::presentation::PresentationSource::Own,
        },
    })
}

pub(super) fn movement_effect_visible(
    observer: oteryn_protocol_oteryn::world_spatial::ActorPosition,
    position: crate::map::overlay::TilePos,
) -> bool {
    let observer =
        crate::movement::interest::VisibilityPosition::new(observer.x, observer.y, observer.floor);
    let target = crate::movement::interest::VisibilityPosition::new(
        i32::from(position.x),
        i32::from(position.y),
        i16::from(position.floor),
    );
    match (observer, target) {
        (Ok(observer), Ok(target)) => {
            crate::movement::interest::VisibilitySettings::REFERENCE.can_see(observer, target)
        }
        _ => false,
    }
}

pub(super) async fn finish<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    dispatch: Dispatch,
    result: Vec<u8>,
    outcome: CommandOutcome,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    if session
        .commands
        .terminalize_with(command.command_id, &result, || {
            authority
                .player_trade_terminalized(session.admitted.game_session_id, command.command_id)
        })
        .is_none()
    {
        return Err(ServeEnd::Disconnected);
    }
    session.admitted.continuity.next_command_id = session.commands.next_command_id();
    if let Dispatch::Logout(LogoutResult::Accepted) = dispatch {
        // §1.6: the release committed before this result. No command is read any more and
        // the transport closes cleanly; a failed write is no control loss either. A second
        // logout read before this result is still the `LOGOUT-RL-01` protocol error, but the
        // committed release keeps this a logout end.
        if write_frame(stream, &result).await.is_ok()
            && (!outcome.second_logout
                || send_error(
                    stream,
                    FoundationProtocolError::TooManyOutstandingCommands,
                    session.admitted.continuity.connection_generation,
                )
                .await
                .is_ok())
        {
            let _ = tokio::time::timeout(
                session.policy.interval,
                tokio::io::AsyncWriteExt::shutdown(stream),
            )
            .await;
        }
        return Err(ServeEnd::LoggedOut);
    }
    if write_frame(stream, &result).await.is_err() {
        return Err(ServeEnd::Disconnected);
    }
    // Native movement can commit an activity return and retire this actor.
    // Send its command result first; the serve loop then crosses the existing
    // scope barrier before observing any destination-owned state.
    if authority
        .current_activity_connection(session.admitted)
        .game_session_id
        != session.admitted.game_session_id
    {
        return Ok(());
    }
    if matches!(dispatch, Dispatch::Logout(_) | Dispatch::NoLogout) {
        if outcome.second_logout {
            return Err(close(
                stream,
                session.admitted,
                FoundationProtocolError::TooManyOutstandingCommands,
            )
            .await);
        }
        if outcome.lost_while_logging_out {
            return Err(ServeEnd::Disconnected);
        }
    }
    emit(
        stream,
        authority,
        session,
        command,
        dispatch,
        outcome.npc_relocation,
        outcome.training_logged_out,
    )
    .await?;
    super::analyser::emit(stream, authority, session).await
}

async fn emit<S, A>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    command: oteryn_protocol_oteryn::ClientCommandView<'_>,
    dispatch: Dispatch,
    npc_relocation: Option<WorldSpatialObservation>,
    training_logged_out: bool,
) -> Result<(), ServeEnd>
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    // Every native action/trade/travel source can grant through the shared account owner.
    // Passive kill/event grants are observed on the existing liveness cadence as well.
    let observe_achievements = matches!(
        &dispatch,
        Dispatch::Use(UseOutcome {
            disposition: UseDisposition::Committed,
            ..
        }) | Dispatch::Npc(..)
            | Dispatch::NpcTrade(..)
            | Dispatch::NpcShop(..)
            | Dispatch::Reply(crate::foundation::CommandStatus::Accepted, _)
    );
    match dispatch {
        Dispatch::NoLogout | Dispatch::Logout(LogoutResult::InFight { .. }) => {
            use oteryn_protocol_oteryn::zones_nologout::{IN_FIGHT_MESSAGE, NO_LOGOUT_MESSAGE};
            let text = if matches!(dispatch, Dispatch::NoLogout) {
                NO_LOGOUT_MESSAGE
            } else {
                IN_FIGHT_MESSAGE
            };
            zones_nologout_transport::emit(
                stream,
                &mut session.admitted,
                &mut session.sequence,
                text,
            )
            .await
            .ok_or(ServeEnd::Disconnected)?;
        }
        Dispatch::Step(outcome, _) => {
            effects::step(stream, authority, session, outcome).await?;
        }
        Dispatch::Use(outcome) => {
            use_effects::use_effects(stream, authority, session, command, outcome).await?;
        }
        Dispatch::Spell(outcome) => {
            effects::spell(stream, authority, session, outcome).await?;
        }
        Dispatch::Timed(_, deltas) | Dispatch::ContainerSort(deltas) => {
            for delta in deltas {
                let Some((delta_sequence, frame)) =
                    item_view_delta(session.generation, session.sequence, &delta)
                else {
                    return Err(ServeEnd::Disconnected);
                };
                session.sequence = delta_sequence;
                session.admitted.continuity.server_sequence = session.sequence;
                if write_frame(stream, &frame).await.is_err() {
                    return Err(ServeEnd::Disconnected);
                }
            }
        }
        Dispatch::UseItem(_, Some(delta)) | Dispatch::ContainerView(_, Some(delta)) => {
            let Some((delta_sequence, frame)) =
                item_view_delta(session.generation, session.sequence, &delta)
            else {
                return Err(ServeEnd::Disconnected);
            };
            session.sequence = delta_sequence;
            session.admitted.continuity.server_sequence = session.sequence;
            if write_frame(stream, &frame).await.is_err() {
                return Err(ServeEnd::Disconnected);
            }
        }
        Dispatch::Stash(reply) => {
            if !session
                .selected_capabilities
                .contains(oteryn_protocol_oteryn::stash::CAPABILITY_STASH_V1)
            {
                return Err(ServeEnd::Disconnected);
            }
            let revision = session
                .stash_revision
                .checked_add(1)
                .ok_or(ServeEnd::Disconnected)?;
            let sequence = session
                .sequence
                .checked_add(1)
                .ok_or(ServeEnd::Disconnected)?;
            let payload = oteryn_protocol_oteryn::stash::encode_stash_reply(&reply)
                .map_err(|_| ServeEnd::Disconnected)?;
            let frame = oteryn_protocol_oteryn::encode_state_delta(
                session.generation,
                sequence,
                oteryn_protocol_oteryn::stash::DOMAIN_STASH,
                session.stash_revision,
                revision,
                1,
                &payload,
            )
            .map_err(|_| ServeEnd::Disconnected)?;
            write_frame(stream, &frame)
                .await
                .map_err(|_| ServeEnd::Disconnected)?;
            session.stash_revision = revision;
            session.sequence = sequence;
            session.admitted.continuity.server_sequence = sequence;
            if reply.outcome == oteryn_protocol_oteryn::stash::StashOutcome::Committed {
                let Some(view) = session.item_view.as_mut() else {
                    return Err(ServeEnd::Disconnected);
                };
                if item_use_view_refresh::refresh(
                    stream,
                    authority,
                    session.actor,
                    &mut session.admitted,
                    view,
                    session.container_views.as_mut(),
                    &mut session.sequence,
                )
                .await
                .is_none()
                {
                    return Err(ServeEnd::Disconnected);
                }
            }
        }
        Dispatch::Depot(reply) => {
            if reply.outcome == oteryn_protocol_oteryn::depot::DepotOutcome::Moved {
                let Some(view) = session.item_view.as_mut() else {
                    return Err(ServeEnd::Disconnected);
                };
                if item_use_view_refresh::refresh(
                    stream,
                    authority,
                    session.actor,
                    &mut session.admitted,
                    view,
                    session.container_views.as_mut(),
                    &mut session.sequence,
                )
                .await
                .is_none()
                {
                    return Err(ServeEnd::Disconnected);
                }
            }
        }
        Dispatch::Reply(_, payload)
            if (command.command_type
                == oteryn_protocol_oteryn::depot_search::COMMAND_TYPE_DEPOT_SEARCH
                && oteryn_protocol_oteryn::depot_search::decode_reply(&payload).is_ok_and(
                    |reply| {
                        reply.outcome == oteryn_protocol_oteryn::depot_search::Outcome::Retrieved
                    },
                ))
                || (command.command_type
                    == u32::from(oteryn_protocol_oteryn::mail::COMMAND_TYPE_MAIL)
                    && payload.as_slice()
                        == [oteryn_protocol_oteryn::mail::MailOutcome::TextWritten as u8]) =>
        {
            let Some(view) = session.item_view.as_mut() else {
                return Err(ServeEnd::Disconnected);
            };
            if item_use_view_refresh::refresh(
                stream,
                authority,
                session.actor,
                &mut session.admitted,
                view,
                session.container_views.as_mut(),
                &mut session.sequence,
            )
            .await
            .is_none()
            {
                return Err(ServeEnd::Disconnected);
            }
        }
        Dispatch::Reply(CommandStatus::Accepted, _) if forge::changes_inventory(command) => {
            forge::refresh(stream, authority, session).await?;
        }
        Dispatch::ItemMove(outcome, deltas, _) => {
            effects::item_move(stream, authority, session, outcome, deltas).await?;
        }
        Dispatch::Reply(_, payload)
            if command.command_type == oteryn_protocol_oteryn::market::COMMAND_TYPE_MARKET
                && session
                    .selected_capabilities
                    .domain_selected(oteryn_protocol_oteryn::market::STATE_DOMAIN_MARKET) =>
        {
            let next = session
                .market_revision
                .checked_add(1)
                .ok_or(ServeEnd::Disconnected)?;
            let sequence = session
                .sequence
                .checked_add(1)
                .ok_or(ServeEnd::Disconnected)?;
            let frame = encode_state_delta(
                session.generation,
                sequence,
                oteryn_protocol_oteryn::market::STATE_DOMAIN_MARKET,
                session.market_revision,
                next,
                1,
                &payload,
            )
            .map_err(|_| ServeEnd::Disconnected)?;
            session.market_revision = next;
            session.sequence = sequence;
            session.admitted.continuity.server_sequence = sequence;
            write_frame(stream, &frame)
                .await
                .map_err(|_| ServeEnd::Disconnected)?;
        }
        Dispatch::QuestLog(domain) => {
            let Some((delta_sequence, frame)) =
                quest_log_delta(session.generation, session.sequence, &domain)
            else {
                return Err(ServeEnd::Disconnected);
            };
            session.sequence = delta_sequence;
            session.admitted.continuity.server_sequence = session.sequence;
            if write_frame(stream, &frame).await.is_err() {
                return Err(ServeEnd::Disconnected);
            }
        }
        // CHAT-WIRE-1: the lines reach the hearers' domain 12 on their own ticks. CHAT-2B: an
        // opened or closed room follows the result as one domain 12 delta type 2.
        Dispatch::Chat(_, spell) => {
            if let Some(outcome) = spell {
                effects::spell(stream, authority, session, outcome).await?;
            }
            if let Some(rooms) = authority.chat_rooms_changed_bound(session.admitted).await {
                let Some((delta_sequence, frame, chat_revision)) = chat_delta(
                    session.generation,
                    session.sequence,
                    session.admitted.continuity.chat_revision,
                    DELTA_TYPE_CHAT_ROOMS_V1,
                    &encode_chat_rooms(rooms),
                ) else {
                    return Err(ServeEnd::Disconnected);
                };
                session.sequence = delta_sequence;
                session.admitted.continuity.server_sequence = session.sequence;
                session.admitted.continuity.chat_revision = chat_revision;
                if write_frame(stream, &frame).await.is_err() {
                    return Err(ServeEnd::Disconnected);
                }
            }
            refresh::channel_list(stream, authority, session).await?;
        }
        // NPC-CMD-1a: the whole domain 7 payload, then the domain 8 window (NPC-CMD-1b-read),
        // follow the result, to the speaker alone.
        Dispatch::NpcTrade(_, update, deltas) | Dispatch::NpcShop(_, _, update, deltas) => {
            effects::npc_trade(stream, session, update, deltas).await?;
        }
        Dispatch::Npc(_, update, _) => {
            effects::npc(stream, session, update).await?;
        }
        Dispatch::Attack(_) => {
            pvp_expert::refresh(stream, authority, session).await?;
            // The changed target or modes reach domain 10 right after the result.
            let Some(state) = authority
                .observe_combat_state(session.actor, session.admitted.game_session_id)
                .await
            else {
                return Ok(());
            };
            let Some(change) = session.admitted.continuity.combat.delta(
                session.generation,
                session.sequence,
                state,
            ) else {
                return Err(ServeEnd::Disconnected);
            };
            if let Some((delta_sequence, frame)) = change {
                session.sequence = delta_sequence;
                session.admitted.continuity.server_sequence = session.sequence;
                if write_frame(stream, &frame).await.is_err() {
                    return Err(ServeEnd::Disconnected);
                }
            }
        }
        Dispatch::Guild(_) if command.command_type == 73 && command.payload.first() == Some(&4) => {
            if let Some(view) = session.item_view.as_mut()
                && item_use_view_refresh::refresh(
                    stream,
                    authority,
                    session.actor,
                    &mut session.admitted,
                    view,
                    session.container_views.as_mut(),
                    &mut session.sequence,
                )
                .await
                .is_none()
            {
                return Err(ServeEnd::Disconnected);
            }
        }
        Dispatch::UseItem(_, None)
        | Dispatch::ContainerView(_, None)
        | Dispatch::Look(..)
        | Dispatch::Achievements(_)
        | Dispatch::CharacterInfo(_)
        | Dispatch::Highscores(_)
        | Dispatch::Guild(_)
        | Dispatch::Reply(..)
        | Dispatch::Vip(_)
        | Dispatch::Appearance(_)
        | Dispatch::Training(_)
        | Dispatch::Logout(LogoutResult::Accepted | LogoutResult::Busy)
        | Dispatch::House(_)
        | Dispatch::Unregistered => {}
    }
    refresh::sounds(stream, authority, session).await?;
    player_trade::refresh(stream, authority, session).await?;
    cyclopedia_tracker::refresh(stream, authority, session).await?;
    native_cues::refresh(stream, authority, session).await?;
    if training_logged_out {
        return Err(ServeEnd::LoggedOut);
    }
    if newhaven_travel_transport::emit_relocation(
        stream,
        authority,
        session.actor,
        &mut session.admitted,
        session.visibility.as_mut(),
        session.item_view.as_mut(),
        session.container_views.as_mut(),
        session.world_map.as_mut(),
        session.generation,
        &mut session.sequence,
        &mut session.revision,
        &mut session.appearances,
        npc_relocation,
    )
    .await
    .is_none()
    {
        return Err(ServeEnd::Disconnected);
    }
    if observe_achievements {
        achievement_notices::refresh(stream, authority, session).await?;
    }
    Ok(())
}

#[cfg(test)]
mod movement_effect_tests {
    use super::*;

    #[test]
    fn movement_effects_are_visible_only_inside_the_viewer_range() {
        let observer = oteryn_protocol_oteryn::world_spatial::ActorPosition {
            x: 100,
            y: 100,
            floor: -7,
        };
        let at = |x, y, floor| crate::map::overlay::TilePos { x, y, floor };
        assert!(movement_effect_visible(observer, at(108, 106, -7)));
        assert!(!movement_effect_visible(observer, at(110, 100, -7)));
        assert!(!movement_effect_visible(observer, at(100, 108, -7)));
        assert!(!movement_effect_visible(observer, at(100, 100, -8)));
    }

    #[test]
    fn trap_effect_encodes_on_capability_23_wire_with_donor_id()
    -> Result<(), Box<dyn std::error::Error>> {
        // Crystal data/scripts/movements/trap.lua:36, CONST_ME_HITAREA = 11.
        let event = movement_magic_effect_event(
            1,
            crate::map::overlay::TilePos {
                x: 7,
                y: 8,
                floor: -7,
            },
            11,
        )
        .ok_or("movement effect position")?;
        let payload = oteryn_protocol_oteryn::presentation::encode_presentation_batch(&[event])
            .map_err(|error| format!("{error:?}"))?;
        let frame = encode_state_delta(
            1,
            1,
            oteryn_protocol_oteryn::presentation::STATE_DOMAIN_WORLD_PRESENTATION,
            0,
            1,
            oteryn_protocol_oteryn::presentation::DELTA_TYPE_WORLD_PRESENTATION_BATCH_V1,
            &payload,
        )
        .map_err(|error| format!("{error:?}"))?;
        let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&frame)
            .map_err(|error| format!("{error:?}"))?;
        let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            delta.domain_id,
            oteryn_protocol_oteryn::presentation::STATE_DOMAIN_WORLD_PRESENTATION
        );
        let decoded =
            oteryn_protocol_oteryn::presentation::decode_presentation_batch(delta.payload)
                .map_err(|error| format!("{error:?}"))?;
        assert_eq!(decoded, vec![event]);
        Ok(())
    }
}
