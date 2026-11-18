//! Crystal 201189f actions_the_last_lore_keeper.lua, including its two-floor room.
use super::*;
use crate::content::CollisionClass;

pub(super) const PATH: &str =
    "data-global/scripts/quests/forgotten_knowledge/actions_the_last_lore_keeper.lua";
const SHA256: &str = "3d2af8f8f4991fe56dc028ce7e878726c43586e84d7fdd5d1b5a932b66672c51";
const CREATURE: &str = "oteryn:creature.the_last_lore_keeper";

impl ActiveGeneration {
    pub fn qualify_lore_keeper_activity_template(
        &self,
        parent: &QualifiedNativeEntryRoom,
        room: &QualifiedNativeEntryRoom,
        content_revision: &str,
    ) -> Result<QualifiedActivityRoomTemplate, ActivityTemplateRefusal> {
        use ActivityTemplateRefusal as R;
        if content_revision.is_empty()
            || content_revision.len() > 128
            || [parent, room].iter().any(|view| {
                view.compiled().server_digest() != self.identity().server_artifact_digest()
                    || view.compiled().client_digest() != self.identity().client_artifact_digest()
            })
            || parent.movement_cells().scope().world_id != room.movement_cells().scope().world_id
        {
            return Err(R::InvalidSourceCapture);
        }
        let cell = |x, y, z| LogicalCell { x, y, z };
        let levers = (32018..=32020)
            .flat_map(|x| (32844..=32848).map(move |y| cell(x, y, 14)))
            .collect::<Vec<_>>();
        let entry = cell(31985, 32851, 14);
        let glyph = cell(31986, 32840, 14);
        let exit = cell(32035, 32859, 14);
        let movement = room.movement_cells();
        if levers
            .iter()
            .copied()
            .chain([
                entry,
                glyph,
                exit,
                cell(31987, 32839, 14),
                cell(31986, 32823, 15),
                cell(31989, 32823, 15),
                cell(31975, 32856, 15),
                cell(31975, 32839, 15),
                cell(31995, 32839, 15),
                cell(31995, 32856, 15),
            ])
            .any(|at| movement.index().lookup(movement.scope(), at) != Ok(CollisionClass::Walkable))
        {
            return Err(R::UnknownPlacement);
        }
        let native = self
            .native_gameplay()
            .ok_or(R::CreatureClosureUnavailable)?;
        for key in [
            CREATURE,
            "oteryn:creature.a_shielded_astral_glyph",
            "oteryn:creature.an_astral_glyph",
            "oteryn:creature.bound_astral_power",
            "oteryn:creature.the_astral_source",
            "oteryn:creature.the_distorted_astral_source",
        ] {
            let record = native
                .creature_profiles()
                .records
                .iter()
                .find(|record| record.profile.target.key == key)
                .ok_or(R::CreatureClosureUnavailable)?;
            if !super::super::primal_closure::abilities_closed(record) {
                return Err(R::CreatureClosureUnavailable);
            }
        }
        let view = room
            .bundle_activity_view(
                [31968, 32821, 32004, 32865, 14, 15],
                crate::content::NativeEntryStart {
                    x: entry.x,
                    y: entry.y,
                    floor: 14,
                },
            )
            .ok_or(R::SourceMapUnavailable)?;
        Ok(QualifiedActivityRoomTemplate {
            spite_source_setup: None,
            boss_key: "oteryn:boss.the_last_lore_keeper".into(),
            template_key: "oteryn:room.boss.the_last_lore_keeper".into(),
            template_revision: format!("crystal-{SHA256}"),
            content_revision: content_revision.into(),
            creature_key: CREATURE.into(),
            source_path: PATH.into(),
            source_sha256: SHA256.into(),
            room: view,
            entries: vec![entry; levers.len()],
            source_levers: levers,
            boss_position: glyph,
            source_exit_position: exit,
            exit_position: exit,
            exit_teleporter: None,
            minimum_players: 1,
            minimum_level: 0,
            cooldown_seconds: 14 * 24 * 60 * 60,
            time_limit_seconds: 30 * 60,
            after_kill_seconds: 0,
            activation_sequence: self.activation_sequence(),
            server_digest: self.identity().server_artifact_digest(),
            client_digest: self.identity().client_artifact_digest(),
            activation_frame_binding_digest: parent.frame_binding().digest(),
        })
    }
}
