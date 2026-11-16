//! Native market form, paged server lists and trade controls.
#![allow(unsafe_code)]



fn select_row(
    panel: &mut oteryn_client_ui::market::MarketPanel,
    offer: [u8; 16],
    definition: u32,
    price: u64,
) {
    panel.definition = definition;
    if price > 0 {
        panel.price = price;
    }
    panel.selected_offer = Some(offer);
}

#[cfg(test)]
mod regression {
    #[test]
    fn selecting_another_definition_keeps_rows_and_cancel_selection() {
        use oteryn_client_ui::market::{MarketPanel, MarketView, edit_details};
        let mut panel = MarketPanel {
            selected: true,
            definition: 1,
            view: MarketView::Own,
            ..Default::default()
        };
        let row = oteryn_client_runtime::market::market::MarketRow {
            offer: [9; 16],
            definition: 42,
            side: 2,
            amount: 3,
            price: 100,
            state: 1,
            anonymous: false,
        };
        panel.rows.push(row);
        super::select_row(&mut panel, [9; 16], 42, 100);
        assert!(edit_details(&mut panel, "42", "1", "100", true, false));
        assert_eq!(panel.rows.len(), 1);
        assert_eq!(panel.selected_offer, Some([9; 16]));
    }
}

#[derive(Debug, PartialEq, Eq)]
struct FormFields {
    item: String,
    amount: String,
    price: String,
    buy: bool,
    anonymous: bool,
}
fn form_fields(panel: &oteryn_client_ui::market::MarketPanel) -> FormFields {
    FormFields {
        item: if panel.definition == 0 {
            String::new()
        } else {
            panel.definition.to_string()
        },
        amount: panel.amount.to_string(),
        price: panel.price.to_string(),
        buy: !panel.sell,
        anonymous: panel.anonymous,
    }
}
#[cfg(test)]
mod reopen_regression {
    #[test]
    fn reopened_market_form_uses_one_retained_snapshot() {
        let panel = oteryn_client_ui::market::MarketPanel {
            definition: 42,
            amount: 7,
            price: 99,
            sell: false,
            anonymous: true,
            ..Default::default()
        };
        assert_eq!(
            super::form_fields(&panel),
            super::FormFields {
                item: "42".into(),
                amount: "7".into(),
                price: "99".into(),
                buy: true,
                anonymous: true
            }
        );
    }
}
