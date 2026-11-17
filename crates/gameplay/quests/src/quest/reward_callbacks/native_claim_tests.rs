use super::*;

fn native_callback() -> serde_json::Value {
    serde_json::json!({
        "key":"oteryn:quest-reward/native-claim", "quest":"oteryn:quest.authored.kingdom_of_kormarak_quest",
        "npc":"", "keywords":[], "topic":0, "next_topic":0,
        "storage":[{"track":"oteryn:quest-progress/authored/kingdom_of_kormarak_quest/s4", "minimum":0,"maximum":0,"set":1}],
        "take":[], "present":[], "give":{"item":"oteryn:item.tibia.i3031","count":1,"contents":[]},
        "outfit":null, "achievement":null, "reply":["You have found a reward."],
        "sources":["docs/reference/content-prep/engclaim-m34/sources.md"],
        "world_claim":{"key":"oteryn:reward-claim.authored.native","revision":"engclaim-r1","authority_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}
    })
}

#[test]
fn native_claim_reward_has_an_exact_bound_world_owner() -> Result<(), String> {
    let source =
        serde_json::json!({"schema":1,"tracks":[],"callbacks":[native_callback()]}).to_string();
    let document = parse_documents(&[&source])?;
    assert!(gated_binding(&document.callbacks[0])?.is_some());
    Ok(())
}

#[test]
fn native_claim_reward_refuses_npc_routing_and_unbound_identity() -> Result<(), String> {
    for mutation in ["npc", "words", "key", "hash", "carried"] {
        let mut callback = native_callback();
        match mutation {
            "npc" => callback["npc"] = "oteryn:npc.sam".into(),
            "words" => callback["keywords"] = serde_json::json!(["reward"]),
            "key" => callback["world_claim"]["key"] = "oteryn:quest-reward/another-claim".into(),
            "hash" => callback["world_claim"]["authority_sha256"] = "unknown".into(),
            _ => callback["carried_item"] = "oteryn:item.tibia.i3031".into(),
        }
        let source = serde_json::json!({"schema":1,"tracks":[],"callbacks":[callback]}).to_string();
        assert!(parse_documents(&[&source]).is_err(), "{mutation}");
    }
    Ok(())
}

#[test]
fn native_claim_production_allocations_are_complete_and_gated() -> Result<(), String> {
    let callbacks = load()?;
    let native: Vec<_> = callbacks
        .iter()
        .filter(|c| c.world_claim.is_some())
        .collect();
    assert_eq!(native.len(), 9);
    let corpse = native
        .iter()
        .find(|c| c.key.ends_with("kormarak-corpse"))
        .ok_or("corpse")?;
    assert_eq!(
        corpse.give.as_ref().ok_or("allocation")?.additional.len(),
        3
    );
    assert!(corpse.completes);
    let guardian = native
        .iter()
        .find(|c| c.key.ends_with("valley-guardian"))
        .ok_or("guardian")?;
    assert_eq!(guardian.take.len(), 3);
    assert!(guardian.requires_premium && guardian.completes);
    assert!(
        guardian
            .requires
            .iter()
            .any(|r| r.track.ends_with("the_ice_islands/mission06") && r.minimum == 8)
    );
    assert!(
        guardian
            .requires
            .iter()
            .any(|r| r.track.ends_with("the_new_frontier/mission08") && r.minimum == 2)
    );
    Ok(())
}

#[test]
fn native_claim_kormarak_accepts_the_once_only_predecessor() -> Result<(), String> {
    let loaded = crate::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|e| format!("{e:?}"))?;
    let catalogue = loaded.catalogue();
    let mut original = load()?
        .into_iter()
        .find(|callback| callback.key.ends_with("kormarak-corpse"))
        .ok_or("Kormarak callback")?;
    let binding = original.world_claim.as_mut().ok_or("Kormarak claim")?;
    assert!(binding.daily);
    binding.daily = false;
    // Authority digest of the once-only allocation in the repeatcd base.
    binding.authority_sha256 =
        "016f6c0b842d5d38f15002f9825ecabf7fe4892e79a64c1d3964cd18ab9f20c6".into();
    for state in &mut original.storage {
        state.maximum = 0;
    }
    let mut transitions = catalogue.transitions.clone();
    transitions.insert(original.key.clone(), primary_transition(&original)?);
    let hash = super::super::definition_hash(&original.quest, &catalogue.tracks, &transitions);
    assert_ne!(catalogue.definition_hash(&original.quest), Some(hash));
    assert!(
        super::super::npc_rules::legacy_defaults(catalogue, &original.quest, &hash)
            .ok_or("Kormarak once definition")?
            .is_empty()
    );
    let mut unknown = hash;
    unknown[0] ^= 1;
    assert!(
        super::super::npc_rules::legacy_defaults(catalogue, &original.quest, &unknown).is_none()
    );
    Ok(())
}
