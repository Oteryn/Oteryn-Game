//! Shared horse-station reports use the existing fenced Quest action owner.
//! Source Palomino/Appaloosa transport/rent gates: Crystal9f5 npc scripts:175-220.
//! data-global/npc/palomino.lua SHA256595387918194d62554ba02e66c381e945fc065a0c0b3b2658ce773fe52f293da;
//! data-global/npc/appaloosa.lua SHA2562972dcf9913edd16f693bf265a97acbdec9acde1de8487141ee776ed98efce19.
//! Explicit escape/return report dialogue is the owner's conservative fallback;
//! this availability check grants no personal mount or unpaid rental.
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
pub const QUEST: &str = "oteryn:quest.horse_station_world_change";
pub const ESCAPE: &str = "oteryn:quest-transition/world-action/horse/escape";
pub const RETURN: &str = "oteryn:quest-transition/world-action/horse/return";
pub const AVAILABLE: &str = "oteryn:quest-transition/world-action/horse/rent-available";

pub fn dialogue(
    npc: &str,
    message: &str,
    topic: u16,
) -> Option<super::world_contributions::Dialogue> {
    if !matches!(npc, "oteryn:npc.palomino" | "oteryn:npc.appaloosa") {
        return None;
    }
    let text = message.trim().to_ascii_lowercase();
    let offer = match text.as_str() {
        "escape" | "horse escaped" => Some((501, "Report that one stable horse escaped?")),
        "return horses" | "horse returned" => {
            Some((502, "Report a horse returning to the stable?"))
        }
        "rent" | "horses" => Some((
            503,
            "Check whether the shared stable can offer its services?",
        )),
        _ => None,
    };
    if let Some((topic, reply)) = offer {
        return Some(super::world_contributions::Dialogue {
            topic,
            transition: None,
            reply,
        });
    }
    if !(501..=503).contains(&topic) {
        return None;
    }
    match text.as_str() {
        "no" => Some(super::world_contributions::Dialogue {
            topic: 0,
            transition: None,
            reply: "Then not.",
        }),
        "yes" => Some(super::world_contributions::Dialogue {
            topic: 0,
            transition: Some(match topic {
                501 => ESCAPE,
                502 => RETURN,
                _ => AVAILABLE,
            }),
            reply: match topic {
                501 => "The escape is recorded for this World.",
                502 => "The horse's return is recorded for this World.",
                _ => "The stable service is available. Ask for transport to use a coach.",
            },
        }),
        _ => None,
    }
}

pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    for (key, name) in [
        (ESCAPE, "escapes_reported"),
        (RETURN, "returns_reported"),
        (AVAILABLE, "availability_checked"),
    ] {
        let track = format!("oteryn:quest-progress/horse-station/{name}");
        tracks.push(QuestTrack {
            key: track.clone(),
            quest: QUEST.into(),
            initial: 0,
            min: 0,
            max: i64::MAX,
        });
        transitions.push(QuestTransition {
            key: key.into(),
            quest: QUEST.into(),
            effects: vec![QuestEffect {
                track,
                from: QuestComparison::Any,
                effect: QuestEffectKind::Add(1),
            }],
            completes: false,
            experience: None,
        });
    }
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|error| format!("{error:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn horse_station_reports_are_npc_bound_and_need_confirmation() {
        assert!(dialogue("oteryn:npc.marvin", "yes", 501).is_none());
        assert_eq!(
            dialogue("oteryn:npc.palomino", "horse escaped", 0).map(|choice| choice.topic),
            Some(501)
        );
        assert_eq!(
            dialogue("oteryn:npc.appaloosa", "yes", 501).and_then(|choice| choice.transition),
            Some(ESCAPE)
        );
        assert_eq!(
            dialogue("oteryn:npc.palomino", "yes", 502).and_then(|choice| choice.transition),
            Some(RETURN)
        );
        assert_eq!(
            dialogue("oteryn:npc.palomino", "yes", 503).and_then(|choice| choice.transition),
            Some(AVAILABLE)
        );
        assert!(dialogue("oteryn:npc.palomino", "transport", 0).is_none());
        assert!(dialogue("oteryn:npc.palomino", "yes", 0).is_none());
    }
}
