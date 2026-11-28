//! Running-node planned reset composition. A failed probe cannot authorize a drain.
use super::*;
use crate::durability::world_reset::BundleHouseCatalogue;
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
            if observe_reset(root, world, state, &drain).await {
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

async fn observe_reset(
    root: &DurabilityRoot,
    world: WorldId,
    state: &ResetWatch,
    drain: &GameplayDrain,
) -> bool {
    if let Ok(Some(reset)) = root.retiring_world_reset(world).await {
        state.requested.store(true, Ordering::Release);
        drain.request(reset);
        return true;
    }
    false
}

pub(super) fn classify_exit(
    state: &ResetWatch,
    error: Option<crate::gameplay_transport::GameplayServeError>,
) -> Result<(), BootError> {
    if state.requested.load(Ordering::Acquire) {
        return Err(BootError::Readiness("World reset drain"));
    }
    if let Some(error) = error {
        return Err(BootError::GameplayServe(error));
    }
    Ok(())
}

pub(crate) async fn recover_retiring(
    root: &DurabilityRoot,
    config: &NodeConfig,
    world: WorldId,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    bundle: Option<&crate::map::boot::CheckedBundle>,
) -> Result<u64, BootError> {
    let retiring = with_retries(
        root,
        "World reset record",
        config.boot_pass_deadline(),
        boot_step_retryable,
        root_busy,
        || root.retiring_world_reset(world),
    )
    .await
    .map_err(|_| BootError::Readiness("World reset record"))?;
    if let Some(reset) = retiring {
        let bundle_houses = match (config.world_bundle.is_some(), bundle) {
            (true, Some(checked)) => {
                let (bytes, digest) = checked.encoded_bundle();
                Some(
                    BundleHouseCatalogue::decode(bytes, digest)
                        .map_err(|_| BootError::Readiness("World reset House qualification"))?,
                )
            }
            (false, None) => None,
            _ => return Err(BootError::Readiness("World reset House qualification")),
        };
        let source_manifest = std::env::var_os("OTERYN_NATIVE_GAMEPLAY_MANIFEST");
        let target_manifest = std::env::var_os("OTERYN_WORLD_RESET_TARGET_MANIFEST")
            .or_else(|| source_manifest.clone());
        let source = qualify_room(world, source_manifest.as_deref())?;
        let target = qualify_room(world, target_manifest.as_deref())?;
        let started = std::time::Instant::now();
        let wait_until =
            tokio::time::Instant::now() + Duration::from_millis(config.scope.assignment_wait_ms);
        loop {
            let finished = match &bundle_houses {
                Some(catalogue) => {
                    root.finish_bundle_world_reset(
                        authority,
                        node,
                        &reset,
                        (&source, &target),
                        catalogue,
                    )
                    .await
                }
                None => {
                    root.finish_world_reset(authority, node, &reset, &source, &target)
                        .await
                }
            };
            match finished {
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
                    if (boot_step_retryable(&error)
                        || matches!(
                            error,
                            crate::durability::DurabilityError::CommitTransactionTimeout
                        ))
                        && tokio::time::Instant::now() < wait_until => {}
                Err(_) => return Err(BootError::Readiness("World reset completion")),
            }
            tokio::time::sleep(ASSIGNMENT_POLL).await;
        }
    }
    Ok(root
        .active_world_event_epoch(world)
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
