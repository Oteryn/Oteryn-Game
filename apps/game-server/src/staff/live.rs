//! GM-1 (D1437): the node's live view for staff actions. Every admitted
//! connection registers its GameSession here, and the staff control executor
//! finds a kick target and pushes a committed mute through it. Nothing here is
//! durable: a mute's durable record is its sanction row, which chat admission
//! reads back, so a restart or reconnect keeps it.
use sha2::{Digest as _, Sha256};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};

/// How long a kick waits for the connection to run its fenced release; it
/// stays inside the control socket's 30 s deadline after the audit commit.
pub const KICK_DEADLINE: std::time::Duration = std::time::Duration::from_secs(10);

/// What the connection did with one kick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KickResult {
    /// The fenced logout release committed and the connection closed.
    Released,
    /// A pending durable spell commit kept the session; nothing changed.
    Busy,
    /// The actor is in fight (ATTACK-0 §4); the logout rule kept the session.
    InFight,
    /// The release outcome is unknown; the control-loss lifecycle reconciles it.
    Unknown,
    /// No connection serves the session any more.
    Gone,
}
impl KickResult {
    pub fn token(self) -> &'static str {
        match self {
            Self::Released => "RELEASED",
            Self::Busy => "BUSY",
            Self::InFight => "IN_FIGHT",
            Self::Unknown => "UNKNOWN",
            Self::Gone => "GONE",
        }
    }
}

/// One kick handed to the connection serving the session.
pub struct KickRequest {
    reply: oneshot::Sender<KickResult>,
}
impl KickRequest {
    pub fn answer(self, result: KickResult) {
        let _ = self.reply.send(result);
    }
}

struct Live {
    registration: u64,
    account: [u8; 16],
    character: Option<[u8; 16]>,
    kick: mpsc::Sender<KickRequest>,
}

/// The live sessions and committed mutes of this node.
#[derive(Default)]
pub struct StaffDesk {
    live: Mutex<HashMap<[u8; 16], Live>>,
    /// CharacterId to the end (Unix ms) of its longest active staff mute.
    mutes: Mutex<HashMap<[u8; 16], u64>>,
    next: AtomicU64,
}

/// The registration of one connection; dropping it unregisters that connection
/// only, never a newer connection of the same session.
pub struct LiveGuard {
    desk: Arc<StaffDesk>,
    session: [u8; 16],
    registration: u64,
    kicks: mpsc::Receiver<KickRequest>,
}
impl LiveGuard {
    /// The next kick for this connection; never ready while none is sent.
    pub async fn next_kick(&mut self) -> KickRequest {
        match self.kicks.recv().await {
            Some(kick) => kick,
            None => std::future::pending().await,
        }
    }
}
impl Drop for LiveGuard {
    fn drop(&mut self) {
        if let Ok(mut live) = self.desk.live.lock()
            && live
                .get(&self.session)
                .is_some_and(|entry| entry.registration == self.registration)
        {
            live.remove(&self.session);
        }
    }
}

