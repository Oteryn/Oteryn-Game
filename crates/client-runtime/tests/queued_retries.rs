use oteryn_client_runtime::connection::{AdmissionAttempt, AdmissionWait, enter};
use oteryn_foundation::CancellationToken;
use oteryn_session::{AdmissionPublicClass, AdmissionQueue};
use std::time::{Duration, Instant};

struct Queued {
    calls: u32,
    succeed: u32,
    until: Instant,
    waits: Vec<AdmissionWait>,
    cancel_after: Option<(u32, CancellationToken)>,
}
impl AdmissionAttempt for Queued {
    type Output = ();
    type Error = AdmissionPublicClass;
    async fn admit(&mut self) -> Result<(), Option<u32>> {
        self.calls += 1;
        if self.calls == self.succeed {
            Ok(())
        } else {
            Err(Some(1115))
        }
    }
    async fn reconcile(&mut self, _: CancellationToken) -> Result<(), Self::Error> {
        Ok(())
    }
    fn usable_until(&self) -> Instant {
        self.until
    }
    fn rejected(class: AdmissionPublicClass) -> Self::Error {
        class
    }
    fn queue(&self) -> Option<AdmissionQueue> {
        Some(AdmissionQueue {
            position: 4u32.saturating_sub(self.calls).max(1),
            retry_after_ms: 1,
        })
    }
    fn waiting(&mut self, wait: AdmissionWait) {
        self.waits.push(wait);
        if let Some((calls, cancellation)) = &self.cancel_after
            && self.calls == *calls
        {
            cancellation.cancel();
        }
    }
}

#[test]
fn authoritative_queue_retries_more_than_once_with_each_server_position()
-> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?;
    let mut queued = Queued {
        calls: 0,
        succeed: 4,
        until: Instant::now() + Duration::from_secs(10),
        waits: vec![],
        cancel_after: None,
    };
    assert_eq!(
        runtime.block_on(enter(
            &mut queued,
            Duration::from_secs(10),
            CancellationToken::new()
        )),
        Ok(())
    );
    assert_eq!(
        queued
            .waits
            .iter()
            .map(|wait| wait.queue_position)
            .collect::<Vec<_>>(),
        vec![Some(3), Some(2), Some(1)]
    );
    assert!(
        queued
            .waits
            .iter()
            .all(|wait| wait.retry_in == Duration::from_millis(1))
    );
    Ok(())
}

#[test]
fn queued_retries_outlast_sixteen_replies_and_never_extend_the_grant()
-> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?;
    let mut queued = Queued {
        calls: 0,
        succeed: 100,
        until: Instant::now() + Duration::from_secs(10),
        waits: vec![],
        cancel_after: None,
    };
    assert_eq!(
        runtime.block_on(enter(&mut queued, Duration::ZERO, CancellationToken::new())),
        Ok(())
    );
    assert_eq!(queued.calls, 100);
    assert_eq!(queued.waits.len(), 99);
    queued.calls = 0;
    queued.waits.clear();
    queued.until = Instant::now();
    assert!(
        runtime
            .block_on(enter(&mut queued, Duration::ZERO, CancellationToken::new()))
            .is_err()
    );
    assert_eq!(queued.calls, 1);
    assert!(queued.waits.is_empty());
    Ok(())
}

#[test]
fn authoritative_queue_can_be_cancelled_after_sixteen_retries()
-> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?;
    let cancellation = CancellationToken::new();
    let mut queued = Queued {
        calls: 0,
        succeed: 100,
        until: Instant::now() + Duration::from_secs(10),
        waits: vec![],
        cancel_after: Some((18, cancellation.clone())),
    };
    assert!(
        runtime
            .block_on(enter(&mut queued, Duration::ZERO, cancellation))
            .is_err()
    );
    assert_eq!(queued.calls, 18);
    assert_eq!(queued.waits.len(), 18);
    Ok(())
}

#[test]
fn expired_while_waiting_does_not_admit_again() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()?;
    runtime.block_on(async {
        let lifetime = Duration::from_millis(100);
        let until = tokio::time::Instant::now().into_std() + lifetime;
        let mut queued = Queued {
            calls: 0,
            succeed: 2,
            until,
            waits: vec![],
            cancel_after: None,
        };
        {
            let mut pending =
                std::pin::pin!(enter(&mut queued, Duration::ZERO, CancellationToken::new()));
            std::future::poll_fn(|cx| {
                assert!(pending.as_mut().poll(cx).is_pending());
                std::task::Poll::Ready(())
            })
            .await;
            tokio::time::advance(lifetime).await;
            assert!(pending.await.is_err());
        }
        assert_eq!(queued.calls, 1);
        assert_eq!(queued.waits.len(), 1);
    });
    Ok(())
}
