use super::*;
#[test]
fn market_panel_builds_requests_and_surfaces_refusals() -> Result<(), &'static str> {
    let mut panel = MarketPanel {
        selected: true,
        definition: 42,
        ..MarketPanel::default()
    };
    let command = request(&mut panel, Intent::Browse).ok_or("valid browse")?;
    assert_eq!(command.0.action, 2);
    let response = MarketResponse {
        status: CommandStatus::Accepted,
        reply: Some(market::MarketReply {
            action: 2,
            code: 12,
            offer: [0; 16],
            rows: vec![],
        }),
    };
    assert!(!apply_result(&mut panel, &response));
    assert!(panel_text(&panel).contains("Premium required"));
    Ok(())
}

#[test]
fn market_form_validates_without_partial_updates_and_projects_total() {
    let mut panel = MarketPanel::default();
    assert!(edit_details(&mut panel, "42", "3", "100", false, true));
    assert_eq!((panel.definition, panel.amount, panel.price), (42, 3, 100));
    assert!(panel_text(&panel).contains("Total: 300"));
    assert!(!edit_details(&mut panel, "0", "3", "100", true, false));
    assert_eq!(panel.definition, 42);
}

#[test]
fn market_open_and_pages_use_valid_queries() -> Result<(), &'static str> {
    let mut panel = MarketPanel {
        selected: true,
        ..Default::default()
    };
    assert_eq!(
        request(&mut panel, Intent::Refresh)
            .ok_or("wares")?
            .0
            .action,
        1
    );
    panel.pending = None;
    panel.definition = 42;
    assert_eq!(
        request(&mut panel, Intent::NextPage).ok_or("page")?.0.page,
        1
    );
    Ok(())
}

#[test]
fn market_server_early_refusal_keeps_reason_and_clears_pending() -> Result<(), &'static str> {
    let mut panel = MarketPanel {
        selected: true,
        definition: 42,
        ..Default::default()
    };
    request(&mut panel, Intent::Browse).ok_or("browse")?;
    assert!(!apply_result(
        &mut panel,
        &MarketResponse {
            status: CommandStatus::Accepted,
            reply: Some(market::MarketReply {
                action: 0,
                code: 11,
                offer: [0; 16],
                rows: vec![]
            }),
        }
    ));
    assert!(panel.pending.is_none());
    assert!(panel_text(&panel).contains("no depot locker"));
    Ok(())
}

#[test]
fn wares_returns_from_item_book_and_keeps_refresh_and_paging_on_wares() -> Result<(), &'static str>
{
    let mut panel = MarketPanel {
        selected: true,
        definition: 42,
        page: 3,
        ..Default::default()
    };
    assert_eq!(
        request(&mut panel, Intent::Wares).ok_or("wares")?.0.action,
        1
    );
    assert!(apply_result(
        &mut panel,
        &MarketResponse {
            status: CommandStatus::Accepted,
            reply: Some(market::MarketReply {
                action: 1,
                code: 1,
                offer: [0; 16],
                rows: vec![]
            }),
        }
    ));
    assert_eq!(panel.definition, 0);
    assert_eq!(
        request(&mut panel, Intent::NextPage)
            .ok_or("next")?
            .0
            .action,
        1
    );
    panel.pending = None;
    let refresh = request(&mut panel, Intent::Refresh).ok_or("refresh")?;
    assert_eq!(
        (refresh.0.action, refresh.0.definition, refresh.0.page),
        (1, 0, 1)
    );
    Ok(())
}

#[test]
fn market_page_statistics_keep_items_and_sides_separate() {
    let mut p = MarketPanel {
        selected: true,
        definition: 42,
        ..Default::default()
    };
    for (definition, side, amount, price) in [
        (42, 1, 2, 10),
        (42, 1, 3, 20),
        (42, 2, 4, 30),
        (7, 1, 8, 999),
    ] {
        p.rows.push(market::MarketRow {
            offer: [1; 16],
            definition,
            side,
            amount,
            price,
            state: 1,
            anonymous: false,
        });
    }
    let text = panel_text(&p);
    assert!(text.contains("Loaded page buy: 2 offers, 5 units, price 10–20, weighted average 16"));
    assert!(text.contains("Loaded page sell: 1 offers, 4 units, price 30–30, weighted average 30"));
    assert!(text.contains("Statistics cover the loaded item page"));
}
