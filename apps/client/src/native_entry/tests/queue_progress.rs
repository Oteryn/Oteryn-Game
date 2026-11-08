use super::test_support::Stack;
use crate::{ClientBootstrap, GameplayEntryError, reentry::Reentry};
use oteryn_platform_client::native_login::PublicClass;
use std::{
    sync::atomic::Ordering,
    time::{Duration, Instant},
};
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn capacity_progress_and_cancel_cross_real_gateway_and_tls_admission() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let run = || -> TestResult {
                let stack = Stack::start(vec![1115])?;
                let mut config = stack.config();
                // The 20s retry lies beyond our 10s observation deadline and within the 30s grant.
                config.retry_unit = Duration::from_secs(10);
                let client = ClientBootstrap::new()?.with_native_login(config);
                let mut entry = Reentry::new(client);
                assert!(entry.request());
                let deadline = Instant::now() + Duration::from_secs(10);
                let mut saw_wait = false;
                loop {
                    if let Some(text) = entry.poll_wait(Instant::now())
                        && text.starts_with("Waiting for capacity.")
                    {
                        saw_wait = true;
                        assert!(entry.cancel_entry());
                    }
                    if let Some(result) = entry.poll() {
                        assert_eq!(
                            result.as_ref().err().copied(),
                            Some(GameplayEntryError::Rejected(
                                PublicClass::SessionUnavailable
                            ))
                        );
                        break;
                    }
                    if Instant::now() >= deadline {
                        return Err("admission cancellation timed out".into());
                    }
                    std::thread::yield_now();
                }
                assert!(saw_wait);
                assert_eq!(stack.admissions.load(Ordering::SeqCst), 1);
                entry.shutdown();
                stack.finish();
                Ok(())
            };
            run().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "capacity progress scenario thread failed")?
        .map_err(Into::into)
}

#[test]
fn server_position_reaches_the_real_login_wait_caller_and_can_be_cancelled() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let run = || -> TestResult {
                use oteryn_session::admission_queue::{AdmissionQueue, encode_queue_refusal};
                let frame = encode_queue_refusal(AdmissionQueue {
                    position: 3,
                    retry_after_ms: 2_000,
                })?;
                let stack = Stack::start_with_frames(vec![], vec![frame])?;
                let client = ClientBootstrap::new()?.with_native_login(stack.config());
                let mut entry = Reentry::new(client);
                assert!(entry.request());
                let deadline = Instant::now() + Duration::from_secs(10);
                let mut saw_position = false;
                loop {
                    if !saw_position && let Some(text) = entry.poll_wait(Instant::now()) {
                        assert!(
                            text.starts_with("Queue position 3."),
                            "missing server position: {text}"
                        );
                        saw_position = true;
                        assert!(entry.cancel_entry());
                    }
                    if let Some(result) = entry.poll() {
                        assert!(result.is_err());
                        break;
                    }
                    if Instant::now() >= deadline {
                        return Err("queue position timed out".into());
                    }
                    std::thread::yield_now();
                }
                assert!(saw_position);
                assert_eq!(stack.admissions.load(Ordering::SeqCst), 1);
                entry.shutdown();
                stack.finish();
                Ok(())
            };
            run().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "queue position scenario failed")?
        .map_err(Into::into)
}
