//! Running-node planned reset composition. A failed probe cannot authorize a drain.
use super::*;
use crate::gameplay_transport::gameplay_drain::GameplayDrain;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
pub(super) struct ResetWatch {
    requested: AtomicBool,
}

pub(super) async fn serve_watching_reset(
    listener: &tokio::net::TcpListener,
    config: GameplayListenerConfig,
    owners: GameplaySeamOwners<'_, '_, '_>,
    shutdown: &CancellationToken,
    state: &ResetWatch,
) -> Result<(), crate::gameplay_transport::GameplayServeError> {
    let drain = GameplayDrain::default();
    let root = owners.root;
    let world = owners.world_id;
    let watch = async {
        loop {
            if let Ok(Some(reset)) = root.retiring_world_reset(world).await {
                state.requested.store(true, Ordering::Release);
                drain.request(reset);
                std::future::pending::<()>().await;
            }
            if first(
                tokio::time::sleep(Duration::from_secs(5)),
                shutdown.cancelled(),
            )
            .await
            .is_none()
            {
                std::future::pending::<()>().await;
            }
        }
    };
    let serving =
        crate::gameplay_transport::serve_gameplay(listener, config, owners, shutdown, Some(&drain));
    match first(serving, watch).await {
        Some(result) => result,
        None => unreachable!("reset watch parks after requesting the sealed drain"),
    }
}

pub(super) fn classify_exit(
    state: &ResetWatch,
    error: Option<crate::gameplay_transport::GameplayServeError>,
) -> Result<(), BootError> {
    if state.requested.load(Ordering::Acquire) {
        return Err(BootError::Readiness("World reset drain"));
    }
    if error.is_some() {
        return Err(BootError::Serve);
    }
    Ok(())
}

pub(super) async fn require_no_retiring(
    root: &DurabilityRoot,
    material: &Material,
) -> Result<(), BootError> {
    let retiring = with_retries(
        root,
        "World reset record",
        material.config.boot_pass_deadline(),
        boot_step_retryable,
        || root.retiring_world_reset(material.world),
    )
    .await
    .map_err(|_| BootError::Readiness("World reset record"))?;
    if retiring.is_some() {
        return Err(BootError::Readiness("World reset recovery required"));
    }
    Ok(())
}

#[cfg(test)]
#[path = "world_reset_tests.rs"]
mod tests;
