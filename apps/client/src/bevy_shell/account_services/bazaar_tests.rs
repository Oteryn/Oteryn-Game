//! The Bazaar, world-transfer and coin-package screens through the real account-services loop.
use super::*;
use crate::account_services::test_server::{
    Authority, MockPlatform, TestResult, coins::EventKind, test_thread,
};
use oteryn_foundation::CancellationToken;
use std::{
    sync::{Arc, Mutex, MutexGuard},
    thread,
    time::{Duration, Instant},
};

type Worker = thread::JoinHandle<Result<(), String>>;
struct Harness {
    app: App,
    mock: Arc<MockPlatform>,
    worker: Option<Worker>,
    shown: Option<u64>,
    opened: Arc<Mutex<Vec<String>>>,
}
impl Harness {
    fn start() -> Result<Self, Box<dyn std::error::Error>> {
        let mock = Arc::new(MockPlatform::start()?);
        let client = mock.client()?;
        let token = mock.token(&client)?;
        let owner = HostOwner::new(None)?;
        let mut config = crate::NativeLoginConfig::from_env(|key| match key {
            "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" => Some(mock.base.clone()),
            "OTERYN_OAUTH_CLIENT_ID" => Some("test-client".into()),
            "OTERYN_DEV_ROOT" => Some("/tmp/unused-bazaar-test-root".into()),
            _ => None,
        })?
        .ok_or("config")?;
        let opened = Arc::new(Mutex::new(Vec::new()));
        let record = opened.clone();
        config.open_browser = Arc::new(move |url: &str| {
            record.lock().is_ok_and(|mut log| {
                log.push(url.to_owned());
                true
            })
        });
        owner.services.configure(&mut config);
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<Font>()
            .init_resource::<InputFocus>()
            .add_message::<bevy::input::keyboard::KeyboardInput>()
            .insert_non_send(owner)
            .add_systems(Startup, super::super::ui::setup);
        install(&mut app);
        super::super::game_ui::modal::install(&mut app);
        let runner = mock.clone();
        let worker = thread::Builder::new().stack_size(64 << 20).spawn(move || {
            runner
                .runtime
                .block_on(crate::account_services::run(
                    &client,
                    &token,
                    &config,
                    CancellationToken::new(),
                ))
                .map_err(|e| format!("{e:?}"))
        })?;
        Ok(Self {
            app,
            mock,
            worker: Some(worker),
            shown: None,
            opened,
        })
    }
    /// Waits for the next screen the worker hands to Bevy and returns its body text.
    fn next(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            self.app.update();
            let id = self
                .app
                .world()
                .non_send::<HostOwner>()
                .services
                .pending()
                .map(|r| r.id);
            if id.is_some() && id != self.shown {
                self.shown = id;
                self.app.update();
                return Ok(self.body());
            }
            thread::yield_now();
        }
        Err("account screen did not reach Bevy".into())
    }
    fn body(&mut self) -> String {
        self.app
            .world_mut()
            .query_filtered::<&Text, With<Body>>()
            .iter(self.app.world())
            .map(|text| text.0.clone())
            .collect()
    }
    fn texts(&mut self) -> Vec<String> {
        self.app
            .world_mut()
            .query::<&Text>()
            .iter(self.app.world())
            .map(|text| text.0.clone())
            .collect()
    }
    /// Activates the widget whose caption is `caption`, as a pointer or keyboard press does.
    fn press(&mut self, caption: &str) -> TestResult {
        let entity = self
            .app
            .world_mut()
            .query::<(&Text, &ChildOf)>()
            .iter(self.app.world())
            .find(|(text, _)| text.0 == caption)
            .map(|(_, parent)| parent.parent())
            .ok_or_else(|| format!("{caption} missing"))?;
        self.app.world_mut().trigger(Activate { entity });
        self.app.update();
        Ok(())
    }
    fn press_row_starting(&mut self, prefix: &str) -> TestResult {
        let caption = self
            .texts()
            .into_iter()
            .find(|text| text.starts_with(prefix))
            .ok_or_else(|| format!("row {prefix} missing"))?;
        self.press(&caption)
    }
    fn edit(&mut self, from: &str, to: &str) -> TestResult {
        let mut found = false;
        let mut query = self
            .app
            .world_mut()
            .query::<&mut bevy::text::EditableText>();
        for mut text in query.iter_mut(self.app.world_mut()) {
            if !found && text.value() == from {
                text.editor_mut().set_text(to);
                found = true;
            }
        }
        found
            .then_some(())
            .ok_or_else(|| format!("field {from} missing").into())
    }
    fn authority(&self) -> Result<MutexGuard<'_, Authority>, &'static str> {
        self.mock.authority.lock().map_err(|_| "lock")
    }
    fn finish(mut self) -> TestResult {
        self.press("Return to characters")?;
        let worker = self.worker.take().ok_or("worker")?;
        worker.join().map_err(|_| "worker")??;
        self.app.update();
        assert!(
            self.app
                .world()
                .non_send::<HostOwner>()
                .services
                .pending()
                .is_none()
        );
        Ok(())
    }
}

