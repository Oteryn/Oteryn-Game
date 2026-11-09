use oteryn_client_ui::cyclopedia_character::{BadgeFilter, CyclopediaCharacterPanel, badge_rows};
#[test]
fn badge_filters_use_confirmed_earned_ids_and_local_hidden_choices() {
    let mut panel = CyclopediaCharacterPanel::default();
    assert!(badge_rows(&panel).is_empty());
    panel.selected = true;
    assert!(
        oteryn_client_ui::cyclopedia_character::request(
            &mut panel,
            oteryn_client_ui::cyclopedia_character::Intent::Badges
        )
        .is_some()
    );
    assert!(oteryn_client_ui::cyclopedia_character::apply_result(
        &mut panel,
        &oteryn_client_runtime::raw_feature::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_client_runtime::CommandStatus::Accepted,
            payload: vec![1, 1],
        }
    ));
    panel.badge_filter = BadgeFilter::Earned;
    assert_eq!(badge_rows(&panel).len(), 1);
    panel.badge_filter = BadgeFilter::Locked;
    assert!(badge_rows(&panel).iter().all(|row| row.id != 1));
    panel.hidden_badges.insert(1);
    panel.badge_filter = BadgeFilter::Hidden;
    assert_eq!(badge_rows(&panel).len(), 1);
    panel.change_target(Some([5; 16]));
    assert!(badge_rows(&panel).is_empty());
}
