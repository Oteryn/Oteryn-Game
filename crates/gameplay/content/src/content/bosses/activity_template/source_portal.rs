//! Exact Rakesh portal; one physical STEP is separate from room occupancy.
use super::*;
const MAP_SHA: &str = "b2875f6d7f5ed654a5e97b3378d3d24f7f8d99f8507d3066bab582a4d4bf0105";
const PROFILE_SHA: &str = "6c5a6279a778ff77f9c02cab006cbe94f4ae1336a8fd07ff54b2a18df66b2e21";
impl ContentActivationController {
    /// Geometry qualification alone never invokes this gameplay constructor.
    /// Compose only with the closed Summer Instance physical MINT descendant.
    pub fn qualify_rakesh_source_portal_template(
        &self,
        origin: &QualifiedNativeEntryRoom,
        movement: &crate::content::NativeEntryMovementCells,
        origin_map_digest: [u8; 32],
        content_revision: &str,
    ) -> Result<QualifiedActivityRoomTemplate, ActivityTemplateRefusal> {
        use ActivityTemplateRefusal as R;
        if !self.is_ready() || content_revision.is_empty() || content_revision.len() > 128 {
            return Err(R::NoActiveGeneration);
        }
        let active = self.active().ok_or(R::NoActiveGeneration)?;
        let child = self.qualify_rakesh_source_geometry(origin, movement, origin_map_digest)?;
        let native = active
            .native_gameplay()
            .ok_or(R::CreatureClosureUnavailable)?;
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|record| record.profile.target.key == "oteryn:creature.rakesh_moonfang")
            .ok_or(R::CreatureClosureUnavailable)?;
        if !matches_source_profile(record)? || record.profile.target.revision != "definition-r1" {
            return Err(R::SourceProfileChanged);
        }
        let ProjectV2AuthoringProfileData::Creature(creature) = &record.profile.data else {
            return Err(R::CreatureClosureUnavailable);
        };
        let body = creature
            .details
            .as_ref()
            .and_then(|details| details.corpse_item.as_ref())
            .ok_or(R::CreatureClosureUnavailable)?;
        if creature.health != Some(25_500)
            || creature.experience != Some(25_500)
            || body.key != "oteryn:item.tibia.i43758"
            || body.revision != "definition-r1"
            || body.family != crate::content::ProjectV2Family::Item
        {
            return Err(R::SourceProfileChanged);
        }
        // Same real public creature/body/loot row consumed by the DUR bridge;
        // no omitted body, synthetic empty loot or substitute actor qualifies.
        native
            .reward_table()
            .row(&record.profile.target.key)
            .map_err(|_| R::CreatureClosureUnavailable)?;
        let Some(crate::content::project::ProjectV2AuthoringProfile {
            data: ProjectV2AuthoringProfileData::Behavior(behavior),
            ..
        }) = &record.behavior
        else {
            return Err(R::CreatureClosureUnavailable);
        };
        if behavior.summons.is_some()
            || !behavior.event_bindings.is_empty()
            || behavior
                .attacks
                .iter()
                .chain(behavior.defenses.iter())
                .any(|attack| {
                    !record
                        .monster_melee
                        .as_ref()
                        .is_some_and(|melee| melee.ability == attack.ability)
                        && !record
                            .source_item_abilities
                            .iter()
                            .any(|profile| profile.target == attack.ability)
                        && !record
                            .attack_definitions
                            .as_ref()
                            .is_some_and(|definitions| {
                                definitions
                                    .profiles
                                    .iter()
                                    .any(|profile| profile.target == attack.ability)
                            })
                })
        {
            return Err(R::CreatureClosureUnavailable);
        }
        Ok(QualifiedActivityRoomTemplate {
            source_portal_origin: Some(SourcePortalOrigin {
                frame_digest: origin.frame_binding().digest(),
                map_digest: origin_map_digest,
            }),
            boss_key: "oteryn:boss.rakesh_moonfang".into(),
            template_key: "oteryn:room.rakesh_moonfang_source_portal".into(),
            template_revision: format!("source-portal-{MAP_SHA}"),
            content_revision: content_revision.into(),
            creature_key: record.profile.target.key.clone(),
            source_path: "data-global/world/15.30/werepanther_boss_map_33035_32967_7.otbm".into(),
            source_sha256: MAP_SHA.into(),
            room: child,
            source_levers: vec![LogicalCell {
                x: 33039,
                y: 32964,
                z: 8,
            }],
            entries: vec![LogicalCell {
                x: 33045,
                y: 32988,
                z: -8,
            }],
            boss_position: LogicalCell {
                x: 33042,
                y: 32993,
                z: -8,
            },
            exit_position: LogicalCell {
                x: 33040,
                y: 32962,
                z: 8,
            },
            minimum_players: 1,
            minimum_level: 1,
            cooldown_seconds: 7200,
            time_limit_seconds: 600,
            after_kill_seconds: 0,
            activation_sequence: active.activation_sequence(),
            server_digest: active.identity().server_artifact_digest(),
            client_digest: active.identity().client_artifact_digest(),
        })
    }
}

