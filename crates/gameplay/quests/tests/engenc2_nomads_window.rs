use oteryn_gameplay_quests::quest::{
    admission_binding,
    world_callbacks::{self, WorldRequirement},
};

#[test]
fn nomads_window_native_binding_requires_the_source_crates_and_amphora() -> Result<(), String> {
    let callback = world_callbacks::load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-callback/engenc2/nomads-land-window")
        .ok_or("Nomads Land s4 has no production window callback")?;
    assert_eq!(callback.quest, "oteryn:quest.authored.nomads_land_quest");
    let native = callback
        .selector
        .native
        .as_ref()
        .ok_or("native window selector")?;
    assert_eq!(
        (
            native.position.x,
            native.position.y,
            native.position.floor,
            native.ordinal,
            native.appearance
        ),
        (33216, 32855, -9, 1, 1744)
    );
    let evidence: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/reference/content-prep/qmech/nomads-land-window.json"
    ))
    .map_err(|e| e.to_string())?;
    assert_eq!(
        serde_json::to_value(native).map_err(|e| e.to_string())?,
        evidence["window"]
    );
    assert_eq!(
        serde_json::to_value(&callback.world_requirements).map_err(|e| e.to_string())?,
        evidence["requirements"]
    );
    assert_eq!(callback.world_requirements.len(), 3);
    for (index, requirement) in callback.world_requirements.iter().enumerate() {
        let WorldRequirement::Ground { position, ground } = requirement else {
            return Err("static map appearance cannot prove mutable crates".into());
        };
        assert_eq!(
            (position.x, position.y, position.floor),
            (33215 + index as u16, 32854, -9)
        );
        assert_eq!(ground.stacks.len(), if index == 1 { 2 } else { 1 });
        assert_eq!(ground.stacks[0].item, "oteryn:item.tibia.i2471");
        if index == 1 {
            assert_eq!(ground.stacks[1].item, "oteryn:item.tibia.i2904");
        }
        assert!(
            ground
                .stacks
                .iter()
                .all(|s| s.quantity == 1 && s.revision == "definition-r1")
        );
    }
    let transition = world_callbacks::primary_transition(&callback)?;
    let loaded =
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("engenc2-nomads-window")
            .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        loaded.catalogue().transition(&callback.key),
        Some(&transition),
        "the normal boot catalogue must contain the physical window operation"
    );
    let track = loaded
        .catalogue()
        .track(&callback.storage[0].track)
        .ok_or("native window observation track")?;
    assert_eq!(track.quest, callback.quest);
    assert_eq!(track.initial, -1);
    admission_binding::validate(&transition, true)?;
    assert!(admission_binding::validate(&transition, false).is_err());
    let mut alias = transition.clone();
    alias.key = "oteryn:quest-transition/engenc2/nomads-window-bypass".into();
    for effect in &mut alias.effects {
        if let oteryn_gameplay_quests::quest::QuestEffectKind::SetWithBinding { value, .. } =
            effect.effect
        {
            effect.effect = oteryn_gameplay_quests::quest::QuestEffectKind::Set(value);
        }
    }
    assert!(admission_binding::validate(&alias, false).is_err());
    assert!(!transition.completes && transition.experience.is_none());
    assert!(callback.effects.is_empty() && callback.offerings.is_empty());
    assert_eq!(transition.effects.len(), 1);
    Ok(())
}
