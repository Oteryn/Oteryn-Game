use super::*;
use oteryn_platform_client::native_login::NativeLoginError;
use std::time::Duration;

pub(super) async fn execute(
    view: &mut View,
    action: Action,
    client: &NativeLoginClient,
    token: &AccessToken,
    cancel: CancellationToken,
) {
    #[cfg(windows)]
    let _progress = super::window::Progress::open(cancel.clone());
    let refresh = matches!(
        action,
        Action::Bid {
            confirmed: true,
            ..
        } | Action::Sell {
            confirmed: true,
            ..
        } | Action::Transfer { confirmed: true }
            | Action::Retry
            | Action::Status
            | Action::Receipt(_)
    );
    let result = apply(view, action, client, token, cancel.clone()).await;
    if result.is_ok()
        && refresh
        && view
            .receipt
            .as_ref()
            .is_some_and(|r| r.state != OperationState::Pending)
    {
        match client
            .bazaar_page(token, view.selected_view, None, cancel)
            .await
        {
            Ok(page) => view.install_page(page),
            Err(_) => view.feedback.push_str(tr!(
                " Balance refresh unavailable; the receipt is retained."
            )),
        }
    }
    if let Err(error) = result {
        view.feedback = lfmt!(
            "Account service unavailable ({:?}). Refresh, or retry the same pending request. You can return to character selection.",
            error.public_class()
        );
    }
}
async fn apply(
    view: &mut View,
    action: Action,
    client: &NativeLoginClient,
    token: &AccessToken,
    cancel: CancellationToken,
) -> Result<(), NativeLoginError> {
    let command = match action {
        Action::Refresh(kind) => {
            let page = client.bazaar_page(token, kind, None, cancel).await?;
            view.install_page(page);
            view.selected_view = kind;
            view.feedback = tr!("Server account data refreshed.").into();
            return Ok(());
        }
        Action::Next => {
            let cursor = view.page.as_ref().and_then(|p| p.next_cursor.as_deref());
            if let Some(cursor) = cursor {
                let page = client
                    .bazaar_page(token, view.selected_view, Some(cursor), cancel)
                    .await?;
                view.install_page(page);
            }
            return Ok(());
        }
        Action::Bid {
            auction_id,
            coins,
            confirmed,
        } => {
            if !confirmed {
                view.feedback = tr!("Bid cancelled.").into();
                return Ok(());
            }
            let auction = view
                .page
                .as_ref()
                .and_then(|p| p.auctions.iter().find(|a| a.id == auction_id));
            let Some(auction) =
                auction.filter(|a| a.state == AuctionState::Open && coins >= a.minimum_next_bid)
            else {
                view.feedback = tr!("Choose an open auction and meet its minimum next bid.").into();
                return Ok(());
            };
            BazaarCommand::Bid {
                auction_id,
                expected_revision: auction.revision,
                coins,
            }
        }
        Action::Sell {
            character_id,
            minimum_bid,
            duration_hours,
            confirmed,
        } => {
            if !confirmed {
                view.feedback = tr!("Listing cancelled.").into();
                return Ok(());
            }
            if !view
                .page
                .as_ref()
                .is_some_and(|p| p.owned_characters.iter().any(|c| c.id == character_id))
            {
                view.feedback = tr!("Choose a character from your server portfolio.").into();
                return Ok(());
            }
            BazaarCommand::Sell {
                character_id,
                minimum_bid,
                duration_hours,
            }
        }
        Action::Quote {
            character_id,
            world_id,
        } => {
            view.quote = None;
            view.quote_until = None;
            let requested_at = Instant::now();
            let quote = client
                .transfer_quote(token, &character_id, &world_id, cancel)
                .await?;
            view.quote_until =
                Some(requested_at + Duration::from_secs(u64::from(quote.valid_for_seconds)));
            view.quote = Some(quote);
            view.feedback = tr!(
                "Review the server's transfer price, cooldown and eligibility before confirming."
            )
            .into();
            return Ok(());
        }
        Action::Transfer { confirmed } => {
            if !confirmed {
                view.feedback = tr!("Transfer cancelled.").into();
                return Ok(());
            }
            if !view.can_transfer() {
                view.feedback = tr!("Request a fresh eligible world-transfer quote first.").into();
                return Ok(());
            }
            let Some(quote) = &view.quote else {
                return Ok(());
            };
            BazaarCommand::Transfer {
                quote_id: quote.quote_id.clone(),
            }
        }
        Action::Retry => {
            if let Some(request) = &view.pending {
                let receipt = client.bazaar_command(token, request, cancel).await?;
                accept(view, receipt);
            }
            return Ok(());
        }
        Action::Status => {
            if let Some(id) = view.pending_id() {
                let receipt = client.bazaar_status(token, id, cancel).await?;
                accept(view, receipt);
            }
            return Ok(());
        }
        Action::Receipt(id) => {
            let receipt = client.bazaar_status(token, &id, cancel).await?;
            accept(view, receipt);
            return Ok(());
        }
        Action::Back | Action::CoinShop | Action::Premium => return Ok(()),
    };
    if view.pending() {
        view.feedback = tr!("Resolve the pending operation before starting another.").into();
        return Ok(());
    }
    if !command.valid() {
        view.feedback = tr!("Enter a positive coin amount and duration of 1–168 hours.").into();
        return Ok(());
    }
    let request = BazaarRequest {
        version: 1,
        request_id: AttemptRef::generate()?.as_str().to_owned(),
        command,
    };
    view.pending = Some(request);
    let Some(request) = &view.pending else {
        return Ok(());
    };
    let receipt = client.bazaar_command(token, request, cancel).await?;
    accept(view, receipt);
    Ok(())
}
fn accept(view: &mut View, receipt: OperationReceipt) {
    if view.receipt.as_ref().is_some_and(|old| {
        old.request_id == receipt.request_id
            && old.state != OperationState::Pending
            && old != &receipt
    }) {
        view.feedback =
            tr!("Conflicting terminal receipt; keep the original receipt and contact support.")
                .into();
        return;
    }
    if receipt.state != OperationState::Pending
        && view
            .pending
            .as_ref()
            .is_some_and(|p| p.request_id == receipt.request_id)
    {
        view.pending = None;
    }
    if receipt.state == OperationState::Pending {
        view.unresolved.insert(receipt.request_id.clone());
    } else {
        view.unresolved.remove(&receipt.request_id);
    }
    view.feedback = match receipt.state {
        OperationState::Pending => {
            tr!("Awaiting the authoritative receipt. Check status or retry with the same identity.")
        }
        OperationState::Committed => tr!("Operation committed by the service authority."),
        OperationState::Rejected => tr!("Operation refused; review the receipt code."),
    }
    .into();
    view.receipt = Some(receipt);
    view.quote = None;
    view.quote_until = None;
}
