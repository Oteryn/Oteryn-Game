//! Restartable private journal delivery beside the coin worker; no client needed.
use super::*;
use crate::durability::journal::ProducerStore;
use crate::native_admission_source::journal::publisher::{
    JournalDescriptor, JournalExportConfig, JournalRateGate, MtlsJournalPeer, Publisher,
};
fn configured(
    config: &super::super::super::config::NodeConfig,
) -> Result<Option<Vec<Publisher<MtlsJournalPeer>>>, BootError> {
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
    let gate = JournalRateGate::default();
    (0..4)
        .map(|_| {
            Publisher::new(
                JournalExportConfig {
                    game_journal_items_export_enabled: true,
                    ..Default::default()
                },
                MtlsJournalPeer::with_gate(descriptor.clone(), gate.clone()),
                journal.source_authority.clone(),
            )
            .map_err(|_| invalid("journal.source_authority"))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

pub(super) async fn run(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    config: &super::super::super::config::NodeConfig,
    stop: &CancellationToken,
) {
    let mut publishers = match configured(config) {
        Ok(value) => value,
        Err(_) => {
            super::emit(
                super::PROCESS,
                &oteryn_error_codes::Line::new(
                    oteryn_error_codes::Level::Error,
                    "node",
                    "game_journal_configuration",
                )
                .field("reason", oteryn_error_codes::FieldValue::Token("invalid")),
            );
            None
        }
    };
    let outbox = ProducerStore { root, authority };
    let mut ticks = tokio::time::interval(Duration::from_millis(250));
    ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        if first(ticks.tick(), stop.cancelled()).await.is_none() {
            break;
        }
        let _ = outbox.maintenance().await;
        let (Some(publishers), Some(journal)) = (publishers.as_mut(), config.journal.as_ref())
        else {
            continue;
        };
        // The shared peer gate limits the pool to 120 requests/minute and
        // one in-flight exchange for the consumer peer. Drain a bounded due batch, not
        // one stream per timer tick. Cancellation leaves expiring claims.
        if first(
            outbox.deliver_due(&journal.source_authority, publishers),
            stop.cancelled(),
        )
        .await
        .is_none()
        {
            break;
        }
    }
}
