//! Bounded Premium-service dialogue over the existing NPC conversation owner.
use oteryn_gameplay_content::content::npc_catalogue::premium_services::PremiumNpcService;

pub fn select<'a>(text: &str, services: &'a [PremiumNpcService]) -> Option<&'a PremiumNpcService> {
    let text = text.trim_matches(' ');
    services.iter().find(|service| match service {
        PremiumNpcService::Henricus => {
            text.eq_ignore_ascii_case("bless")
                || text.eq_ignore_ascii_case("blessing")
                || text.eq_ignore_ascii_case("blessings")
        }
        PremiumNpcService::Promotion { .. } => text.eq_ignore_ascii_case("promotion"),
        PremiumNpcService::Blessing(blessing) => {
            text.eq_ignore_ascii_case("blessing") || text.eq_ignore_ascii_case(&blessing.name)
        }
    })
}

pub fn prompt(service: &PremiumNpcService) -> String {
    match service {
        PremiumNpcService::Henricus => "Do you want all five regular blessings at your current level price plus 10%? You must have completed The Inquisition Quest and hold none of these blessings.".into(),
        PremiumNpcService::Promotion { .. } => {
            "Do you want to be promoted for 20000 gold coins?".into()
        }
        PremiumNpcService::Blessing(blessing) => format!(
            "Do you want to buy {} at the price for your current level?",
            blessing.name
        ),
    }
}

#[cfg(test)]
#[path = "npc_promotion_tests.rs"]
mod tests;
