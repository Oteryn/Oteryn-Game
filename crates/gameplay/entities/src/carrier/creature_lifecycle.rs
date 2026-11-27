//! Physical creature lifecycle diagnostics for correlation with monster AI and wire views.
use super::{ExactActorRef, LocalPosition};
use oteryn_error_codes::{FieldValue, Level, Line, emit};
use std::sync::OnceLock;

pub(super) fn record(
    actor: ExactActorRef,
    outcome: &'static str,
    reason: &'static str,
    movement: Option<(LocalPosition, LocalPosition)>,
) {
    const MODULE: &str = "monster_ai";
    static LEVEL: OnceLock<Level> = OnceLock::new();
    let level = *LEVEL.get_or_init(|| {
        if std::env::var("OTERYN_QUALIFICATION_LOG_PROFILE").as_deref() == Ok("monster_ai") {
            Level::Info
        } else {
            Level::Debug
        }
    });
    if !oteryn_error_codes::filter().allows(level, MODULE, false) {
        return;
    }
    let identity = u128::from_be_bytes(actor.placement_identity());
    let mut line = Line::new(level, MODULE, "creature_lifecycle")
        .field(
            "monster",
            FieldValue::Owned(format!(
                "{identity:032x}/{}",
                actor.actor_local_generation()
            )),
        )
        .field("outcome", FieldValue::Token(outcome))
        .field("reason", FieldValue::Token(reason));
    if let Some((from, to)) = movement {
        line = line
            .field("from", FieldValue::Owned(cell(from)))
            .field("to", FieldValue::Owned(cell(to)));
    }
    emit("oteryn-game-server", &line);
}

fn cell(position: LocalPosition) -> String {
    format!("{}/{}/{}", position.x, position.y, position.floor)
}