// Preserve the exact raw source seal, then compare every field through the same
// typed owner schema as activation. Serde defaults are not source changes.
fn matches_source_profile(
    actual: &crate::content::native_gameplay::CreatureProfileRecord,
) -> Result<bool, ActivityTemplateRefusal> {
    let raw: serde_json::Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/content-schema/native-gameplay/rakesh-source-native-record.json"
    )))
    .map_err(|_| ActivityTemplateRefusal::SourceProfileChanged)?;
    let digest = sha256(
        &serde_json::to_vec(&raw).map_err(|_| ActivityTemplateRefusal::SourceProfileChanged)?,
    );
    let digest: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    if digest != PROFILE_SHA {
        return Err(ActivityTemplateRefusal::SourceProfileChanged);
    }
    let expected: crate::content::native_gameplay::CreatureProfileRecord =
        serde_json::from_value(raw).map_err(|_| ActivityTemplateRefusal::SourceProfileChanged)?;
    Ok(actual == &expected)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srv7_rakesh_source_seal_uses_owner_schema_and_rejects_field_drift()
    -> Result<(), Box<dyn std::error::Error>> {
        let document: crate::content::native_gameplay::CreatureProfilesDocument =
            serde_json::from_slice(&std::fs::read(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/creatures/definitions/spell-native-profiles.json"
            ))?)?;
        let actual = document
            .records
            .iter()
            .find(|record| record.profile.target.key == "oteryn:creature.rakesh_moonfang")
            .ok_or("actual pinned Rakesh record")?;
        assert!(matches_source_profile(actual).map_err(|error| format!("{error:?}"))?);
        let normalized = serde_json::to_value(actual)?;
        let normalized = sha256(&serde_json::to_vec(&normalized)?);
        eprintln!(
            "SRV7_RAKESH_OWNING_SCHEMA_SOURCE_SEAL raw={} normalized={}",
            PROFILE_SHA,
            normalized
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        for field in ["health", "experience", "corpse", "attack"] {
            let mut drift = actual.clone();
            if field == "attack" {
                let behavior = drift.behavior.as_mut().ok_or("source behavior")?;
                let ProjectV2AuthoringProfileData::Behavior(behavior) = &mut behavior.data else {
                    return Err("source behavior kind".into());
                };
                behavior
                    .attacks
                    .first_mut()
                    .ok_or("source attack")?
                    .ability
                    .key
                    .push_str("_drift");
            } else {
                let ProjectV2AuthoringProfileData::Creature(creature) = &mut drift.profile.data
                else {
                    return Err("source Creature kind".into());
                };
                match field {
                    "health" => creature.health = Some(25_501),
                    "experience" => creature.experience = Some(25_501),
                    "corpse" => creature
                        .details
                        .as_mut()
                        .and_then(|details| details.corpse_item.as_mut())
                        .ok_or("source corpse")?
                        .key
                        .push_str("_drift"),
                    _ => return Err("unknown drift fixture".into()),
                }
            }
            assert!(
                !matches_source_profile(&drift).map_err(|error| format!("{error:?}"))?,
                "actual source field drift must remain refused: {field}"
            );
        }
        Ok(())
    }
}
