//! Candidate readonly own-actor cooldown producer. Exact active book/group ordinals, absolute
//! monotonic expiries and current session binding are retained. RTT uses only matched existing
//! FND-02 probe/ack frames and owner monotonic send/receive time, never a client timestamp.
use super::super::actor_spell::ChannelSpellStates;
use super::*;
use crate::content::native_gameplay::NativeGameplayState;
use oteryn_protocol_oteryn::{Direction, MessageType, WireEnvelopeView};
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::collections::BTreeMap;
type Key = (wire::CooldownKind, u32);
pub(crate) struct CooldownRead<'a> {
    pub(crate) source: &'a ObserverSource<'a>,
    pub(crate) states: &'a ChannelSpellStates,
    pub(crate) active: &'a NativeGameplayState,
    pub(crate) now: SemanticTimeMicros,
}
#[derive(Debug, Default)]
pub(crate) struct CandidateCooldownPublisher {
    binding: Option<(ScopeContext, GameSessionId, u64)>,
    published: BTreeMap<Key, u64>,
    before_ack: BTreeMap<Key, wire::Cooldown>,
    probe: Option<(u64, u64)>,
    probe_high_water: u64,
    rtt: Option<u32>,
}
fn map_owned(
    read: &CooldownRead<'_>,
    observer: &Observer,
) -> Result<(BTreeMap<Key, u64>, usize), Error> {
    if read.active.source_digest() != observer.context.content {
        return Err(Error::Stale);
    }
    let state = read
        .states
        .get(read.source.runtime, observer.actor, observer.session)
        .ok_or(Error::Stale)?;
    let book = read.active.spell_book();
    let limit = book.source_len().checked_add(16).ok_or(Error::Bound)?;
    if limit > wire::MAX_COOLDOWNS {
        return Err(Error::Bound);
    }
    let groups: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tools/content-schema/spell-authoring/cooldown-groups.json"
    ))
    .map_err(|_| Error::Source)?;
    let groups = groups
        .get("groups")
        .and_then(|value| value.as_array())
        .ok_or(Error::Source)?;
    let mut mapped = BTreeMap::new();
    for (key, micros) in state.source_cooldown_deadlines() {
        let entry = if let Some(key) = key.strip_prefix("spell:") {
            let index = (1..=book.source_len())
                .find(|index| {
                    book.source_indexed(
                        std::num::NonZeroU32::new(*index as u32)
                            .expect("positive bounded book index"),
                    )
                    .is_some_and(|(spell, _)| spell.key == key)
                })
                .ok_or(Error::Source)?;
            (wire::CooldownKind::Spell, index as u32)
        } else if let Some(key) = key.strip_prefix("group:") {
            let ordinal = groups
                .iter()
                .position(|group| group.get("key").and_then(|value| value.as_str()) == Some(key))
                .ok_or(Error::Source)?;
            (wire::CooldownKind::Group, ordinal as u32 + 1)
        } else {
            return Err(Error::Source);
        };
        let expiry = micros.checked_add(999).ok_or(Error::Bound)? / 1000;
        if expiry > wire::MAX_CLOCK_MS || mapped.insert(entry, expiry).is_some() {
            return Err(Error::Bound);
        }
    }
    if mapped.len() > limit {
        return Err(Error::Bound);
    }
    Ok((mapped, limit))
}
impl CandidateCooldownPublisher {
    fn bind(&mut self, observer: &Observer) {
        let current = (
            observer.context.clone(),
            observer.session,
            observer.connection,
        );
        if self.binding.as_ref() != Some(&current) {
            *self = Self {
                binding: Some(current),
                ..Self::default()
            };
        }
    }
    fn message(
        &self,
        now: SemanticTimeMicros,
        entries: Vec<wire::Cooldown>,
    ) -> Result<wire::Cooldowns, Error> {
        let value = wire::Cooldowns {
            server_now_ms: now.get() / 1000,
            rtt_ms: self.rtt.unwrap_or(0),
            rtt_estimated: self.rtt.is_some(),
            entries,
        };
        wire::encode_cooldowns(&value).map_err(|_| Error::Wire)?;
        Ok(value)
    }
    fn retain_before_ack(&mut self, entries: &[wire::Cooldown], limit: usize) -> Result<(), Error> {
        if self.rtt.is_none() {
            for entry in entries {
                self.before_ack.insert((entry.kind, entry.id), *entry);
            }
            if self.before_ack.len() > limit {
                return Err(Error::Bound);
            }
        }
        Ok(())
    }
    pub(crate) async fn snapshot_current(
        &mut self,
        tx: &mut Transaction<'_, Postgres>,
        read: &CooldownRead<'_>,
    ) -> Result<Vec<u8>, SpellItemError> {
        let observer = current_observer(tx, read.source).await?;
        self.bind(&observer);
        let (mapped, limit) = map_owned(read, &observer).map_err(rejected)?;
        let entries = mapped
            .iter()
            .filter(|(_, expiry)| **expiry > read.now.get() / 1000)
            .map(|((kind, id), expiry)| wire::Cooldown {
                kind: *kind,
                id: *id,
                expires_at_ms: *expiry,
            })
            .collect::<Vec<_>>();
        let value = self.message(read.now, entries).map_err(rejected)?;
        let bytes = wire::encode_cooldowns(&value).map_err(|_| rejected(Error::Wire))?;
        self.before_ack.clear();
        self.retain_before_ack(&value.entries, limit)
            .map_err(rejected)?;
        self.published = mapped;
        Ok(bytes)
    }
    pub(crate) async fn delta_current(
        &mut self,
        tx: &mut Transaction<'_, Postgres>,
        read: &CooldownRead<'_>,
    ) -> Result<Option<Vec<u8>>, SpellItemError> {
        let observer = current_observer(tx, read.source).await?;
        self.bind(&observer);
        let (mapped, limit) = map_owned(read, &observer).map_err(rejected)?;
        let mut entries = Vec::new();
        for ((kind, id), expiry) in &mapped {
            if self.published.get(&(*kind, *id)) != Some(expiry) {
                entries.push(wire::Cooldown {
                    kind: *kind,
                    id: *id,
                    expires_at_ms: *expiry,
                });
            }
        }
        for ((kind, id), expiry) in &self.published {
            if !mapped.contains_key(&(*kind, *id)) && *expiry > read.now.get() / 1000 {
                entries.push(wire::Cooldown {
                    kind: *kind,
                    id: *id,
                    expires_at_ms: 0,
                });
            }
        }
        if entries.is_empty() {
            self.published = mapped;
            return Ok(None);
        }
        entries.sort_by_key(|entry| (entry.kind, entry.id));
        let value = self.message(read.now, entries).map_err(rejected)?;
        let bytes = wire::encode_cooldowns(&value).map_err(|_| rejected(Error::Wire))?;
        self.retain_before_ack(&value.entries, limit)
            .map_err(rejected)?;
        self.published = mapped;
        Ok(Some(bytes))
    }
    /// Called by the actual sender after handing off its existing FND probe. The encoded server
    /// frame, current canonical connection and monotonic timestamp are checked together.
    pub(crate) async fn record_after_probe_sent_current(
        &mut self,
        tx: &mut Transaction<'_, Postgres>,
        source: &ObserverSource<'_>,
        frame: &WireEnvelopeView<'_>,
        sent: SemanticTimeMicros,
    ) -> Result<(), SpellItemError> {
        let observer = current_observer(tx, source).await?;
        self.bind(&observer);
        frame
            .validate(Direction::ServerToClient, true)
            .map_err(|_| rejected(Error::Wire))?;
        if frame.message_type() != MessageType::LivenessProbe
            || frame.connection_generation() != observer.connection
        {
            return Err(rejected(Error::Stale));
        }
        let id = oteryn_protocol_oteryn::decode_liveness_probe(frame.payload())
            .map_err(|_| rejected(Error::Wire))?;
        self.register_probe(id, sent).map_err(rejected)
    }
    fn register_probe(&mut self, id: u64, sent: SemanticTimeMicros) -> Result<(), Error> {
        if id <= self.probe_high_water || self.probe.is_some() {
            return Err(Error::Stale);
        }
        self.probe = Some((id, sent.get()));
        self.probe_high_water = id;
        Ok(())
    }
    pub(crate) async fn observe_ack_current(
        &mut self,
        tx: &mut Transaction<'_, Postgres>,
        source: &ObserverSource<'_>,
        frame: &WireEnvelopeView<'_>,
        received: SemanticTimeMicros,
    ) -> Result<Option<Vec<u8>>, SpellItemError> {
        let observer = current_observer(tx, source).await?;
        self.bind(&observer);
        let ack = frame
            .liveness_ack(observer.connection)
            .map_err(|_| rejected(Error::Wire))?;
        self.accept_ack(ack.probe_id, received)
            .map_err(rejected)?
            .map(|value| wire::encode_cooldowns(&value).map_err(|_| rejected(Error::Wire)))
            .transpose()
    }
    fn accept_ack(
        &mut self,
        id: u64,
        received: SemanticTimeMicros,
    ) -> Result<Option<wire::Cooldowns>, Error> {
        let (probe, sent) = self.probe.ok_or(Error::Stale)?;
        if probe != id {
            return Err(Error::Stale);
        }
        let elapsed = received.get().checked_sub(sent).ok_or(Error::Stale)?;
        let sample = u32::try_from(elapsed.checked_add(999).ok_or(Error::Bound)? / 1000)
            .map_err(|_| Error::Bound)?;
        // Explicit candidate smoothing rule, not a new protocol or client timestamp: alpha=1/8.
        let first = self.rtt.is_none();
        self.rtt = Some(self.rtt.map_or(sample, |old| {
            ((u64::from(old) * 7 + u64::from(sample)) / 8) as u32
        }));
        self.probe = None;
        if !first {
            return Ok(None);
        }
        let entries = self.before_ack.values().copied().collect();
        let message = self.message(received, entries)?;
        self.before_ack.clear();
        Ok(Some(message))
    }
}
fn rejected(_error: Error) -> SpellItemError {
    SpellItemError::Rejected("candidate current cooldown source unavailable")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_ack_retains_only_latest_renewal_and_expired_entries_as_tombstones() {
        let mut owner = CandidateCooldownPublisher::default();
        let old = wire::Cooldown {
            kind: wire::CooldownKind::Spell,
            id: 1,
            expires_at_ms: 10,
        };
        owner.retain_before_ack(&[old], 10).unwrap();
        owner
            .retain_before_ack(
                &[
                    wire::Cooldown {
                        expires_at_ms: 100,
                        ..old
                    },
                    wire::Cooldown {
                        kind: wire::CooldownKind::Group,
                        id: 1,
                        expires_at_ms: 5,
                    },
                ],
                10,
            )
            .unwrap();
        owner
            .register_probe(1, SemanticTimeMicros::from_micros(1000))
            .unwrap();
        let correction = owner
            .accept_ack(1, SemanticTimeMicros::from_micros(21_000))
            .unwrap()
            .unwrap();
        assert_eq!(correction.rtt_ms, 20);
        assert!(correction.rtt_estimated);
        assert_eq!(correction.entries.len(), 2);
        assert_eq!(correction.entries[0].expires_at_ms, 100);
        assert_eq!(correction.entries[1].expires_at_ms, 5);
        assert!(owner.before_ack.is_empty());
        assert!(
            owner
                .accept_ack(1, SemanticTimeMicros::from_micros(22_000))
                .is_err()
        );
    }
    #[test]
    fn unknown_or_backward_ack_never_becomes_measurement() {
        let mut owner = CandidateCooldownPublisher::default();
        owner
            .register_probe(1, SemanticTimeMicros::from_micros(100))
            .unwrap();
        assert!(
            owner
                .accept_ack(2, SemanticTimeMicros::from_micros(200))
                .is_err()
        );
        assert!(
            owner
                .accept_ack(1, SemanticTimeMicros::from_micros(99))
                .is_err()
        );
        assert!(owner.rtt.is_none());
        assert_eq!(owner.probe, Some((1, 100)));
    }
}
