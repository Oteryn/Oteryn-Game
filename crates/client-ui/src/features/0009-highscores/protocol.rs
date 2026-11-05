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
    let mut query = HighscoresQuery {
        category: p.category,
        vocation: p.vocation,
        page: 1,
        snapshot_id: 0,
    };
    match action {
        Action::Refresh => {}
        Action::Filter { category, vocation } => {
            if !(1..=11).contains(&category) || vocation > 5 {
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
    let command = command(query).ok()?;
    if (p.category, p.vocation) != (query.category, query.vocation) {
        p.reply = None;
    }
    p.category = query.category;
    p.vocation = query.vocation;
    p.query = Some(query);
    p.pending = true;
    p.feedback = None;
    Some(command)
}
pub fn apply_result(p: &mut HighscoresPanel, r: &ServerMechanicResponse) {
    let Some(query) = p.query.take() else {
        return;
    };
    p.pending = false;
    if !matches!(
        r.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        p.feedback = Some(format!(
            "Server refused highscores: {:?}. Refresh to retry with the current snapshot.",
            r.status
        ));
        return;
    }
    match decode_highscores_result(&r.payload) {
        Ok(reply)
            if reply.category == query.category
                && reply.page == query.page
                && (query.snapshot_id == 0 || query.snapshot_id == reply.snapshot_id) =>
        {
            p.reply = Some(reply);
            p.feedback = None;
        }
        _ => p.feedback = Some("The server returned invalid highscores".into()),
    }
}
