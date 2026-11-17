use crate::highscores::{self, Action, HighscoresPanel};
use crate::highscores_loyalty::{
    LOYALTY_CATEGORY, categories, category_for_selection, category_selection,
};

fn panel(loyalty: bool) -> HighscoresPanel {
    let mut panel = HighscoresPanel::default();
    panel.selected = true;
    panel.loyalty_selected = loyalty;
    panel
}

#[test]
fn c0498_loyalty_is_offered_in_id_order_only_when_selected() {
    let with = categories(true);
    let at = category_selection(LOYALTY_CATEGORY, true);
    assert_eq!(at, Some(12));
    assert_eq!(with[12].1, "Loyalty points");
    assert_eq!(with[11].0, 12);
    assert_eq!(with[13].0, 14);
    assert_eq!(with.len(), 16);
    assert_eq!(category_for_selection(12, true), Some(13));
    // Without the capability the list is the registered one.
    assert_eq!(categories(false).len(), 15);
    assert_eq!(category_selection(LOYALTY_CATEGORY, false), None);
    assert_eq!(category_for_selection(12, false), Some(14));
}

#[test]
fn c0498_the_filter_requests_category_13_only_with_the_capability() {
    let mut with = panel(true);
    let command = highscores::request(
        &mut with,
        Action::Filter {
            category: LOYALTY_CATEGORY,
            vocation: 0,
        },
    );
    assert!(command.is_some());
    assert_eq!(with.category, LOYALTY_CATEGORY);
    assert!(highscores::panel_text(&with).contains("Loyalty points"));
    let mut without = panel(false);
    let command = highscores::request(
        &mut without,
        Action::Filter {
            category: LOYALTY_CATEGORY,
            vocation: 0,
        },
    );
    assert!(command.is_none());
    assert_eq!(without.category, 1);
}
