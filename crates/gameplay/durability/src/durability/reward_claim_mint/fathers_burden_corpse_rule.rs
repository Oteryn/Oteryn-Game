/// Donor actions_corpse.lua:2–19. The claim key is character-scoped,
/// while the physical corpse remains available to everyone in reach.
#[derive(Clone, Copy)]
pub struct FatherCorpseReward {
    pub(super) creature: &'static str,
    pub(super) corpse: &'static str,
    pub(super) item: &'static str,
    pub(super) claim: &'static str,
    pub feedback: &'static str,
}

pub fn father_corpse_reward(corpse: &str) -> Option<FatherCorpseReward> {
    let (corpse, creature, item, claim, feedback) = match corpse {
        "oteryn:item.tibia.i11589" => (
            "oteryn:item.tibia.i11589",
            "oteryn:creature.glitterscale",
            "oteryn:item.tibia.i11550",
            "oteryn:reward-claim.quest.fathers-burden.scale",
            "You acquired Glitterscale's scale.",
        ),
        "oteryn:item.tibia.i11590" => (
            "oteryn:item.tibia.i11590",
            "oteryn:creature.heoni",
            "oteryn:item.tibia.i11548",
            "oteryn:reward-claim.quest.fathers-burden.sinew",
            "You acquired Heoni's sinew",
        ),
        _ => return None,
    };
    Some(FatherCorpseReward {
        creature,
        corpse,
        item,
        claim,
        feedback,
    })
}

#[cfg(test)]
mod tests {
    use super::{FatherCorpseReward, father_corpse_reward};

    #[test]
    fn qfather_exact_corpse_definitions_grant_distinct_materials() {
        let fields = |rule: FatherCorpseReward| {
            (
                rule.corpse,
                rule.creature,
                rule.item,
                rule.claim,
                rule.feedback,
            )
        };
        let scale = father_corpse_reward("oteryn:item.tibia.i11589").map(fields);
        let sinew = father_corpse_reward("oteryn:item.tibia.i11590").map(fields);
        assert_eq!(
            scale,
            Some((
                "oteryn:item.tibia.i11589",
                "oteryn:creature.glitterscale",
                "oteryn:item.tibia.i11550",
                "oteryn:reward-claim.quest.fathers-burden.scale",
                "You acquired Glitterscale's scale.",
            ))
        );
        assert_eq!(
            sinew,
            Some((
                "oteryn:item.tibia.i11590",
                "oteryn:creature.heoni",
                "oteryn:item.tibia.i11548",
                "oteryn:reward-claim.quest.fathers-burden.sinew",
                "You acquired Heoni's sinew",
            ))
        );
        assert!(father_corpse_reward("oteryn:item.tibia.i11548").is_none());
    }
}
