//! Crystal 201189f hireling.lua:676-708; gamestore/init.lua:1914-2020.
//! The modal selects an identity; the ordinary Store owner alone can buy the service.
use super::{ComposedFreshAdmission, owner::Root};
use crate::gameplay_transport::connection::AdmittedSession;
use oteryn_protocol_oteryn::{
    CommandStatus,
    server_modal::{Answer, Dialog},
    store::{self as wire, StoreHirelingCustomization, StorePurchaseRequest},
};

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Selection {
    request: StorePurchaseRequest,
    customization: Option<StoreHirelingCustomization>,
    identities: Vec<[u8; 16]>,
}

fn selected(admitted: AdmittedSession) -> bool {
    [
        wire::CAPABILITY_STORE_V1,
        oteryn_protocol_oteryn::hireling::management::CAPABILITY_HIRELING_MANAGEMENT_V1,
    ]
    .iter()
    .all(|id| admitted.continuity.selected_capabilities.contains(*id))
}

pub(super) async fn start(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    root: Root,
    payload: &[u8],
) -> Option<Vec<u8>> {
    if !selected(admitted) {
        return None;
    }
    let (request, customization) = wire::decode_purchase_request_with_hireling(payload).ok()?;
    match (request.offer_key.as_str(), customization.as_ref()) {
        ("store:hireling:25438", Some(c)) if c.hireling_id.is_none() && c.sex.is_none() => {
            crate::durability::store::canonical_hireling_name(c.name.as_deref()?)?;
        }
        ("store:hireling:25437", None) => {}
        _ => return None,
    }
    let hirelings = owner
        .root
        .read_owned_hirelings(
            owner.character,
            owner.holder,
            admitted.item_fence?,
            *admitted.channel_id.as_bytes(),
        )
        .await
        .ok()?;
    if hirelings.is_empty() {
        return None;
    }
    let dialog = Dialog {
        id: 1,
        title: "Choose a Hireling".into(),
        message: "Select a hireling below".into(),
        choices: hirelings
            .iter()
            .enumerate()
            .map(|(i, hireling)| ((i + 1) as u8, format!("#{} - {}", i + 1, hireling.name)))
            .collect(),
        buttons: vec![(1, "Select".into()), (2, "Cancel".into())],
        cancel_button: Some(2),
        image: None,
    };
    // Re-prove the connected actor after the fenced read, before publishing the offer.
    let runtime = owner.runtime.lock().await;
    if !super::super::battle_instinct_status::actor_is_current(&runtime, admitted)
        || owner.states.server_messages.connection_root(
            admitted.game_session_id,
            admitted.continuity.connection_generation,
        ) != Some(root)
    {
        return None;
    }
    owner.states.server_modals.offer_rule(
        root,
        dialog,
        super::rules::Selection::HirelingStore(Selection {
            request,
            customization,
            identities: hirelings.into_iter().map(|h| h.id).collect(),
        }),
    )
}

/// Outer None means an informational dialog; inner None preserves a retryable selection.
pub(super) async fn answer(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    root: Root,
    command_id: u64,
    payload: &[u8],
) -> Option<Option<Vec<u8>>> {
    let answer = Answer::decode(payload).ok()?;
    // The Hireling dialog asks for no input, so an answer carrying a value never reaches it.
    let (super::rules::Selection::HirelingStore(selection), None, _) =
        owner.states.server_modals.rule_selection(root, answer)?
    else {
        return None;
    };
    Some(complete(owner, admitted, root, command_id, answer, selection).await)
}

async fn complete(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    root: Root,
    command_id: u64,
    answer: Answer,
    selection: Selection,
) -> Option<Vec<u8>> {
    if !selected(admitted) {
        return None;
    }
    if answer.button_id == 2 {
        return owner.states.server_modals.finish_rule(root, answer);
    }
    let hireling = *selection
        .identities
        .get(usize::from(answer.choice_id?.checked_sub(1)?))?;
    let mut customization = selection.customization.unwrap_or_default();
    customization.hireling_id = Some(hireling);
    let purchase =
        wire::encode_hireling_purchase_request(&selection.request, &customization).ok()?;
    let (status, bytes) = super::super::store::execute(
        owner,
        admitted,
        command_id,
        wire::COMMAND_TYPE_STORE_PURCHASE,
        &purchase,
    )
    .await;
    if status != CommandStatus::Accepted {
        return None;
    }
    let result = wire::decode_purchase_result(&bytes).ok()?;
    let runtime = owner.runtime.lock().await;
    if !super::super::battle_instinct_status::actor_is_current(&runtime, admitted)
        || owner.states.server_messages.connection_root(
            admitted.game_session_id,
            admitted.continuity.connection_generation,
        ) != Some(root)
    {
        return None;
    }
    let reply = owner.states.server_modals.finish_rule(root, answer)?;
    owner.states.server_modals.notify(
        root,
        Dialog {
            id: 1,
            title: "Store purchase".into(),
            message: format!("Purchase: {:?}", result.code),
            choices: Vec::new(),
            buttons: vec![(1, "Close".into())],
            cancel_button: Some(1),
            image: None,
        },
    )?;
    Some(reply)
}
