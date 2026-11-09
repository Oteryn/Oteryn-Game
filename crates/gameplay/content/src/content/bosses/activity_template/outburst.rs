//! Crystal actions_outburst_lever.lua1–41 and BossLever defaults64–70.
use super::*;
impl ActiveGeneration {
    pub fn qualify_outburst_activity_template(
        &self,
        room: &QualifiedNativeEntryRoom,
        content_revision: &str,
    ) -> Result<QualifiedActivityRoomTemplate, ActivityTemplateRefusal> {
        use ActivityTemplateRefusal as R;
        if room.pinned_activity_source_room() != Some(PinnedActivitySourceRoom::CallbackOutburst)
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
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|r| r.profile.target.key == "oteryn:creature.outburst")
            .ok_or(R::CreatureClosureUnavailable)?;
        let rewards = native
            .reward_table()
            .row("oteryn:creature.outburst")
            .map_err(|_| R::CreatureClosureUnavailable)?;
        qualify_instance_death_descendant(&record.profile.data, &rewards)?;
        let cell = |x, y| LogicalCell { x, y, z: 14 };
        let levers = (31284..=31288).map(|y| cell(32207, y)).collect::<Vec<_>>();
        let entries = vec![cell(32234, 31292); 5];
        let boss = cell(32234, 31284);
        let exit = cell(32218, 31375);
        for at in levers
            .iter()
            .chain(entries.iter())
            .copied()
            .chain([boss, exit])
        {
            if room
                .movement_cells()
                .index()
                .lookup(room.movement_cells().scope(), at)
                .ok()
                != Some(crate::content::CollisionClass::Walkable)
            {
                return Err(R::UnknownPlacement);
            }
        }
        Ok(QualifiedActivityRoomTemplate {
            spite_source_setup: None,
            boss_key: "oteryn:boss.outburst".into(),
            template_key: "oteryn:room.callback-outburst-source".into(),
            template_revision: "crystal-201189f".into(),
            content_revision: content_revision.into(),
            creature_key: "oteryn:creature.outburst".into(),
            source_path:
                "data-global/scripts/quests/heart_of_destruction/actions_outburst_lever.lua".into(),
            source_sha256: "89acba0c46b779140af5fcff854f9e9c37bec19af05937c8112de219fcc454f7"
                .into(),
            room: room.clone(),
            source_levers: levers,
            entries,
            boss_position: boss,
            source_exit_position: exit,
            exit_position: exit,
            exit_teleporter: None,
            minimum_players: 1,
            minimum_level: 1,
            cooldown_seconds: 72000,
            time_limit_seconds: 1200,
            after_kill_seconds: 60,
            activation_sequence: self.activation_sequence(),
            server_digest: self.identity().server_artifact_digest(),
            client_digest: self.identity().client_artifact_digest(),
            activation_frame_binding_digest: room.frame_binding().digest(),
        })
    }
}
