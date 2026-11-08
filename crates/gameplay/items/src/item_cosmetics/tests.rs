use super::*;
use std::collections::BTreeSet;

struct Content {
    admitted: bool,
}
impl SourceCosmetics for Content {
    fn outfit_pair(&self, looks: &[u32]) -> Option<String> {
        (looks.len() == 2).then(|| format!("test.outfit.{}-{}", looks[0], looks[1]))
    }
    fn mount_look(&self, look: u32) -> Option<String> {
        Some(format!("test.mount.{look}"))
    }
}
impl AppearanceContent<String> for Content {
    fn palette_size(&self) -> u16 {
        133
    }
    fn outfit_addon_count(&self, _: &String) -> Option<u8> {
        Some(2)
    }
    fn outfit_is_starter(&self, _: &String, _: Sex) -> bool {
        false
    }
    fn addon_is_starter(&self, _: &String, _: Sex, _: u8) -> bool {
        false
    }
    fn mount_exists(&self, _: &String) -> bool {
        true
    }
    fn earned_unlock(&self, _: &str, provenance: &str, _: EarnedCosmetic<'_>) -> UnlockFact {
        if self.admitted && provenance == PROVENANCE {
            UnlockFact::Compatible
        } else {
            UnlockFact::Absent
        }
    }
}
#[derive(Default)]
struct Owned {
    outfits: BTreeSet<String>,
    addons: BTreeSet<(String, u8)>,
    mounts: BTreeSet<String>,
}
impl AccountUnlocks<String> for Owned {
    fn outfit(&self, key: &String) -> UnlockFact {
        if self.outfits.contains(key) {
            UnlockFact::Compatible
        } else {
            UnlockFact::Absent
        }
    }
    fn addon(&self, key: &String, addon: u8) -> UnlockFact {
        if self.addons.contains(&(key.clone(), addon)) {
            UnlockFact::Compatible
        } else {
            UnlockFact::Absent
        }
    }
    fn mount(&self, key: &String) -> UnlockFact {
        if self.mounts.contains(key) {
            UnlockFact::Compatible
        } else {
            UnlockFact::Absent
        }
    }
}

#[test]
fn source_catalogue_keeps_every_item_family_and_numeric_mount_identity() -> Result<(), &'static str>
{
    let c = catalogue().ok_or("catalogue")?;
    let captured: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tools/content-schema/native-gameplay/item-cosmetic-actions.json"
    ))
    .map_err(|_| "immutable source capture")?;
    assert_eq!(
        captured["rules"].as_array().ok_or("source rules")?.len(),
        48
    );
    assert_eq!(
        captured["grants"].as_array().ok_or("source grants")?.len(),
        205
    );
    assert_eq!(c.rules.len(), 49);
    assert_eq!(c.grants.len(), 202);
    let doll = rule("oteryn:item.tibia.i6579").ok_or("doll")?;
    assert_eq!(doll.choices.len(), 88);
    let mount = rule("oteryn:item.tibia.i37746").ok_or("mount")?;
    assert_eq!(mount.choices.len(), 35);
    let first = c
        .grants
        .iter()
        .find(|g| g.key == mount.choices[0].owned)
        .ok_or("grant")?;
    // mount_random.lua calls addMount(23) despite the table's misleading display name.
    // Crystal data/XML/mounts.xml:52 defines id 23 as appearance 426.
    assert_eq!(first.mount_id, Some(23));
    assert_eq!(first.look_type, Some(426));
    assert_eq!(
        rule("oteryn:item.tibia.i12551")
            .ok_or("pieces")?
            .required_items,
        [12551, 12552, 12553, 12554, 12555, 12556]
    );
    Ok(())
}

