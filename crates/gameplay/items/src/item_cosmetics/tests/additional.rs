use super::*;

#[test]
fn crystal_revenant_addons_keep_the_primary_donor_order() -> Result<(), &'static str> {
    let c = catalogue().ok_or("catalogue")?;
    for (id, addon) in [(34075, 2), (34076, 1)] {
        let r = rule(&format!("oteryn:item.tibia.i{id}")).ok_or("Revenant")?;
        let grant =
            resolve(c, &Content { admitted: true }, &r.choices[0].owned).map_err(|_| "resolve")?;
        assert_eq!(grant.addon, addon, "Canary has the reverse mapping");
    }
    Ok(())
}

#[test]
fn random_draws_are_bounded_and_repeat_for_the_same_command() -> Result<(), &'static str> {
    assert!(rule("not-an-item").is_none());
    assert!(rule("oteryn:item.tibia.i0").is_none());
    for id in [6579, 37746] {
        let r = rule(&format!("oteryn:item.tibia.i{id}")).ok_or("random rule")?;
        assert!(select(r, |_, upper| Some(upper)).is_none());
        assert!(select(r, |_, _| None).is_none());
        let first = select_for_command(r, [1; 16], 93, [2; 16]).ok_or("draw")?;
        for _ in 0..8 {
            let replay = select_for_command(r, [1; 16], 93, [2; 16]).ok_or("replay")?;
            assert_eq!(replay.owned, first.owned);
        }
    }
    Ok(())
}

#[test]
fn fourth_fiery_horseshoe_unlocks_and_progress_overflow_is_refused() -> Result<(), &'static str> {
    let c = catalogue().ok_or("catalogue")?;
    let r = rule("oteryn:item.tibia.i36938").ok_or("horseshoe")?;
    let content = Content { admitted: true };
    for before in 0..4 {
        let progress = BTreeMap::from([("fiery-horseshoe".into(), before)]);
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
        .map_err(|_| "decide")?;
        assert!(result.consume);
        assert_eq!(
            result.progress.as_ref().map(|p| (p.before, p.after)),
            Some((before, before + 1))
        );
        assert_eq!(result.grants.len(), usize::from(before == 3));
        if before == 3 {
            assert_eq!(result.achievements, ["oteryn:achievement/hot_on_the_trail"]);
            assert_eq!(
                result.feedback.as_deref(),
                Some("Singeing Steed is now yours!")
            );
        } else {
            assert!(result.achievements.is_empty());
            assert!(result.feedback.is_none());
        }
    }
    let overflow = BTreeMap::from([("fiery-horseshoe".into(), u32::MAX)]);
    assert!(matches!(
        decide(
            c,
            r,
            &r.choices[0],
            &content,
            &Owned::default(),
            Sex::Male,
            false,
            &overflow
        ),
        Err(Refusal::InvalidProgress)
    ));
    Ok(())
}

#[test]
fn extra_spectral_horseshoes_do_not_substitute_for_the_saddle_or_tac() -> Result<(), &'static str> {
    let c = catalogue().ok_or("catalogue")?;
    let r = rule("oteryn:item.tibia.i34072").ok_or("horseshoe")?;
    let result = decide(
        c,
        r,
        &r.choices[0],
        &Content { admitted: true },
        &Owned::default(),
        Sex::Female,
        false,
        &BTreeMap::from([("spectral-horseshoes".into(), 4)]),
    )
    .map_err(|_| "decide")?;
    assert!(
        result.consume,
        "the donor removes an additional contributed horseshoe"
    );
    assert_eq!(
        result.progress.as_ref().map(|p| (p.before, p.after)),
        Some((4, 5))
    );
    assert!(result.grants.is_empty());
    assert!(result.achievements.is_empty());
    Ok(())
}

