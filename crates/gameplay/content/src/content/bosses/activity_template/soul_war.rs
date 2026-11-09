//! Fixed Soul War source-entry recipe facts; callback completion remains separate.
use super::*;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Recipe {
    slug: String,
    room: String,
    source_path: String,
    source_sha256: String,
    line: u32,
    boss_key: String,
    creature_key: String,
    boss: [i32; 3],
    levers: Vec<[i32; 3]>,
    entries: Vec<[i32; 3]>,
    exit: [i32; 3],
    minimum_players: u32,
    minimum_level: u32,
    cooldown_seconds: u32,
    time_limit_seconds: u32,
    after_kill_seconds: u32,
}
impl ActiveGeneration {
    pub fn qualify_soul_war_activity_template(
        &self,
        room: &QualifiedNativeEntryRoom,
        content_revision: &str,
        source: PinnedActivitySourceRoom,
    ) -> Result<QualifiedActivityRoomTemplate, ActivityTemplateRefusal> {
        use ActivityTemplateRefusal as R;
        let (slug, role) = match source {
            PinnedActivitySourceRoom::GoshnarsMalice => ("malice", "GoshnarsMalice"),
            PinnedActivitySourceRoom::GoshnarsGreed => ("greed", "GoshnarsGreed"),
            PinnedActivitySourceRoom::GoshnarsSpite => ("spite", "GoshnarsSpite"),
            PinnedActivitySourceRoom::GoshnarsHatred => ("hatred", "GoshnarsHatred"),
            PinnedActivitySourceRoom::GoshnarsCruelty => ("cruelty", "GoshnarsCruelty"),
            PinnedActivitySourceRoom::GoshnarsMegalomania => {
                ("megalomania_purple", "GoshnarsMegalomania")
            }
            _ => return Err(R::InvalidSourceCapture),
        };
        if room.pinned_activity_source_room() != Some(source)
            || room.compiled().server_digest() != self.identity().server_artifact_digest()
            || room.compiled().client_digest() != self.identity().client_artifact_digest()
            || content_revision.is_empty()
            || content_revision.len() > 128
        {
            return Err(R::SourceMapUnavailable);
        }
        // Source-only recipes remain refused until their callback owners and
        // source rule facts qualify for production admission.
        if !super::soul_war_entry_gate::callbacks_available(source) {
            return Err(R::SourceHookUnavailable);
        }
        let native = self
            .native_gameplay()
            .ok_or(R::CreatureClosureUnavailable)?;
        if !native.qualifies_soul_war_callbacks(self.identity().server_artifact_digest()) {
            return Err(R::SourceHookUnavailable);
        }
        let recipes: Vec<Recipe> = serde_json::from_slice(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../tools/content-schema/native-gameplay/srx3-soul-war-entry-recipes.json"
        )))
        .map_err(|_| R::InvalidSourceCapture)?;
        let recipe = recipes
            .into_iter()
            .find(|r| r.slug == slug && r.room == role)
            .ok_or(R::InvalidSourceCapture)?;
        if recipe.source_path != "data-global/lib/quests/soul_war.lua"
            || recipe.source_sha256
                != "89c0024914b0593b9b7d2bfb535a24025a552f54b158319697cdf56a66375e75"
            || recipe.line == 0
            || recipe.minimum_players != 1
            || recipe.minimum_level != 250
            || recipe.time_limit_seconds != 1200
            || recipe.after_kill_seconds != 60
            || recipe.levers.len() != 5
            || recipe.entries.len() != 5
        {
            return Err(R::InvalidSourceCapture);
        }
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|r| r.profile.target.key == recipe.creature_key)
            .ok_or(R::CreatureClosureUnavailable)?;
        let rewards = native
            .reward_table()
            .row(&recipe.creature_key)
            .map_err(|_| R::CreatureClosureUnavailable)?;
        qualify_instance_death_descendant(&record.profile.data, &rewards)?;
        let cell = |[x, y, z]: [i32; 3]| LogicalCell { x, y, z };
        let levers: Vec<_> = recipe.levers.into_iter().map(cell).collect();
        let entries: Vec<_> = recipe.entries.into_iter().map(cell).collect();
        let boss = cell(recipe.boss);
        let exit = cell(recipe.exit);
        for position in levers
            .iter()
            .chain(entries.iter())
            .copied()
            .chain([boss, exit])
        {
            if room
                .movement_cells()
                .index()
                .lookup(room.movement_cells().scope(), position)
                .ok()
                != Some(crate::content::CollisionClass::Walkable)
            {
                return Err(R::UnknownPlacement);
            }
        }
        Ok(QualifiedActivityRoomTemplate {
            boss_key: recipe.boss_key,
            template_key: format!("oteryn:room.{}-source", source.name()),
            template_revision: "crystal-201189f".into(),
            content_revision: content_revision.into(),
            creature_key: recipe.creature_key,
            source_path: recipe.source_path,
            source_sha256: recipe.source_sha256,
            room: room.clone(),
            source_levers: levers,
            entries,
            boss_position: boss,
            source_exit_position: exit,
            exit_position: exit,
            exit_teleporter: None,
            minimum_players: recipe.minimum_players,
            minimum_level: recipe.minimum_level,
            cooldown_seconds: recipe.cooldown_seconds,
            time_limit_seconds: recipe.time_limit_seconds,
            after_kill_seconds: recipe.after_kill_seconds,
            activation_sequence: self.activation_sequence(),
            server_digest: self.identity().server_artifact_digest(),
            client_digest: self.identity().client_artifact_digest(),
            activation_frame_binding_digest: room.frame_binding().digest(),
        })
    }
}
