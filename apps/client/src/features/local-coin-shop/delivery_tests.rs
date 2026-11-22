use super::*;
use crate::account_services::test_server::{
    CHARACTER, MockPlatform, TestResult, coins::PACKAGE, test_thread,
};
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Point {
    OAuth,
    ExplicitCheckout,
    OrderCheckout,
    Store,
}

type Gate = (Point, mpsc::Sender<()>, mpsc::Receiver<()>);
thread_local! {
    static DELIVERY: std::cell::RefCell<Option<Gate>> = const { std::cell::RefCell::new(None) };
}

pub(crate) fn pause(point: Point) {
    DELIVERY.with(|gate| {
        let mut gate = gate.borrow_mut();
        if gate.as_ref().is_some_and(|gate| gate.0 == point)
            && let Some((_, ready, release)) = gate.take()
        {
            assert!(ready.send(()).is_ok());
            assert!(release.recv_timeout(Duration::from_secs(5)).is_ok());
        }
    });
}

fn gate(point: Point) -> (Gate, mpsc::Receiver<()>, mpsc::Sender<()>) {
    let (ready_tx, ready) = mpsc::channel();
    let (release, release_rx) = mpsc::channel();
    ((point, ready_tx, release_rx), ready, release)
}

fn cancel_at(point: Point, cancel: &CancellationToken, operation: impl FnOnce()) -> TestResult {
    let (gate, ready, release) = gate(point);
    let cancel = cancel.clone();
    let canceller = std::thread::spawn(move || {
        ready.recv_timeout(Duration::from_secs(5))?;
        cancel.cancel();
        release.send(())?;
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    });
    DELIVERY.set(Some(gate));
    operation();
    DELIVERY.take();
    canceller
        .join()
        .map_err(|_| "cancellation worker")?
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn config(platform: &str) -> Result<crate::NativeLoginConfig, Box<dyn std::error::Error>> {
    Ok(crate::NativeLoginConfig::from_env(|name| match name {
        "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" => Some(platform.into()),
        "OTERYN_OAUTH_CLIENT_ID" => Some("native-client".into()),
        _ => None,
    })?
    .ok_or("checkout config")?)
}

#[test]
fn cancelled_oauth_does_not_deliver_browser_launch() -> TestResult {
    test_thread(|| {
        let mock = MockPlatform::start()?;
        let mut config = config(&mock.base)?;
        let calls = Arc::new(AtomicUsize::new(0));
        let opened = calls.clone();
        config.open_browser = Arc::new(move |_| {
            opened.fetch_add(1, Ordering::SeqCst);
            false
        });
        let cancel = CancellationToken::new();
        cancel_at(Point::OAuth, &cancel, || {
            let _ = mock.runtime.block_on(crate::native_entry::open_coin_shop(
                &config,
                CHARACTER,
                cancel.clone(),
            ));
        })?;
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "cancelled OAuth opened a browser"
        );
        Ok(())
    })
}

fn checkout_browser(point: Point) -> TestResult {
    test_thread(move || {
        let mock = MockPlatform::start()?;
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let calls = Arc::new(AtomicUsize::new(0));
        let opened = calls.clone();
        let browser: Browser = Arc::new(move |_| {
            opened.fetch_add(1, Ordering::SeqCst);
            true
        });
        let mut view = View::new(Some(CHARACTER.into()));
        let purchase = || Action::Purchase {
            package_id: PACKAGE.into(),
            confirmed: true,
        };
        mock.runtime.block_on(controller::execute(
            &mut view,
            Action::Refresh,
            &client,
            &token,
            &browser,
            CancellationToken::new(),
        ));
        let action = if point == Point::ExplicitCheckout {
            mock.runtime.block_on(controller::execute(
                &mut view,
                purchase(),
                &client,
                &token,
                &browser,
                CancellationToken::new(),
            ));
            assert_eq!(calls.swap(0, Ordering::SeqCst), 1);
            Action::OpenCheckout
        } else {
            purchase()
        };
        let cancel = CancellationToken::new();
        cancel_at(point, &cancel, || {
            mock.runtime.block_on(controller::execute(
                &mut view,
                action,
                &client,
                &token,
                &browser,
                cancel.clone(),
            ));
        })?;
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "cancelled checkout opened a browser"
        );
        Ok(())
    })
}

#[test]
fn cancelled_confirmed_order_does_not_deliver_browser_launch() -> TestResult {
    checkout_browser(Point::OrderCheckout)
}

#[test]
fn cancelled_explicit_checkout_does_not_deliver_browser_launch() -> TestResult {
    checkout_browser(Point::ExplicitCheckout)
}

pub(crate) fn store_worker_gate() -> (Presenter, mpsc::Receiver<()>, mpsc::Sender<()>) {
    let (gate, ready, release) = gate(Point::Store);
    let gate = Mutex::new(Some(gate));
    (
        Arc::new(move |_, _| {
            if let Ok(mut gate) = gate.lock() {
                DELIVERY.set(gate.take());
            }
            Action::Back
        }),
        ready,
        release,
    )
}