#[test]
fn addon_doll_has_two_independent_draws_and_never_rerolls_an_owned_addon()
-> Result<(), &'static str> {
    let c = catalogue().ok_or("catalogue")?;
    let r = rule("oteryn:item.tibia.i6579").ok_or("rule")?;
    let mut calls = Vec::new();
    let selected = select(r, |domain, upper| {
        calls.push((domain, upper));
        Some(upper - 1)
    })
    .ok_or("choice")?;
    assert_eq!(calls, [(224, 44), (225, 2)]);
    assert_eq!(selected.owned, r.choices[87].owned);
    let content = Content { admitted: true };
    let grant = resolve(c, &content, &selected.owned).map_err(|_| "resolve")?;
    let mut owned = Owned::default();
    owned.addons.insert((grant.cosmetic_key, grant.addon));
    let result = decide(
        c,
        r,
        selected,
        &content,
        &owned,
        Sex::Female,
        false,
        &BTreeMap::new(),
    )
    .map_err(|_| "decision")?;
    assert!(!result.consume);
    assert!(result.grants.is_empty());
    Ok(())
}

#[test]
fn matching_source_look_without_an_admitted_grant_mapping_authorizes_nothing()
-> Result<(), &'static str> {
    let c = catalogue().ok_or("catalogue")?;
    let r = rule("oteryn:item.tibia.i16252").ok_or("rule")?;
    assert!(matches!(
        decide(
            c,
            r,
            &r.choices[0],
            &Content { admitted: false },
            &Owned::default(),
            Sex::Female,
            true,
            &BTreeMap::new()
        ),
        Err(Refusal::ContentUnavailable)
    ));
    assert!(matches!(
        decide(
            c,
            r,
            &r.choices[0],
            &Content { admitted: true },
            &Owned::default(),
            Sex::Female,
            false,
            &BTreeMap::new()
        ),
        Err(Refusal::PremiumRequired)
    ));
    let addon = rule("oteryn:item.tibia.i16253").ok_or("addon")?;
    assert!(matches!(
        decide(
            c,
            addon,
            &addon.choices[0],
            &Content { admitted: true },
            &Owned::default(),
            Sex::Male,
            true,
            &BTreeMap::new()
        ),
        Err(Refusal::OutfitRequired)
    ));
    Ok(())
}

#[test]
fn spectral_parts_grant_only_after_all_six_sourced_contributions() -> Result<(), &'static str> {
    let c = catalogue().ok_or("catalogue")?;
    let r = rule("oteryn:item.tibia.i34072").ok_or("horseshoe")?;
    let content = Content { admitted: true };
    let mut progress = BTreeMap::new();
    progress.insert("spectral-horseshoes".to_owned(), 3);
    progress.insert("spectral-saddle".to_owned(), 1);
    let result = decide(
        c,
        r,
        &r.choices[0],
        &content,
        &Owned::default(),
        Sex::Male,
        false,
        &progress,
    )
    .map_err(|_| "partial")?;
    assert!(result.consume);
    assert!(result.grants.is_empty());
    assert!(result.achievements.is_empty());
    assert_eq!(
        result.progress.as_ref().map(|p| (p.before, p.after)),
        Some((3, 4))
    );
    assert!(
        result.feedback.is_none(),
        "the source completion message waits for all parts"
    );
    progress.insert("spectral-horse-tac".to_owned(), 1);
    let result = decide(
        c,
        r,
        &r.choices[0],
        &content,
        &Owned::default(),
        Sex::Male,
        false,
        &progress,
    )
    .map_err(|_| "complete")?;
    assert_eq!(result.grants.len(), 1);
    assert_eq!(
        result.achievements,
        [
            "oteryn:achievement/natural_born_cowboy",
            "oteryn:achievement/you_got_horse_power"
        ]
    );
    assert_eq!(
        result.feedback.as_deref(),
        Some("Phantasmal jade is now yours!")
    );
    let mut owned = Owned::default();
    owned.mounts.insert(result.grants[0].cosmetic_key.clone());
    let duplicate = decide(
        c,
        r,
        &r.choices[0],
        &content,
        &owned,
        Sex::Male,
        false,
        &progress,
    )
    .map_err(|_| "duplicate")?;
    assert!(!duplicate.consume);
    assert!(duplicate.progress.is_none());
    assert!(duplicate.feedback.is_none());
    Ok(())
}

