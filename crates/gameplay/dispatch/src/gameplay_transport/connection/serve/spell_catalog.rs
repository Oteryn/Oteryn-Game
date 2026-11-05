//! Connection-local catalog revisions; complete replacement on change.
use super::*;
use oteryn_protocol_oteryn::actor_spell_catalog as wire;

pub(super) struct SessionCatalog {
    payload: Vec<u8>,
    revision: u64,
}

impl SessionCatalog {
    pub(super) async fn initial<A: FreshAdmissionAuthority>(
        authority: &A,
        actor: ExactActorRef,
        admitted: &AdmittedSession,
        domains: &mut Vec<crate::gameplay_transport::item_view::ItemViewSnapshotDomain>,
    ) -> Result<Option<Self>, ServeEnd> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_ACTOR_SPELL_CATALOG_V1)
        {
            return Ok(None);
        }
        let payload = crate::gameplay_transport::spell_catalog::observe(
            authority,
            actor,
            admitted.game_session_id,
        )
        .await
        .ok_or(ServeEnd::Disconnected)?;
        domains.push(
            crate::gameplay_transport::item_view::ItemViewSnapshotDomain {
                domain_id: wire::STATE_DOMAIN_ACTOR_SPELL_CATALOG,
                revision: 1,
                snapshot_type: 1,
                payload: payload.clone(),
            },
        );
        Ok(Some(Self {
            payload,
            revision: 1,
        }))
    }

    pub(super) async fn refresh<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
        &mut self,
        stream: &mut S,
        authority: &A,
        actor: ExactActorRef,
        admitted: &mut AdmittedSession,
        sequence: &mut u64,
    ) -> Result<(), ServeEnd> {
        // A temporarily unavailable read cannot manufacture a negative knowledge projection.
        // Retain the prior catalog and retry on the next owner cadence.
        let Some(payload) = crate::gameplay_transport::spell_catalog::observe(
            authority,
            actor,
            admitted.game_session_id,
        )
        .await
        else {
            return Ok(());
        };
        if payload == self.payload {
            return Ok(());
        }
        let next_sequence = sequence.checked_add(1).ok_or(ServeEnd::Disconnected)?;
        let revision = self.revision.checked_add(1).ok_or(ServeEnd::Disconnected)?;
        let frame = encode_state_delta(
            admitted.continuity.connection_generation,
            next_sequence,
            wire::STATE_DOMAIN_ACTOR_SPELL_CATALOG,
            self.revision,
            revision,
            1,
            &payload,
        )
        .map_err(|_| ServeEnd::Disconnected)?;
        write_frame(stream, &frame)
            .await
            .map_err(|_| ServeEnd::Disconnected)?;
        self.payload = payload;
        self.revision = revision;
        *sequence = next_sequence;
        admitted.continuity.server_sequence = next_sequence;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use tokio::io::AsyncReadExt;

    struct Authority {
        book: crate::spell::SpellBook,
        state: RefCell<crate::spell::cast::PlayerSpellState>,
    }
    impl FreshAdmissionAuthority for Authority {
        async fn admit(
            &self,
            _: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }
        async fn observe_spell_catalog(
            &self,
            _: ExactActorRef,
            _: GameSessionId,
        ) -> Option<Vec<wire::SpellCatalogEntry>> {
            crate::gameplay_transport::spell_catalog::catalog(&self.book, &self.state.borrow(), 0)
        }
    }

    #[test]
    fn spell_catalog_refreshes_level_learning_and_vocation_without_duplicates()
    -> Result<(), Box<dyn std::error::Error>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| e.to_string())?
                    .block_on(async {
                        use crate::gameplay_transport::actor_spell::tests as fixture;
                        let (_, actor, session) = fixture::runtime_with_player(0x70);
                        let authority = Authority {
                            book: crate::spell::cast::v1_spell_book()?,
                            state: RefCell::new(
                                crate::spell::cast::PlayerSpellState::new(fixture::FACTS, 0, 0)
                                    .ok_or("state")?,
                            ),
                        };
                        let mut admitted = AdmittedSession {
                            game_session_id: session,
                            world_id: actor.world_id(),
                            channel_id: actor.channel_id().ok_or("channel")?,
                            runtime_actor: Some(actor),
                            first_entry: FirstEntryOutcome::Positioned,
                            controller: None,
                            continuity: SessionContinuity::FRESH,
                            item_fence: None,
                        };
                        let mut domains = Vec::new();
                        assert!(
                            SessionCatalog::initial(&authority, actor, &admitted, &mut domains)
                                .await
                                .map_err(|_| "unselected")?
                                .is_none()
                        );
                        assert!(domains.is_empty());
                        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                            &[OfferedCapability {
                                id: wire::CAPABILITY_ACTOR_SPELL_CATALOG_V1,
                                requires: &[],
                            }],
                            &[wire::CAPABILITY_ACTOR_SPELL_CATALOG_V1],
                        )
                        .ok_or("selection")?;
                        let mut catalog =
                            SessionCatalog::initial(&authority, actor, &admitted, &mut domains)
                                .await
                                .map_err(|_| "initial")?
                                .ok_or("catalog")?;
                        assert_eq!(domains.len(), 1);
                        let (mut server, mut client) = tokio::io::duplex(65536);
                        let mut sequence = 0;
                        for change in 0..3 {
                            match change {
                                0 => authority.state.borrow_mut().facts.level = 1,
                                1 => authority.state.borrow_mut().install_learned_spells(
                                    std::collections::BTreeSet::from([authority.book.spells[0]
                                        .key
                                        .clone()]),
                                ),
                                _ => {
                                    authority.state.borrow_mut().facts.vocation =
                                        crate::spell::Vocation::Knight
                                }
                            }
                            catalog
                                .refresh(
                                    &mut server,
                                    &authority,
                                    actor,
                                    &mut admitted,
                                    &mut sequence,
                                )
                                .await
                                .map_err(|_| "refresh")?;
                            assert_eq!(sequence, change + 1);
                            let length = client.read_u32().await.map_err(|e| e.to_string())?;
                            let mut frame = vec![0; length as usize];
                            client
                                .read_exact(&mut frame)
                                .await
                                .map_err(|e| e.to_string())?;
                            let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&frame)
                                .map_err(|e| e.to_string())?;
                            let delta =
                                oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                                    .map_err(|e| e.to_string())?;
                            assert_eq!(delta.domain_id, wire::STATE_DOMAIN_ACTOR_SPELL_CATALOG);
                            assert_eq!(
                                (delta.base_revision, delta.new_revision),
                                (change + 1, change + 2)
                            );
                            assert_eq!(
                                wire::decode_catalog(delta.payload).map_err(|_| "decode")?,
                                authority
                                    .observe_spell_catalog(actor, session)
                                    .await
                                    .ok_or("observation")?
                            );
                            catalog
                                .refresh(
                                    &mut server,
                                    &authority,
                                    actor,
                                    &mut admitted,
                                    &mut sequence,
                                )
                                .await
                                .map_err(|_| "unchanged")?;
                            assert_eq!(sequence, change + 1);
                        }
                        Ok::<(), String>(())
                    })
            })?
            .join()
            .map_err(|_| "catalog scenario thread panicked")?
            .map_err(Into::into)
    }
}
