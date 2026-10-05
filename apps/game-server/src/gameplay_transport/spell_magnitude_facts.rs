//! Numerical observations from the already fenced Character/Equipment/Wheel reads.
//! Baseline approximation is selected by qualified Content, never by a client or env flag.
use super::spell_access_facts::AccessFactsError;
use crate::content::native_gameplay::NativeGameplayState;
use crate::durability::character_equipment::RawCastDurableFacts;
use crate::foundation::CharacterId;
use crate::spell::magnitude_owner::{
    MagnitudeOwnedAttributes, MagnitudeOwnerBinding, MagnitudePolicy,
};
use crate::spell::owned_cast_facts::{CastFactsBinding, CurrentProjection};
use std::collections::BTreeMap;

pub(crate) fn project(
    raw: &RawCastDurableFacts,
    binding: &CastFactsBinding,
    native: &NativeGameplayState,
    wheel: Option<&CurrentProjection<BTreeMap<String, u8>>>,
    premium: Option<&CurrentProjection<bool>>,
    now_micros: u64,
) -> Result<Option<MagnitudeOwnedAttributes>, AccessFactsError> {
    let equipment = &raw.equipment;
    if equipment.character != binding.character
        || *raw.fence.character_id.as_bytes() != binding.character
        || raw.fence.game_session_id != binding.session
        || raw.command.game_session_id() != binding.session
        || raw.fence.character_lease_generation != binding.lease_generation
        || raw.fence.connection_generation.get() != binding.connection_generation
        || equipment.character_revision != binding.character_revision
        || equipment.revision != binding.equipment_revision
        || equipment.content_digest != binding.content_digest
        || native.source_digest() != binding.content_digest
        || raw.level == 0
        || binding.character_revision == 0
        || binding.equipment_revision == 0
    {
        return Err(AccessFactsError::Unavailable(
            "magnitude durable owner binding",
        ));
    }
    // No advanced modifier owner is registered yet. Strict Content keeps its refusal.
    if native.magnitude_policy() == MagnitudePolicy::Strict {
        return Ok(None);
    }
    for item in &equipment.items {
        if item.quantity == 0
            || item.state_revision == 0
            || native
                .item_policy(
                    &item.definition.production_key,
                    &item.definition.revision_ref,
                )
                .is_none()
        {
            return Err(AccessFactsError::Unavailable(
                "magnitude current Item policy",
            ));
        }
    }
    let wheel_observation = project_current_wheel_flat(
        binding,
        raw.build.vocation(),
        native.wheel_profile(),
        wheel,
        premium,
        now_micros,
    )?;
    let wheel_flat = wheel_observation.map(|(flat, _)| flat);
    let wheel_revision = wheel_observation.map(|(_, revision)| revision);
    // Empty actual equipment proves no equipment bonus. Nonempty equipment has not
    // qualified modifier units, phases and stacking; its numeric fields stay unknown.
    let empty_equipment = equipment.items.is_empty().then_some(0);
    let attributes = MagnitudeOwnedAttributes {
        policy: native.magnitude_policy(),
        owner_revisions: Some((
            equipment.character_revision,
            equipment.revision,
            wheel_revision,
        )),
        binding: MagnitudeOwnerBinding {
            actor: binding.actor,
            session: binding.session,
            character: CharacterId::decode(&binding.character)
                .map_err(|_| AccessFactsError::Unavailable("magnitude Character identity"))?,
            lease_generation: binding.lease_generation,
            player_revision: binding.player_revision,
            content_digest: binding.content_digest,
        },
        base_critical_chance_permyriad: None,
        base_critical_extra_permyriad: None,
        equipment_critical_chance_permyriad: empty_equipment,
        equipment_critical_extra_permyriad: empty_equipment.map(i32::from),
        fatal_chance_permyriad: None,
        armor_penetration_permyriad: None,
        elemental_pierce_permyriad: None,
        wheel_flat_damage: wheel_flat.map(|value| value.0),
        wheel_flat_healing: wheel_flat.map(|value| value.1),
        source_damage_multiplier_percent: None,
        source_healing_multiplier_percent: None,
        healing_dealt_percent: None,
        monster_armor_disabled: None,
        inactive_extra_stages: None,
    };
    #[cfg(test)]
    eprintln!(
        "spell magnitude baseline omitted={:?}",
        attributes.omitted_modifiers()
    );
    Ok(Some(attributes))
}

type WheelFlatObservation = ((i32, i32), u64);

