use super::{DepotAction, DepotIntent, DepotOutcome, DepotPanel, DepotReply, Intent, ItemEntry};
pub fn request(p: &mut DepotPanel, intent: Intent) -> Option<DepotIntent> {
    if !p.selected {
        p.feedback = tr!("Depot is unavailable on this server");
        return None;
    }
    if p.pending {
        return None;
    }
    let town = p.reply.as_ref().and_then(|r| r.town);
    let (box_number, page, action) = match intent {
        Intent::Browse { box_number, page } if (1..=17).contains(&box_number) && page <= 468 => {
            (box_number, page, DepotAction::Browse)
        }
        Intent::Browse { .. } => return None,
        Intent::Deposit(handle) => {
            town?;
            if !deposit_items(p).iter().any(|item| item.handle == handle) {
                return None;
            }
            let r = p.reply.as_ref()?;
            (r.box_number, r.page, DepotAction::Deposit(handle))
        }
        Intent::Withdraw(index) => {
            town?;
            let r = p.reply.as_ref()?;
            (
                r.box_number,
                r.page,
                DepotAction::Withdraw(r.entries.get(index)?.ordinal),
            )
        }
    };
    p.pending = true;
    p.feedback = tr!("Waiting for server");
    Some(DepotIntent {
        town,
        box_number,
        page,
        action,
    })
}

/// Only server-issued carried inventory and bag handles can be deposited.
pub fn deposit_items(p: &DepotPanel) -> Vec<ItemEntry> {
    let mut items = Vec::new();
    for item in p
        .items
        .inventory
        .main_backpack
        .iter()
        .chain(p.items.inventory.entries.iter())
        .chain(p.items.inventory.equipment.iter().map(|e| &e.item))
        .chain(p.items.views.iter())
    {
        if !items.iter().any(|e: &ItemEntry| e.handle == item.handle) {
            items.push(*item);
        }
    }
    items
}

pub fn apply_result(p: &mut DepotPanel, reply: Option<DepotReply>) {
    p.pending = false;
    let Some(reply) = reply else {
        p.reply = None;
        p.feedback = tr!("The server refused the depot request");
        return;
    };
    p.feedback = match reply.outcome {
        DepotOutcome::Opened => tr!("Depot box loaded"),
        DepotOutcome::Moved => tr!("Item moved"),
        DepotOutcome::NoLocker => tr!("No depot locker nearby"),
        DepotOutcome::Stale => tr!("The depot or item changed; refresh and try again"),
        DepotOutcome::Full => tr!("The destination is full"),
        DepotOutcome::NoBackpack => tr!("Equip a main backpack first"),
        DepotOutcome::Overweight => tr!("You cannot carry this weight"),
        DepotOutcome::Unsupported => tr!("This item cannot be moved through the depot"),
        DepotOutcome::Rejected => tr!("The server refused the depot move"),
    };
    p.reply = Some(reply);
}

/// The depot source grid may open a carried container without moving it.
pub fn request_container(
    p: &mut DepotPanel,
    handle: super::ItemHandle,
) -> Option<super::ItemHandle> {
    if p.pending || !p.selected {
        return None;
    }
    if !p.items.containers_selected {
        p.feedback = tr!("Container browsing is unavailable on this server");
        return None;
    }
    if !deposit_items(p).iter().any(|e| e.handle == handle) {
        return None;
    }
    p.pending = true;
    p.feedback = tr!("Waiting for server");
    Some(handle)
}
pub fn apply_container(p: &mut DepotPanel, outcome: Option<super::ContainerViewOutcome>) {
    use super::ContainerViewOutcome::*;
    p.pending = false;
    p.feedback = match outcome {
        Some(Opened) => tr!("Container loaded"),
        Some(Closed) => tr!("Container closed"),
        Some(Stale) => tr!("The container changed; refresh and try again"),
        Some(TooFar) => tr!("The container is too far away"),
        Some(NotAContainer) => tr!("This item is not a container"),
        Some(TooManyViews) => tr!("Too many containers are open"),
        None => tr!("The server refused the container request"),
    };
}
