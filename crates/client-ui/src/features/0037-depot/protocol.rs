use super::{DepotAction, DepotIntent, DepotOutcome, DepotPanel, DepotReply, Intent, ItemEntry};
pub fn request(p: &mut DepotPanel, intent: Intent) -> Option<DepotIntent> {
    if !p.selected {
        p.feedback = tr!("Depot is unavailable on this server");
        return None;
    }
    if p.pending {
        return None;
    }
    let reply_town = p.reply.as_ref().and_then(|r| r.town);
    let selected_town = p.selected_town.filter(|_| p.cross_town_selected);
    let (town, box_number, page, action) = match intent {
        Intent::Browse { box_number, page } if (1..=17).contains(&box_number) && page <= 468 => (
            selected_town.or(reply_town),
            box_number,
            page,
            DepotAction::Browse,
        ),
        Intent::Browse { .. } => return None,
        Intent::SelectTown(town) => {
            let box_number = p.reply.as_ref().map_or(1, |r| r.box_number);
            if p.cross_town_selected {
                p.selected_town = town;
                p.held = None;
            }
            (
                if p.cross_town_selected {
                    town
                } else {
                    reply_town
                },
                box_number,
                0,
                DepotAction::Browse,
            )
        }
        Intent::Deposit(handle) => {
            reply_town?;
            if !deposit_items(p).iter().any(|item| item.handle == handle) {
                return None;
            }
            let r = p.reply.as_ref()?;
            (
                reply_town,
                r.box_number,
                r.page,
                DepotAction::Deposit(handle),
            )
        }
        Intent::Withdraw(index) => {
            reply_town?;
            let r = p.reply.as_ref()?;
            (
                reply_town,
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

/// Tibia Global's Manual Sort Mode (capability 510) in a depot box. While `manual_sort` is on, a
/// deposit holds its item until the player picks a displayed entry row: the item is then
/// deposited at that row's box slot (`32 * page + row`). Picking the held item again deposits it
/// at the first slot. Without the mode nothing is held and every request is [`request`]'s.
pub fn request_in_mode(
    p: &mut DepotPanel,
    intent: Intent,
    manual_sort: bool,
) -> Option<(DepotIntent, Option<u32>)> {
    if !manual_sort {
        p.held = None;
        return request(p, intent).map(|intent| (intent, None));
    }
    match (intent, p.held) {
        (Intent::Deposit(handle), held) if held != Some(handle) => {
            if !p.selected
                || p.pending
                || p.reply.as_ref().and_then(|r| r.town).is_none()
                || !deposit_items(p).iter().any(|item| item.handle == handle)
            {
                return None;
            }
            p.held = Some(handle);
            p.feedback = tr!("Choose the depot slot for the item");
            None
        }
        (Intent::Withdraw(row), Some(handle)) => {
            let slot = p.placement_slot(row)?;
            let intent = request(p, Intent::Deposit(handle))?;
            p.held = None;
            Some((intent, Some(slot)))
        }
        (Intent::Deposit(handle), Some(_)) => {
            let intent = request(p, Intent::Deposit(handle))?;
            p.held = None;
            Some((intent, None))
        }
        (intent, _) => request(p, intent).map(|intent| (intent, None)),
    }
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
    let fallback_to_local = p.selected_town.is_some()
        && reply.as_ref().is_none_or(|reply| {
            matches!(reply.outcome, DepotOutcome::Stale | DepotOutcome::Rejected)
        });
    if fallback_to_local {
        p.selected_town = None;
        p.held = None;
    }
    let Some(mut reply) = reply else {
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
    if fallback_to_local {
        // Keep the box and page available for refresh while dropping the remote contents.
        reply.town = None;
        reply.total = 0;
        reply.entries.clear();
    }
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
