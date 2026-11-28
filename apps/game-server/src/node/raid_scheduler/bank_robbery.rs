//! Supervise normal-map projection of the committed World occurrence.
use super::*;
use oteryn_gameplay_world::bank_robbery::project;

pub(super) async fn observe(
    scheduler: &RaidScheduler<'_, '_, '_>,
    generation: ScopeOwnershipGeneration,
    sites: [bool; 4],
) {
    let scope = crate::foundation::RuntimeScopeRefV1::channel(scheduler.world, scheduler.channel);
    let projection = match scheduler
        .root
        .poll_bank_robbery(
            scheduler.character,
            scheduler.node,
            scope,
            generation,
            scheduler.reset_epoch,
            sites,
        )
        .await
    {
        Ok(projection) => projection,
        Err(error) => {
            emit(
                PROCESS,
                &Line::new(Level::Warn, "node", "bank_robbery_poll")
                    .field("state", FieldValue::Token("retry"))
                    .field("error", FieldValue::Owned(error.to_string())),
            );
            return;
        }
    };
    if let Some(robbery) = projection
        && let Ok(mut runtime) = scheduler.runtime.try_lock()
        && let Err(error) = project(&mut runtime, scheduler.movement_cells, robbery)
    {
        emit(
            PROCESS,
            &Line::new(Level::Warn, "node", "bank_robbery_projection")
                .field("state", FieldValue::Token("retry"))
                .field("error", FieldValue::Owned(format!("{error:?}"))),
        );
    }
}
