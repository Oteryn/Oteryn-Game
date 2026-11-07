//! Deadline regressions for browse retry while visibility refresh remains active.

use super::*;
use std::collections::VecDeque;
use tokio::io::AsyncWriteExt;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn browse_retry_waits_the_full_four_hundred_milliseconds_after_walk_completion() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            tokio::time::pause();
            let walk_finished = tokio::time::Instant::now();
            let mut deadline = WaitDeadline::new(walk_finished + BROWSE_RETRY_DELAY);
            assert_eq!(deadline.due - walk_finished, Duration::from_millis(400));
            let mut wait = std::pin::pin!(deadline.next(std::future::pending::<()>()));
            tokio::time::advance(Duration::from_millis(399)).await;
            assert!(
                std::future::poll_fn(|context| std::task::Poll::Ready(
                    std::future::Future::poll(wait.as_mut(), context).is_pending(),
                ))
                .await
            );
            tokio::time::advance(Duration::from_millis(1)).await;
            assert_eq!(wait.await, None);
            // Tokio 1.53.1 rounds deadlines up to its next millisecond tick
            // (vendor/tokio-1.53.1/src/runtime/time/source.rs:17-20). The paused
            // executor may therefore wake one tick after the exact 400 ms due time.
            let elapsed = tokio::time::Instant::now() - walk_finished;
            assert!(elapsed >= Duration::from_millis(400));
            assert!(elapsed <= Duration::from_millis(400) + Duration::from_millis(1));
        });
    Ok(())
}

#[test]
fn browse_retry_deadline_progresses_after_a_slow_owner_refresh() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            tokio::time::pause();
            let due = tokio::time::Instant::now() + Duration::from_millis(400);
            let mut deadline = WaitDeadline::new(due);
            assert_eq!(
                deadline.next(std::future::ready("visibility")).await,
                Some("visibility")
            );
            // A healthy but slow database read can outlast both the 250 ms visibility cadence
            // and the retry delay. A newly ready visibility tick must not win again.
            tokio::time::advance(Duration::from_millis(600)).await;
            let calls = std::cell::Cell::new(0);
            let another_refresh = std::future::poll_fn(|_| {
                calls.set(calls.get() + 1);
                std::task::Poll::Ready("visibility")
            });
            assert_eq!(deadline.next(another_refresh).await, None);
            assert_eq!(calls.get(), 0);
        });
    Ok(())
}

#[test]
fn browse_retry_deadline_probe_prioritizes_held_input_without_waiting() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (mut server, mut client) = tokio::io::duplex(64);
            let mut frames = FrameReader::default();
            let mut held = VecDeque::from([(vec![1, 2], true)]);
            client.write_all(&[0, 0, 0, 3, 7, 8, 9]).await?;
            match ready_input(&mut server, &mut frames, &mut held, false).await {
                Some(poll::Next::Held(frame, behind_step)) => {
                    assert_eq!(frame, [1, 2]);
                    assert!(behind_step);
                }
                _ => return Err("held client input must keep priority".into()),
            }
            match ready_input(&mut server, &mut frames, &mut held, false).await {
                Some(poll::Next::Frame(frame)) => assert_eq!(frame?, [7, 8, 9]),
                _ => {
                    return Err(
                        "complete client frame must reach normal cancellation handling".into(),
                    );
                }
            }
            assert!(
                ready_input(&mut server, &mut frames, &mut held, false)
                    .await
                    .is_none()
            );
            Ok(())
        })
}

#[test]
fn browse_retry_deadline_probe_retains_partial_frames_and_reports_disconnect() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (mut server, mut client) = tokio::io::duplex(64);
            let mut frames = FrameReader::default();
            let mut held = VecDeque::new();
            client.write_all(&[0, 0]).await?;
            assert!(
                ready_input(&mut server, &mut frames, &mut held, false)
                    .await
                    .is_none()
            );
            client.write_all(&[0, 3, 7]).await?;
            assert!(
                ready_input(&mut server, &mut frames, &mut held, false)
                    .await
                    .is_none()
            );
            client.write_all(&[8, 9]).await?;
            match ready_input(&mut server, &mut frames, &mut held, false).await {
                Some(poll::Next::Frame(frame)) => assert_eq!(frame?, [7, 8, 9]),
                _ => return Err("partial frame must remain buffered across the deadline".into()),
            }
            drop(client);
            assert!(matches!(
                ready_input(&mut server, &mut frames, &mut held, false).await,
                Some(poll::Next::Frame(Err(_)))
            ));
            Ok(())
        })
}
