use super::task_board;
use std::fmt::Write;

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
    let _ = lwriteln!(
        text,
        "Bounty: state {}, difficulty {}, rerolls {}, free offer reroll at {}",
        bounty.state,
        bounty.difficulty,
        bounty.rerolls,
        bounty.daily_reroll_at
    );
    for (slot, offer) in bounty.offers.iter().enumerate() {
        let _ = lwriteln!(
            text,
            "Bounty {}. {}: {} kills, {} XP, {} points, grade {}",
            slot + 1,
            super::creature_name(&offer.race),
            offer.required,
            offer.experience,
            offer.points,
            offer.grade
        );
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
