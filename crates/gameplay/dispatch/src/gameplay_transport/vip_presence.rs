//! S04a presence: World hints are advisory; only current fenced SQL truth is disclosed.
use super::ComposedFreshAdmission;
use super::connection::AdmittedSession;
use crate::chat::{ChatText, relay::RelayDestination};
use crate::domain::character_name::CharacterName;
use crate::durability::vip_presence::VipPresenceRow;
use crate::foundation::GameSessionId;
use oteryn_protocol_oteryn::CharacterId;
use oteryn_protocol_oteryn::vip_presence::{self as wire, PresenceBatch, PresenceEntry, Status};
use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;
use std::time::Duration;
use tokio::time::Instant;

// Accepted VIP0 section 8.2 / VIP0-RL-09 and -16.
const RECONCILE: Duration = Duration::from_secs(60);
const CONFIRM: Duration = Duration::from_millis(wire::CONFIRMED_FOR_MS as u64);

struct Known {
    target: [u8; 16],
    visible: bool,
    value: PresenceEntry,
}

struct Watcher {
    generation: u64,
    serial: u64,
    ready: bool,
    due: Instant,
    dirty: bool,
    known: BTreeMap<u64, Known>,
    hints: BTreeMap<[u8; 16], bool>,
}

/// The existing Channel chat runtime owns this bounded index of its own sessions.
/// Each session holds at most 100 entries/hints; an unwatched target allocates nothing.
#[derive(Default)]
pub(super) struct VipPresenceRuntime {
    sessions: Mutex<HashMap<GameSessionId, Watcher>>,
}

struct ReadPlan {
    generation: u64,
    serial: u64,
    hints: BTreeMap<[u8; 16], bool>,
}

enum Refresh {
    Read(ReadPlan),
    Send(PresenceBatch),
}

impl VipPresenceRuntime {
    pub(super) fn forget(&self, session: GameSessionId) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.remove(&session);
        }
    }

    pub(super) fn hint(&self, target: CharacterId, online: bool) {
        if let Ok(mut sessions) = self.sessions.lock() {
            for watcher in sessions.values_mut() {
                if watcher.ready
                    && watcher
                        .known
                        .values()
                        .any(|entry| entry.target == *target.as_bytes())
                {
                    watcher.hints.insert(*target.as_bytes(), online);
                }
            }
        }
    }

    /// A local VIP list/consent write invalidates every local observer. Other nodes reconcile
    /// the same durable rows; in particular no ONLINE survives its ten-second confirmation.
    pub(super) fn invalidate(&self) {
        if let Ok(mut sessions) = self.sessions.lock() {
            for watcher in sessions.values_mut() {
                watcher.dirty = true;
            }
        }
    }

    fn begin(
        &self,
        session: GameSessionId,
        generation: u64,
        ready: bool,
        now: Instant,
    ) -> Option<Refresh> {
        let mut sessions = self.sessions.lock().ok()?;
        if sessions
            .get(&session)
            .is_some_and(|watcher| watcher.generation > generation)
        {
            return None;
        }
        if sessions
            .get(&session)
            .is_some_and(|watcher| watcher.generation != generation)
        {
            sessions.remove(&session);
        }
        let watcher = sessions.entry(session).or_insert_with(|| Watcher {
            generation,
            serial: 0,
            ready,
            due: now,
            dirty: true,
            known: BTreeMap::new(),
            hints: BTreeMap::new(),
        });
        if !ready {
            let changed = watcher.ready || watcher.dirty;
            watcher.ready = false;
            watcher.dirty = false;
            watcher.hints.clear();
            return changed.then(|| Refresh::Send(unconfirm(watcher)));
        }
        if !watcher.ready {
            watcher.ready = true;
            watcher.dirty = true;
            // Missed transitions after LISTEN reconnect are correction, never login notices.
            watcher.hints.clear();
        }
        if !watcher.dirty && watcher.hints.is_empty() && now < watcher.due {
            return None;
        }
        watcher.serial = watcher.serial.checked_add(1)?;
        watcher.dirty = false;
        watcher.due = now + RECONCILE;
        Some(Refresh::Read(ReadPlan {
            generation,
            serial: watcher.serial,
            hints: std::mem::take(&mut watcher.hints),
        }))
    }

    fn complete(
        &self,
        session: GameSessionId,
        plan: ReadPlan,
        rows: Option<Vec<VipPresenceRow>>,
        now: Instant,
    ) -> Option<PresenceBatch> {
        let mut sessions = self.sessions.lock().ok()?;
        let watcher = sessions.get_mut(&session)?;
        if watcher.generation != plan.generation || watcher.serial != plan.serial || !watcher.ready
        {
            return None;
        }
        let Some(rows) = rows.filter(|rows| rows.len() <= wire::MAX_ENTRIES) else {
            watcher.due = now + CONFIRM;
            return Some(unconfirm(watcher));
        };
        let previous = std::mem::take(&mut watcher.known);
        let mut entries = Vec::with_capacity(rows.len());
        for row in rows {
            let prior = previous
                .get(&row.entry)
                .filter(|entry| entry.target == *row.target.as_bytes());
            // Crystal player_vip.cpp:48-66; accepted VIP0 section 8.2 additionally requires
            // an authoritative read confirming the hinted transition. Initial snapshots,
            // consent/privacy changes, stale hints and periodic correction never announce.
            let announce = row.notify
                && row.visible
                && prior.is_some_and(|old| {
                    old.visible
                        && matches!(
                            (old.value.status, row.status),
                            (Status::Offline, Status::Online) | (Status::Online, Status::Offline)
                        )
                        && plan.hints.get(row.target.as_bytes())
                            == Some(&(row.status == Status::Online))
                });
            let value = PresenceEntry {
                entry: row.entry,
                name: row.name,
                status: row.status,
                announce,
                confirmed_for_ms: if row.status == Status::Online {
                    wire::CONFIRMED_FOR_MS
                } else {
                    0
                },
            };
            watcher.known.insert(
                row.entry,
                Known {
                    target: *row.target.as_bytes(),
                    visible: row.visible,
                    value: value.clone(),
                },
            );
            entries.push(value);
        }
        watcher.due = now
            + if entries.iter().any(|entry| entry.status == Status::Online) {
                CONFIRM
            } else {
                RECONCILE
            };
        Some(PresenceBatch { entries })
    }
}

