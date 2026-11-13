//! End-to-end consumer loop against a peer that rejects overlapping exchanges.
use super::*;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use tokio::sync::Notify;
#[derive(Clone, Default)]
struct Clock(Arc<AtomicU64>);
impl ReportClock for Clock {
    fn elapsed(&self) -> Duration {
        Duration::from_millis(self.0.load(Ordering::SeqCst))
    }
    fn unix_now(&self) -> i64 {
        1790000202
    }
    async fn sleep(&self, duration: Duration) {
        self.0.fetch_add(
            duration.as_millis().try_into().unwrap_or(u64::MAX),
            Ordering::SeqCst,
        );
        tokio::task::yield_now().await;
    }
}
#[derive(Default)]
struct State {
    operations: Vec<Operation>,
    active: usize,
    maximum: usize,
    published: bool,
}
struct Peer {
    state: Arc<Mutex<State>>,
    done: Arc<Notify>,
}
impl CommandSource for Peer {
    async fn exchange(&mut self, operation: Operation, body: &str) -> Result<Vec<u8>, Failure> {
        {
            let mut s = self.state.lock().map_err(|_| Failure::Unavailable)?;
            s.active += 1;
            s.maximum = s.maximum.max(s.active);
            assert_eq!(s.active, 1, "one exchange per consumer");
            s.operations.push(operation);
        }
        tokio::task::yield_now().await;
        let reply = match operation {
            Operation::ListPendingCharacterCommandsV1 => {
                br#"{"contract_version":1,"pending":[]}"#.to_vec()
            }
            Operation::ListPendingCharacterCommandsV2 => {
                crate::character_services_v2::ListPendingCharacterCommandsV2::decode(
                    body.as_bytes(),
                )
                .map_err(|_| Failure::InvalidResponse)?;
                br#"{"contract_version":2,"pending":[{"command":"RenameCharacter","operation_id":"0192a000-0000-7000-8000-000000000003","source_revision":"19"}]}"#.to_vec()
            }
            Operation::ReadCharacterCommandIntentV2 => {
                crate::character_services_v2::ReadCharacterCommandIntentV2::decode(body.as_bytes())
                    .map_err(|_| Failure::InvalidResponse)?;
                include_bytes!(
                    "../../../../docs/contracts/fixtures/character-services-v2/rename.json"
                )
                .to_vec()
            }
            Operation::PublishCharacterCommandReceiptV2 => {
                assert_eq!(
                    body.as_bytes(),
                    include_bytes!(
                        "../../../../docs/contracts/fixtures/character-services-v2/rejected.json"
                    )
                );
                self.state
                    .lock()
                    .map_err(|_| Failure::Unavailable)?
                    .published = true;
                br#"{"contract_version":2,"result":"accepted"}"#.to_vec()
            }
            _ => {
                self.state.lock().map_err(|_| Failure::Unavailable)?.active -= 1;
                return Err(Failure::InvalidResponse);
            }
        };
        let finish = {
            let mut s = self.state.lock().map_err(|_| Failure::Unavailable)?;
            s.active -= 1;
            s.published
                || s.operations
                    .iter()
                    .filter(|&&op| op == Operation::ListPendingCharacterCommandsV1)
                    .count()
                    >= 2
        };
        if finish {
            self.done.notify_one();
        }
        Ok(reply)
    }
}
struct FixedDecider;
impl Decider for FixedDecider {
    async fn decide(&mut self, _command: &str, _operation: &str, _input: Input<'_>) -> Decision {
        Decision::Transient
    }
    async fn decide_services(
        &mut self,
        command: &str,
        operation: &str,
        input: Input<'_>,
    ) -> Decision {
        assert_eq!(command, "RenameCharacter");
        assert_eq!(operation, "0192a000-0000-7000-8000-000000000003");
        assert_eq!(
            input,
            Input::Intent(include_bytes!(
                "../../../../docs/contracts/fixtures/character-services-v2/rename.json"
            ))
        );
        Decision::Publish {
            receipt: include_bytes!(
                "../../../../docs/contracts/fixtures/character-services-v2/rejected.json"
            )
            .to_vec(),
            result_code: "CHAR_CMD_NAME_UNAVAILABLE".into(),
        }
    }
}
fn scenario(enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let state = Arc::new(Mutex::new(State::default()));
            let done = Arc::new(Notify::new());
            let consumer = Consumer::new(
                Clock::default(),
                Peer {
                    state: state.clone(),
                    done: done.clone(),
                },
                FixedDecider,
                "oteryn:character-authority:primary".into(),
                |_| {},
            )
            .with_services(enabled);
            let mut running = std::pin::pin!(consumer.run());
            let mut completed = std::pin::pin!(done.notified());
            std::future::poll_fn(|cx| {
                if completed.as_mut().poll(cx).is_ready() {
                    return std::task::Poll::Ready(Ok::<(), Box<dyn std::error::Error>>(()));
                }
                if running.as_mut().poll(cx).is_ready() {
                    return std::task::Poll::Ready(Err("configured consumer ended".into()));
                }
                std::task::Poll::Pending
            })
            .await?;
            let state = state.lock().map_err(|_| "peer state poisoned")?;
            assert_eq!(state.maximum, 1);
            assert_eq!(state.active, 0);
            assert_eq!(state.published, enabled);
            assert_eq!(
                state
                    .operations
                    .contains(&Operation::ReadCharacterCommandIntentV2),
                enabled
            );
            assert_eq!(
                state
                    .operations
                    .contains(&Operation::ListPendingCharacterCommandsV2),
                enabled
            );
            assert!(
                state
                    .operations
                    .contains(&Operation::ListPendingCharacterCommandsV1)
            );
            Ok::<(), Box<dyn std::error::Error>>(())
        })?;
    Ok(())
}
#[test]
fn character_services_v2_consumer_interleaves_versions_and_never_overlaps_exchanges()
-> Result<(), Box<dyn std::error::Error>> {
    scenario(true)
}
#[test]
fn character_services_v2_consumer_off_keeps_v1_requests_unchanged()
-> Result<(), Box<dyn std::error::Error>> {
    scenario(false)
}

