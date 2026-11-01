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
