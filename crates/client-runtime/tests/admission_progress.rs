use oteryn_client_runtime::connection::{AdmissionAttempt, AdmissionWait, enter};
use oteryn_foundation::CancellationToken;
use oteryn_session::AdmissionPublicClass;
use std::time::{Duration, Instant};
struct Attempt {
    calls: u8,
    waits: Vec<AdmissionWait>,
    cancel_on_wait: Option<CancellationToken>,
}
impl AdmissionAttempt for Attempt {
    type Output = u8;
    type Error = AdmissionPublicClass;
    async fn admit(&mut self) -> Result<u8, Option<u32>> {
        self.calls += 1;
        if self.calls == 1 {
            Err(Some(1115))
        } else {
            Ok(42)
        }
    }
    async fn reconcile(&mut self, _: CancellationToken) -> Result<(), Self::Error> {
        Ok(())
    }
    fn usable_until(&self) -> Instant {
        Instant::now() + Duration::from_secs(60)
    }
    fn rejected(class: AdmissionPublicClass) -> Self::Error {
        class
    }
    fn waiting(&mut self, wait: AdmissionWait) {
        self.waits.push(wait);
        if let Some(token) = &self.cancel_on_wait {
            token.cancel();
        }
    }
}
#[test]
fn capacity_countdown_is_published_before_one_retry() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?;
    let mut attempt = Attempt {
        calls: 0,
        waits: Vec::new(),
        cancel_on_wait: None,
    };
    assert_eq!(
        runtime.block_on(enter(
            &mut attempt,
            Duration::ZERO,
            CancellationToken::new()
        )),
        Ok(42)
    );
    assert_eq!(attempt.calls, 2);
    assert_eq!(attempt.waits.len(), 1);
    assert_eq!(attempt.waits[0].code, 1115);
    Ok(())
}
#[test]
fn cancelled_wait_never_retries_admission() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?;
    let token = CancellationToken::new();
    let mut attempt = Attempt {
        calls: 0,
        waits: Vec::new(),
        cancel_on_wait: Some(token.clone()),
    };
    assert!(
        runtime
            .block_on(enter(&mut attempt, Duration::from_secs(10), token))
            .is_err()
    );
    assert_eq!(attempt.calls, 1);
    Ok(())
}
