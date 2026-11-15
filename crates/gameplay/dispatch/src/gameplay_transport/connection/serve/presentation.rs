//! Negotiated presentation snapshots and authoritative own-actor cooldown deltas.
use super::*;
use crate::gameplay_transport::item_view::ItemViewSnapshotDomain;
use oteryn_protocol_oteryn::presentation as wire;

pub(super) struct SessionPresentation {
    revision: u64,
    rtt_ms: Option<u32>,
    entries: Vec<wire::ActorCooldown>,
}
impl SessionPresentation {
    pub(super) async fn initial<A: FreshAdmissionAuthority>(
        authority: &A,
        admitted: &AdmittedSession,
        domains: &mut Vec<ItemViewSnapshotDomain>,
    ) -> Result<Option<Self>, ServeEnd> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_PRESENTATION_V1)
        {
            return Ok(None);
        }
        let value = authority
            .observe_combat_cooldowns(*admitted)
            .await
            .ok_or(ServeEnd::Disconnected)?;
        let payload = wire::encode_actor_cooldowns(&value).map_err(|_| ServeEnd::Disconnected)?;
        domains.push(ItemViewSnapshotDomain {
            domain_id: wire::STATE_DOMAIN_WORLD_PRESENTATION,
            revision: admitted.continuity.presentation_revision,
            snapshot_type: wire::SNAPSHOT_TYPE_WORLD_PRESENTATION_V1,
            payload: Vec::new(),
        });
        domains.push(ItemViewSnapshotDomain {
            domain_id: wire::STATE_DOMAIN_ACTOR_COOLDOWNS,
            revision: admitted.continuity.server_sequence,
            snapshot_type: wire::SNAPSHOT_TYPE_ACTOR_COOLDOWNS_V1,
            payload,
        });
        Ok(Some(Self {
            rtt_ms: None,
            revision: admitted.continuity.server_sequence,
            entries: value.entries,
        }))
    }
    pub(super) async fn refresh<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
        &mut self,
        stream: &mut S,
        authority: &A,
        admitted: &mut AdmittedSession,
        sequence: &mut u64,
        rtt_ms: Option<u32>,
    ) -> Result<(), ServeEnd> {
        let Some(mut value) = authority.observe_combat_cooldowns(*admitted).await else {
            return Ok(());
        };
        value.rtt_ms = rtt_ms;
        let correction = self.rtt_ms.is_none() && rtt_ms.is_some();
        let current = value.entries.clone();
        value
            .entries
            .retain(|entry| correction || !self.entries.contains(entry));
        for previous in &self.entries {
            if !current
                .iter()
                .any(|entry| (entry.kind, entry.id) == (previous.kind, previous.id))
            {
                value.entries.push(wire::ActorCooldown {
                    expires_at_ms: 0,
                    ..*previous
                });
            }
        }
        if value.entries.is_empty() && !correction {
            return Ok(());
        }
        value.entries.sort_by_key(|entry| (entry.kind, entry.id));
        let payload = wire::encode_actor_cooldowns(&value).map_err(|_| ServeEnd::Disconnected)?;
        let next_sequence = sequence.checked_add(1).ok_or(ServeEnd::Disconnected)?;
        let frame = encode_state_delta(
            admitted.continuity.connection_generation,
            next_sequence,
            wire::STATE_DOMAIN_ACTOR_COOLDOWNS,
            self.revision,
            next_sequence,
            wire::DELTA_TYPE_ACTOR_COOLDOWNS_V1,
            &payload,
        )
        .map_err(|_| ServeEnd::Disconnected)?;
        write_frame(stream, &frame)
            .await
            .map_err(|_| ServeEnd::Disconnected)?;
        self.rtt_ms = rtt_ms;
        self.entries = current;
        self.revision = next_sequence;
        *sequence = next_sequence;
        admitted.continuity.server_sequence = next_sequence;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gameplay_transport::spell_presentation_publisher::cooldowns::active_cooldowns;
    use std::cell::{Cell, RefCell};
    use tokio::io::AsyncReadExt;

    struct Owner {
        book: crate::spell::SpellBook,
        state: RefCell<crate::spell::cast::PlayerSpellState>,
        now: Cell<u64>,
        available: Cell<bool>,
    }
    impl FreshAdmissionAuthority for Owner {
        async fn admit(
            &self,
            _: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }
        async fn observe_combat_cooldowns(
            &self,
            _: AdmittedSession,
        ) -> Option<wire::ActorCooldowns> {
            self.available.get().then_some(())?;
            active_cooldowns(&self.book, &self.state.borrow(), self.now.get())
        }
    }
    #[test]
    fn combat_cooldowns_snapshot_renewal_reset_expiry_and_reconnect_use_owner_deadlines()
    -> Result<(), String> {
        std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
            tokio::runtime::Builder::new_current_thread().enable_all().build()
                .map_err(|e| e.to_string())?.block_on(async {
                use crate::gameplay_transport::actor_spell::tests as fixture;
                let (_, actor, session) = fixture::runtime_with_player(0x73);
                let book = crate::spell::cast::v1_spell_book().map_err(|e| e.to_string())?;
                let mut state = crate::spell::cast::PlayerSpellState::new(fixture::FACTS, 0, 0).ok_or("state")?;
                let spell = book.spells.iter().find(|spell| !spell.groups.is_empty()).ok_or("spell")?;
                let spell_key = spell.key.clone();
                state.cooldowns.rearm(spell, oteryn_simulation_determinism::SemanticTimeMicros::from_micros(1001))
                    .map_err(|e| format!("{e:?}"))?;
                let owner = Owner { book, state: RefCell::new(state), now: Cell::new(2), available: Cell::new(true) };
                let mut admitted = AdmittedSession {
                    game_session_id: session, world_id: actor.world_id(), channel_id: actor.channel_id().ok_or("channel")?,
                    runtime_actor: Some(actor), first_entry: FirstEntryOutcome::Positioned, controller: None,
                    continuity: SessionContinuity::FRESH, item_fence: None,
                };
                let mut domains = Vec::new();
                assert!(SessionPresentation::initial(&owner, &admitted, &mut domains).await.map_err(|_| "unselected")?.is_none());
                assert!(domains.is_empty());
                admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                    &[OfferedCapability { id: wire::CAPABILITY_PRESENTATION_V1, requires: &[] }],
                    &[wire::CAPABILITY_PRESENTATION_V1]).ok_or("selection")?;
                let mut presentation = SessionPresentation::initial(&owner, &admitted, &mut domains)
                    .await.map_err(|_| "snapshot")?.ok_or("presentation")?;
                assert_eq!(domains.iter().map(|d| d.domain_id).collect::<Vec<_>>(), vec![19, 20]);
                assert!(domains[0].payload.is_empty());
                let snapshot = wire::decode_actor_cooldowns(&domains[1].payload).map_err(|e| format!("{e:?}"))?;
                assert!(snapshot.entries.iter().any(|entry| entry.kind == wire::ActorCooldownKind::Spell));
                assert!(snapshot.entries.iter().any(|entry| entry.kind == wire::ActorCooldownKind::Group));
                assert_eq!(snapshot.rtt_ms, None);
                let (mut server, mut client) = tokio::io::duplex(65536);
                let mut sequence = 0;
                presentation.refresh(&mut server, &owner, &mut admitted, &mut sequence, None).await.map_err(|_| "unchanged")?;
                assert_eq!(sequence, 0);
                for change in 0..4 {
                    match change {
                        0 => owner.now.set(3), // First matched ack corrects the active deadlines.
                        1 => {
                            let spell = owner.book.spells.iter().find(|s| s.key == spell_key).ok_or("spell")?;
                            owner.state.borrow_mut().cooldowns.rearm(spell, oteryn_simulation_determinism::SemanticTimeMicros::from_micros(2001))
                                .map_err(|e| format!("{e:?}"))?;
                        }
                        2 => owner.state.borrow_mut().cooldowns.groups.clear(),
                        _ => owner.now.set(wire::MAX_CLOCK_MS),
                    }
                    presentation.refresh(&mut server, &owner, &mut admitted, &mut sequence, Some(20)).await.map_err(|_| "refresh")?;
                    assert_eq!(sequence, change + 1);
                    let length = client.read_u32().await.map_err(|e| e.to_string())?;
                    let mut bytes = vec![0; length as usize];
                    client.read_exact(&mut bytes).await.map_err(|e| e.to_string())?;
                    let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&bytes).map_err(|e| e.to_string())?;
                    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(|e| e.to_string())?;
                    assert_eq!((delta.domain_id, delta.base_revision, delta.new_revision), (20, change, change + 1));
                    let value = wire::decode_actor_cooldowns(delta.payload).map_err(|e| format!("{e:?}"))?;
                    assert_eq!(value.rtt_ms, Some(20));
                    if change == 0 { assert_eq!(value.entries, snapshot.entries); }
                    if change >= 2 { assert!(value.entries.iter().all(|entry| entry.expires_at_ms == 0)); }
                    if change == 1 { assert!(value.entries.iter().all(|entry| entry.expires_at_ms % 1000 == 3)); }
                }
                owner.available.set(false);
                presentation.refresh(&mut server, &owner, &mut admitted, &mut sequence, Some(20)).await.map_err(|_| "unavailable")?;
                assert_eq!(sequence, 4);
                assert!(SessionPresentation::initial(&owner, &admitted, &mut Vec::new()).await.is_err());
                owner.available.set(true);
                let mut reconnect = Vec::new();
                SessionPresentation::initial(&owner, &admitted, &mut reconnect).await.map_err(|_| "reconnect")?;
                assert!(wire::decode_actor_cooldowns(&reconnect[1].payload).map_err(|e| format!("{e:?}"))?.entries.is_empty());
                Ok(())
            })
        }).map_err(|e| e.to_string())?.join().map_err(|_| "thread failed")?
    }
}
