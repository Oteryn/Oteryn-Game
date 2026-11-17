use super::*;
use oteryn_platform_client::PlatformClientError;
use std::time::Duration;

pub(super) async fn execute(
    view: &mut View,
    action: Action,
    client: &NativeLoginClient,
    token: &AccessToken,
    browser: &Browser,
    cancel: CancellationToken,
) {
    #[cfg(windows)]
    let _progress = crate::account_services::window::Progress::open(cancel.clone());
    if let Err(error) = apply(view, action, client, token, browser, cancel).await {
        view.feedback = lfmt!(
            "Coin service unavailable ({:?}). Pending requests keep their identity; check status or retry. No browser result credits coins.",
            error.public_class()
        );
    }
}
async fn apply(
    view: &mut View,
    action: Action,
    client: &NativeLoginClient,
    token: &AccessToken,
    browser: &Browser,
    cancel: CancellationToken,
) -> Result<(), NativeLoginError> {
    match action {
        Action::Refresh => {
            refresh(view, client, token, cancel).await?;
            view.feedback = tr!("Server packages and wallet balance refreshed.").into();
        }
        Action::Purchase {
            package_id,
            confirmed,
        } => {
            if !confirmed {
                view.feedback = tr!("Purchase cancelled.").into();
                return Ok(());
            }
            if !view.can_purchase(&package_id) {
                view.feedback=tr!("Refresh the package quote, resolve your pending request and check account review status before buying.").into();
                return Ok(());
            }
            let Some(package) = view
                .catalogue
                .as_ref()
                .and_then(|c| c.packages.iter().find(|p| p.id == package_id))
                .cloned()
            else {
                return Ok(());
            };
            view.pending = Some(CoinOrderRequest {
                version: 1,
                request_id: AttemptRef::generate()?.as_str().into(),
                package_id: package.id.clone(),
                quote_id: package.quote_id.clone(),
                character_id: view.character_id.clone(),
            });
            view.confirmed = Some(package);
            submit(view, client, token, browser, cancel).await?;
        }
        Action::Retry => {
            if view.pending.is_some() {
                submit(view, client, token, browser, cancel).await?;
            }
        }
        Action::Status | Action::Inspect(_) => {
            let id = match action {
                Action::Inspect(id) => id,
                _ => match view
                    .pending
                    .as_ref()
                    .map(|p| &p.request_id)
                    .or_else(|| view.order.as_ref().map(|o| &o.request_id))
                {
                    Some(id) => id.clone(),
                    None => return Ok(()),
                },
            };
            if view.pending.as_ref().is_some_and(|p| p.request_id != id) {
                view.feedback =
                    tr!("Resolve the pending order request before inspecting another order.")
                        .into();
                return Ok(());
            }
            let order = client.coin_order_status(token, &id, cancel.clone()).await?;
            accept(view, order)?;
            refresh(view, client, token, cancel).await?;
            view.feedback = tr!("Purchase status and wallet balance refreshed from Oteryn.").into();
        }
        Action::OpenCheckout => open_checkout(view, client, browser),
        Action::Back => {}
    }
    Ok(())
}
async fn submit(
    view: &mut View,
    client: &NativeLoginClient,
    token: &AccessToken,
    browser: &Browser,
    cancel: CancellationToken,
) -> Result<(), NativeLoginError> {
    let Some(request) = &view.pending else {
        return Ok(());
    };
    let order = client
        .create_coin_order(token, request, cancel.clone())
        .await?;
    accept(view, order)?;
    open_checkout(view, client, browser);
    // Browser launch is not payment evidence. Always use a separate authenticated wallet read.
    refresh(view, client, token, cancel).await?;
    Ok(())
}
async fn refresh(
    view: &mut View,
    client: &NativeLoginClient,
    token: &AccessToken,
    cancel: CancellationToken,
) -> Result<(), NativeLoginError> {
    let requested_at = Instant::now();
    let catalogue = client
        .coin_catalogue(token, view.character_id.as_deref(), cancel.clone())
        .await?;
    view.quote_until = catalogue
        .packages
        .iter()
        .map(|p| {
            (
                p.quote_id.clone(),
                requested_at + Duration::from_secs(u64::from(p.valid_for_seconds)),
            )
        })
        .collect();
    if let Some(pending) = &view.pending
        && let Some(order) = catalogue
            .recent_orders
            .iter()
            .find(|o| o.request_id == pending.request_id)
    {
        accept(view, order.clone())?;
    }
    if view.order.is_none()
        && !view.pending()
        && let Some(order) = catalogue
            .recent_orders
            .iter()
            .find(|o| !o.state.completed())
    {
        accept(view, order.clone())?;
    }
    view.catalogue = Some(catalogue);
    let balance = client.coin_balance(token, cancel).await?;
    if view.balance.as_ref().is_some_and(|old| {
        balance.revision < old.revision || (balance.revision == old.revision && &balance != old)
    }) {
        return Err(PlatformClientError::InvalidPayload.into());
    }
    view.balance = Some(balance);
    Ok(())
}
fn accept(view: &mut View, order: CoinOrder) -> Result<(), NativeLoginError> {
    if let Some(request) = &view.pending {
        if request.request_id != order.request_id
            || request.package_id != order.package_id
            || request.quote_id != order.quote_id
            || request.character_id != order.character_id
        {
            return Err(PlatformClientError::InvalidPayload.into());
        }
        if let Some(package) = &view.confirmed
            && (package.coins != order.coins || package.price != order.price)
        {
            return Err(PlatformClientError::InvalidPayload.into());
        }
    }
    if let Some(old) = &view.order
        && old.request_id == order.request_id
        && (old.order_id != order.order_id
            || old.package_id != order.package_id
            || old.quote_id != order.quote_id
            || old.character_id != order.character_id
            || old.coins != order.coins
            || old.price != order.price
            || order.revision < old.revision
            || (order.revision == old.revision && old != &order)
            || !order.state.can_follow(old.state))
    {
        return Err(PlatformClientError::InvalidPayload.into());
    }
    if view
        .pending
        .as_ref()
        .is_some_and(|p| p.request_id == order.request_id)
    {
        view.pending = None;
        view.confirmed = None;
    }
    view.quote_until.remove(&order.quote_id);
    view.order = Some(order);
    Ok(())
}
fn open_checkout(view: &mut View, client: &NativeLoginClient, browser: &Browser) {
    let Some(order) = view
        .order
        .as_ref()
        .filter(|o| o.state == PurchaseState::AwaitingPayment)
    else {
        view.feedback = tr!("No checkout is available for this purchase state.").into();
        return;
    };
    let Some(url) = &order.checkout_url else {
        return;
    };
    if !client.valid_coin_checkout(url, &order.order_id) {
        view.feedback = tr!("Checkout address refused.").into();
        return;
    }
    view.feedback = if browser(url.as_str()) {
        tr!("Sandbox checkout opened. Return here and check purchase status after completing it.")
    } else {
        tr!(
            "Could not open the browser. Use Open checkout to retry without creating another order."
        )
    }
    .into();
}
