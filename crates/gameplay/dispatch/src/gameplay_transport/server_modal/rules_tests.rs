use super::*;
use crate::foundation::GameSessionId;
use crate::gameplay_transport::server_modal::ModalHub;
use oteryn_protocol_oteryn::server_modal::{
    COMMAND_TYPE_SERVER_MODAL_ANSWER, COMMAND_TYPE_SERVER_MODAL_QUERY, Reply,
};

fn root() -> Result<Root, String> {
    let mut id = [1; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    Ok((GameSessionId::decode(&id).map_err(|e| format!("{e:?}"))?, 1))
}

fn bed(placement_key: u64) -> Result<Selection, String> {
    use crate::foundation::{
        ChannelId, ConnectionGeneration, RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId,
    };
    let session = root()?.0;
    Ok(Selection::Bed(BedSelection {
        target: ItemKey::MapBase {
            bundle_digest: [7; 32],
            placement_key,
            reset_epoch: 1,
        },
        revision: 0,
        fence: CurrentCharacterItemFence {
            character_id: crate::domain::CharacterId::from_bytes(*session.as_bytes())
                .map_err(|e| format!("{e:?}"))?,
            game_session_id: session,
            connection_generation: ConnectionGeneration::new(1).map_err(|e| format!("{e:?}"))?,
            character_lease_generation: 1,
            runtime_scope: RuntimeScopeRefV1::channel(
                WorldId::decode(session.as_bytes()).map_err(|e| format!("{e:?}"))?,
                ChannelId::decode(session.as_bytes()).map_err(|e| format!("{e:?}"))?,
            ),
            scope_ownership_generation: ScopeOwnershipGeneration::new(1)
                .map_err(|e| format!("{e:?}"))?,
        },
    }))
}

fn offered(hub: &ModalHub, root: Root, selection: Selection) -> Result<Dialog, String> {
    hub.execute(
        root,
        COMMAND_TYPE_SERVER_MODAL_QUERY,
        oteryn_protocol_oteryn::server_modal::QUERY_WITH_TERMINAL_REPLY,
        None,
    )
    .ok_or("terminal opt-in")?;
    let bytes = hub
        .offer_rule(root, bed_dialog(), selection)
        .ok_or("offer refused")?;
    let Reply::Dialog(dialog) = Reply::decode(&bytes)? else {
        return Err("missing dialog".into());
    };
    Ok(dialog)
}

#[test]
fn server_modal_terminal_rules_require_connection_opt_in_and_replay_exactly() -> Result<(), String>
{
    let hub = ModalHub::default();
    let root = root()?;
    assert!(!hub.accepts_terminal_reply(root));
    assert_eq!(
        hub.execute(root, COMMAND_TYPE_SERVER_MODAL_QUERY, &[], None),
        Some(Reply::None.encode()?)
    );
    assert!(!hub.accepts_terminal_reply(root));
    assert!(
        hub.offer_rule(root, bed_dialog(), bed(1)?).is_none(),
        "legacy query cannot expose a terminal rule"
    );
    assert!(
        hub.execute(root, COMMAND_TYPE_SERVER_MODAL_QUERY, &[1, 0], None)
            .is_none()
    );
    assert!(!hub.accepts_terminal_reply(root));
    let dialog = offered(&hub, root, bed(1)?)?;
    assert!(hub.accepts_terminal_reply(root));
    assert!(!hub.accepts_terminal_reply((root.0, root.1 + 1)));
    assert!(
        hub.execute(root, COMMAND_TYPE_SERVER_MODAL_QUERY, &[], None)
            .is_some()
    );
    assert!(
        hub.accepts_terminal_reply(root),
        "opt-in persists for this connection"
    );
    let answer = Answer {
        dialog_id: dialog.id,
        button_id: 1,
        choice_id: Some(6),
    };
    assert!(hub.rule_selection(root, answer).is_some());
    let bytes = hub
        .finish_terminal_rule(root, answer)
        .ok_or("terminal acknowledgement")?;
    assert_eq!(Reply::decode(&bytes)?, Reply::AnsweredAndLoggedOut(answer));
    assert_eq!(
        hub.execute(
            root,
            COMMAND_TYPE_SERVER_MODAL_ANSWER,
            &answer.encode()?,
            None
        ),
        Some(bytes)
    );
    assert!(hub.rule_selection(root, answer).is_none());
    hub.leave(root);
    assert!(!hub.accepts_terminal_reply(root));
    assert!(hub.offer_rule(root, bed_dialog(), bed(1)?).is_none());
    Ok(())
}

#[test]
fn server_modal_bed_rules_keep_exact_source_and_require_listed_skill() -> Result<(), String> {
    let hub = ModalHub::default();
    let root = root()?;
    let dialog = offered(&hub, root, bed(1)?)?;
    assert_eq!(
        dialog.choices.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        [4, 2, 5, 1, 6, 3]
    );
    assert_eq!(offered(&hub, root, bed(1)?)?, dialog);
    assert!(hub.offer_rule(root, bed_dialog(), bed(2)?).is_none());
    let Selection::Bed(mut changed) = bed(1)? else {
        return Err("bed selection".into());
    };
    changed.fence.character_lease_generation += 1;
    assert!(
        hub.offer_rule(root, bed_dialog(), Selection::Bed(changed.clone()))
            .is_none()
    );
    changed.fence.character_lease_generation -= 1;
    changed.revision += 1;
    assert!(
        hub.offer_rule(root, bed_dialog(), Selection::Bed(changed))
            .is_none()
    );
    let answer = Answer {
        dialog_id: dialog.id,
        button_id: 1,
        choice_id: Some(6),
    };
    assert!(
        hub.rule_selection(
            root,
            Answer {
                choice_id: None,
                ..answer
            }
        )
        .is_none()
    );
    assert!(
        hub.rule_selection(
            root,
            Answer {
                choice_id: Some(99),
                ..answer
            }
        )
        .is_none()
    );
    assert!(hub.rule_selection(root, answer).is_some());
    assert!(
        hub.execute(
            root,
            COMMAND_TYPE_SERVER_MODAL_ANSWER,
            &answer.encode()?,
            None
        )
        .is_none(),
        "a gameplay choice cannot fall through to an informational ACK"
    );
    assert!(
        hub.rule_selection(
            root,
            Answer {
                choice_id: Some(1),
                ..answer
            }
        )
        .is_none()
    );
    assert!(
        hub.rule_selection(root, answer).is_some(),
        "an uncertain callback retains its exact answer"
    );
    Ok(())
}

#[test]
fn server_modal_rule_cancel_replay_and_connection_generation_are_isolated() -> Result<(), String> {
    let hub = ModalHub::default();
    let root = root()?;
    let dialog = offered(&hub, root, bed(1)?)?;
    let cancel = Answer {
        dialog_id: dialog.id,
        button_id: 2,
        choice_id: None,
    };
    assert!(hub.rule_selection(root, cancel).is_some());
    assert!(
        hub.finish_terminal_rule(root, cancel).is_none(),
        "cancellation never emits terminal acceptance"
    );
    let receipt = hub.finish_rule(root, cancel).ok_or("cancel")?;
    assert_eq!(Reply::decode(&receipt)?, Reply::Answered(cancel));
    assert!(
        hub.rule_selection(root, cancel).is_none(),
        "completed rule cannot run twice"
    );
    assert_eq!(
        hub.execute(
            root,
            COMMAND_TYPE_SERVER_MODAL_ANSWER,
            &cancel.encode()?,
            None
        ),
        Some(receipt)
    );
    assert_eq!(
        hub.execute(root, COMMAND_TYPE_SERVER_MODAL_QUERY, &[], None),
        Some(Reply::None.encode()?)
    );
    let next = offered(&hub, root, bed(2)?)?;
    assert_ne!(next.id, dialog.id);
    assert!(hub.rule_selection(root, cancel).is_none());
    assert!(
        hub.rule_selection(
            (root.0, root.1 + 1),
            Answer {
                dialog_id: next.id,
                ..cancel
            }
        )
        .is_none()
    );
    hub.leave(root);
    assert!(
        hub.rule_selection(
            root,
            Answer {
                dialog_id: next.id,
                ..cancel
            }
        )
        .is_none()
    );
    Ok(())
}

#[test]
fn server_modal_proved_bed_refusal_allows_cancel_without_abandoning_uncertain_answer()
-> Result<(), String> {
    let hub = ModalHub::default();
    let root = root()?;
    let dialog = offered(&hub, root, bed(1)?)?;
    let answer = Answer {
        dialog_id: dialog.id,
        button_id: 1,
        choice_id: Some(6),
    };
    let cancel = Answer {
        button_id: 2,
        choice_id: None,
        ..answer
    };
    assert!(hub.rule_selection(root, answer).is_some());
    assert!(hub.rule_selection(root, cancel).is_none());
    assert!(hub.refused_rule(root, answer).is_some());
    assert!(hub.rule_selection(root, cancel).is_some());
    assert!(hub.finish_rule(root, cancel).is_some());
    Ok(())
}
