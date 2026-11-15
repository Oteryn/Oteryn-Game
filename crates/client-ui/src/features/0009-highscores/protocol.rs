use super::{Action, HighscoresPanel};
use oteryn_client_runtime::highscores::{
    command,
    wire::{HighscoresQuery, decode_highscores_result},
};
use oteryn_client_runtime::server_mechanic::{
    CommandStatus, ServerMechanicCommand, ServerMechanicResponse,
};
pub fn request(p: &mut HighscoresPanel, action: Action) -> Option<ServerMechanicCommand> {
    if !p.selected || p.pending {
        return None;
    }
    if matches!(action, Action::Catalogue)
        || (matches!(action, Action::Refresh) && p.catalogue_selected && !p.catalogue_loaded)
    {
        if !p.catalogue_selected {
            return None;
        }
        p.pending = true;
        p.catalogue_pending = true;
        p.query = None;
        p.reply = None;
        p.feedback = None;
        return Some(ServerMechanicCommand {
            capability: 507,
            command_type: 6244,
            payload: Vec::new(),
        });
    }
    p.catalogue_pending = false;
    let mut world_type = p.world_type;
    let mut query = HighscoresQuery {
        world: p.world,
        details: true,
        category: p.category,
        vocation: p.vocation,
        page: 1,
        snapshot_id: 0,
    };
    match action {
        Action::Catalogue => return None,
        Action::WorldType(kind) => {
            if !p.catalogue_selected {
                return None;
            }
            world_type = kind;
            let target = query
                .world
                .or_else(|| p.reply.as_ref().and_then(|reply| reply.world));
            if !p.catalogue.iter().any(|entry| {
                Some(entry.world) == target && kind.is_none_or(|kind| entry.world_type == kind)
            }) {
                query.world = Some(
                    p.catalogue
                        .iter()
                        .find(|entry| kind.is_none_or(|kind| entry.world_type == kind))?
                        .world,
                );
            }
        }
        Action::Refresh => {}
        Action::World(world) => {
            query.world = world;
        }
        Action::ShowMe => {
            let reply = p.reply.as_ref()?;
            if reply.own_page.is_none()
                && reply.own_row.as_ref().is_some_and(|own| {
                    own.rank
                        > u64::from(
                            oteryn_client_runtime::highscores::wire::MAX_HIGHSCORES_LIST_ROWS,
                        )
                })
            {
                p.feedback = Some(
                    tr!(
                        "Your rank is outside the published top 1000; your rank remains shown below"
                    )
                    .into(),
                );
                return None;
            }
            let Some(page) = reply.own_page else {
                p.feedback = Some("Your character is not in this World's selected list".into());
                return None;
            };
            query.page = page;
            query.snapshot_id = reply.snapshot_id;
        }
        Action::Filter { category, vocation } => {
            if crate::highscores_loyalty::category_selection(category, p.loyalty_selected).is_none()
                || vocation > 5
            {
                return None;
            }
            query.category = category;
            query.vocation = vocation;
        }
        Action::Next | Action::Previous => {
            let reply = p.reply.as_ref()?;
            if matches!(action, Action::Next) {
                if !reply.has_more {
                    return None;
                }
                query.page = reply.page + 1;
            } else {
                if reply.page <= 1 {
                    return None;
                }
                query.page = reply.page - 1;
            }
            query.snapshot_id = reply.snapshot_id;
        }
    }
    let command = if p.catalogue_selected {
        ServerMechanicCommand {
            capability: 507,
            command_type: 6245,
            payload: oteryn_client_runtime::highscores::catalogue::encode_query(query, world_type)
                .ok()?,
        }
    } else {
        command(query).ok()?
    };
    if (p.world, p.category, p.vocation, p.world_type)
        != (query.world, query.category, query.vocation, world_type)
    {
        p.reply = None;
    }
    p.world_type = world_type;
    p.world = query.world;
    p.category = query.category;
    p.vocation = query.vocation;
    p.query = Some(query);
    p.pending = true;
    p.feedback = None;
    Some(command)
}
pub fn apply_result(p: &mut HighscoresPanel, r: &ServerMechanicResponse) {
    if p.catalogue_pending {
        p.catalogue_pending = false;
        p.pending = false;
        if matches!(
            r.status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        ) && let Ok(catalogue) =
            oteryn_client_runtime::highscores::catalogue::decode_catalogue(&r.payload)
        {
            p.catalogue = catalogue;
            p.catalogue_loaded = true;
            p.feedback = None;
        } else {
            p.feedback = Some("The server returned an invalid World catalogue".into());
        }
        return;
    }
    let Some(query) = p.query.take() else {
        return;
    };
    p.pending = false;
    if !matches!(
        r.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        p.feedback = Some(lfmt!(
            "Server refused highscores: {:?}. Refresh to retry with the current snapshot.",
            r.status
        ));
        return;
    }
    match decode_highscores_result(&r.payload) {
        Ok(reply)
            if query.world.is_none_or(|world| reply.world == Some(world))
                && reply.category == query.category
                && reply.page == query.page
                && (query.snapshot_id == 0 || query.snapshot_id == reply.snapshot_id) =>
        {
            p.reply = Some(reply);
            p.feedback = None;
        }
        _ => p.feedback = Some(tr!("The server returned invalid highscores").into()),
    }
}

pub fn world_choices(
    p: &HighscoresPanel,
) -> impl Iterator<Item = &oteryn_client_runtime::highscores::catalogue::Entry> {
    p.catalogue
        .iter()
        .filter(|entry| p.world_type.is_none_or(|kind| entry.world_type == kind))
}
