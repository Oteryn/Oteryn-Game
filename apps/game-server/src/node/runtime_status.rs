//! Wires the runtime-status reporter into the serve lifecycle
//! (`oteryn-game-native-runtime-status-v1` §8). Nothing here gates boot,
//! serving or shutdown.

use super::uuid_text;
use crate::durability::DurabilityRoot;
use crate::durability::runtime_scope_assignment::{AssignmentState, NodeRegistrationFact};
use crate::foundation::RuntimeScopeRefV1;
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardStateV1, AdmissionAuthorityPublicationV1,
};
use crate::native_admission_source::runtime_status::{self, Publication, RuntimeStatusDescriptor};
use oteryn_foundation::CancellationToken;
use std::sync::Arc;
use tokio::sync::watch;

pub(super) struct Reporter {
    latest: watch::Sender<Option<Publication>>,
    serving: CancellationToken,
    task: tokio::task::JoinHandle<()>,
    epoch: u64,
    node_id: String,
    scope: (String, String),
}

/// Field-by-field projection of the one committed Runtime change (§4).
fn project(
    committed: &AdmissionAuthorityPublicationV1,
    world_id: &str,
    channel_id: &str,
    node_id: &str,
    epoch: u64,
) -> Option<Publication> {
    let [change] = committed.changes() else {
        return None;
    };
    let AdmissionAuthorityGuardStateV1::Runtime {
        ownership_generation,
        ready,
        route_revision,
        runtime_observation_revision,
        protocol_major,
        transport_profile,
        ruleset_revision,
        content_revision,
        map_revision,
        world_policy_revision,
        offer_revision,
    } = &change.state
    else {
        return None;
    };
    Some(Publication {
        source_authority: change.source.authority.clone(),
        world_id: world_id.to_owned(),
        channel_id: channel_id.to_owned(),
        node_id: node_id.to_owned(),
        assignment_epoch: epoch,
        scope_ownership_generation: *ownership_generation,
        source_revision: change.source.source_revision,
        decision_identity: change.source.decision_identity.clone(),
        ready: *ready,
        published_at: change.source.source_observed_at,
        protocol_major: *protocol_major,
        transport_profile: *transport_profile,
        route_revision: route_revision.clone(),
        runtime_observation_revision: runtime_observation_revision.clone(),
        ruleset_revision: ruleset_revision.clone(),
        content_revision: content_revision.clone(),
        map_revision: map_revision.clone(),
        world_policy_revision: world_policy_revision.clone(),
        offer_revision: offer_revision.clone(),
    })
}

