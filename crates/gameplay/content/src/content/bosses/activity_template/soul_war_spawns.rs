//! Bounded native XML spawners; the existing physical spawn owner keeps timers.
use super::*;

/// Exact retained XML rows for one admitted source room. Only Content can issue
/// this placement policy; generic authored spawns never acquire XML force flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedAmbientXmlSpawns {
    role: PinnedActivitySourceRoom,
    world: oteryn_protocol_oteryn::WorldId,
    activation: u64,
    pins: [[u8; 32]; 4],
    sources: Vec<crate::content::project::NativeEntrySpawnFacts>,
}
impl QualifiedAmbientXmlSpawns {
    pub const fn role(&self) -> PinnedActivitySourceRoom {
        self.role
    }
    pub const fn world(&self) -> oteryn_protocol_oteryn::WorldId {
        self.world
    }
    pub const fn activation_sequence(&self) -> u64 {
        self.activation
    }
    pub const fn pins(&self) -> [[u8; 32]; 4] {
        self.pins
    }
    pub fn sources(&self) -> &[crate::content::project::NativeEntrySpawnFacts] {
        &self.sources
    }
}
impl ActiveGeneration {
    pub fn qualify_ambient_xml_spawn_policy(
        &self,
        room: &QualifiedNativeEntryRoom,
    ) -> Result<QualifiedAmbientXmlSpawns, ActivityTemplateRefusal> {
        let sources = self.qualify_soul_war_ambient_spawns(room)?;
        Ok(QualifiedAmbientXmlSpawns {
            world: room.movement_cells().scope().world_id,
            activation: self.activation_sequence(),
            role: room
                .pinned_activity_source_room()
                .ok_or(ActivityTemplateRefusal::InvalidSourceCapture)?,
            pins: [
                room.compiled().server_digest(),
                room.compiled().client_digest(),
                room.frame_binding().digest(),
                room.map_revision_digest(),
            ],
            sources,
        })
    }

    pub fn qualify_soul_war_ambient_spawns(
        &self,
        room: &QualifiedNativeEntryRoom,
    ) -> Result<Vec<crate::content::project::NativeEntrySpawnFacts>, ActivityTemplateRefusal> {
        use ActivityTemplateRefusal as R;
        let role = room
            .pinned_activity_source_room()
            .ok_or(R::InvalidSourceCapture)?;
        let native = self
            .native_gameplay()
            .ok_or(R::CreatureClosureUnavailable)?;
        if native.source_digest() != room.compiled().server_digest()
            || (role != PinnedActivitySourceRoom::CallbackRagiaz
                && !native.qualifies_soul_war_callbacks(native.source_digest()))
        {
            return Err(R::SourceHookUnavailable);
        }
        let map = room.source_world();
        let expected = crate::content::native_soul_war_spawns::expected(role)
            .ok_or(R::SourceHookUnavailable)?;
        let retained;
        let data = if let Some(map) = map {
            map.ambient_xml_spawn_data()
        } else {
            if !matches!(
                room.movement_cells().index(),
                crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(_)
            ) {
                return Err(R::OutsideQualifiedMap);
            }
            // Only the exact capture-qualified role retains recipe DATA. A
            // generic bundle_view clears that role and already refused above.
            retained = serde_json::from_slice::<serde_json::Value>(role.bytes())
                .map_err(|_| R::InvalidSourceCapture)?;
            retained.get("ambient_xml_spawns")
        };
        if data != Some(&expected) {
            return Err(R::SourceHookUnavailable);
        }
        let rows: Vec<crate::content::native_soul_war_spawns::AmbientXmlSpawn> =
            serde_json::from_value(expected).map_err(|_| R::InvalidSourceCapture)?;
        rows.into_iter()
            .map(|row| {
                let [x, y, floor] = row.position;
                if let Some(map) = map {
                    map.immutable_tile(
                        map.scope(),
                        crate::content::LogicalCell { x, y, z: -floor },
                    )
                    .map_err(|_| R::OutsideQualifiedMap)?;
                } else {
                    room.movement_cells()
                        .index()
                        .lookup(
                            room.movement_cells().scope(),
                            crate::content::LogicalCell { x, y, z: floor },
                        )
                        .map_err(|_| R::OutsideQualifiedMap)?;
                }
                let record = native
                    .creature_profiles()
                    .records
                    .iter()
                    .find(|record| {
                        record.profile.target.key == row.creature_key
                            && record.profile.target.revision == "definition-r1"
                    })
                    .ok_or(R::CreatureClosureUnavailable)?;
                let crate::content::ProjectV2AuthoringProfileData::Creature(profile) =
                    &record.profile.data
                else {
                    return Err(R::CreatureClosureUnavailable);
                };
                let maximum = profile
                    .health
                    .filter(|value| *value > 0)
                    .ok_or(R::CreatureClosureUnavailable)?;
                let health = profile.initial_health.unwrap_or(maximum);
                if health == 0 || health > maximum {
                    return Err(R::CreatureClosureUnavailable);
                }
                Ok((
                    vec![(
                        x,
                        y,
                        i16::try_from(floor).map_err(|_| R::OutsideQualifiedMap)?,
                    )],
                    row.respawn_seconds
                        .checked_mul(1000)
                        .ok_or(R::InvalidSourceCapture)?,
                    5000,
                    row.creature_key,
                    health,
                ))
            })
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::native_gameplay::test_support::activated_with_loot_tables;
    #[test]
    fn srx3_soul_war_spite_real_xml_spawners_keep_exact_native_points_and_ten_seconds()
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut id = [84; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        let world = oteryn_protocol_oteryn::WorldId::decode(&id).map_err(|e| format!("{e:?}"))?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("actual source activation")?;
        let source_room = pin
            .qualified_room()
            .pinned_activity_source_view(PinnedActivitySourceRoom::GoshnarsSpite)
            .map_err(|error| format!("{error:?}"))?;
        let room = &source_room;
        assert_eq!(
            room.pinned_activity_source_room(),
            Some(PinnedActivitySourceRoom::GoshnarsSpite),
            "fixture must retain its exact source room before the new qualifier"
        );
        let facts = active
            .qualify_soul_war_ambient_spawns(room)
            .map_err(|e| format!("REACHED actual Spite source spawner qualification: {e:?}"))?;
        assert_eq!(facts.len(), 3);
        let mut actual = facts
            .into_iter()
            .map(|(cells, delay, retry, key, health)| {
                assert_eq!(cells.len(), 1);
                assert_eq!(delay, 10000);
                assert_eq!(retry, 5000);
                (key, cells[0], health)
            })
            .collect::<Vec<_>>();
        actual.sort();
        assert_eq!(
            actual,
            vec![
                (
                    "oteryn:creature.spiteful_spitter".into(),
                    (33744, 31636, 14),
                    28000
                ),
                (
                    "oteryn:creature.weeping_soul".into(),
                    (33738, 31632, 14),
                    7000
                ),
                (
                    "oteryn:creature.weeping_soul".into(),
                    (33748, 31634, 14),
                    7000
                ),
            ]
        );
        Ok(())
    }
}
