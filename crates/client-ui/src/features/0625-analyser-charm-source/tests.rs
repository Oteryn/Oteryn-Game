use super::*;

#[test]
fn charm_damage_line_routes_only_system_damage() {
    let text = "A rat loses 40 hitpoints due to your attack. (enflame charm)";
    assert_eq!(
        damage_line(&ChatLine::System { text: text.into() }),
        Some(text)
    );
    assert_eq!(
        damage_line(&ChatLine::Private {
            speaker_name: "Someone".into(),
            text: text.into()
        }),
        None
    );
    assert_eq!(
        damage_line(&ChatLine::System {
            text: "You triggered the Enflame charm.".into()
        }),
        None
    );
}