#[test]
fn full_outfit_achievement_waits_for_both_addons_and_duplicates_keep_the_item()
-> Result<(), &'static str> {
    let c = catalogue().ok_or("catalogue")?;
    let content = Content { admitted: true };
    let base = rule("oteryn:item.tibia.i16252").ok_or("base")?;
    let base = resolve(c, &content, &base.choices[0].owned).map_err(|_| "base mapping")?;
    let mut owned = Owned::default();
    owned.outfits.insert(base.cosmetic_key);
    for (id, expected) in [(16253, 0), (16254, 1)] {
        let r = rule(&format!("oteryn:item.tibia.i{id}")).ok_or("addon")?;
        let result = decide(
            c,
            r,
            &r.choices[0],
            &content,
            &owned,
            Sex::Male,
            true,
            &BTreeMap::new(),
        )
        .map_err(|_| "addon decision")?;
        assert_eq!(result.achievements.len(), expected);
        if expected == 1 {
            assert_eq!(result.achievements, ["oteryn:achievement/funghitastic"]);
        }
        let grant = &result.grants[0];
        owned
            .addons
            .insert((grant.cosmetic_key.clone(), grant.addon));
        let duplicate = decide(
            c,
            r,
            &r.choices[0],
            &content,
            &owned,
            Sex::Male,
            true,
            &BTreeMap::new(),
        )
        .map_err(|_| "duplicate")?;
        assert!(!duplicate.consume);
        assert!(duplicate.achievements.is_empty());
        assert_eq!(
            duplicate.feedback.as_deref(),
            Some("You already own this outfit part.")
        );
    }
    Ok(())
}

#[test]
fn generated_random_feedback_preserves_the_source_messages() -> Result<(), &'static str> {
    let c = catalogue().ok_or("catalogue")?;
    let content = Content { admitted: true };
    for id in [6579, 37746] {
        let r = rule(&format!("oteryn:item.tibia.i{id}")).ok_or("rule")?;
        for choice in &r.choices {
            let success = decide(
                c,
                r,
                choice,
                &content,
                &Owned::default(),
                Sex::Male,
                false,
                &BTreeMap::new(),
            )
            .map_err(|_| "success")?;
            assert!(
                success
                    .feedback
                    .as_deref()
                    .is_some_and(|s| s.starts_with("You received the "))
            );
            let grant = resolve(c, &content, &choice.owned).map_err(|_| "owned mapping")?;
            let mut owned = Owned::default();
            match grant.kind {
                GrantKind::Mount => {
                    owned.mounts.insert(grant.cosmetic_key);
                }
                GrantKind::Addon => {
                    owned.addons.insert((grant.cosmetic_key, grant.addon));
                }
                GrantKind::Outfit => {
                    assert_eq!(id, 6579);
                    assert!(matches!(
                        choice.feedback.as_deref(),
                        Some(
                            "You received the Wedding outfit!"
                                | "You received the Jungle Explorer outfit!"
                        )
                    ));
                    assert_eq!(
                        success.grants.len(),
                        1,
                        "addonless looks grant only their base"
                    );
                    owned.outfits.insert(grant.cosmetic_key);
                }
            }
            let duplicate = decide(
                c,
                r,
                choice,
                &content,
                &owned,
                Sex::Male,
                false,
                &BTreeMap::new(),
            )
            .map_err(|_| "duplicate")?;
            assert!(!duplicate.consume);
            if id == 6579 {
                assert_eq!(
                    duplicate.feedback.as_deref(),
                    Some(if grant.kind == GrantKind::Outfit {
                        "You already have this outfit."
                    } else {
                        "You already have this outfit with this addon."
                    })
                );
            } else {
                assert!(
                    duplicate
                        .feedback
                        .as_deref()
                        .is_some_and(|s| s.starts_with("You already own the store mount: "))
                );
            }
        }
    }
    Ok(())
}

