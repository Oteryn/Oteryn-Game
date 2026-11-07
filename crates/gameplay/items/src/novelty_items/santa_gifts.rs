//! Closed Christmas gift leaves from pinned Crystal piggy_bank.lua and dolls.lua.
//! Selection is receipt-bound by the existing consume/transform owner.
use super::*;

pub const DONOR: &str = "crystal-201189f871d23597b82caf88fd611454a2ddefb8";

#[derive(Debug, PartialEq, Eq)]
pub struct GiftSelection {
    pub effects: NoveltyEffects,
    /// Transform the same bank instance; never consume/remint its identity.
    pub transform: Option<u32>,
    /// Always delivered to the actor inventory, including Ground/House sources.
    pub currency: Option<u32>,
}

// Current TibiaWiki article12222 identifies5791/6566 as the same usable
// Stuffed Dragon. Pinned donor registers only transient6566; the base alias
// and current third voice are explicit source corrections (SRV7 evidence).
pub fn recognizes(id: u32) -> bool {
    matches!(id, 2995 | 5080 | 5791 | 6511 | 6566)
}

pub fn select(id: u32, mut draw: impl FnMut(u8, u64) -> Option<u64>) -> Option<GiftSelection> {
    if !recognizes(id) {
        return None;
    }
    let mut effects = NoveltyEffects {
        kind: NoveltyKind::ConsumeEffect,
        source_item_id: id,
        donor: format!(
            "{DONOR}:data/scripts/actions/items/{}",
            if id == 2995 {
                "piggy_bank.lua:3-13"
            } else if id == 6511 {
                "dolls.lua:23-30,111-149"
            } else {
                "dolls.lua:2,15-21,111-149"
            }
        ),
        ground: None,
        cues: Vec::new(),
        speech: None,
        private_speech: false,
        require_head: false,
        require_player_target: false,
        dice_value: None,
        costume: None,
        damage: None,
        achievement: None,
        achievement_progress: Vec::new(),
        timed_output: None,
    };
    let mut transform = None;
    let mut currency = None;
    let mut cue = None;
    match id {
        2995 => {
            let roll = draw(210, 6)?;
            if roll >= 6 {
                return None;
            }
            if roll == 0 {
                transform = Some(2996);
                currency = Some(3031);
                cue = Some(("poff", 3));
                effects.achievement_progress.push(AchievementProgress {
                    key: "oteryn:achievement/allowance_collector".into(),
                    target: 50,
                });
            } else {
                currency = Some(3035);
                cue = Some(("sound_yellow", 22));
            }
        }
        5080 => effects.speech = Some("Hug me!".into()),
        6511 => {
            const VOICES: [&str; 6] = [
                "Ho ho ho",
                "Jingle bells, jingle bells...",
                "Have you been naughty?",
                "Have you been nice?",
                "Merry Christmas!",
                "Can you stop squeezing me now... I'm starting to feel a little sick.",
            ];
            effects.speech = Some(
                VOICES
                    .get(usize::try_from(draw(212, 6)?).ok()?)?
                    .to_string(),
            );
        }
        5791 | 6566 => {
            const SPEECH: [&str; 5] = [
                "Fchhhhhh!",
                "Zchhhhhh!",
                "Groaaarrr.... *cough*",
                "Aaa... CHOO!",
                "You... will.... burn!!",
            ];
            let roll = draw(211, 5)?;
            effects.speech = Some(SPEECH.get(usize::try_from(roll).ok()?)?.to_string());
            cue = match roll {
                2 => Some(("poff", 3)),
                3 => Some(("firearea", 7)),
                4 => {
                    effects.damage = Some(1);
                    Some(("explosionhit", 6))
                }
                _ => None,
            };
        }
        _ => return None,
    }
    if let Some((binding, effect_id)) = cue {
        effects.cues.push(NoveltyCue {
            binding: format!("appearance:effect/{binding}"),
            effect_id,
        });
    }
    Some(GiftSelection {
        effects,
        transform,
        currency,
    })
}

/// Both inventory and world USE retain the exact donor speech/effect coordinate.
#[allow(clippy::too_many_arguments)]
pub fn plan(
    native: &NativeGameplayState,
    used: &TypedDefinitionRef,
    session: [u8; 16],
    command: u64,
    source: [u8; 16],
    ground: Option<GroundEffect>,
) -> Option<NoveltyPlan> {
    let policy = native.item_policy(&used.production_key, &used.revision_ref)?;
    if !policy.record().admission.materializable
        || policy.record().production_definition.family != used.family
    {
        return None;
    }
    let id = used
        .production_key
        .strip_prefix("oteryn:item.tibia.i")?
        .parse()
        .ok()?;
    let mut selection = select(id, |domain, upper| {
        reward_bag_bounded_draw(session, command, source, domain, upper)
    })?;
    selection.effects.ground = ground;
    let mut outputs = Vec::new();
    if let Some(currency) = selection.currency {
        let policy = native.item_policy_for_source_id(currency)?;
        if !policy.record().admission.materializable {
            return None;
        }
        let item = &policy.record().production_definition;
        outputs.push((
            TypedDefinitionRef {
                family: item.family.clone(),
                production_key: item.production_key.clone(),
                revision_ref: item.revision_ref.clone(),
            },
            1,
        ));
    }
    if let Some(target) = selection.transform
        && !native
            .item_policy_for_source_id(target)?
            .record()
            .admission
            .materializable
    {
        return None;
    }
    Some(NoveltyPlan {
        recipe: format!("oteryn:novelty-item.i{id}.v1"),
        outputs,
        consume_source: false,
        effects: serde_json::to_value(selection.effects).ok()?,
    })
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::panic,
        reason = "finite donor branch fixtures reject unexpected random draws"
    )]
    use super::*;
    #[test]
    fn srv7_santa_gift_leaf_all_donor_branches_are_exact() {
        for roll in 0..6 {
            let plan = select(2995, |domain, upper| {
                assert_eq!((domain, upper), (210, 6));
                Some(roll)
            })
            .unwrap();
            assert_eq!(plan.transform, (roll == 0).then_some(2996));
            assert_eq!(plan.currency, Some(if roll == 0 { 3031 } else { 3035 }));
            assert_eq!(
                plan.effects.achievement_progress.len(),
                usize::from(roll == 0)
            );
        }
        let panda = select(5080, |_, _| panic!("panda has no random branch")).unwrap();
        assert_eq!(panda.effects.speech.as_deref(), Some("Hug me!"));
        for roll in 0..5 {
            let plan = select(6566, |domain, upper| {
                assert_eq!((domain, upper), (211, 5));
                Some(roll)
            })
            .unwrap();
            assert!(plan.transform.is_none() && plan.currency.is_none());
            assert_eq!(plan.effects.damage, (roll == 4).then_some(1));
            assert_eq!(plan.effects.cues.len(), usize::from(roll >= 2));
        }
        assert!(select(6566, |_, _| Some(5)).is_none());
        assert!(select(2995, |_, _| Some(6)).is_none());
        assert!(select(6387, |_, _| Some(0)).is_none());
    }
}
