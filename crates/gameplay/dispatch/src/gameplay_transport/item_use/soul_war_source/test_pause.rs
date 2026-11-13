//! Deterministic cancellation at actual source preparation and durable boundaries.
use super::{CommandId, CommandRef, UseCommand};
use std::cell::RefCell;
use std::sync::Arc;
use tokio::sync::Notify;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Point {
    BeforeNames,
    BeforeWrite,
    AfterWrite,
}

thread_local! {
    static PAUSE: RefCell<Option<(CommandRef, Point, Arc<Notify>)>> = const { RefCell::new(None) };
    static REFUSE_HANDOFF: RefCell<Option<CommandRef>> = const { RefCell::new(None) };
}

fn install(command: CommandRef, point: Point) -> Arc<Notify> {
    let reached = Arc::new(Notify::new());
    PAUSE.set(Some((command, point, Arc::clone(&reached))));
    reached
}

pub(super) async fn at(point: Point, command: UseCommand) {
    let Ok(id) = CommandId::new(command.command_id) else {
        return;
    };
    let command = CommandRef::new(command.game_session_id, id);
    let reached = PAUSE.with(|pause| {
        let mut pause = pause.borrow_mut();
        if pause
            .as_ref()
            .is_some_and(|(original, selected, _)| *original == command && *selected == point)
        {
            pause.take().map(|(_, _, reached)| reached)
        } else {
            None
        }
    });
    if let Some(reached) = reached {
        reached.notify_one();
        std::future::pending::<()>().await;
    }
}

pub(super) fn refuse_handoff(command: CommandRef) -> bool {
    REFUSE_HANDOFF.with(|selected| {
        let mut selected = selected.borrow_mut();
        if *selected == Some(command) {
            selected.take();
            true
        } else {
            false
        }
    })
}

impl super::ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) fn pause_soul_war_before_names_for_test(
        &self,
        command: CommandRef,
    ) -> Arc<Notify> {
        install(command, Point::BeforeNames)
    }

    pub(in crate::gameplay_transport) fn pause_soul_war_before_write_for_test(
        &self,
        command: CommandRef,
    ) -> Arc<Notify> {
        install(command, Point::BeforeWrite)
    }

    pub(in crate::gameplay_transport) fn pause_soul_war_after_write_for_test(
        &self,
        command: CommandRef,
    ) -> Arc<Notify> {
        install(command, Point::AfterWrite)
    }

    pub(in crate::gameplay_transport) fn refuse_soul_war_handoff_once_for_test(
        &self,
        command: CommandRef,
    ) {
        REFUSE_HANDOFF.set(Some(command));
    }
}