/// Consumes the same-transaction projection, not the admitted session's cached allocation.
/// Missing authority stays unknown; known zero requires a complete current four-domain read.
fn project_current_wheel_flat(
    binding: &CastFactsBinding,
    vocation: &str,
    profile: Option<&crate::durability::spell_wheel_abi::CompiledWheelProfile>,
    wheel: Option<&CurrentProjection<BTreeMap<String, u8>>>,
    premium: Option<&CurrentProjection<bool>>,
    now: u64,
) -> Result<Option<WheelFlatObservation>, AccessFactsError> {
    let Some(wheel) = wheel.filter(|p| p.current(binding, now)) else {
        return Ok(None);
    };
    let Some(premium) = premium.filter(|p| p.current(binding, now) && p.value) else {
        return Ok(None);
    };
    if wheel.valid_until_micros > premium.valid_until_micros
        || wheel.authority_revision != binding.character_revision
    {
        return Err(AccessFactsError::Unavailable(
            "Wheel magnitude current authority",
        ));
    }
    let Some(profile) = profile else {
        return Ok(None);
    };
    if profile.source_digest() != binding.content_digest {
        return Err(AccessFactsError::Unavailable("Wheel magnitude Content pin"));
    }
    let ruleset = crate::wheel_gem_data::WheelGemData::embedded()
        .ok()
        .and_then(|data| data.wheel_ruleset().ok())
        .ok_or(AccessFactsError::Unavailable(
            "Wheel magnitude accepted ruleset",
        ))?;
    if !matches!(
        vocation,
        "elder_druid" | "elite_knight" | "exalted_monk" | "royal_paladin" | "master_sorcerer"
    ) {
        return Ok(None);
    }
    let rules = ruleset
        .vocation(vocation)
        .ok_or(AccessFactsError::Unavailable("Wheel magnitude vocation"))?;
    let mut colours = BTreeMap::new();
    for (colour, key) in ["green", "red", "purple", "blue"]
        .iter()
        .zip(&rules.revelations)
    {
        let stage = wheel
            .value
            .get(key)
            .copied()
            .ok_or(AccessFactsError::Unavailable(
                "Wheel magnitude complete stage read",
            ))?;
        colours.insert((*colour).to_owned(), stage);
    }
    let flat = profile
        .flat_stats(&colours)
        .map_err(|_| AccessFactsError::Unavailable("Wheel magnitude source stages"))?;
    Ok(Some((flat, wheel.authority_revision)))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod wheel_magnitude_tests {
    use super::*;
    use crate::durability::spell_wheel_abi::CompiledWheelProfile;
    fn binding() -> CastFactsBinding {
        let (_, actor, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(87);
        CastFactsBinding {
            actor,
            session,
            character: [7; 16],
            character_revision: 4,
            lease_generation: 1,
            connection_generation: 1,
            player_revision: 3,
            content_digest: [9; 32],
            equipment_revision: 2,
        }
    }
    fn profile(digest: [u8; 32]) -> CompiledWheelProfile {
        CompiledWheelProfile::from_active_artifact(
            include_bytes!("../../../../tools/content-schema/native-gameplay/wheel-profile.json"),
            digest,
        )
        .expect("actual accepted source profile")
    }
    fn wheel(
        binding: &CastFactsBinding,
        stages: [u8; 4],
    ) -> CurrentProjection<BTreeMap<String, u8>> {
        let ruleset = crate::wheel_gem_data::WheelGemData::embedded()
            .expect("accepted Wheel")
            .wheel_ruleset()
            .expect("accepted ruleset");
        let keys = &ruleset
            .vocation("elder_druid")
            .expect("actual vocation")
            .revelations;
        CurrentProjection {
            binding: binding.clone(),
            authority_revision: binding.character_revision,
            valid_until_micros: 100,
            value: keys.iter().cloned().zip(stages).collect(),
        }
    }
    fn premium(binding: &CastFactsBinding) -> CurrentProjection<bool> {
        CurrentProjection {
            binding: binding.clone(),
            authority_revision: 8,
            valid_until_micros: 100,
            value: true,
        }
    }
    #[test]
    fn wheel_magnitude_uses_exact_source_stage_numbers_and_current_zero_is_proven() {
        let b = binding();
        let p = profile(b.content_digest);
        let prem = premium(&b);
        assert_eq!(
            project_current_wheel_flat(
                &b,
                "elder_druid",
                Some(&p),
                Some(&wheel(&b, [0, 1, 2, 3])),
                Some(&prem),
                99
            )
            .expect("current source flat"),
            Some(((33, 33), 4))
        );
        assert_eq!(
            project_current_wheel_flat(
                &b,
                "elder_druid",
                Some(&p),
                Some(&wheel(&b, [0; 4])),
                Some(&prem),
                99
            )
            .expect("current zero"),
            Some(((0, 0), 4))
        );
    }
    #[test]
    fn wheel_magnitude_missing_expired_or_negative_authority_stays_unknown() {
        let b = binding();
        let p = profile(b.content_digest);
        let w = wheel(&b, [3; 4]);
        let mut prem = premium(&b);
        assert!(
            project_current_wheel_flat(&b, "elder_druid", Some(&p), None, Some(&prem), 1)
                .expect("missing Wheel")
                .is_none()
        );
        assert!(
            project_current_wheel_flat(&b, "elder_druid", None, Some(&w), Some(&prem), 1)
                .expect("missing profile")
                .is_none()
        );
        assert!(
            project_current_wheel_flat(&b, "elder_druid", Some(&p), Some(&w), None, 1)
                .expect("missing Premium")
                .is_none()
        );
        assert!(
            project_current_wheel_flat(&b, "elder_druid", Some(&p), Some(&w), Some(&prem), 100)
                .expect("expired")
                .is_none()
        );
        prem.value = false;
        assert!(
            project_current_wheel_flat(&b, "elder_druid", Some(&p), Some(&w), Some(&prem), 1)
                .expect("negative Premium")
                .is_none()
        );
        prem.value = true;
        assert!(
            project_current_wheel_flat(&b, "druid", Some(&p), Some(&w), Some(&prem), 1)
                .expect("unpromoted")
                .is_none()
        );
    }
    #[test]
    fn wheel_magnitude_rejects_partial_malformed_wrong_revision_and_wrong_content_proofs() {
        let b = binding();
        let p = profile(b.content_digest);
        let prem = premium(&b);
        let mut w = wheel(&b, [1; 4]);
        let key = w.value.keys().next().expect("real source key").clone();
        w.value.remove(&key);
        assert!(
            project_current_wheel_flat(&b, "elder_druid", Some(&p), Some(&w), Some(&prem), 1)
                .is_err()
        );
        w = wheel(&b, [4, 0, 0, 0]);
        assert!(
            project_current_wheel_flat(&b, "elder_druid", Some(&p), Some(&w), Some(&prem), 1)
                .is_err()
        );
        w = wheel(&b, [1; 4]);
        w.authority_revision += 1;
        assert!(
            project_current_wheel_flat(&b, "elder_druid", Some(&p), Some(&w), Some(&prem), 1)
                .is_err()
        );
        w = wheel(&b, [1; 4]);
        w.valid_until_micros += 1;
        assert!(
            project_current_wheel_flat(&b, "elder_druid", Some(&p), Some(&w), Some(&prem), 1)
                .is_err()
        );
        w = wheel(&b, [1; 4]);
        assert!(
            project_current_wheel_flat(
                &b,
                "elder_druid",
                Some(&profile([8; 32])),
                Some(&w),
                Some(&prem),
                1
            )
            .is_err()
        );
        assert!(
            project_current_wheel_flat(&b, "master_sorcerer", Some(&p), Some(&w), Some(&prem), 1)
                .is_err()
        );
    }
    #[test]
    fn wheel_magnitude_retained_proof_cannot_follow_replaced_cast_owners() {
        let b = binding();
        let p = profile(b.content_digest);
        let w = wheel(&b, [3; 4]);
        let prem = premium(&b);
        let (mut runtime, _, _) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(87);
        let other_session =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(88).2;
        let reservation = runtime
            .reserve_fresh_session(other_session)
            .expect("actual second actor reservation");
        let other_actor = runtime
            .commit_fresh_session(reservation)
            .expect("actual second actor");
        assert_ne!(other_actor, b.actor);
        for field in 0..9 {
            let mut current = b.clone();
            match field {
                0 => current.character[0] ^= 1,
                1 => current.character_revision += 1,
                2 => current.equipment_revision += 1,
                3 => current.lease_generation += 1,
                4 => current.connection_generation += 1,
                5 => current.player_revision += 1,
                6 => current.content_digest[0] ^= 1,
                7 => current.actor = other_actor,
                _ => current.session = other_session,
            }
            assert!(
                project_current_wheel_flat(
                    &current,
                    "elder_druid",
                    Some(&p),
                    Some(&w),
                    Some(&prem),
                    1
                )
                .expect("stale withheld")
                .is_none(),
                "owner field {field}"
            );
        }
    }
}
