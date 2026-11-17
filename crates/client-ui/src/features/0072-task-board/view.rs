use super::task_board;
use std::fmt::Write;

/// Bounty Reroll Tokens are stored up to this many (Tibia 15.33 client text).
pub const REROLL_TOKEN_CAP: u64 = 10;

pub const DIFFICULTIES: [(&str, &str); 4] = [
    ("Beginner", "Beginner tasks pick easy creatures."),
    ("Adept", "Adept tasks pick easy and medium creatures."),
    ("Expert", "Expert tasks pick medium and hard creatures."),
    (
        "Master",
        "Master tasks pick hard and challenging creatures.",
    ),
];

/// Tooltip text of one bounty difficulty; unknown levels have none.
#[must_use]
pub fn difficulty_tooltip(level: u8) -> &'static str {
    DIFFICULTIES
        .get(usize::from(level))
        .map_or("", |(_, tooltip)| tooltip)
}

#[must_use]
pub fn difficulty_name(level: u8) -> &'static str {
    DIFFICULTIES
        .get(usize::from(level))
        .map_or("Unknown", |(name, _)| name)
}

/// Task rarity shown for an offer's grade: Ordinary, Silver, Gold, Platinum.
#[must_use]
pub fn rarity_name(grade: u8) -> &'static str {
    match grade {
        0 => "Ordinary",
        1 => "Silver",
        2 => "Gold",
        _ => "Platinum",
    }
}

fn bounty_header_text(bounty: &task_board::full::Bounty) -> String {
    let mut text = format!(
        "Bounty: state {}, Task Difficulty: {}\n",
        bounty.state,
        difficulty_name(bounty.difficulty)
    );
    text.push_str(difficulty_tooltip(bounty.difficulty));
    text.push('\n');
    for (name, tooltip) in DIFFICULTIES {
        let _ = writeln!(text, "  {name}: {tooltip}");
    }
    let _ = writeln!(
        text,
        "Bounty Reroll Token: {} / {}{}",
        bounty.rerolls,
        REROLL_TOKEN_CAP,
        if bounty.rerolls >= REROLL_TOKEN_CAP {
            " (maximum)"
        } else {
            ""
        }
    );
    text.push_str(
        "Tasks can be re-rolled using Reroll Tokens. One token can be claimed for free each day from the Daily Reward Wall. You can store up to 10 Reroll Tokens at a time.\n",
    );
    text.push_str(
        "Elite: monsters deal 100% more damage to you and take 50% less damage from you. Completing an Elite task gives 100% more XP and additional dust based on difficulty. Dust is only awarded to Premium accounts. Difficulty and Elite changes only apply to your next task.\n",
    );
    if bounty.elite || bounty.active.as_ref().is_some_and(|task| task.elite) {
        text.push_str("Elite tasks are active.\n");
    }
    text.push_str(
        "Each time a creature is offered and not selected, its chance to appear as a Silver (up to 75%), Gold (up to 20%) or Platinum (up to 5%) task grows. A creature must be rerolled at least once before it can appear as Platinum.\n",
    );
    text
}

pub(super) fn panel_text(reply: &task_board::full::Reply, feedback: Option<&str>) -> String {
    let mut text = lfmt!(
        "Task Board — week {}\nPoints: {}  Bounty points: {}  Soulseals: {}\n",
        reply.period,
        reply.points,
        reply.bounty_points,
        reply.soulseals
    );
    let weekly = &reply.weekly;
    let _ = lwriteln!(
        text,
        "Weekly: started {}, difficulty {}, expanded {}, multiplier {}",
        weekly.started,
        weekly.difficulty,
        weekly.expanded,
        weekly.multiplier
    );
    let _ = lwriteln!(
        text,
        "Kill XP: {}  Delivery XP: {}  Pending points: {}  Pending soulseals: {}",
        weekly.kill_experience,
        weekly.delivery_experience,
        weekly.pending_points,
        weekly.pending_soulseals
    );
    for (slot, task) in weekly.kills.iter().enumerate() {
        let race = if task.race.is_empty() {
            tr!("Any creature").into()
        } else {
            super::creature_name(&task.race)
        };
        let _ = lwriteln!(
            text,
            "Kill {}. {race}: {}/{}",
            slot + 1,
            task.current,
            task.required,
            race = race
        );
    }
    for (slot, task) in weekly.deliveries.iter().enumerate() {
        let _ = lwriteln!(
            text,
            "Delivery {}. {}: {}/{}  delivered: {}",
            slot + 1,
            task.item,
            task.current,
            task.required,
            task.delivered
        );
    }
    let bounty = &reply.bounty;
    text.push_str(&bounty_header_text(bounty));
    for (slot, offer) in bounty.offers.iter().enumerate().chain(
        bounty
            .active
            .iter()
            .map(|offer| (usize::from(bounty.selected.saturating_sub(1)), offer)),
    ) {
        let _ = lwriteln!(
            text,
            "Bounty {}. {}: {} kills, {} XP, {} points, grade {}",
            slot + 1,
            super::creature_name(&offer.race),
            offer.required,
            offer.experience,
            offer.points,
            rarity_name(offer.grade)
        );
        if offer.elite {
            let _ = writeln!(
                text,
                "{} — {}: {}",
                tr!("Elite"),
                tr!("Dust (Premium)"),
                offer.dust
            );
        }
    }
    if let Some(active) = &bounty.active {
        let _ = lwriteln!(
            text,
            "Active bounty: {} {}/{} (offer {})",
            super::creature_name(&active.race),
            bounty.current,
            active.required,
            bounty.selected
        );
    }
    for (slot, list) in bounty.lists.iter().enumerate() {
        let _ = lwriteln!(
            text,
            "List {}: unlocked {}, preferred {}, unwanted {}",
            slot + 1,
            list.unlocked,
            list.preferred,
            list.unwanted
        );
    }
    text.push_str(tr!("\nTask shop\n"));
    for offer in &reply.shop {
        let _ = writeln!(text, "{}", shop_offer_text(offer, reply.points));
    }
    text.push_str(tr!("\nBounty Talisman upgrade paths\n"));
    for (path, value) in reply.talisman.iter().enumerate() {
        let _ = lwriteln!(
            text,
            "  Level {} → {}",
            value.level,
            value.level.saturating_add(1).min(355)
        );
        let _ = lwriteln!(
            text,
            "Talisman {} ({}): level {}, bonus {}, next cost {}",
            path + 1,
            [
                tr!("Damage"),
                tr!("Life leech"),
                tr!("Loot"),
                tr!("Bestiary")
            ][path],
            value.level,
            value.bonus,
            value.next_cost
        );
    }
    if let Some(feedback) = feedback {
        text.push_str(feedback);
    }
    text
}