impl Reporter {
    /// Starts reporting after the `ready = true` commit (§8.1). Heartbeats
    /// run only while serving, the durability root is ready and the scope is
    /// still assigned to this incarnation under `generation` (§8.2).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn start(
        descriptor: Arc<RuntimeStatusDescriptor>,
        epoch: u64,
        root: DurabilityRoot,
        scope: RuntimeScopeRefV1,
        (world_id, channel_id): (&str, &str),
        fact: NodeRegistrationFact,
        generation: u64,
        committed: &AdmissionAuthorityPublicationV1,
        clock: fn() -> i64,
    ) -> Self {
        let node_id = uuid_text(fact.node_id().as_bytes());
        let first = project(committed, world_id, channel_id, &node_id, epoch);
        let (latest, receiver) = watch::channel(first);
        let serving = CancellationToken::new();
        let gate_serving = serving.clone();
        let gate = move || {
            let (root, serving) = (root.clone(), gate_serving.clone());
            async move {
                !serving.is_cancelled()
                    && root.is_ready()
                    && matches!(
                        root.read_runtime_scope_assignment(scope).await,
                        Ok(Some(current)) if current.state == AssignmentState::Assigned
                            && current.holder == Some(fact)
                            && current.ownership_generation == generation
                    )
            }
        };
        let task = tokio::spawn(runtime_status::run(
            descriptor,
            receiver,
            runtime_status::HEARTBEAT,
            clock,
            gate,
        ));
        Self {
            latest,
            serving,
            task,
            epoch,
            node_id,
            scope: (world_id.to_owned(), channel_id.to_owned()),
        }
    }

    /// Shutdown began: no further heartbeat (§8.3).
    pub(super) fn stop_heartbeats(&self) {
        self.serving.cancel();
    }

    /// A later definite commit (the `ready = false` withdrawal, §8.1).
    pub(super) fn publish(&self, committed: &AdmissionAuthorityPublicationV1) {
        let (world_id, channel_id) = &self.scope;
        if let Some(publication) =
            project(committed, world_id, channel_id, &self.node_id, self.epoch)
        {
            self.latest.send_replace(Some(publication));
        }
    }

    /// Best effort within the shutdown budget: the pending report is sent,
    /// then the reporter ends or is abandoned at the deadline.
    pub(super) async fn finish(self, deadline: tokio::time::Instant) {
        self.serving.cancel();
        drop(self.latest);
        let mut task = self.task;
        if tokio::time::timeout_at(deadline, &mut task).await.is_err() {
            task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use crate::foundation::admission_authority_publication::{
        AdmissionAuthorityGuardKeyV1, AdmissionAuthorityOwningPublisherV1,
        AdmissionAuthorityPublicationChangeV1, AdmissionAuthorityPublicationErrorV1,
        AdmissionPublicationPreconditionV1, AdmissionPublicationPurposeV1,
        AdmissionPublicationSourceV1,
    };
    use crate::foundation::{ChannelId, WorldId};

    struct Committed(AdmissionAuthorityPublicationChangeV1);
    impl crate::foundation::fnd04_verifier::fresh_source_sealed::Sealed for Committed {}
    impl AdmissionAuthorityOwningPublisherV1 for Committed {
        fn resolve_publication(
            &self,
            _now: i64,
        ) -> Result<Vec<AdmissionAuthorityPublicationChangeV1>, AdmissionAuthorityPublicationErrorV1>
        {
            Ok(vec![self.0.clone()])
        }
    }

    const WORLD: &str = "01934f10-7c02-7001-805b-3b1122334401";
    const CHANNEL: &str = "01934f10-7c03-7001-805b-3b1122334401";
    const NODE: &str = "01934f10-7c04-7001-805b-3b1122334401";

    #[test]
    fn the_report_equals_the_committed_publication_field_by_field() {
        let uuid = |text| super::super::uuid_bytes(text).expect("uuid");
        let world = WorldId::decode(&uuid(WORLD)).expect("world");
        let channel = ChannelId::decode(&uuid(CHANNEL)).expect("channel");
        let now = 1_790_000_000;
        let change = AdmissionAuthorityPublicationChangeV1 {
            key: AdmissionAuthorityGuardKeyV1::Runtime(RuntimeScopeRefV1::channel(world, channel)),
            source: AdmissionPublicationSourceV1 {
                authority: "oteryn:runtime:world-1:channel-1".into(),
                purpose: AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness,
                source_revision: 7,
                decision_identity: "runtime-readiness:0a:3:7:true".into(),
                source_observed_at: now,
                clock_uncertainty_seconds: 0,
            },
            precondition: AdmissionPublicationPreconditionV1::Bootstrap {
                restored_publication_high_water: Some(0),
            },
            publication_revision: 1,
            state: AdmissionAuthorityGuardStateV1::Runtime {
                ownership_generation: 3,
                ready: true,
                route_revision: "rt.4.0f3a9c1d2b7e4a5f6c8d9e0a1b2c3d4e".into(),
                runtime_observation_revision: "observation-1".into(),
                protocol_major: 1,
                transport_profile: 1,
                ruleset_revision: "ruleset-1".into(),
                content_revision: "content-1".into(),
                map_revision: "map-1".into(),
                world_policy_revision: "policy-1".into(),
                offer_revision: "offer-1".into(),
            },
        };
        let committed = AdmissionAuthorityPublicationV1::prepare(&Committed(change), now)
            .expect("committed publication");
        let report = project(&committed, WORLD, CHANNEL, NODE, 2).expect("runtime change");
        assert_eq!(
            report,
            Publication {
                source_authority: "oteryn:runtime:world-1:channel-1".into(),
                world_id: WORLD.into(),
                channel_id: CHANNEL.into(),
                node_id: NODE.into(),
                assignment_epoch: 2,
                scope_ownership_generation: 3,
                source_revision: 7,
                decision_identity: "runtime-readiness:0a:3:7:true".into(),
                ready: true,
                published_at: now,
                protocol_major: 1,
                transport_profile: 1,
                route_revision: "rt.4.0f3a9c1d2b7e4a5f6c8d9e0a1b2c3d4e".into(),
                runtime_observation_revision: "observation-1".into(),
                ruleset_revision: "ruleset-1".into(),
                content_revision: "content-1".into(),
                map_revision: "map-1".into(),
                world_policy_revision: "policy-1".into(),
                offer_revision: "offer-1".into(),
            }
        );
        assert!(runtime_status::encode(&report, now).is_ok());
    }
}
