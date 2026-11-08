//! Literal pinned Rotten levers; Quest admission and runtime callbacks retain their owners.
use super::*;
#[path = "rotten_blood/vemiath_exit.rs"]
mod vemiath_exit;

pub(crate) fn source_recipe(
    source: PinnedActivitySourceRoom,
) -> Option<(&'static str, &'static str, &'static str, &'static str)> {
    Some(match source {
        PinnedActivitySourceRoom::Murcion => (
            "murcion",
            "rotten_murcion.lua",
            "48f572ec9c418340bb07490334a216e81f4361627495e5c221d2a2d45b71d727",
            "extra_monster_closure",
        ),
        PinnedActivitySourceRoom::Ichgahal => (
            "ichgahal",
            "rotten_ichgahal.lua",
            "0822f2c430b389aa74bc3109db975e2bb96cd69385e6fc8fe26991b9b69f9b7c",
            "",
        ),
        PinnedActivitySourceRoom::Vemiath => (
            "vemiath",
            "rotten_vemiath.lua",
            "9b163af79214effbb5e9d2bd3d54290c266b988b33a89d809a70563e24c0c6c5",
            "",
        ),
        PinnedActivitySourceRoom::Chagorz => (
            "chagorz",
            "rotten_chagorz.lua",
            "d3f632880a7e871e0303994905ef3e814b22d6f797e51b732d4ce7f7c49050c2",
            "extra_monster_closure",
        ),
        PinnedActivitySourceRoom::Bakragore => (
            "bakragore",
            "rotten_bakragore.lua",
            "821b49caf5e1c55f8a0e9a83b5f1158e5f34d3b3876681c5d9adfb6eedd96189",
            "nonliteral.timeToFightAgain",
        ),
        _ => return None,
    })
}
impl ActiveGeneration {
    pub fn qualify_rotten_blood_activity_template(
        &self,
        room: &QualifiedNativeEntryRoom,
        content_revision: &str,
        source: PinnedActivitySourceRoom,
    ) -> Result<QualifiedActivityRoomTemplate, ActivityTemplateRefusal> {
        self.qualify_rotten_blood_activity_template_composed(room, content_revision, source, None)
    }

    /// Only the pinned Vemiath raw exit/Teleport arrival pair can compose here.
    pub fn qualify_vemiath_activity_template(
        &self,
        room: &QualifiedNativeEntryRoom,
        content_revision: &str,
        resolved: LogicalCell,
    ) -> Result<QualifiedActivityRoomTemplate, ActivityTemplateRefusal> {
        self.qualify_rotten_blood_activity_template_composed(
            room,
            content_revision,
            PinnedActivitySourceRoom::Vemiath,
            Some(resolved),
        )
    }

