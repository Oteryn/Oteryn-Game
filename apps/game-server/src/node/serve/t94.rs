//! T94 owner 2026-11-06: Game production is on by default; transport refuses missing identity.
use super::*;
use crate::durability::world_event_outcomes::ProducerStore;
use crate::native_admission_source::world_event_outcomes::fence::LeagueEpochFenceFile as EpochFenceFile;
use crate::native_admission_source::world_event_outcomes::publisher::{
    LeagueDescriptor, MtlsLeagueSink, Publisher,
};
use std::sync::{Arc, Mutex};
pub(super) struct Material {
    pub(super) audit_key_id: Option<String>,
    descriptor: Option<LeagueDescriptor>,
    authority: String,
    fence: std::path::PathBuf,
    defaults: crate::native_admission_source::world_event_outcomes::ProducerDefaults,
    request: Option<String>,
}
pub(super) fn material(
    platform: &super::super::config::PlatformConfig,
    endpoint: (String, u16),
    roots: &[CertificateDer<'static>],
    evidence: &[CertificateDer<'static>],
) -> Result<Option<Material>, BootError> {
    let Some(c) = &platform.world_event_outcomes else {
        return Ok(None);
    };
    if !c.enabled {
        return Ok(None);
    }
    let secret =
        |key, path: &Path, max| read_file(key, path, FileClass::Secret, effective_uid(), max);
    let read_chain = |path: &Path| {
        certificates(&secret(
            "platform.world_event_outcomes",
            path,
            MAX_PEM_BYTES,
        )?)
        .map_err(|_| invalid("platform.world_event_outcomes"))
    };
    // Invalid/missing T94 identity refuses only this transport. Source capture and
    // the Game authority remain functional; no fallback certificate is allowed.
    let descriptor = (|| -> Result<LeagueDescriptor, BootError> {
        let own = read_chain(&c.client_certificate_file)?;
        let key = private_key(
            "platform.world_event_outcomes.client_key_file",
            &secret(
                "platform.world_event_outcomes.client_key_file",
                &c.client_key_file,
                MAX_GAMEPLAY_KEY_BYTES,
            )?,
        )?;
        let mut others = vec![evidence.to_vec()];
        if let Some(c) = &platform.runtime_status {
            others.push(read_chain(&c.client_certificate_file)?);
        }
        if let Some(c) = &platform.account_characters {
            others.push(read_chain(&c.client_certificate_file)?);
        }
        if let Some(c) = &platform.character_commands {
            others.push(read_chain(&c.client_certificate_file)?);
        }
        if let Some(c) = platform.public_projections.as_ref().filter(|p| p.enabled) {
            others.push(read_chain(&c.client_certificate_file)?);
        }
        if let Some(c) = &platform.bazaar {
            others.push(read_chain(&c.client_certificate_file)?);
        }
        let references: Vec<_> = others.iter().map(Vec::as_slice).collect();
        LeagueDescriptor::new(
            endpoint,
            platform.peer_name.clone(),
            roots.to_vec(),
            own,
            key,
            &references,
        )
        .map_err(|_| invalid("platform.world_event_outcomes"))
    })()
    .ok();
    if descriptor.is_none() {
        event("event=t94 family=all operation=connect result=PLATFORM_UNAVAILABLE timing_ms=0");
    }
    Ok(Some(Material {
        audit_key_id: descriptor.as_ref().map(audit_key_id),
        descriptor,
        authority: c.source_authority.clone(),
        fence: c.epoch_fence_file.clone(),
        defaults: c.defaults.clone(),
        request: c.resync_request_id.clone(),
    }))
}
// The fingerprint is provenance for locally captured T94 envelopes, not a delivery receipt.
pub(super) fn audit_key_id(descriptor: &LeagueDescriptor) -> String {
    descriptor.public_key_id().to_owned()
}

pub(super) async fn run(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    material: Option<Material>,
    world: WorldId,
    stop: &CancellationToken,
) {
    let Some(material) = material else { return };
    let Some(descriptor) = material.descriptor else {
        let mut bootstrap = EpochFenceFile::new(material.fence.clone());
        while !stop.is_cancelled() {
            let _ = root
                .t94_initialize(authority, &material.authority, &mut bootstrap)
                .await;
            let _ = root
                .t94_start_world(authority, world, &material.defaults, &material.authority)
                .await;
            let _ = root.t94_produce(authority).await;
            if first(
                tokio::time::sleep(std::time::Duration::from_secs(1)),
                stop.cancelled(),
            )
            .await
            .is_none()
            {
                return;
            }
        }
        return;
    };
    let mut retained_fence = EpochFenceFile::new(material.fence.clone());
    if let Some(request) = &material.request {
        let _ = root
            .t94_start_world(authority, world, &material.defaults, &material.authority)
            .await;
        loop {
            let _ = root
                .t94_initialize(authority, &material.authority, &mut retained_fence)
                .await;
            if root
                .t94_begin_resync(authority, request, &mut retained_fence)
                .await
                .is_ok()
            {
                break;
            }
            event("event=t94 family=all operation=resync result=PLATFORM_UNAVAILABLE timing_ms=0");
            if first(
                tokio::time::sleep(std::time::Duration::from_secs(1)),
                stop.cancelled(),
            )
            .await
            .is_none()
            {
                return;
            }
        }
    }
    let fence = Arc::new(Mutex::new(retained_fence));
    let a = Publisher {
        enabled: true,
        clock: SystemClock::default(),
        store: ProducerStore {
            root,
            authority,
            world,
            defaults: material.defaults.clone(),
            source_authority: material.authority.clone(),
            epoch_fence_file: material.fence.clone(),
        },
        sink: MtlsLeagueSink::new(descriptor.clone()),
        fence: Arc::clone(&fence),
        family: "world_event_outcome",
        source_authority: material.authority.clone(),
    };
    let b = Publisher {
        enabled: true,
        clock: SystemClock::default(),
        store: ProducerStore {
            root,
            authority,
            world,
            defaults: material.defaults.clone(),
            source_authority: material.authority.clone(),
            epoch_fence_file: material.fence.clone(),
        },
        sink: MtlsLeagueSink::new(descriptor),
        fence,
        family: "guild_season",
        source_authority: material.authority,
    };
    first(both(a.run(), b.run()), stop.cancelled()).await;
}