struct PartialMapping {
    denied: String,
}
impl SourceCosmetics for PartialMapping {
    fn outfit_pair(&self, looks: &[u32]) -> Option<String> {
        Content { admitted: true }.outfit_pair(looks)
    }
    fn mount_look(&self, look: u32) -> Option<String> {
        Content { admitted: true }.mount_look(look)
    }
}
impl AppearanceContent<String> for PartialMapping {
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
    fn earned_unlock(&self, grant: &str, provenance: &str, _: EarnedCosmetic<'_>) -> UnlockFact {
        if grant != self.denied.as_str() && provenance == PROVENANCE {
            UnlockFact::Compatible
        } else {
            UnlockFact::Absent
        }
    }
}

#[test]
fn one_missing_random_mapping_refuses_the_entire_pool_before_any_draw() -> Result<(), &'static str>
{
    let c = catalogue().ok_or("catalogue")?;
    for id in [6579, 37746] {
        let r = rule(&format!("oteryn:item.tibia.i{id}")).ok_or("random rule")?;
        let content = PartialMapping {
            denied: r.choices.last().ok_or("last choice")?.owned.clone(),
        };
        assert!(
            resolve(c, &content, &r.choices[0].owned).is_ok(),
            "the first choice alone is usable"
        );
        assert_eq!(
            qualify_rule(c, r, &content),
            Err(Refusal::ContentUnavailable),
            "admission must not silently filter the source pool to its mapped choices"
        );
        assert!(qualify_rule(c, r, &Content { admitted: true }).is_ok());
    }
    Ok(())
}

struct OneSexStarter;
impl SourceCosmetics for OneSexStarter {
    fn outfit_pair(&self, looks: &[u32]) -> Option<String> {
        Content { admitted: true }.outfit_pair(looks)
    }
    fn mount_look(&self, look: u32) -> Option<String> {
        Content { admitted: true }.mount_look(look)
    }
}
impl AppearanceContent<String> for OneSexStarter {
    fn palette_size(&self) -> u16 {
        133
    }
    fn outfit_addon_count(&self, _: &String) -> Option<u8> {
        Some(2)
    }
    fn outfit_is_starter(&self, _: &String, sex: Sex) -> bool {
        sex == Sex::Female
    }
    fn addon_is_starter(&self, _: &String, sex: Sex, _: u8) -> bool {
        sex == Sex::Female
    }
    fn mount_exists(&self, _: &String) -> bool {
        true
    }
    fn earned_unlock(&self, _: &str, provenance: &str, _: EarnedCosmetic<'_>) -> UnlockFact {
        if provenance == PROVENANCE {
            UnlockFact::Compatible
        } else {
            UnlockFact::Absent
        }
    }
}

#[test]
fn random_doll_completes_both_sexes_when_only_one_is_a_starter() -> Result<(), &'static str> {
    let c = catalogue().ok_or("catalogue")?;
    let r = rule("oteryn:item.tibia.i6579").ok_or("doll")?;
    // Crystal addon_doll.lua grants both look variants when either one lacks the addon.
    for sex in [Sex::Female, Sex::Male] {
        let mut owned = Owned::default();
        let result = decide(
            c,
            r,
            &r.choices[0],
            &OneSexStarter,
            &owned,
            sex,
            false,
            &BTreeMap::new(),
        )
        .map_err(|_| "partial starter")?;
        assert!(result.consume);
        assert_eq!(
            result.grants.len(),
            2,
            "the other sex must gain both base and addon"
        );
        for grant in result.grants {
            match grant.kind {
                GrantKind::Outfit => {
                    owned.outfits.insert(grant.cosmetic_key);
                }
                GrantKind::Addon => {
                    owned.addons.insert((grant.cosmetic_key, grant.addon));
                }
                GrantKind::Mount => return Err("unexpected mount"),
            }
        }
        let duplicate = decide(
            c,
            r,
            &r.choices[0],
            &OneSexStarter,
            &owned,
            sex,
            false,
            &BTreeMap::new(),
        )
        .map_err(|_| "duplicate")?;
        assert!(!duplicate.consume);
        assert!(duplicate.grants.is_empty());
    }
    Ok(())
}