    fn qualify_rotten_blood_activity_template_composed(
        &self,
        room: &QualifiedNativeEntryRoom,
        content_revision: &str,
        source: PinnedActivitySourceRoom,
        resolved_exit: Option<LogicalCell>,
    ) -> Result<QualifiedActivityRoomTemplate, ActivityTemplateRefusal> {
        use ActivityTemplateRefusal as R;
        let (slug, file, sha, blocker) = source_recipe(source).ok_or(R::InvalidSourceCapture)?;
        if room.pinned_activity_source_room() != Some(source)
            || room.compiled().server_digest() != self.identity().server_artifact_digest()
            || room.compiled().client_digest() != self.identity().client_artifact_digest()
            || content_revision.is_empty()
            || content_revision.len() > 128
        {
            return Err(R::SourceMapUnavailable);
        }
        let native = self
            .native_gameplay()
            .ok_or(R::CreatureClosureUnavailable)?;
        if !native.qualifies_rotten_blood_callbacks(self.identity().server_artifact_digest()) {
            return Err(R::SourceHookUnavailable);
        }
        let path = format!("data-global/scripts/quests/rotten_blood_quest/{file}");
        let mut candidate = captured_boss_levers()?
            .into_iter()
            .find(|c| c.path == path && c.sha256 == sha)
            .ok_or(R::InvalidSourceCapture)?;
        let creature = format!("oteryn:creature.{slug}");
        let expected_blockers: Vec<String> = if blocker.is_empty() {
            vec![]
        } else {
            vec![blocker.into()]
        };
        let literal_level = if matches!(
            source,
            PinnedActivitySourceRoom::Murcion | PinnedActivitySourceRoom::Chagorz
        ) {
            0
        } else {
            250
        };
        if candidate.creature_key.as_deref() != Some(creature.as_str())
            || candidate.blockers != expected_blockers
            || !candidate.complete_literal_layout
            || candidate.minimum_players != Some(1)
            || candidate.minimum_level != Some(literal_level)
            || candidate.time_limit_seconds != Some(1200)
            || candidate.after_kill_seconds != Some(60)
            || candidate.lever_positions.len() != 5
            || candidate.entry_positions.len() != 5
            || candidate.cooldown_seconds
                != if source == PinnedActivitySourceRoom::Bakragore {
                    None
                } else {
                    Some(72000)
                }
        {
            return Err(R::InvalidSourceCapture);
        }
        // The extra-monsters tables in Murcion/Chagorz contain only comments.
        // Bakragore's primary literal ParseDuration68h selects the existing final
        // boss cooldown. Its separately pinned alternate lever reaches the same room.
        if source == PinnedActivitySourceRoom::Bakragore {
            let alt = captured_boss_levers()?
                .into_iter()
                .find(|c| {
                    c.path == "data-global/scripts/quests/rotten_blood_quest/rotten_bakragore1.lua"
                        && c.sha256
                            == "f6c8fc46dfde8d43bec59b0a2355871afbd19e6cb745f3096456d40c2bdc59a1"
                })
                .ok_or(R::InvalidSourceCapture)?;
            if !alt.complete_literal_layout
                || alt.creature_key.as_deref() != Some(creature.as_str())
                || alt.lever_positions.len() != 5
                || alt.entry_positions != candidate.entry_positions
                || alt.boss_position != candidate.boss_position
                || alt.exit_position != candidate.exit_position
            {
                return Err(R::InvalidSourceCapture);
            }
            candidate.lever_positions.extend(alt.lever_positions);
            candidate.entry_positions.extend(alt.entry_positions);
        }
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|r| r.profile.target.key == creature)
            .ok_or(R::CreatureClosureUnavailable)?;
        let rewards = native
            .reward_table()
            .row(&creature)
            .map_err(|_| R::CreatureClosureUnavailable)?;
        qualify_instance_death_descendant(&record.profile.data, &rewards)?;
        let cell = |[x, y, z]: [i32; 3]| LogicalCell { x, y, z };
        let levers: Vec<_> = candidate.lever_positions.into_iter().map(cell).collect();
        let entries: Vec<_> = candidate.entry_positions.into_iter().map(cell).collect();
        let boss = cell(candidate.boss_position.ok_or(R::IncompleteLayout)?);
        let source_exit = cell(candidate.exit_position.ok_or(R::IncompleteLayout)?);
        let exit = if source == PinnedActivitySourceRoom::Vemiath {
            vemiath_exit::qualify(room, source_exit, resolved_exit.ok_or(R::UnknownPlacement)?)?
        } else {
            if resolved_exit.is_some() {
                return Err(R::InvalidSourceCapture);
            }
            source_exit
        };
        for p in levers
            .iter()
            .chain(entries.iter())
            .copied()
            .chain([boss, exit])
        {
            if room
                .movement_cells()
                .index()
                .lookup(room.movement_cells().scope(), p)
                .ok()
                != Some(crate::content::CollisionClass::Walkable)
            {
                return Err(R::UnknownPlacement);
            }
        }
        Ok(QualifiedActivityRoomTemplate {
            boss_key: format!("oteryn:boss.{slug}"),
            template_key: format!("oteryn:room.{}-source", source.name()),
            template_revision: "crystal-201189f".into(),
            content_revision: content_revision.into(),
            creature_key: creature,
            source_path: path,
            source_sha256: sha.into(),
            room: room.clone(),
            source_levers: levers,
            entries,
            boss_position: boss,
            source_exit_position: source_exit,
            exit_position: exit,
            exit_teleporter: None,
            minimum_players: 1,
            minimum_level: 250,
            cooldown_seconds: if source == PinnedActivitySourceRoom::Bakragore {
                244800
            } else {
                72000
            },
            time_limit_seconds: 1200,
            after_kill_seconds: 60,
            activation_sequence: self.activation_sequence(),
            server_digest: self.identity().server_artifact_digest(),
            client_digest: self.identity().client_artifact_digest(),
            activation_frame_binding_digest: room.frame_binding().digest(),
        })
    }
}
