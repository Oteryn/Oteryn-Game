//! Scripted owner outcomes through the real connection dispatch and VIS-2 interest query.
use super::*;
use crate::gameplay_transport::appearance_change::observation::Observation;
use oteryn_protocol_oteryn::appearance;
use oteryn_protocol_oteryn::{decode_command_result, decode_state_delta};

struct AppearanceChannel {
    channel: Channel,
    outcome: Option<appearance::ChangeResult>,
    revision: Cell<u64>,
    calls: Cell<usize>,
}
impl FreshAdmissionAuthority for AppearanceChannel {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }
    async fn observe(&self, actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        self.channel.observe(actor).await
    }
    async fn observe_visible_entities(&self, actor: ExactActorRef) -> Option<ChannelEntities> {
        self.channel.observe_visible_entities(actor).await
    }
    async fn appearance_change(
        &self,
        _: ExactActorRef,
        _: UseCommand,
        _: appearance::Change,
    ) -> Option<appearance::ChangeResult> {
        self.calls.set(self.calls.get() + 1);
        if let Some(appearance::ChangeResult::Committed(revision)) = self.outcome {
            self.revision.set(revision);
        }
        self.outcome
    }
    async fn appearance_observation(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
        entities: Vec<EntityRef>,
    ) -> Option<Observation> {
        let context = appearance::Context {
            content_generation: CONTENT,
            cosmetics_digest: [7; 32],
        };
        Some(Observation {
            catalogue: appearance::Catalogue {
                context,
                root_revision: self.revision.get(),
                selected: None,
                palette: vec![[0, 0, 0]; 133],
                outfits: vec![appearance::OutfitOption {
                    key: "oteryn:outfit.citizen".into(),
                    look_type: 128,
                    permitted_addons: 0,
                }],
                mounts: Vec::new(),
            },
            bindings: entities
                .into_iter()
                .map(|entity| appearance::Binding {
                    context,
                    appearance_ref: 1,
                    entity,
                    root_revision: self.revision.get(),
                    condition_revision: 1,
                    owner_time_us: 1,
                    look: appearance::Look::Outfit {
                        look_type: if self.revision.get() == 1 { 128 } else { 129 },
                        colours: [0; 4],
                        addons: 0,
                    },
                    selection_revision: None,
                })
                .collect(),
        })
    }
}

