//! Stable diagnostics for errors that cross the combat crate boundary.

use oteryn_error_codes::{Category, ErrorCode, Progression};

macro_rules! boundary_codes {
    ($type:path, $($variant:pat => ($number:literal, $name:literal)),+ $(,)?) => {
        impl $type {
            /// The stable, registered diagnostic code for this boundary failure.
            #[must_use]
            pub const fn code(&self) -> ErrorCode {
                let (number, name) = match self {
                    $($variant => ($number, $name)),+
                };
                ErrorCode::new(number, name, Category::InternalUnavailable, Progression::Terminal)
            }
        }
    };
}

boundary_codes!(crate::combat::CombatResourceLimitError,
    Self::InflightLootMintsPerScopeExceeded => (4010, "COMBAT_INFLIGHT_LOOT_LIMIT"),
    Self::RewardPrincipalsExceeded => (4011, "COMBAT_REWARD_PRINCIPAL_LIMIT"),
    Self::CorpseContainerEntriesExceeded => (4012, "COMBAT_CORPSE_ENTRY_LIMIT"),
);
boundary_codes!(crate::combat::CreatureDeathRewardAdmissionError,
    Self::Limit(_) => (4013, "COMBAT_DEATH_REWARD_LIMIT"),
    Self::PrincipalMismatch => (4014, "COMBAT_DEATH_REWARD_PRINCIPAL_MISMATCH"),
);
boundary_codes!(crate::combat::CombatDeathRewardLootError,
    Self::Plan(_) => (4015, "COMBAT_DEATH_LOOT_PLAN"),
    Self::Capacity(_) => (4016, "COMBAT_DEATH_LOOT_CAPACITY"),
    Self::Corpse(_) => (4017, "COMBAT_DEATH_CORPSE_MINT"),
    Self::Mint(_, _) => (4018, "COMBAT_DEATH_LOOT_MINT"),
);
boundary_codes!(crate::combat::CombatDeathRewardXpError,
    Self::NoProgressionBinding => (4019, "COMBAT_DEATH_XP_BINDING_MISSING"),
    Self::NoExperienceAward => (4020, "COMBAT_DEATH_XP_AWARD_MISSING"),
    Self::InvalidOccurrence => (4021, "COMBAT_DEATH_XP_OCCURRENCE_INVALID"),
    Self::Progression(_) => (4022, "COMBAT_DEATH_XP_PROGRESSION"),
);
boundary_codes!(crate::combat::CombatDeathRewardBestiaryError,
    Self::NoProgressionBinding => (4023, "COMBAT_DEATH_BESTIARY_BINDING_MISSING"),
    Self::Credit(_) => (4024, "COMBAT_DEATH_BESTIARY_CREDIT"),
    Self::InvalidOccurrence => (4025, "COMBAT_DEATH_BESTIARY_OCCURRENCE_INVALID"),
    Self::Progress(_) => (4026, "COMBAT_DEATH_BESTIARY_PROGRESS"),
);
boundary_codes!(crate::combat_pvp::lock::PvpReadmissionError,
    Self::Read(_) => (4027, "COMBAT_PVP_READMISSION_READ"),
    Self::DeadlineOverflow => (4028, "COMBAT_PVP_READMISSION_DEADLINE"),
);
boundary_codes!(crate::combat_pickup::GroundPickupError,
    Self::Content(_) => (4029, "COMBAT_GROUND_PICKUP_CONTENT"),
    Self::SourceMismatch => (4030, "COMBAT_GROUND_PICKUP_SOURCE_MISMATCH"),
    Self::Transfer(_) => (4031, "COMBAT_GROUND_PICKUP_TRANSFER"),
);
boundary_codes!(crate::creature_condition_content::ConditionContentError,
    Self::NotCondition => (4032, "COMBAT_CONDITION_NOT_CONDITION"),
    Self::UnsupportedCondition => (4033, "COMBAT_CONDITION_UNSUPPORTED"),
    Self::UnsupportedParameters => (4034, "COMBAT_CONDITION_PARAMETERS_UNSUPPORTED"),
    Self::UnsupportedDamageSchedule => (4035, "COMBAT_CONDITION_DAMAGE_SCHEDULE_UNSUPPORTED"),
    Self::MissingOrMismatchedFormula => (4036, "COMBAT_CONDITION_FORMULA_MISMATCH"),
    Self::UnsupportedFormula => (4037, "COMBAT_CONDITION_FORMULA_UNSUPPORTED"),
    Self::InexactSpeedCoefficient => (4038, "COMBAT_CONDITION_SPEED_INEXACT"),
    Self::NumericOverflow => (4039, "COMBAT_CONDITION_NUMERIC_OVERFLOW"),
    Self::InvalidDefinition => (4040, "COMBAT_CONDITION_DEFINITION_INVALID"),
);
boundary_codes!(crate::creature_area_heal::AreaHealError,
    Self::InvalidSource => (4041, "COMBAT_AREA_HEAL_SOURCE_INVALID"),
    Self::UnsupportedSource => (4042, "COMBAT_AREA_HEAL_SOURCE_UNSUPPORTED"),
    Self::MissingCurrentPolicy => (4043, "COMBAT_AREA_HEAL_POLICY_MISSING"),
    Self::StaleOwner => (4044, "COMBAT_AREA_HEAL_OWNER_STALE"),
    Self::InvalidOccurrence => (4045, "COMBAT_AREA_HEAL_OCCURRENCE_INVALID"),
    Self::ReplayConflict => (4046, "COMBAT_AREA_HEAL_REPLAY_CONFLICT"),
    Self::Numeric => (4047, "COMBAT_AREA_HEAL_NUMERIC"),
    Self::Capacity => (4048, "COMBAT_AREA_HEAL_CAPACITY"),
    Self::Carrier(_) => (4049, "COMBAT_AREA_HEAL_CARRIER"),
);
boundary_codes!(crate::creature_auto_attack::AttackError,
    Self::InvalidSource => (4050, "COMBAT_ATTACK_SOURCE_INVALID"),
    Self::UnsupportedShape => (4051, "COMBAT_ATTACK_SHAPE_UNSUPPORTED"),
    Self::StaleIssuer => (4052, "COMBAT_ATTACK_ISSUER_STALE"),
    Self::StaleTarget => (4053, "COMBAT_ATTACK_TARGET_STALE"),
    Self::ContentChanged => (4054, "COMBAT_ATTACK_CONTENT_CHANGED"),
    Self::TargetChanged => (4055, "COMBAT_ATTACK_TARGET_CHANGED"),
    Self::NotDue => (4056, "COMBAT_ATTACK_NOT_DUE"),
    Self::OutOfRange => (4057, "COMBAT_ATTACK_OUT_OF_RANGE"),
    Self::Protected => (4058, "COMBAT_ATTACK_PROTECTED"),
    Self::OccurrenceConflict => (4059, "COMBAT_ATTACK_OCCURRENCE_CONFLICT"),
    Self::OccurrenceSuperseded => (4060, "COMBAT_ATTACK_OCCURRENCE_SUPERSEDED"),
    Self::LedgerFull => (4061, "COMBAT_ATTACK_LEDGER_FULL"),
    Self::InvalidPlan => (4062, "COMBAT_ATTACK_PLAN_INVALID"),
    Self::MissingVitals => (4063, "COMBAT_ATTACK_VITALS_MISSING"),
    Self::NumericOverflow => (4064, "COMBAT_ATTACK_NUMERIC_OVERFLOW"),
    Self::MissingCombatFacts => (4065, "COMBAT_ATTACK_FACTS_MISSING"),
    Self::StaleOwner => (4066, "COMBAT_ATTACK_OWNER_STALE"),
    Self::StaleAiSelection => (4067, "COMBAT_ATTACK_AI_SELECTION_STALE"),
);
boundary_codes!(crate::ability::exact_actor_resolution::ExactActorResolutionError,
    Self::NotCurrentActor => (4068, "COMBAT_EXACT_ACTOR_NOT_CURRENT"),
    Self::OccurrenceSubstitution => (4069, "COMBAT_EXACT_ACTOR_OCCURRENCE_SUBSTITUTION"),
    Self::RevisionSubstitution => (4070, "COMBAT_EXACT_ACTOR_REVISION_SUBSTITUTION"),
    Self::TargetSubstitution => (4071, "COMBAT_EXACT_ACTOR_TARGET_SUBSTITUTION"),
    Self::SourceSubstitution => (4072, "COMBAT_EXACT_ACTOR_SOURCE_SUBSTITUTION"),
);
boundary_codes!(crate::ability::commit::OwnerCommitError,
    Self::InvalidPlan => (4073, "COMBAT_OWNER_COMMIT_PLAN_INVALID"),
    Self::Plan(_) => (4074, "COMBAT_OWNER_COMMIT_PLAN"),
    Self::Owner(_) => (4075, "COMBAT_OWNER_COMMIT_OWNER"),
);
boundary_codes!(crate::creature_appearance_content::AppearanceContentError,
    Self::ContentChanged => (4076, "COMBAT_APPEARANCE_CONTENT_CHANGED"),
    Self::UnsupportedShape => (4077, "COMBAT_APPEARANCE_SHAPE_UNSUPPORTED"),
    Self::MissingQualifiedMember => (4078, "COMBAT_APPEARANCE_MEMBER_MISSING"),
    Self::InvalidDefinition => (4079, "COMBAT_APPEARANCE_DEFINITION_INVALID"),
);

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    #[test]
    fn every_combat_boundary_variant_has_a_unique_registered_code() -> Result<(), String> {
        let codes: Vec<u32> = (4010..=4079).collect();
        let unique: BTreeSet<_> = codes.iter().copied().collect();
        assert_eq!(unique.len(), codes.len());

        let registry: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/contracts/OTERYN_GAME_ERROR_CODE_REGISTRY.json"
        ))
        .map_err(|error| error.to_string())?;
        let registered: BTreeSet<_> = registry["codes"]
            .as_array()
            .ok_or_else(|| "registry codes must be an array".to_owned())?
            .iter()
            .filter_map(|entry| entry["code"].as_u64())
            .collect();
        for code in codes {
            assert!(
                registered.contains(&u64::from(code)),
                "E{code:04} is not registered"
            );
        }

        assert_eq!(
            crate::creature_auto_attack::AttackError::InvalidSource
                .code()
                .number,
            4050
        );
        Ok(())
    }
}