#[test]
fn bazaar_bevy_screen_browses_bids_sells_and_recovers_receipts() -> TestResult {
    test_thread(|| {
        let mut shell = Harness::start()?;
        let menu = shell.next()?;
        assert!(
            menu.contains("Last refreshed balance: 1000 Oteryn coins"),
            "{menu}"
        );
        assert!(
            shell
                .texts()
                .iter()
                .any(|t| t.starts_with("Ari — level 100 — North — next 10 coins")),
        );
        shell.press("Auction history")?;
        shell.next()?;
        assert!(shell.texts().iter().any(|t| t.contains("— Sold —")));
        shell.press("My auctions")?;
        shell.next()?;
        assert!(shell.authority()?.requests.iter().any(|r| {
            r.starts_with("GET /api/v1/native-account/bazaar/auctions?") && r.contains("view=own")
        }));
        // A typed amount that is not a number keeps the screen and explains, as the native form did.
        shell.press_row_starting("Ari — level 100")?;
        shell.edit("10", "ten")?;
        shell.press("Place bid…")?;
        let explained = shell.next()?;
        assert!(explained.contains("Choose an auction and enter a valid coin amount."));
        shell.edit("ten", "10")?;
        shell.press("Place bid…")?;
        let confirm = shell.next()?;
        assert!(
            confirm.contains("Bid 10 Oteryn coins for Ari?"),
            "{confirm}"
        );
        shell.press("No")?;
        assert!(shell.next()?.contains("Bid cancelled."));
        assert_eq!(shell.authority()?.effects, 0);
        shell.press_row_starting("Ari — level 100")?;
        shell.press("Place bid…")?;
        shell.next()?;
        shell.press("Yes")?;
        let bid = shell.next()?;
        assert!(bid.contains("Committed: MOCK_COMMITTED"), "{bid}");
        assert_eq!(shell.authority()?.effects, 1);
        shell.press("List for auction…")?;
        let listing = shell.next()?;
        assert!(
            listing.contains("List Ari for 24 hours, minimum 1 coins?"),
            "{listing}"
        );
        shell.press("Yes")?;
        shell.next()?;
        assert_eq!(shell.authority()?.effects, 2);
        // The operation receipts list recovers a receipt by its request identity.
        let receipt = shell
            .authority()?
            .commands
            .keys()
            .next()
            .cloned()
            .ok_or("receipt")?;
        shell.press_row_starting(&format!("Committed — MOCK_COMMITTED — {receipt}"))?;
        shell.press("Check receipt")?;
        let recovered = shell.next()?;
        assert!(
            recovered.contains(&format!("receipt {receipt}")),
            "{recovered}"
        );
        assert!(
            shell
                .authority()?
                .requests
                .iter()
                .any(|r| r.starts_with(&format!(
                    "GET /api/v1/native-account/bazaar/commands/{receipt} "
                )))
        );
        shell.finish()
    })
}

