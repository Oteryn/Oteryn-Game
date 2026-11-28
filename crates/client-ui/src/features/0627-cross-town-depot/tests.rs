use super::*;
use crate::depot::{Intent, request};

#[test]
fn routes_only_an_explicit_negotiated_town() -> Result<(), &'static str> {
    for negotiated in [false, true] {
        let mut panel = DepotPanel {
            selected: true,
            cross_town_selected: negotiated,
            ..Default::default()
        };
        let intent = request(&mut panel, Intent::SelectTown(Some(6))).ok_or("browse")?;
        let request = request_for_selection(&panel, intent, None);
        assert_eq!(request.cross_town, negotiated);
        assert_eq!(request.intent.town, negotiated.then_some(6));
    }
    Ok(())
}
