//! Restartable private journal delivery beside the coin worker; no client needed.
use super::*;
use crate::durability::journal::ProducerStore;
use crate::native_admission_source::journal::publisher::{
    JournalDescriptor, JournalExportConfig, MtlsJournalPeer, Publisher, Step,
};
fn configured(
    config: &super::super::super::config::NodeConfig,
) -> Result<Option<Publisher<MtlsJournalPeer>>, BootError> {
    if !config.game_journal_items_export_enabled {
        return Ok(None);
    }
    let journal = config.journal.as_ref().ok_or_else(|| invalid("journal"))?;
    let secret = |key: &'static str, path: &std::path::Path, max| {
        read_file(key, path, FileClass::Secret, effective_uid(), max)
    };
    let platform = &config.platform;
    let roots = certificates(&secret(
        "platform.trust_roots_file",
        &platform.trust_roots_file,
        MAX_PEM_BYTES,
    )?)
    .map_err(|_| invalid("journal"))?;
    let chain = certificates(&secret(
        "journal.client_certificate_file",
        &journal.client_certificate_file,
        MAX_PEM_BYTES,
    )?)
    .map_err(|_| invalid("journal"))?;
    let key = private_key(
        "journal.client_key_file",
        &secret(
            "journal.client_key_file",
            &journal.client_key_file,
            MAX_GAMEPLAY_KEY_BYTES,
        )?,
    )?;
    let mut paths = vec![
        &platform.client_certificate_file,
        &config.listener.certificate_chain_file,
    ];
    paths.extend(
        platform
            .runtime_status
            .as_ref()
            .map(|c| &c.client_certificate_file),
    );
    paths.extend(
        platform
            .account_characters
            .as_ref()
            .map(|c| &c.client_certificate_file),
    );
    paths.extend(
        platform
            .character_commands
            .as_ref()
            .map(|c| &c.client_certificate_file),
    );
    paths.extend(platform.bazaar.as_ref().map(|c| &c.client_certificate_file));
    paths.extend(
        platform
            .public_projections
            .as_ref()
            .map(|c| &c.client_certificate_file),
    );
    paths.extend(
        platform
            .world_event_outcomes
            .as_ref()
            .map(|c| &c.client_certificate_file),
    );
    let other = paths
        .into_iter()
        .map(|p| {
            certificates(&secret("journal.identity_separation", p, MAX_PEM_BYTES)?)
                .map_err(|_| invalid("journal.identity_separation"))
        })
        .collect::<Result<Vec<_>, BootError>>()?;
    let refs = other.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let descriptor = JournalDescriptor::new(
        (platform.endpoint.ip().to_string(), platform.endpoint.port()),
        platform.peer_name.clone(),
        roots,
        chain,
        key,
        &refs,
    )
    .map_err(|_| invalid("journal.identity_separation"))?;
    Publisher::new(
        JournalExportConfig {
            game_journal_items_export_enabled: true,
            ..Default::default()
        },
        MtlsJournalPeer::new(descriptor),
        journal.source_authority.clone(),
    )
    .map(Some)
    .map_err(|_| invalid("journal.source_authority"))
}
pub(super) async fn run(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    config: &super::super::super::config::NodeConfig,
    stop: &CancellationToken,
) {
    let mut publisher = match configured(config) {
        Ok(value) => value,
        Err(_) => {
            eprintln!("event=game_journal_configuration reason=invalid");
            None
        }
    };
    let outbox = ProducerStore { root, authority };
    let mut ticks = tokio::time::interval(Duration::from_secs(1));
    ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut pending = None;
    loop {
        if first(ticks.tick(), stop.cancelled()).await.is_none() {
            break;
        }
        let _ = outbox.maintenance().await;
        if let Some((stream, attempt, delay, progress)) = &pending {
            if outbox
                .schedule(stream, *attempt, *delay, *progress)
                .await
                .is_ok()
            {
                pending = None;
            }
            continue;
        }
        let Some(publisher) = publisher.as_mut() else {
            continue;
        };
        let Some(journal) = config.journal.as_ref() else {
            continue;
        };
        let Ok(Some((stream, attempt))) = outbox.next_due(&journal.source_authority).await else {
            continue;
        };
        publisher.restore_retry_attempt(attempt);
        let started = std::time::Instant::now();
        let Some(step) = first(publisher.step(&stream, &outbox), stop.cancelled()).await else {
            break;
        };
        let (delay, progress) = match step {
            Step::Accepted => (Duration::ZERO, true),
            Step::Idle => (
                Duration::from_secs(28).saturating_sub(started.elapsed()),
                true,
            ),
            Step::RetryAfter(delay) => (delay, false),
            Step::Disabled => (Duration::from_secs(29), false),
            Step::Quarantined => {
                eprintln!("event=game_journal_unavailable reason=authority_or_history");
                (Duration::from_secs(29), false)
            }
        };
        if outbox
            .schedule(&stream, publisher.retry_attempt(), delay, progress)
            .await
            .is_err()
        {
            // A failed schedule write never permits an early network retry.
            pending = Some((stream, publisher.retry_attempt(), delay, progress));
        }
    }
}