#[test]
fn transfer_bevy_screen_quotes_and_confirms_world_transfer() -> TestResult {
    test_thread(|| {
        let mut shell = Harness::start()?;
        shell.next()?;
        // Without a fresh quote the confirmation cannot be answered.
        let before = shell.shown;
        shell.press("Confirm transfer…")?;
        shell.app.update();
        assert_eq!(
            shell
                .app
                .world()
                .non_send::<HostOwner>()
                .services
                .pending()
                .map(|r| r.id),
            before
        );
        shell.authority()?.in_guild = true;
        shell.press("Check transfer")?;
        let blocked = shell.next()?;
        assert!(
            blocked.contains(
                "Transfer Ari → South: 50 coins; cooldown 86400 seconds. Leave the guild"
            ),
            "{blocked}"
        );
        shell.press("Confirm transfer…")?;
        shell.app.update();
        assert_eq!(
            shell
                .app
                .world()
                .non_send::<HostOwner>()
                .services
                .pending()
                .map(|r| r.id),
            shell.shown
        );
        shell.authority()?.in_guild = false;
        shell.press("Check transfer")?;
        shell.next()?;
        shell.press("Confirm transfer…")?;
        let confirm = shell.next()?;
        assert!(
            confirm.contains("Game rechecks offline, guild, house and market restrictions"),
            "{confirm}"
        );
        shell.press("Yes")?;
        let done = shell.next()?;
        assert!(done.contains("Committed: MOCK_COMMITTED"), "{done}");
        assert_eq!(shell.authority()?.effects, 1);
        assert!(
            shell
                .authority()?
                .commands
                .values()
                .any(|(request, _)| matches!(
                    request.command,
                    oteryn_platform_client::native_login::bazaar::BazaarCommand::Transfer { .. }
                ))
        );
        shell.finish()
    })
}

#[test]
fn coin_packages_bevy_screen_buys_opens_checkout_and_reports_paid_status() -> TestResult {
    test_thread(|| {
        let mut shell = Harness::start()?;
        shell.next()?;
        shell.press("Get coins")?;
        let shop = shell.next()?;
        assert!(shop.contains("Balance: 1000 coins."), "{shop}");
        assert!(
            shell
                .texts()
                .iter()
                .any(|t| t == "Sandbox coins — 100 coins — USD 4.99")
        );
        shell.press("Sandbox coins — 100 coins — USD 4.99")?;
        shell.press("Buy selected package…")?;
        let confirm = shell.next()?;
        assert!(
            confirm.contains("Buy 100 Oteryn coins for USD 4.99 in sandbox checkout?"),
            "{confirm}"
        );
        shell.press("Yes")?;
        let ordered = shell.next()?;
        assert!(
            ordered.contains("AwaitingPayment, 100 coins for USD 4.99"),
            "{ordered}"
        );
        assert_eq!(shell.opened.lock().map_err(|_| "lock")?.len(), 1);
        shell.press("Open checkout")?;
        shell.next()?;
        assert_eq!(shell.opened.lock().map_err(|_| "lock")?.len(), 2);
        let order = shell
            .authority()?
            .coins
            .as_ref()
            .and_then(|c| c.orders.values().next().map(|(_, o)| o.clone()))
            .ok_or("order")?;
        let (body, signature) = shell.mock.signed_event(&order, EventKind::Paid)?;
        assert_eq!(shell.mock.webhook(&body, &signature)?, 200);
        shell.press("Check purchase status")?;
        let paid = shell.next()?;
        assert!(paid.contains("Paid, 100 coins"), "{paid}");
        assert!(paid.contains("Balance: 1100 coins."), "{paid}");
        shell.press_row_starting("Paid — 100 coins")?;
        shell.press("Inspect selected order")?;
        assert!(shell.next()?.contains(&format!("Order {}", order.order_id)));
        shell.press("Back")?;
        let bazaar = shell.next()?;
        assert!(
            bazaar.contains("Last refreshed balance: 1100 Oteryn coins"),
            "{bazaar}"
        );
        let credits = shell.authority()?.coins.as_ref().map(|c| c.credits);
        assert_eq!(credits, Some(1));
        shell.finish()
    })
}