#[test]
fn character_services_v2_backoff_never_delays_v1_and_grows_independently()
-> Result<(), Box<dyn std::error::Error>> {
    struct ThrottledPeer {
        clock: Clock,
        seen: Arc<Mutex<Vec<(Operation, Duration)>>>,
        done: Arc<Notify>,
    }
    impl CommandSource for ThrottledPeer {
        async fn exchange(&mut self, op: Operation, _: &str) -> Result<Vec<u8>, Failure> {
            self.seen
                .lock()
                .map_err(|_| Failure::Unavailable)?
                .push((op, self.clock.elapsed()));
            tokio::task::yield_now().await;
            if op == Operation::ListPendingCharacterCommandsV2 {
                return Err(Failure::RateLimited);
            }
            if self.clock.elapsed() >= Duration::from_secs(8) {
                self.done.notify_one();
            }
            Ok(br#"{"contract_version":1,"pending":[]}"#.to_vec())
        }
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let clock = Clock::default();
            let seen = Arc::new(Mutex::new(Vec::new()));
            let done = Arc::new(Notify::new());
            let consumer = Consumer::new(
                clock.clone(),
                ThrottledPeer {
                    clock,
                    seen: seen.clone(),
                    done: done.clone(),
                },
                FixedDecider,
                "oteryn:character-authority:primary".into(),
                |_| {},
            )
            .with_services(true);
            let mut running = std::pin::pin!(consumer.run());
            let mut completed = std::pin::pin!(done.notified());
            std::future::poll_fn(|cx| {
                if completed.as_mut().poll(cx).is_ready() {
                    return std::task::Poll::Ready(Ok::<(), Box<dyn std::error::Error>>(()));
                }
                if running.as_mut().poll(cx).is_ready() {
                    return std::task::Poll::Ready(Err("consumer ended".into()));
                }
                std::task::Poll::Pending
            })
            .await?;
            let seen = seen.lock().map_err(|_| "seen")?;
            let v1: Vec<_> = seen
                .iter()
                .filter(|(op, _)| *op == Operation::ListPendingCharacterCommandsV1)
                .map(|(_, t)| *t)
                .collect();
            let v2: Vec<_> = seen
                .iter()
                .filter(|(op, _)| *op == Operation::ListPendingCharacterCommandsV2)
                .map(|(_, t)| *t)
                .collect();
            assert!(v1.len() >= 8);
            assert!(v1.windows(2).all(|w| w[1] - w[0] <= POLL));
            assert!(v2.len() >= 3);
            assert!(v2[1] - v2[0] >= Duration::from_secs(2));
            assert!(v2[2] - v2[1] >= Duration::from_secs(4));
            Ok::<(), Box<dyn std::error::Error>>(())
        })
}