pub fn shop_offer_text(offer: &task_board::full::ShopOffer, points: u64) -> String {
    let status = if offer.bought {
        tr!("Already owned").into()
    } else if points >= u64::from(offer.price) {
        tr!("Available — server confirms purchase").into()
    } else {
        lfmt!(
            "{} points short",
            u64::from(offer.price).saturating_sub(points)
        )
    };
    format!(
        "Shop {} — {}\n  Reward {} / {} (addon {})\n  {} task points — {status}",
        offer.index + 1,
        super::shop_names::name(offer),
        offer.primary,
        offer.secondary,
        offer.addon,
        offer.price
    )
}

pub fn shop_offer_choice(offer: &task_board::full::ShopOffer, points: u64) -> String {
    format!(
        "{}. {} — {} task points — {}",
        offer.index + 1,
        super::shop_names::name(offer),
        offer.price,
        if offer.bought {
            "Owned"
        } else if points >= u64::from(offer.price) {
            "Available"
        } else {
            "More points needed"
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn reply(difficulty: u8, rerolls: u64, elite: bool, grade: u8) -> task_board::full::Reply {
        task_board::full::Reply {
            refusal: 0,
            period: 1,
            points: 0,
            bounty_points: 0,
            soulseals: 0,
            weekly: task_board::full::Weekly {
                started: false,
                difficulty: 0,
                expanded: false,
                multiplier: 1,
                kill_experience: 0,
                delivery_experience: 0,
                pending_points: 0,
                pending_soulseals: 0,
                kills: vec![],
                deliveries: vec![],
            },
            weekly_results: None,
            bounty: task_board::full::Bounty {
                elite,
                state: 1,
                difficulty,
                selected: 0,
                current: 0,
                rerolls,
                daily_reroll_at: 0,
                offers: vec![task_board::full::BountyOffer {
                    elite,
                    dust: 560,
                    race: "oteryn:creature.rat".into(),
                    required: 10,
                    experience: 100,
                    points: 5,
                    grade,
                }],
                active: None,
                lists: vec![],
            },
            talisman: Default::default(),
            shop: vec![],
        }
    }
    #[test]
    fn difficulty_names_and_tooltips_follow_the_test_client() {
        let names: Vec<_> = (0..4).map(difficulty_name).collect();
        assert_eq!(names, ["Beginner", "Adept", "Expert", "Master"]);
        assert!(difficulty_tooltip(1).contains("easy and medium"));
        assert!(difficulty_tooltip(2).contains("medium and hard"));
        assert!(difficulty_tooltip(3).contains("hard and challenging"));
        assert_eq!(difficulty_tooltip(9), "");
        let text = panel_text(&reply(2, 4, false, 0), None);
        assert!(text.contains("Task Difficulty: Expert"));
        assert!(text.contains("Master: Master tasks pick hard and challenging creatures."));
    }
    #[test]
    fn elite_text_rarity_and_reroll_token_cap_are_shown_without_daily_claim() {
        let text = panel_text(&reply(0, 10, true, 3), None);
        assert!(
            text.contains(
                "monsters deal 100% more damage to you and take 50% less damage from you"
            )
        );
        assert!(text.contains("100% more XP and additional dust"));
        assert!(text.contains("Dust is only awarded to Premium accounts"));
        assert!(text.contains("Silver (up to 75%), Gold (up to 20%) or Platinum (up to 5%)"));
        assert!(text.contains("grade Platinum"));
        assert!(text.contains("Bounty Reroll Token: 10 / 10 (maximum)"));
        assert!(text.contains("You can store up to 10 Reroll Tokens at a time."));
        assert!(!text.contains("Claim Daily"));
        assert!(!text.contains("each reroll costs"));
        assert!(panel_text(&reply(0, 3, false, 1), None).contains("Bounty Reroll Token: 3 / 10\n"));
        assert!(panel_text(&reply(0, 3, false, 1), None).contains("grade Silver"));
    }
    #[test]
    fn shop_cards_name_rewards_and_show_affordability_and_ownership() {
        let mut offer = task_board::full::ShopOffer {
            index: 7,
            kind: 1,
            price: 500,
            primary: 42,
            secondary: 0,
            addon: 0,
            bought: false,
        };
        assert!(shop_offer_text(&offer, 499).contains("1 points short"));
        assert!(shop_offer_text(&offer, 500).contains("Mount"));
        assert!(shop_offer_text(&offer, 500).contains("Available"));
        offer.bought = true;
        assert!(shop_offer_text(&offer, 500).contains("Already owned"));
    }
}