/// D1437: a Kick by account names `sha256("oteryn-ops-account-ref-v1" ||
/// environment || raw AccountId)` in lowercase hex, never the raw AccountId.
pub fn account_ref(environment: &str, account: &[u8; 16]) -> String {
    let mut digest = Sha256::new();
    digest.update(b"oteryn-ops-account-ref-v1");
    digest.update(environment.as_bytes());
    digest.update(account);
    digest
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

impl StaffDesk {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Registers the connection serving `session`, replacing an older one.
    pub fn register(
        self: &Arc<Self>,
        session: [u8; 16],
        account: [u8; 16],
        character: Option<[u8; 16]>,
    ) -> LiveGuard {
        let registration = self.next.fetch_add(1, Ordering::Relaxed);
        let (kick, kicks) = mpsc::channel(1);
        if let Ok(mut live) = self.live.lock() {
            live.insert(
                session,
                Live {
                    registration,
                    account,
                    character,
                    kick,
                },
            );
        }
        LiveGuard {
            desk: Arc::clone(self),
            session,
            registration,
            kicks,
        }
    }

    pub fn session_for_character(&self, character: &[u8; 16]) -> Option<[u8; 16]> {
        let live = self.live.lock().ok()?;
        live.iter()
            .find(|(_, entry)| entry.character.as_ref() == Some(character))
            .map(|(session, _)| *session)
    }

    pub fn session_for_account_ref(&self, environment: &str, wanted: &str) -> Option<[u8; 16]> {
        let live = self.live.lock().ok()?;
        live.iter()
            .find(|(_, entry)| account_ref(environment, &entry.account) == wanted)
            .map(|(session, _)| *session)
    }

    pub fn is_live(&self, session: &[u8; 16]) -> bool {
        self.live
            .lock()
            .is_ok_and(|live| live.contains_key(session))
    }

    /// Asks the connection serving `session` to run its fenced release.
    pub async fn kick(&self, session: &[u8; 16]) -> KickResult {
        let Some(sender) = self
            .live
            .lock()
            .ok()
            .and_then(|live| live.get(session).map(|entry| entry.kick.clone()))
        else {
            return KickResult::Gone;
        };
        let (reply, answer) = oneshot::channel();
        let exchange = async {
            if sender.send(KickRequest { reply }).await.is_err() {
                return KickResult::Gone;
            }
            answer.await.unwrap_or(KickResult::Gone)
        };
        tokio::time::timeout(KICK_DEADLINE, exchange)
            .await
            .unwrap_or(KickResult::Unknown)
    }

    /// Records the end of `character`'s longest active staff mute, or none.
    pub fn set_mute(&self, character: [u8; 16], ends_at_ms: Option<u64>) {
        if let Ok(mut mutes) = self.mutes.lock() {
            match ends_at_ms {
                Some(ends) => {
                    mutes.insert(character, ends);
                }
                None => {
                    mutes.remove(&character);
                }
            }
        }
    }

    /// Whole seconds left of `character`'s staff mute at `now_ms`, rounded up.
    pub fn mute_seconds(&self, character: &[u8; 16], now_ms: u64) -> Option<u32> {
        let ends = *self.mutes.lock().ok()?.get(character)?;
        (ends > now_ms).then(|| u32::try_from((ends - now_ms).div_ceil(1_000)).unwrap_or(u32::MAX))
    }
}

/// Unix wall-clock milliseconds, the clock a sanction's `ends_at` is written on.
pub fn wall_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    #[test]
    fn a_dropped_older_registration_keeps_the_newer_one() {
        let desk = StaffDesk::new();
        let older = desk.register([1; 16], [2; 16], Some([3; 16]));
        let newer = desk.register([1; 16], [2; 16], Some([3; 16]));
        drop(older);
        assert_eq!(desk.session_for_character(&[3; 16]), Some([1; 16]));
        assert_eq!(
            desk.session_for_account_ref("alpha", &account_ref("alpha", &[2; 16])),
            Some([1; 16])
        );
        assert_eq!(
            desk.session_for_account_ref("beta", &account_ref("alpha", &[2; 16])),
            None
        );
        drop(newer);
        assert!(!desk.is_live(&[1; 16]));
    }

    #[test]
    fn mute_seconds_round_up_and_end() {
        let desk = StaffDesk::new();
        desk.set_mute([4; 16], Some(10_500));
        assert_eq!(desk.mute_seconds(&[4; 16], 10_000), Some(1));
        assert_eq!(desk.mute_seconds(&[4; 16], 8_000), Some(3));
        assert_eq!(desk.mute_seconds(&[4; 16], 10_500), None);
        desk.set_mute([4; 16], None);
        assert_eq!(desk.mute_seconds(&[4; 16], 0), None);
    }

    #[test]
    fn a_kick_reaches_the_registered_connection_or_is_gone() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        runtime.block_on(async {
            let desk = StaffDesk::new();
            assert_eq!(desk.kick(&[9; 16]).await, KickResult::Gone);
            let mut guard = desk.register([9; 16], [8; 16], None);
            let serving = tokio::spawn(async move {
                guard.next_kick().await.answer(KickResult::Released);
                guard
            });
            assert_eq!(desk.kick(&[9; 16]).await, KickResult::Released);
            drop(serving.await.expect("serving"));
            assert_eq!(desk.kick(&[9; 16]).await, KickResult::Gone);
        });
    }
}
