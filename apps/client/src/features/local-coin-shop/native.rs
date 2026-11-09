use super::*;
use crate::account_services::window::Window;

pub(super) fn present(view: &View, cancel: CancellationToken) -> Action {
    form(view, cancel).unwrap_or(Action::Back)
}
fn form(view: &View, cancel: CancellationToken) -> Option<Action> {
    let window = Window::new(tr!("Oteryn — Get coins (sandbox)"), 850, 660, cancel)?;
    let refresh = window.button(tr!("Refresh packages / balance"), 1, [15, 15, 300, 32])?;
    window.label(
        tr!("Choose a server-priced package. Sandbox checkout uses no real money."),
        [15, 60, 800, 25],
    )?;
    let packages = window.list(100, [15, 95, 800, 150])?;
    let buy = window.button(tr!("Buy selected package…"), 2, [15, 260, 270, 32])?;
    let checkout = window.button(tr!("Open checkout"), 3, [305, 260, 200, 32])?;
    let status = window.button(tr!("Check purchase status"), 4, [525, 260, 290, 32])?;
    window.label(
        tr!("Your recent orders (choose one to inspect or resume)"),
        [15, 315, 780, 25],
    )?;
    let orders = window.combo(101, [15, 345, 590, 250])?;
    let inspect = window.button(tr!("Inspect selected order"), 7, [620, 345, 195, 32])?;
    let retry = window.button(tr!("Retry same order request"), 5, [15, 395, 300, 32])?;
    window.button(tr!("Back"), 99, [650, 395, 165, 32])?;
    window.output(&view.status_text(), [15, 445, 800, 160])?;
    if let Some(page) = &view.catalogue {
        for p in &page.packages {
            packages.add(&lfmt!(
                "{} — {} coins — {}",
                p.title,
                p.coins,
                price_text(&p.price)
            ));
        }
        for order in &page.recent_orders {
            orders.add(&lfmt!(
                "{:?} — {} coins — {} — {}",
                order.state,
                order.coins,
                price_text(&order.price),
                order.order_id
            ));
        }
        packages.select(0);
        orders.select(0);
    }
    refresh.enable(!view.unavailable);
    retry.enable(view.pending());
    status.enable(view.pending() || view.order.is_some());
    checkout.enable(view.can_checkout());
    inspect.enable(
        !view.pending()
            && view
                .catalogue
                .as_ref()
                .is_some_and(|p| !p.recent_orders.is_empty()),
    );
    loop {
        let event = window.event();
        let package = view
            .catalogue
            .as_ref()
            .and_then(|p| packages.selected().and_then(|i| p.packages.get(i)));
        buy.enable(package.is_some_and(|p| view.can_purchase(&p.id)));
        match event {
            1 => return Some(Action::Refresh),
            2 => {
                if let Some(p) = package.filter(|p| view.can_purchase(&p.id)) {
                    return Some(Action::Purchase {package_id:p.id.clone(),confirmed:window.confirm(&lfmt!("Buy {} Oteryn coins for {} in sandbox checkout? No real money is charged. Coins appear only after payment verification.",p.coins,price_text(&p.price)))});
                }
            }
            3 => return Some(Action::OpenCheckout),
            4 => return Some(Action::Status),
            5 => return Some(Action::Retry),
            7 => {
                if let Some(order) = view
                    .catalogue
                    .as_ref()
                    .and_then(|p| orders.selected().and_then(|i| p.recent_orders.get(i)))
                {
                    return Some(Action::Inspect(order.request_id.clone()));
                }
            }
            99 => return Some(Action::Back),
            _ => {}
        }
    }
}
