use super::*;
use crate::imbue::{ImbueResponse, apply_result, imbuement, panel_text, selected_detail_text};
use oteryn_client_runtime::imbue::CommandStatus;

fn offers() -> Vec<imbuement::ImbuementOffer> {
    [(3, 2), (3, 3)]
        .into_iter()
        .map(|(family, tier)| imbuement::ImbuementOffer {
            family,
            tier,
            gold: 1000,
            requires_premium: false,
        })
        .collect()
}
fn reply() -> imbuement::ImbuementReply {
    imbuement::ImbuementReply {
        result: imbuement::ImbuementResult::Ok,
        window: Some(imbuement::ImbuementWindow {
            slot_count: 1,
            applied: vec![],
            offers: offers(),
            clear_gold: 500,
        }),
    }
}
fn panel(selected: bool, success: Vec<u8>) -> ImbuePanel {
    let mut panel = ImbuePanel {
        selected: true,
        success_selected: selected,
        ..ImbuePanel::default()
    };
    apply_result(
        &mut panel,
        &ImbueResponse {
            status: CommandStatus::Accepted,
            reply: Some(reply()),
            success,
        },
    );
    panel
}

#[test]
fn c0491_selected_offers_show_the_servers_success_percentages() {
    let mut p = panel(true, vec![85, 60]);
    assert_eq!(success_percent(&p, 0), Some(85));
    assert_eq!(success_percent(&p, 1), Some(60));
    assert_eq!(success_percent(&p, 2), None);
    assert!(selected_detail_text(&p).contains("Success chance: 85% (server)"));
    assert!(!selected_detail_text(&p).contains("100%"));
    assert!(panel_text(&p).contains(", 60% success"));
    p.offer = 1;
    assert!(selected_detail_text(&p).contains("Success chance: 60% (server)"));
}

#[test]
fn c0491_unselected_session_keeps_the_fixed_hundred_percent_line() {
    let p = panel(false, vec![85, 60]);
    assert_eq!(success_percent(&p, 0), None);
    assert!(p.success.is_empty());
    assert!(selected_detail_text(&p).contains("Success chance: 100% (no protection charge)"));
    assert!(!panel_text(&p).contains("% success"));
}

#[test]
fn c0491_a_mismatched_percentage_list_is_never_shown() {
    let p = panel(true, vec![85]);
    assert_eq!(success_percent(&p, 0), None);
    let mut p = panel(true, vec![85, 60]);
    p.success[0] = 101;
    assert_eq!(success_percent(&p, 0), None);
}

#[test]
fn c0491_a_mutation_reply_discards_the_old_percentages() {
    let mut p = panel(true, vec![85, 60]);
    apply_result(
        &mut p,
        &ImbueResponse {
            status: CommandStatus::Accepted,
            reply: Some(imbuement::ImbuementReply {
                result: imbuement::ImbuementResult::Ok,
                window: None,
            }),
            success: Vec::new(),
        },
    );
    assert!(p.window.is_none() && p.success.is_empty());
}
