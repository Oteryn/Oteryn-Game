use super::*;
#[test]
fn task_board_shows_server_tasks_only() {
    let panel = TaskBoardPanel {
        weekly: Some(full_reply()),
        ..Default::default()
    };
    assert!(board_text(&panel, Tab::Weekly).contains("25/50"));
    assert!(!board_text(&panel, Tab::Bounty).contains("Invented race"));
    assert!(board_text(&TaskBoardPanel::default(), Tab::Weekly).contains("Refresh"));
}
#[test]
fn task_board_actions_are_bound_and_confirm_paid() {
    assert!(paid(&TaskBoardControl::Request(FullIntent::Reroll)));
    assert!(!paid(&TaskBoardControl::Request(FullIntent::ClaimBounty)));
}
#[test]
fn soulseals_and_weekly_results_show_server_data() {
    let panel = TaskBoardPanel::default();
    assert!(board_text(&panel, Tab::Soulseals).contains("No fight catalogue loaded"));
    assert!(!board_text(&panel, Tab::Results).contains("Weekly rewards received"));
    let mut reply = full_reply();
    reply.weekly_results = Some(task_board::task_board::full::WeeklyResults {
        period: 42,
        points: 731,
        soulseals: 19,
    });
    let panel = TaskBoardPanel {
        weekly: Some(reply),
        ..Default::default()
    };
    assert!(board_text(&panel, Tab::Results).contains("731"));
    assert!(board_text(&panel, Tab::Results).contains("19"));
}
#[test]
fn task_board_actions_rejected_when_session_replaced() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut incoming, _) = crate::play::play_channel();
    let panel = link.task_board_panel();
    assert!(!link.task_board_action_bound(&panel, TaskBoardControl::Request(FullIntent::Refresh)));
    assert_eq!(link.task_board_panel(), panel);
    assert!(incoming.try_recv().is_err());
    Ok(())
}

fn full_reply() -> task_board::task_board::full::Reply {
    use task_board::task_board::full::*;
    Reply {
        weekly_results: None,
        refusal: 0,
        period: 2965,
        points: 150,
        bounty_points: 20,
        soulseals: 2,
        weekly: Weekly {
            started: true,
            difficulty: 0,
            expanded: false,
            multiplier: 1,
            kill_experience: 1850,
            delivery_experience: 1850,
            pending_points: 25,
            pending_soulseals: 0,
            kills: vec![KillTask {
                race: "oteryn:creature.rat".into(),
                required: 50,
                current: 25,
            }],
            deliveries: vec![DeliveryTask {
                item: 3031,
                required: 100,
                current: 75,
                delivered: false,
            }],
        },
        bounty: Bounty {
            elite: false,
            state: 0,
            difficulty: 0,
            selected: 0,
            current: 0,
            rerolls: 3,
            daily_reroll_at: 0,
            offers: vec![],
            active: None,
            lists: vec![],
        },
        talisman: [TalismanPath {
            level: 1,
            bonus: 250,
            next_cost: 17,
        }; 4],
        shop: vec![ShopOffer {
            index: 0,
            kind: 2,
            price: 100_000,
            primary: 1907,
            secondary: 0,
            addon: 0,
            bought: false,
        }],
    }
}