fn unconfirm(watcher: &mut Watcher) -> PresenceBatch {
    let entries = watcher
        .known
        .values_mut()
        .map(|known| {
            if known.value.status == Status::Online {
                known.value.status = Status::Unconfirmed;
            }
            known.value.announce = false;
            known.value.confirmed_for_ms = 0;
            known.value.clone()
        })
        .collect();
    PresenceBatch { entries }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Called after committed login/terminal logout, including sessions without CHAT.
    /// The caller supplies the CharacterId from the admission or canonical terminal row,
    /// never a client target. Only a confirmed coarse status change can later announce.
    pub(super) async fn vip_presence_publish_target(&self, target: CharacterId, online: bool) {
        let Some(relay) = self.states.chat.relay() else {
            return;
        };
        let (Ok(name), Ok(text)) = (CharacterName::parse("Game"), ChatText::parse("presence"))
        else {
            return;
        };
        // No durable gameplay result depends on a best-effort relay send; the existing relay
        // bound and replay window apply and the indexed correction read recovers lost hints.
        let _ = relay
            .send(RelayDestination::Presence { target, online }, name, text)
            .await;
    }

    pub(super) async fn refresh_vip_presence(&self, admitted: AdmittedSession) -> Option<Vec<u8>> {
        let selected = admitted.continuity.selected_capabilities;
        if !selected.contains(wire::CAPABILITY_VIP_PRESENCE_V1)
            || !selected.contains(oteryn_protocol_oteryn::vip::CAPABILITY_VIP_V1)
            || !selected.contains(oteryn_protocol_oteryn::chat::CAPABILITY_CHAT_V1)
        {
            return None;
        }
        let (Some(actor), Some(fence)) = (admitted.runtime_actor, admitted.item_fence) else {
            return None;
        };
        if fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
            || fence.runtime_scope
                != crate::foundation::RuntimeScopeRefV1::channel(
                    admitted.world_id,
                    admitted.channel_id,
                )
            || self
                .runtime
                .lock()
                .await
                .player_control_facts(actor, admitted.game_session_id)
                .is_err()
        {
            return None;
        }
        let ready = self
            .states
            .chat
            .relay()
            .is_some_and(super::chat_relay::ChatRelay::is_ready);
        let refresh = self.states.chat.vip.begin(
            admitted.game_session_id,
            admitted.continuity.connection_generation,
            ready,
            Instant::now(),
        )?;
        let batch = match refresh {
            Refresh::Send(batch) => batch,
            Refresh::Read(plan) => {
                let rows = self
                    .root
                    .read_current_vip_presence(self.character, self.holder, fence)
                    .await
                    .ok();
                // A listener lost during the database read grants no ONLINE confirmation.
                let rows = self
                    .states
                    .chat
                    .relay()
                    .is_some_and(super::chat_relay::ChatRelay::is_ready)
                    .then_some(rows)
                    .flatten();
                self.states.chat.vip.complete(
                    admitted.game_session_id,
                    plan,
                    rows,
                    Instant::now(),
                )?
            }
        };
        for entry in batch.entries.iter().filter(|entry| entry.announce) {
            let verb = if entry.status == Status::Online {
                "in"
            } else {
                "out"
            };
            self.states.chat.push_party_spell_feedback(
                admitted.game_session_id,
                &format!("{} has logged {verb}.", entry.name),
            );
        }
        wire::encode(&batch).ok()
    }
}

#[cfg(test)]
#[path = "vip_presence_tests.rs"]
mod tests;
