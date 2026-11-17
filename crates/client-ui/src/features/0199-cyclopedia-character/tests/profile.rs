use super::*;
use oteryn_client_runtime::{
    CommandStatus,
    cyclopedia_character::{Badge, CharacterTab, EquippedItem, profile::Profile},
    raw_feature::GatedCommandResult,
};
#[test]
fn profile_badges_inspection_and_target_changes_are_coherent()
-> Result<(), Box<dyn std::error::Error>> {
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        target: Some([2; 16]),
        ..Default::default()
    };
    let cmd = request(&mut p, Intent::ProfileBadges).ok_or("request")?;
    assert_eq!(cmd.kind, 388);
    assert_eq!(
        cmd.payload,
        oteryn_client_runtime::cyclopedia_character::profile::request(Some([2; 16]))
    );
    assert!(request(&mut p, Intent::Tab).is_none());
    let profile = Profile {
        character: [2; 16],
        name: "Other Hero".into(),
        badges: vec![Badge::FledgelingHero],
        equipment: vec![EquippedItem {
            slot: 1,
            definition: "oteryn:item/hat".into(),
            quantity: 3,
        }],
    };
    let mut reply = GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: profile.encode().map_err(|_| "encode")?,
    };
    assert!(apply_result(&mut p, &reply));
    assert!(profile_heading(&p).contains("Other Hero"));
    let rows = profile_rows(&p);
    assert_eq!(rows.len(), Badge::ALL.len());
    assert!(rows[0].label.contains("Earned"));
    assert!(rows.iter().any(|r| r.label.contains("Not earned")));
    p.badge_filter = BadgeFilter::Earned;
    assert_eq!(profile_rows(&p).len(), 1);
    p.hidden_badges.insert(Badge::FledgelingHero.id());
    p.badge_filter = BadgeFilter::Hidden;
    assert!(profile_rows(&p)[0].label.contains("hidden locally"));
    p.badge_filter = BadgeFilter::All;

    request(&mut p, Intent::ProfileInspection).ok_or("inspection")?;
    assert!(apply_result(&mut p, &reply));
    assert_eq!(profile_rows(&p).len(), 1);
    assert!(profile_rows(&p)[0].detail.contains("Quantity: 3"));
    p.change_target(Some([3; 16]));
    assert!(p.profile.is_none());
    assert!(profile_rows(&p).is_empty());
    request(&mut p, Intent::ProfileBadges).ok_or("request")?;
    assert!(!apply_result(&mut p, &reply));
    assert!(p.profile.is_none());
    request(&mut p, Intent::ProfileBadges).ok_or("request")?;
    reply.status = CommandStatus::Rejected;
    assert!(!apply_result(&mut p, &reply));
    assert!(profile_rows(&p).is_empty());
    p.tab = CharacterTab::Defence;
    assert!(profile_rows(&p).is_empty());
    Ok(())
}
