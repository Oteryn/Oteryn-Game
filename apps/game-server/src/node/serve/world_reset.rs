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
    reset_epoch: u64,
) -> Result<(), crate::gameplay_transport::GameplayServeError> {
    let drain = GameplayDrain::at_reset_epoch(reset_epoch);
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

pub(super) async fn recover_retiring(
    root: &DurabilityRoot,
    material: &Material,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
) -> Result<u64, BootError> {
    let retiring = with_retries(
        root,
        "World reset record",
        material.config.boot_pass_deadline(),
        boot_step_retryable,
        || root.retiring_world_reset(material.world),
    )
    .await
    .map_err(|_| BootError::Readiness("World reset record"))?;
    if let Some(reset) = retiring {
        // A full bundle still has Unknown House metadata (D1246). Never use
        // the entry-room proof to authorize retirement of a bundle World.
        if material.config.world_bundle.is_some() {
            return Err(BootError::Readiness("World reset House qualification"));
        }
        let source_manifest = std::env::var_os("OTERYN_NATIVE_GAMEPLAY_MANIFEST");
        let target_manifest = std::env::var_os("OTERYN_WORLD_RESET_TARGET_MANIFEST")
            .or_else(|| source_manifest.clone());
        let source = qualify_room(material.world, source_manifest.as_deref())?;
        let target = qualify_room(material.world, target_manifest.as_deref())?;
        let started = std::time::Instant::now();
        let wait_until = tokio::time::Instant::now()
            + Duration::from_millis(material.config.scope.assignment_wait_ms);
        loop {
            match root
                .finish_world_reset(authority, node, &reset, &source, &target)
                .await
            {
                Ok(Some(epoch)) => {
                    let elapsed_ms = started.elapsed().as_millis();
                    eprintln!(
                        "event=world_reset_activated elapsed_ms={elapsed_ms} limit=MAP01-RESET-RETIRE-MS limit_ms=600000 world_reset_alarm={} reset_epoch={epoch}",
                        elapsed_ms > 600_000
                    );
                    return Ok(epoch);
                }
                Ok(None) if tokio::time::Instant::now() < wait_until => {}
                Ok(None) => return Err(BootError::Readiness("World reset drain wait")),
                Err(crate::durability::item_decay_retire::DecayRetireError::Unavailable(error))
                    if boot_step_retryable(&error) && tokio::time::Instant::now() < wait_until => {}
                Err(_) => return Err(BootError::Readiness("World reset completion")),
            }
            tokio::time::sleep(ASSIGNMENT_POLL).await;
        }
    }
    Ok(root
        .active_world_reset_epoch(material.world)
        .await
        .map_err(|_| BootError::Readiness("World reset active epoch"))?
        .unwrap_or(0))
}

fn qualify_room(
    world: WorldId,
    manifest: Option<&std::ffi::OsStr>,
) -> Result<crate::content::QualifiedNativeEntryRoom, BootError> {
    match manifest {
        Some(path) => {
            crate::content::qualify_native_entry_room_from_gameplay_manifest(world, Path::new(path))
        }
        None => crate::content::qualify_native_entry_room(world),
    }
    .map_err(|_| BootError::Readiness("World reset content qualification"))
}

#[cfg(test)]
#[path = "world_reset_tests.rs"]
mod tests;