#[test]
fn appearance_change_real_dispatch_orders_stationary_updates_and_excludes_hidden_players()
-> TestResult {
    run(async {
        let outcome = Some(appearance::ChangeResult::Committed(2));
        scenario(&[6, 25], outcome).await
    })
}
#[test]
fn appearance_change_rejected_unresolved_and_downgraded_clients_publish_nothing() -> TestResult {
    run(async {
        scenario(&[6, 25], Some(appearance::ChangeResult::Rejected)).await?;
        scenario(&[6, 25], None).await?;
        scenario(&[6], Some(appearance::ChangeResult::Committed(2))).await
    })
}
async fn scenario(selected: &[u32], outcome: Option<appearance::ChangeResult>) -> TestResult {
    let mut shown = creature([9; 16], at(102, 100), 1);
    shown.kind = VisibleKind::Player;
    let mut hidden = creature([8; 16], at(1000, 1000), 1);
    hidden.kind = VisibleKind::Player;
    let authority = AppearanceChannel {
        channel: Channel::new(vec![shown, hidden]),
        outcome,
        revision: Cell::new(1),
        calls: Cell::new(0),
    };
    let mut admitted = session(&[6], SessionContinuity::FRESH);
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        PRODUCTION_OFFERED_CAPABILITIES,
        true,
    );
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(&offered, selected).ok_or("capabilities")?;
    let (mut client, mut server) = tokio::io::duplex(256 * 1024);
    let change = appearance::Change {
        context: appearance::Context {
            content_generation: CONTENT,
            cosmetics_digest: [7; 32],
        },
        expected_root_revision: 1,
        prior: None,
        selection: appearance::Selection {
            outfit_key: "oteryn:outfit.citizen".into(),
            colours: [1, 2, 3, 4],
            addons: 0,
            mount_key: None,
        },
        content_revision: "r1".into(),
    };
    let payload = change.encode().map_err(|e| format!("{e:?}"))?;
    let command = encode_client_command(
        1,
        &ClientCommandValue {
            command_id: 1,
            command_type: 25,
            payload: &payload,
        },
    )?;
    let serving = serve_admitted(&mut server, admitted, &authority, IDLE_LIVENESS);
    let observing = async {
        // Consume the actual join framing. Domain 21 must precede VIS-2 in its body.
        let _begin = read_one(&mut client).await?;
        let chunk = read_one(&mut client).await?;
        let envelope = decode_wire_envelope(&chunk)?;
        let framing = oteryn_protocol_oteryn::decode_snapshot_chunk_framing(envelope.payload())?;
        let domains = oteryn_protocol_oteryn::decode_snapshot_body(framing.1)?;
        if selected.contains(&25) {
            assert_eq!(domains[0].domain_id, 21);
            let snapshot =
                appearance::Snapshot::decode(domains[0].payload).map_err(|e| format!("{e:?}"))?;
            assert_eq!(snapshot.bindings.len(), 2);
            assert!(
                !snapshot
                    .bindings
                    .iter()
                    .any(|b| b.entity.identity == [8; 16])
            );
        } else {
            assert!(!domains.iter().any(|d| d.domain_id == 21));
        }
        let _commit = read_one(&mut client).await?;
        write_frame(&mut client, &command).await?;
        if selected.contains(&25) && outcome.is_none() {
            assert!(read_one(&mut client).await.is_err());
        } else {
            let result = read_one(&mut client).await?;
            let envelope = decode_wire_envelope(&result)?;
            let result = decode_command_result(envelope.payload())?;
            if selected.contains(&25) {
                assert_eq!(
                    appearance::ChangeResult::decode(result.payload)
                        .map_err(|e| format!("{e:?}"))?,
                    outcome.ok_or("outcome")?
                );
            } else {
                assert!(result.payload.is_empty());
            }
            if selected.contains(&25)
                && matches!(outcome, Some(appearance::ChangeResult::Committed(_)))
            {
                let catalogue = read_one(&mut client).await?;
                let envelope = decode_wire_envelope(&catalogue)?;
                assert_eq!(decode_state_delta(envelope.payload())?.domain_id, 21);
                let mappings = read_one(&mut client).await?;
                let envelope = decode_wire_envelope(&mappings)?;
                let delta = decode_state_delta(envelope.payload())?;
                assert_eq!((delta.domain_id, delta.delta_type), (21, 1));
                assert_eq!(
                    appearance::Mappings::decode(delta.payload)
                        .map_err(|e| format!("{e:?}"))?
                        .upserts
                        .len(),
                    2
                );
                let spatial = read_one(&mut client).await?;
                let envelope = decode_wire_envelope(&spatial)?;
                let delta = decode_state_delta(envelope.payload())?;
                assert_eq!(delta.domain_id, 1);
                let delta=oteryn_protocol_oteryn::world_spatial_entities::decode_world_spatial_entities_delta(delta.payload).map_err(|e|format!("{e:?}"))?;
                assert_eq!(delta.actor_position, at(100, 100));
                assert_eq!(delta.update.len(), 2);
                assert!(delta.enter.is_empty());
            } else {
                assert!(
                    tokio::time::timeout(
                        std::time::Duration::from_millis(150),
                        read_one(&mut client)
                    )
                    .await
                    .is_err()
                );
            }
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn Error>>(())
    };
    let observed = RefCell::new(None);
    let end = crate::gameplay_transport::first(serving, async {
        observed.replace(Some(observing.await));
    })
    .await;
    drop(server);
    if let Some(result) = observed.into_inner() {
        result?;
    } else {
        assert!(selected.contains(&25) && outcome.is_none());
        let Some(ConnectionEnd::AdmittedThenDisconnected(admitted)) = end else {
            return Err("unresolved connection".into());
        };
        assert_eq!(admitted.continuity.next_command_id, 1);
        assert_eq!(admitted.continuity.server_sequence, 0);
        assert!(read_one(&mut client).await.is_err());
    }
    assert_eq!(authority.calls.get(), usize::from(selected.contains(&25)));
    Ok(())
}