#[test]
fn random_doll_adds_the_base_outfit_and_only_its_selected_addon() -> Result<(), &'static str> {
    let c = catalogue().ok_or("catalogue")?;
    let r = rule("oteryn:item.tibia.i6579").ok_or("doll")?;
    let result = decide(
        c,
        r,
        &r.choices[0],
        &Content { admitted: true },
        &Owned::default(),
        Sex::Male,
        false,
        &BTreeMap::new(),
    )
    .map_err(|_| "decision")?;
    assert_eq!(result.grants.len(), 2);
    assert_eq!(result.grants[0].kind, GrantKind::Outfit);
    assert_eq!(result.grants[1].kind, GrantKind::Addon);
    assert_eq!(result.grants[1].addon, 1);
    assert!(result.achievements.is_empty());
    Ok(())
}

#[test]
fn ground_afflicted_trigger_keeps_itself_and_requires_all_six_carried_pieces()
-> Result<(), &'static str> {
    let c = catalogue().ok_or("catalogue")?;
    let content = Content { admitted: true };
    for id in 12551..=12556 {
        let r = rule(&format!("oteryn:item.tibia.i{id}")).ok_or("piece")?;
        let result = decide(
            c,
            r,
            &r.choices[0],
            &content,
            &Owned::default(),
            Sex::Female,
            true,
            &BTreeMap::new(),
        )
        .map_err(|_| "decision")?;
        let ground = inputs(r, &result, false);
        assert!(
            !ground.consume_source,
            "Crystal removes carried pieces, not the clicked Ground item"
        );
        assert_eq!(
            ground.carried_items,
            [12551, 12552, 12553, 12554, 12555, 12556]
        );
        let carried = inputs(r, &result, true);
        assert!(carried.consume_source);
        assert_eq!(carried.carried_items.len(), 5);
        assert!(!carried.carried_items.contains(&id));
        let mut owned = Owned::default();
        owned.outfits.insert(result.grants[0].cosmetic_key.clone());
        let duplicate = decide(
            c,
            r,
            &r.choices[0],
            &content,
            &owned,
            Sex::Female,
            true,
            &BTreeMap::new(),
        )
        .map_err(|_| "owned")?;
        let duplicate_inputs = inputs(r, &duplicate, false);
        assert!(!duplicate_inputs.consume_source);
        assert!(duplicate_inputs.carried_items.is_empty());
    }
    let doll = rule("oteryn:item.tibia.i6579").ok_or("doll")?;
    let result = decide(
        c,
        doll,
        &doll.choices[0],
        &content,
        &Owned::default(),
        Sex::Male,
        false,
        &BTreeMap::new(),
    )
    .map_err(|_| "doll decision")?;
    let ground_doll = inputs(doll, &result, false);
    assert!(
        ground_doll.consume_source,
        "item:remove uses the actual Ground doll"
    );
    assert!(ground_doll.carried_items.is_empty());
    Ok(())
}

#[test]
fn source_cosmetic_text_keeps_private_speech_separate_from_game_feedback()
-> Result<(), &'static str> {
    for id in [12551, 12552, 12553, 12554, 12555, 12556, 12786, 12787] {
        let r = rule(&format!("oteryn:item.tibia.i{id}")).ok_or("Afflicted rule")?;
        assert!(r.choices[0].private_speech);
        assert!(r.choices[0].speech.is_some());
        assert_eq!(
            r.choices[0].cues[0].effect_id, 3,
            "POFF remains a separately authored public visual"
        );
    }
    let c = catalogue().ok_or("catalogue")?;
    let plan = rule("oteryn:item.tibia.i27655").ok_or("makeshift plan")?;
    assert!(
        plan.choices[0].speech.is_none(),
        "whiteText is not the player's public speech"
    );
    assert!(!plan.choices[0].private_speech);
    let decision = decide(
        c,
        plan,
        &plan.choices[0],
        &Content { admitted: true },
        &Owned::default(),
        Sex::Male,
        true,
        &BTreeMap::new(),
    )
    .map_err(|_| "plan decision")?;
    assert_eq!(
        decision.feedback.as_deref(),
        Some(
            "By using the plan you knock together a makeshift armour out of wooden planks, rusty nails and leather rags."
        )
    );
    let public = rule("oteryn:item.tibia.i32630").ok_or("spooky hood")?;
    assert!(!public.choices[0].private_speech);
    assert_eq!(
        public.choices[0].speech.as_deref(),
        Some("The spooky hood is yours!")
    );
    assert!(public.choices[0].feedback.is_none());
    Ok(())
}

#[path = "tests/additional.rs"]
mod additional;
