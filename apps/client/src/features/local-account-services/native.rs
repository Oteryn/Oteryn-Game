use super::*;

fn navigation_action(event: usize) -> Option<Action> {
    match event {
        1 => Some(Action::Refresh(AuctionView::All)),
        2 => Some(Action::Refresh(AuctionView::Own)),
        3 => Some(Action::Refresh(AuctionView::History)),
        4 => Some(Action::Next),
        10 => Some(Action::Retry),
        11 => Some(Action::Status),
        13 => Some(Action::CoinShop),
        14 => Some(Action::Premium),
        99 => Some(Action::Back),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn premium_overview_button_opens_premium_action() {
        assert!(matches!(navigation_action(14), Some(Action::Premium)));
        assert!(matches!(navigation_action(13), Some(Action::CoinShop)));
        assert!(navigation_action(0).is_none());
    }
}
