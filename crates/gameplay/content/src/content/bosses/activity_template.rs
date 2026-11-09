//! Boss room views issued by the existing Content activation owner.
//!
//! Source capture is inert authoring. A template requires the current artifact,
//! its already qualified map, a complete native creature closure and every
//! source hook. Unknown map cells or custom hooks never become a vanilla boss.
use crate::content::{
    ActiveGeneration, ContentActivationController, LogicalCell, QualifiedNativeEntryRoom,
    channel_content_pin::ChannelContentPin, project::ProjectV2AuthoringProfileData,
};
use serde::{Deserialize, Serialize};

mod first_dragon;
#[path = "activity_template/galthen.rs"]
mod galthen;
pub use galthen::{BossCallbackSourceClosure, QualifiedBossExitTeleporter};
mod outburst;
#[path = "activity_template/rotten_blood.rs"]
mod rotten_blood;
#[path = "activity_template/soul_war.rs"]
mod soul_war;
mod soul_war_entry_gate;
#[path = "activity_template/soul_war_spawns.rs"]
mod soul_war_spawns;
pub use soul_war_spawns::QualifiedAmbientXmlSpawns;

pub const CRYSTAL_REVISION: &str = "201189f871d23597b82caf88fd611454a2ddefb8";
pub use crate::content::project::native_spell_world::PinnedActivitySourceRoom;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BossLeverCandidate {
    pub path: String,
    pub sha256: String,
    pub line: u32,
    pub boss_name: Option<String>,
    pub creature_key: Option<String>,
    pub boss_position: Option<[i32; 3]>,
    pub lever_positions: Vec<[i32; 3]>,
    pub entry_positions: Vec<[i32; 3]>,
    pub zone_from: Option<[i32; 3]>,
    pub zone_to: Option<[i32; 3]>,
    pub exit_position: Option<[i32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_exit_teleporter: Option<[i32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_callback_closure: Option<BossCallbackSourceClosure>,
    pub blockers: Vec<String>,
    pub required_callbacks: Vec<String>,
    pub callbacks_known: bool,
    pub native_profile_sha256: Option<String>,
    pub complete_literal_layout: bool,
    pub minimum_players: Option<u32>,
    pub minimum_level: Option<u32>,
    pub cooldown_seconds: Option<u32>,
    pub time_limit_seconds: Option<u32>,
    pub after_kill_seconds: Option<u32>,
}

impl BossLeverCandidate {
    /// Crystal 201189f actions/bosses_levers/the_primal_manace.lua:2-4:
    /// the lever boss name identifies the captured creature; the room keeps
    /// that source identity in the boss namespace.
    pub fn boss_key(&self) -> Option<String> {
        self.creature_key
            .as_deref()?
            .strip_prefix("oteryn:creature.")
            .map(|suffix| format!("oteryn:boss.{suffix}"))
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateDocument {
    schema: String,
    source_repository: String,
    source_revision: String,
    defaults_source: String,
    records: Vec<BossLeverCandidate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityTemplateRefusal {
    InvalidSourceCapture,
    NoActiveGeneration,
    IncompleteLayout,
    SourceMapUnavailable,
    OutsideQualifiedMap,
    UnknownPlacement,
    SourceHookUnavailable,
    CreatureClosureUnavailable,
    SourceProfileChanged,
    InstanceCorpseMintUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityTemplateDiagnostic {
    pub path: String,
    pub reason: ActivityTemplateRefusal,
    pub unavailable_source_hooks: Vec<String>,
}

impl ActivityTemplateDiagnostic {
    fn rejected(candidate: &BossLeverCandidate, reason: ActivityTemplateRefusal) -> Self {
        Self {
            path: candidate.path.clone(),
            reason,
            unavailable_source_hooks: if reason == ActivityTemplateRefusal::SourceHookUnavailable {
                unavailable_source_hooks(candidate)
                    .map(str::to_owned)
                    .collect()
            } else {
                Vec::new()
            },
        }
    }
}

fn unavailable_source_hooks(candidate: &BossLeverCandidate) -> impl Iterator<Item = &str> {
    // Crystal 201189f data-global/monster/quests/primal_ordeal_quest/
    // the_primal_menace.lua:66-69,338-347: captured death and think callbacks
    // require native owners. A diagnostic must not imply BossLeverOnDeath
    // implements those distinct source hooks.
    candidate
        .required_callbacks
        .iter()
        .map(String::as_str)
        .filter(|hook| *hook != "BossLeverOnDeath")
}

/// Produced only by the active Content owner, never by a client or SQL catalogue
/// row. Its bounded room keeps the original frame/map/artifact identity and only
/// exposes the admitted encounter region. Entry order stays authored; the raw exit
/// remains explicit when a pinned source Teleport composes its final landing.
#[derive(Debug)]
pub struct QualifiedActivityRoomTemplate {
    boss_key: String,
    template_key: String,
    template_revision: String,
    content_revision: String,
    creature_key: String,
    source_path: String,
    source_sha256: String,
    room: QualifiedNativeEntryRoom,
    source_levers: Vec<LogicalCell>,
    entries: Vec<LogicalCell>,
    boss_position: LogicalCell,
    source_exit_position: LogicalCell,
    exit_position: LogicalCell,
    exit_teleporter: Option<QualifiedBossExitTeleporter>,
    minimum_players: u32,
    minimum_level: u32,
    cooldown_seconds: u32,
    time_limit_seconds: u32,
    after_kill_seconds: u32,
    activation_sequence: u64,
    server_digest: [u8; 32],
    client_digest: [u8; 32],
    activation_frame_binding_digest: [u8; 32],
    spite_source_setup: Option<super::spite_closure::QualifiedSpiteClosure>,
}

impl QualifiedActivityRoomTemplate {
    pub fn spite_source_setup(&self) -> Option<&super::spite_closure::QualifiedSpiteClosure> {
        self.spite_source_setup.as_ref()
    }

    pub fn activation_frame_binding_digest(&self) -> [u8; 32] {
        self.activation_frame_binding_digest
    }
    pub fn boss_key(&self) -> &str {
        &self.boss_key
    }
    pub fn template_key(&self) -> &str {
        &self.template_key
    }
    pub fn template_revision(&self) -> &str {
        &self.template_revision
    }
    pub fn content_revision(&self) -> &str {
        &self.content_revision
    }
    pub fn creature_key(&self) -> &str {
        &self.creature_key
    }
    pub fn source_path(&self) -> &str {
        &self.source_path
    }
    pub fn source_sha256(&self) -> &str {
        &self.source_sha256
    }
    pub fn room(&self) -> &QualifiedNativeEntryRoom {
        &self.room
    }
    pub fn lever_positions(&self) -> &[LogicalCell] {
        &self.source_levers
    }
    pub fn entry_positions(&self) -> &[LogicalCell] {
        &self.entries
    }
    pub fn boss_position(&self) -> LogicalCell {
        self.boss_position
    }
    pub fn source_exit_position(&self) -> LogicalCell {
        self.source_exit_position
    }
    pub fn exit_position(&self) -> LogicalCell {
        self.exit_position
    }
    pub fn exit_teleporter(&self) -> Option<QualifiedBossExitTeleporter> {
        self.exit_teleporter
    }
    pub fn minimum_players(&self) -> u32 {
        self.minimum_players
    }
    pub fn maximum_players(&self) -> usize {
        if self.boss_key == "oteryn:boss.bakragore" {
            5
        } else {
            self.entries.len()
        }
    }
    pub fn minimum_level(&self) -> u32 {
        self.minimum_level
    }
    pub fn cooldown_seconds(&self) -> u32 {
        self.cooldown_seconds
    }
    pub fn time_limit_seconds(&self) -> u32 {
        self.time_limit_seconds
    }
    pub fn after_kill_seconds(&self) -> u32 {
        self.after_kill_seconds
    }

    /// Issue the exact child view under its still-current parent activation.
    /// This is not an origin Channel pin clone and grants no Instance assignment.
    pub fn issue_runtime_pin(
        &self,
        active: &ActiveGeneration,
    ) -> Result<ChannelContentPin, ActivityTemplateRefusal> {
        if active.activation_sequence() != self.activation_sequence
            || active.identity().server_artifact_digest() != self.server_digest
            || active.identity().client_artifact_digest() != self.client_digest
        {
            return Err(ActivityTemplateRefusal::NoActiveGeneration);
        }
        let start = self.room.entry_start();
        Ok(ChannelContentPin::from_activation(
            self.room.movement_cells().scope().world_id,
            self.activation_sequence,
            self.server_digest,
            self.client_digest,
            self.room.frame_binding().digest(),
            self.room.map_revision_digest(),
            (start.x, start.y, start.floor),
        ))
    }
}

pub fn captured_boss_levers() -> Result<Vec<BossLeverCandidate>, ActivityTemplateRefusal> {
    let document: CandidateDocument = serde_json::from_slice(include_bytes!(
        "../../../../../../content/encounters/bosses/crystal-lever-candidates.json"
    ))
    .map_err(|_| ActivityTemplateRefusal::InvalidSourceCapture)?;
    if document.schema != "OTERYN_CRYSTAL_BOSS_LEVER_CANDIDATES/v1"
        || document.source_repository != "zimbadev/crystalserver"
        || document.source_revision != CRYSTAL_REVISION
        || document.defaults_source.is_empty()
        || document.records.is_empty()
        || document.records.len() > 256
        || document
            .records
            .windows(2)
            .any(|w| (w[0].path.as_str(), w[0].line) >= (w[1].path.as_str(), w[1].line))
    {
        return Err(ActivityTemplateRefusal::InvalidSourceCapture);
    }
    let mut records = document.records;
    records.push(first_dragon::candidate()?);
    records.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(records)
}

impl ActiveGeneration {
    /// Only the two embedded, source-qualified SRX-3 room captures are accepted.
    /// This creates no Instance assignment, readiness, participant or teleport grant.
    pub fn qualify_activity_source_room(
        &self,
        room: &QualifiedNativeEntryRoom,
        source: PinnedActivitySourceRoom,
    ) -> Result<QualifiedNativeEntryRoom, ActivityTemplateRefusal> {
        if room.compiled().server_digest() != self.identity().server_artifact_digest()
            || room.compiled().client_digest() != self.identity().client_artifact_digest()
        {
            return Err(ActivityTemplateRefusal::NoActiveGeneration);
        }
        let native = self
            .native_gameplay()
            .ok_or(ActivityTemplateRefusal::CreatureClosureUnavailable)?;
        let index = native
            .item_index()
            .ok_or(ActivityTemplateRefusal::CreatureClosureUnavailable)?;
        let view = room
            .pinned_activity_source_view(source)
            .map_err(|_| ActivityTemplateRefusal::SourceMapUnavailable)?;
        let map = view
            .source_world()
            .ok_or(ActivityTemplateRefusal::SourceMapUnavailable)?;
        if map.source_identity_bindings().iter().any(|binding| {
            let static_binding = binding.external_id.parse::<u32>().ok().is_some_and(|id| {
                crate::content::qualified_static_binding(
                    match binding.target.family {
                        crate::content::project::ProjectV2Family::Terrain => "Terrain",
                        crate::content::project::ProjectV2Family::WorldObject => "WorldObject",
                        _ => "",
                    },
                    &binding.target.key,
                    &binding.target.revision,
                    id,
                )
            });
            !static_binding
                && (binding.target.family != crate::content::project::ProjectV2Family::Item
                    || index
                        .definition_ref("Item", &binding.target.key, &binding.target.revision)
                        .is_none())
        }) {
            return Err(ActivityTemplateRefusal::CreatureClosureUnavailable);
        }
        Ok(view)
    }
}
impl ActiveGeneration {
    pub fn qualifies_primal_source_lobby(&self, room: &QualifiedNativeEntryRoom) -> bool {
        room.compiled().server_digest() == self.identity().server_artifact_digest()
            && room.compiled().client_digest() == self.identity().client_artifact_digest()
            && crate::content::project::native_spell_world::qualifies_primal_lobby_cells(room)
    }
    /// Primal's custom callbacks are implemented by the native W10 owners.
    /// The ordinary lever qualifier still refuses every other unknown hook.
    pub fn qualify_primal_activity_template(
        &self,
        room: &QualifiedNativeEntryRoom,
        content_revision: &str,
    ) -> Result<QualifiedActivityRoomTemplate, ActivityTemplateRefusal> {
        use ActivityTemplateRefusal as R;
        const CREATURE: &str = "oteryn:creature.the_primal_menace";
        const PATH: &str = "data-global/scripts/actions/bosses_levers/the_primal_manace.lua";
        const SOURCE: &str = "59768c964e9e6319810e3c0619a1e322b8a8a2c3f03232b021667ae595927703";
        if room.pinned_activity_source_room() != Some(PinnedActivitySourceRoom::Primal)
            || !matches!(
                room.movement_cells().index(),
                crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(_)
            )
            || room.compiled().server_digest() != self.identity().server_artifact_digest()
            || room.compiled().client_digest() != self.identity().client_artifact_digest()
            || content_revision.is_empty()
            || content_revision.len() > 128
        {
            return Err(R::InvalidSourceCapture);
        }
        let candidate = captured_boss_levers()?
            .into_iter()
            .find(|candidate| candidate.path == PATH && candidate.sha256 == SOURCE)
            .ok_or(R::InvalidSourceCapture)?;
        if candidate.creature_key.as_deref() != Some(CREATURE)
            || candidate.blockers != ["config.disableCooldown"]
            || candidate.minimum_players != Some(1)
            || candidate.minimum_level != Some(500)
            || candidate.cooldown_seconds != Some(72_000)
            || candidate.time_limit_seconds != Some(1_200)
            || candidate.after_kill_seconds != Some(60)
            || !candidate.complete_literal_layout
        {
            return Err(R::InvalidSourceCapture);
        }
        // The captured generic diagnostic treats any disableCooldown token as
        // unresolved. The pinned source's literal false (:18) preserves cooldown.
        let native = self
            .native_gameplay()
            .ok_or(R::CreatureClosureUnavailable)?;
        if !native.qualifies_primal_callbacks(self.identity().server_artifact_digest()) {
            return Err(R::SourceHookUnavailable);
        }
        let pod = native
            .item_policy("oteryn:item.tibia.i39176", "definition-r1")
            .ok_or(R::SourceHookUnavailable)?;
        if !pod.record().admission.materializable
            || pod.record().admission.legal_destinations
                != [crate::content::native_gameplay::NativeItemDestination::Ground]
        {
            return Err(R::SourceHookUnavailable);
        }
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|record| record.profile.target.key == CREATURE)
            .ok_or(R::CreatureClosureUnavailable)?;
        let rewards = native
            .reward_table()
            .row(CREATURE)
            .map_err(|_| R::CreatureClosureUnavailable)?;
        qualify_instance_death_descendant(&record.profile.data, &rewards)?;
        let cell = |[x, y, z]: [i32; 3]| LogicalCell { x, y, z };
        let levers = candidate
            .lever_positions
            .iter()
            .copied()
            .map(cell)
            .collect::<Vec<_>>();
        let entries = candidate
            .entry_positions
            .iter()
            .copied()
            .map(cell)
            .collect::<Vec<_>>();
        let boss = cell(candidate.boss_position.ok_or(R::IncompleteLayout)?);
        let exit = cell(candidate.exit_position.ok_or(R::IncompleteLayout)?);
        if levers.len() != 5 || entries.len() != 5 {
            return Err(R::IncompleteLayout);
        }
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
            boss_key: "oteryn:boss.the_primal_menace".into(),
            template_key: "oteryn:room.primal-source".into(),
            template_revision: "crystal-201189f".into(),
            content_revision: content_revision.into(),
            creature_key: CREATURE.into(),
            source_path: PATH.into(),
            source_sha256: SOURCE.into(),
            room: room.clone(),
            source_levers: levers,
            entries,
            boss_position: boss,
            source_exit_position: exit,
            exit_position: exit,
            exit_teleporter: None,
            minimum_players: 1,
            minimum_level: 500,
            cooldown_seconds: 72_000,
            time_limit_seconds: 1_200,
            after_kill_seconds: 60,
            activation_sequence: self.activation_sequence(),
            server_digest: self.identity().server_artifact_digest(),
            client_digest: self.identity().client_artifact_digest(),
            activation_frame_binding_digest: room.frame_binding().digest(),
        })
    }
}
impl ContentActivationController {
    pub fn qualify_activity_source_room(
        &self,
        room: &QualifiedNativeEntryRoom,
        source: PinnedActivitySourceRoom,
    ) -> Result<QualifiedNativeEntryRoom, ActivityTemplateRefusal> {
        self.active()
            .filter(|_| self.is_ready())
            .ok_or(ActivityTemplateRefusal::NoActiveGeneration)?
            .qualify_activity_source_room(room, source)
    }
    /// Qualify every captured room; preserve a diagnostic for every rejection.
    pub fn qualify_initial_activity_template(
        &self,
        parent: &QualifiedNativeEntryRoom,
        room: &QualifiedNativeEntryRoom,
        content_revision: &str,
        candidates: &[BossLeverCandidate],
    ) -> (
        Vec<QualifiedActivityRoomTemplate>,
        Vec<ActivityTemplateDiagnostic>,
    ) {
        let mut diagnostics = Vec::new();
        let mut templates = Vec::new();
        if !self.is_ready() || self.active().is_none() {
            diagnostics.push(ActivityTemplateDiagnostic {
                path: String::new(),
                reason: ActivityTemplateRefusal::NoActiveGeneration,
                unavailable_source_hooks: Vec::new(),
            });
            return (Vec::new(), diagnostics);
        }
        let mut ordered: Vec<_> = candidates.iter().collect();
        ordered.sort_by(|a, b| a.path.cmp(&b.path));
        for candidate in ordered {
            match self.qualify_activity_template(parent, room, content_revision, candidate) {
                Ok(template) => templates.push(template),
                Err(reason) => {
                    diagnostics.push(ActivityTemplateDiagnostic::rejected(candidate, reason))
                }
            }
        }
        (templates, diagnostics)
    }

    pub fn qualify_activity_template(
        &self,
        parent: &QualifiedNativeEntryRoom,
        room: &QualifiedNativeEntryRoom,
        content_revision: &str,
        candidate: &BossLeverCandidate,
    ) -> Result<QualifiedActivityRoomTemplate, ActivityTemplateRefusal> {
        use ActivityTemplateRefusal as R;
        let active = self
            .active()
            .filter(|_| self.is_ready())
            .ok_or(R::NoActiveGeneration)?;
        // A caller cannot move a donor room into the currently loaded map or
        // remove its callbacks while retaining the original path/hash label.
        if !captured_boss_levers()?
            .iter()
            .any(|captured| captured == candidate)
        {
            return Err(R::InvalidSourceCapture);
        }
        if content_revision.is_empty()
            || content_revision.len() > 128
            || !hex_digest(&candidate.sha256)
            || candidate.path.is_empty()
            || candidate.path.starts_with('/')
            || candidate.path.split('/').any(|s| s == ".." || s.is_empty())
            || parent.compiled().server_digest() != active.identity().server_artifact_digest()
            || parent.compiled().client_digest() != active.identity().client_artifact_digest()
            || parent.movement_cells().scope().world_id != room.movement_cells().scope().world_id
            || room.compiled().server_digest() != active.identity().server_artifact_digest()
            || room.compiled().client_digest() != active.identity().client_artifact_digest()
        {
            return Err(R::InvalidSourceCapture);
        }
        let (
            Some(creature),
            Some(boss),
            Some(low),
            Some(high),
            Some(exit),
            Some(minimum),
            Some(level),
            Some(cooldown),
            Some(timeout),
            Some(after),
        ) = (
            candidate.creature_key.as_deref(),
            candidate.boss_position,
            candidate.zone_from,
            candidate.zone_to,
            candidate.exit_position,
            candidate.minimum_players,
            candidate.minimum_level,
            candidate.cooldown_seconds,
            candidate.time_limit_seconds,
            candidate.after_kill_seconds,
        )
        else {
            return Err(R::IncompleteLayout);
        };
        if !candidate.complete_literal_layout
            || candidate.lever_positions.is_empty()
            || candidate.lever_positions.len() > 15
            || candidate.lever_positions.len() != candidate.entry_positions.len()
            || minimum == 0
            || minimum as usize > candidate.entry_positions.len()
            || timeout == 0
            || cooldown == 0
            || low.iter().zip(high).any(|(a, b)| *a > b)
        {
            return Err(R::IncompleteLayout);
        }
        let source = room.source_world();
        let frame = if source.is_some() {
            let frame_document =
                crate::content::project::native_spell_world::channel_source_document(
                    active
                        .native_source_world()
                        .ok_or(R::SourceMapUnavailable)?,
                )
                .map_err(|_| R::SourceMapUnavailable)?;
            let frame: SourceFrameDocument =
                serde_json::from_slice(&frame_document).map_err(|_| R::SourceMapUnavailable)?;
            if frame.native_frame.floor_transform != "native_floor=-source_z"
                || parent.frame_binding().digest() != room.frame_binding().digest()
            {
                return Err(R::SourceMapUnavailable);
            }
            Some(frame.native_frame)
        } else if matches!(
            room.movement_cells().index(),
            crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(_)
        ) {
            None
        } else {
            return Err(R::SourceMapUnavailable);
        };
        // Crystal data/libs/functions/boss_lever.lua:63-76,190-225 keeps literal
        // source positions. Bundle runtime cells retain source x/y/+z; the
        // separately qualified Canary capture uses its own translated x/y/-z.
        let project = |[x, y, z]: [i32; 3]| -> Result<LogicalCell, R> {
            let cell = if let Some(frame) = &frame {
                LogicalCell {
                    x: x.checked_sub(frame.origin_x)
                        .ok_or(R::OutsideQualifiedMap)?,
                    y: y.checked_sub(frame.origin_y)
                        .ok_or(R::OutsideQualifiedMap)?,
                    z: z.checked_neg().ok_or(R::OutsideQualifiedMap)?,
                }
            } else {
                LogicalCell { x, y, z }
            };
            if i16::try_from(cell.z).is_err()
                || source.is_some_and(|source| !source.contains_capture_cell(cell))
            {
                return Err(R::OutsideQualifiedMap);
            }
            Ok(cell)
        };
        let from = project(low)?;
        let to = project(high)?;
        let bounds = [
            from.x,
            from.y,
            to.x,
            to.y,
            from.z.min(to.z),
            from.z.max(to.z),
        ];
        let boss = project(boss)?;
        let exit = project(exit)?;
        let galthen = galthen::exact_closure(candidate);
        if (candidate.source_callback_closure.is_some()
            || candidate.source_exit_teleporter.is_some())
            && !galthen
        {
            return Err(R::SourceHookUnavailable);
        }
        let exit_teleporter = candidate
            .source_exit_teleporter
            .map(project)
            .transpose()?
            .map(|trigger| QualifiedBossExitTeleporter {
                trigger,
                destination: exit,
            });
        if let Some(teleporter) = exit_teleporter
            && (source.is_some()
                || room
                    .movement_cells()
                    .index()
                    .lookup(room.movement_cells().scope(), teleporter.trigger)
                    .ok()
                    != Some(crate::content::CollisionClass::Walkable))
        {
            return Err(R::UnknownPlacement);
        }

        let levers = candidate
            .lever_positions
            .iter()
            .copied()
            .map(project)
            .collect::<Result<Vec<_>, _>>()?;
        let entries = candidate
            .entry_positions
            .iter()
            .copied()
            .map(project)
            .collect::<Result<Vec<_>, _>>()?;
        let inside = |p: LogicalCell| {
            bounds[0] <= p.x
                && p.x <= bounds[2]
                && bounds[1] <= p.y
                && p.y <= bounds[3]
                && bounds[4] <= p.z
                && p.z <= bounds[5]
        };
        if !inside(boss) || entries.iter().any(|p| !inside(*p)) {
            return Err(R::IncompleteLayout);
        }
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
        let native = active
            .native_gameplay()
            .ok_or(R::CreatureClosureUnavailable)?;
        let primal = (creature == super::primal_closure::PRIMAL_MENACE_KEY)
            .then(|| super::primal_closure::qualify(native))
            .flatten();
        let selected = first_dragon::qualify(native, candidate);
        if !candidate.blockers.is_empty()
        if (matches!(
            creature,
            "oteryn:creature.ahau" | "oteryn:creature.mitmah_vanguard"
        ) && !galthen)
            || !candidate.blockers.is_empty()
            || !candidate.callbacks_known
        // Only this exact captured source world proves the entry cleanup empty.
        // A bundle map cannot borrow that proof from a different map capture.
        let spite = source.and_then(|_| super::spite_closure::qualify(native, candidate));
        if candidate.blockers.iter().any(|hook| {
            spite
                .as_ref()
                .is_none_or(|closure| !closure.implements_setup(hook))
        }) || !candidate.callbacks_known
            || unavailable_source_hooks(candidate).any(|hook| {
                primal
                    .as_ref()
                    .is_none_or(|closure| !closure.implements(hook))
                    && selected
                    && spite
                        .as_ref()
                        .is_none_or(|closure| !closure.implements(hook))
            })
        {
            return Err(R::SourceHookUnavailable);
        }
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|record| record.profile.target.key == creature)
            .ok_or(R::CreatureClosureUnavailable)?;
        let fingerprint = native
            .creature_profile_fingerprint(creature)
            .ok_or(R::SourceProfileChanged)?;
        let fingerprint: String = fingerprint
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        if candidate.native_profile_sha256.as_deref() != Some(fingerprint.as_str()) {
            return Err(R::SourceProfileChanged);
        }
        // The native death dispatcher captures its descendants from this exact
        // admitted reward row. A template cannot promote an unadmitted corpse.
        let rewards = native
            .reward_table()
            .row(creature)
            .map_err(|_| R::CreatureClosureUnavailable)?;
        qualify_instance_death_descendant(&record.profile.data, &rewards)?;
        if !super::primal_closure::abilities_closed(record) {
            return Err(R::CreatureClosureUnavailable);
        }
        let first = entries[0];
        let start = crate::content::NativeEntryStart {
            x: first.x,
            y: first.y,
            floor: i16::try_from(first.z).map_err(|_| R::IncompleteLayout)?,
        };
        let view = if let Some(source) = source {
            room.activity_view(
                source
                    .activity_region(bounds)
                    .ok_or(R::OutsideQualifiedMap)?,
                start,
            )
        } else if let Some(teleporter) = exit_teleporter {
            room.bundle_activity_view_with_exit(bounds, start, teleporter.trigger())
        } else {
            room.bundle_activity_view(bounds, start)
        }
        .ok_or(R::SourceMapUnavailable)?;
        let suffix = creature
            .strip_prefix("oteryn:creature.")
            .ok_or(R::InvalidSourceCapture)?;
        Ok(QualifiedActivityRoomTemplate {
            boss_key: candidate.boss_key().ok_or(R::InvalidSourceCapture)?,
            template_key: format!("oteryn:room.boss.{suffix}"),
            template_revision: if candidate.path == first_dragon::PATH {
                format!("selected-{}", candidate.sha256)
            } else {
                format!("crystal-{}", candidate.sha256)
            },
            content_revision: content_revision.into(),
            creature_key: creature.into(),
            source_path: candidate.path.clone(),
            source_sha256: candidate.sha256.clone(),
            room: view,
            source_levers: levers,
            entries,
            boss_position: boss,
            source_exit_position: exit,
            exit_position: exit,
            exit_teleporter,
            minimum_players: minimum,
            minimum_level: level,
            cooldown_seconds: cooldown,
            time_limit_seconds: timeout,
            after_kill_seconds: after,
            activation_sequence: active.activation_sequence(),
            server_digest: active.identity().server_artifact_digest(),
            client_digest: active.identity().client_artifact_digest(),
            activation_frame_binding_digest: parent.frame_binding().digest(),
            spite_source_setup: spite,
        })
    }
}

#[derive(Deserialize)]
struct SourceFrameDocument {
    native_frame: SourceFrame,
}
#[derive(Deserialize)]
struct SourceFrame {
    origin_x: i32,
    origin_y: i32,
    floor_transform: String,
}
/// Crystal creature.cpp:711-721,777-782 still creates the physical corpse for
/// reward bosses. A private reward entitlement does not implement that descendant.
/// The scoped DUR-03 MINT bridge preserves that physical descendant in the
/// Instance. Qualification still requires the exact generation's admitted Item.
fn qualify_instance_death_descendant(
    data: &ProjectV2AuthoringProfileData,
    rewards: &crate::content::creature_reward::CreatureRewardRow,
) -> Result<(), ActivityTemplateRefusal> {
    use ActivityTemplateRefusal as R;
    let ProjectV2AuthoringProfileData::Creature(creature) = data else {
        return Err(R::CreatureClosureUnavailable);
    };
    let details = creature
        .details
        .as_ref()
        .ok_or(R::CreatureClosureUnavailable)?;
    let corpse = details
        .corpse_item
        .as_ref()
        .ok_or(R::CreatureClosureUnavailable)?;
    let reward_corpse = rewards
        .corpse_item
        .as_ref()
        .ok_or(R::CreatureClosureUnavailable)?;
    if reward_corpse.family != "Item"
        || reward_corpse.production_key != corpse.key
        || reward_corpse.revision_ref != corpse.revision
    {
        return Err(R::CreatureClosureUnavailable);
    }
    Ok(())
}

fn hex_digest(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn debug(error: impl std::fmt::Debug) -> String {
        format!("{error:?}")
    }

    fn test_world() -> Result<oteryn_protocol_oteryn::WorldId, String> {
        let mut bytes = [42; 16];
        // Runtime wire identifiers require the UUIDv7 version and RFC variant.
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        oteryn_protocol_oteryn::WorldId::decode(&bytes).map_err(debug)
    }

    #[test]
    fn pinned_activity_maps_retain_active_content_and_independent_source_frames()
    -> Result<(), String> {
        let world = test_world()?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let base = pin.qualified_room();
        for source in [
            PinnedActivitySourceRoom::Primal,
            PinnedActivitySourceRoom::Soulpit,
        ] {
            let room = controller
                .qualify_activity_source_room(base, source)
                .map_err(debug)?;
            assert_eq!(
                room.compiled().server_digest(),
                base.compiled().server_digest()
            );
            assert_eq!(
                room.compiled().client_digest(),
                base.compiled().client_digest()
            );
            assert_ne!(room.frame_binding().digest(), base.frame_binding().digest());
            assert_ne!(room.map_revision_digest(), base.map_revision_digest());
            assert_eq!(
                room.map_revision_digest(),
                crate::content::digest::sha256(source.bytes())
            );
            assert!(room.source_world().is_some());
            let pair = crate::content::native_source_world_carrier::compile(
                base.compiled(),
                source.bytes(),
            )
            .map_err(debug)?;
            let decoded =
                crate::content::native_source_world_carrier::decode(&pair.server, &pair.client)
                    .map_err(debug)?;
            assert_eq!(decoded.source_world, source.bytes());
        }
        assert!(
            ContentActivationController::new()
                .qualify_activity_source_room(base, PinnedActivitySourceRoom::Primal)
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn scoped_corpse_bridge_admits_both_ordinary_and_reward_boss_templates() -> Result<(), String> {
        let world = test_world()?;
        for reward_boss in [false, true] {
            let (controller, _pin) = if reward_boss {
                crate::content::native_gameplay::test_support::activated_reward_boss_fixture(
                    world, None,
                )
            } else {
                crate::content::native_gameplay::test_support::activated_with_loot_tables(
                    world, None,
                )
            };
            let native = controller
                .active()
                .and_then(|generation| generation.native_gameplay())
                .ok_or("active source gameplay")?;
            let record = native
                .creature_profiles()
                .records
                .iter()
                .find(|record| record.profile.target.key == "oteryn:creature.rat")
                .ok_or("source creature record")?;
            let ProjectV2AuthoringProfileData::Creature(creature) = &record.profile.data else {
                return Err("creature authoring profile".into());
            };
            let details = creature.details.as_ref().ok_or("source creature details")?;
            assert_eq!(details.system_eligibility.reward_boss, reward_boss);
            assert!(details.corpse_item.is_some());
            let rewards = native
                .reward_table()
                .row("oteryn:creature.rat")
                .map_err(debug)?;
            assert_eq!(
                qualify_instance_death_descendant(&record.profile.data, &rewards),
                Ok(())
            );
            let mut mismatched = rewards.as_ref().clone();
            mismatched
                .corpse_item
                .as_mut()
                .ok_or("admitted corpse")?
                .production_key = "oteryn:item.unadmitted_corpse".into();
            assert_eq!(
                qualify_instance_death_descendant(&record.profile.data, &mismatched),
                Err(ActivityTemplateRefusal::CreatureClosureUnavailable)
            );
            let mut incomplete = record.profile.data.clone();
            if let ProjectV2AuthoringProfileData::Creature(creature) = &mut incomplete {
                creature.details = None;
            }
            assert_eq!(
                qualify_instance_death_descendant(&incomplete, &rewards),
                Err(ActivityTemplateRefusal::CreatureClosureUnavailable)
            );
        }
        Ok(())
    }

    #[test]
    fn current_qualified_source_map_cannot_activate_any_captured_crystal_room() -> Result<(), String>
    {
        let world = test_world()?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
        let candidates = captured_boss_levers().map_err(debug)?;
        let (templates, diagnostics) = controller.qualify_initial_activity_template(
            pin.qualified_room(),
            pin.qualified_room(),
            "rev-1",
            &candidates,
        );
        assert!(templates.is_empty());
        assert_eq!(diagnostics.len(), candidates.len());
        for candidate in candidates.iter().filter(|candidate| {
            candidate.complete_literal_layout
                && candidate.minimum_players.is_some()
                && candidate.minimum_level.is_some()
                && candidate.cooldown_seconds.is_some()
                && candidate.time_limit_seconds.is_some()
                && candidate.after_kill_seconds.is_some()
        }) {
            assert_eq!(
                diagnostics
                    .iter()
                    .find(|diagnostic| diagnostic.path == candidate.path)
                    .ok_or("candidate qualification diagnostic")?
                    .reason,
                ActivityTemplateRefusal::OutsideQualifiedMap,
                "{}",
                candidate.path
            );
        }
        let mut forged = candidates
            .iter()
            .find(|candidate| candidate.complete_literal_layout)
            .ok_or("complete captured room")?
            .clone();
        forged.boss_position = Some([5854, 5298, 5]);
        assert!(matches!(
            controller.qualify_activity_template(
                pin.qualified_room(),
                pin.qualified_room(),
                "rev-1",
                &forged
            ),
            Err(ActivityTemplateRefusal::InvalidSourceCapture)
        ));
        Ok(())
    }

    #[test]
    fn primal_hook_diagnostic_names_the_captured_unimplemented_callbacks() -> Result<(), String> {
        let candidates = captured_boss_levers().map_err(debug)?;
        let primal = candidates
            .iter()
            .find(|candidate| candidate.path.ends_with("/the_primal_manace.lua"))
            .ok_or("captured Primal lever")?;
        let diagnostic = ActivityTemplateDiagnostic::rejected(
            primal,
            ActivityTemplateRefusal::SourceHookUnavailable,
        );
        assert_eq!(
            diagnostic.unavailable_source_hooks,
            vec![
                "ThePrimalMenaceDeath",
                "ThePrimeOrdealBossDeath",
                "mType.onThink",
            ]
        );
        let map_rejection = ActivityTemplateDiagnostic::rejected(
            primal,
            ActivityTemplateRefusal::OutsideQualifiedMap,
        );
        assert!(map_rejection.unavailable_source_hooks.is_empty());
        assert_eq!(
            map_rejection.reason,
            ActivityTemplateRefusal::OutsideQualifiedMap
        );
        Ok(())
    }

    #[test]
    fn bounded_activity_map_never_exposes_an_origin_cell_outside_the_authored_zone()
    -> Result<(), String> {
        let world = test_world()?;
        let (_, pin) =
            crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
        let source = pin
            .qualified_room()
            .source_world()
            .ok_or("qualified source world")?;
        let at = LogicalCell { x: 0, y: 0, z: -5 };
        let view = source
            .activity_region([at.x, at.y, at.x, at.y, at.z, at.z])
            .ok_or("authored activity region")?;
        assert_eq!(view.scope(), source.scope());
        assert_eq!(
            view.lookup(view.scope(), at),
            source.lookup(source.scope(), at)
        );
        let outside = LogicalCell { x: at.x + 1, ..at };
        assert!(!view.contains_capture_cell(outside));
        assert!(view.lookup(view.scope(), outside).is_err());
        assert!(view.immutable_tile(view.scope(), outside).is_err());
        assert!(
            view.pending_mutable_placements()
                .iter()
                .all(|placement| placement.cell() == at)
        );
        assert!(
            source
                .activity_region([i32::MIN, 0, 0, 0, -5, -5])
                .is_none()
        );
        Ok(())
    }

    #[test]
    fn m25_exact_crystal_boss_callbacks_and_exit_trigger_are_captured() -> Result<(), String> {
        let rows = captured_boss_levers().map_err(|e| format!("{e:?}"))?;
        for key in ["oteryn:creature.ahau", "oteryn:creature.mitmah_vanguard"] {
            let row = rows
                .iter()
                .find(|row| row.creature_key.as_deref() == Some(key))
                .ok_or("M25 pinned source candidate")?;
            assert!(row.callbacks_known, "{key} exact Crystal callback closure");
            assert_eq!(row.required_callbacks, ["BossLeverOnDeath"]);
            assert!(
                row.blockers.is_empty(),
                "{key} typed supported source hooks"
            );
            assert!(galthen::exact_closure(row));
            let mut altered = row.clone();
            altered
                .source_callback_closure
                .as_mut()
                .ok_or("exact monster source proof")?
                .monster_sha256 = "0".repeat(64);
            assert!(!galthen::exact_closure(&altered));
            altered = row.clone();
            altered.native_profile_sha256 = Some("0".repeat(64));
            assert!(!galthen::exact_closure(&altered));
            altered = row.clone();
            altered.required_callbacks.push("unimplemented".into());
            assert!(!galthen::exact_closure(&altered));
        }
        let ahau = rows
            .iter()
            .find(|row| row.creature_key.as_deref() == Some("oteryn:creature.ahau"))
            .ok_or("Ahau source")?;
        assert_eq!(ahau.source_exit_teleporter, Some([34002, 31706, 10]));
        assert_eq!(ahau.zone_to, Some([34018, 31705, 10]));
        assert_eq!(ahau.exit_position, Some([34036, 31717, 10]));
        let mut altered = ahau.clone();
        altered.source_exit_teleporter = Some([34003, 31706, 10]);
        assert!(!galthen::exact_closure(&altered));
        Ok(())
    }

    #[test]
    fn captured_first_complete_crystal_room_is_preserved_without_spacing_or_callback_loss()
    -> Result<(), String> {
        let candidates = captured_boss_levers().map_err(debug)?;
        let first = candidates
            .iter()
            .find(|candidate| candidate.complete_literal_layout)
            .ok_or("complete captured room")?;
        assert_eq!(
            first.path,
            "data-global/scripts/actions/bosses_levers/faceless_bane.lua"
        );
        assert_eq!(first.entry_positions, vec![[33617, 32567, 13]; 5]);
        assert_eq!(first.cooldown_seconds, Some(72000));
        assert_eq!(first.time_limit_seconds, Some(900));
        assert_eq!(
            first.required_callbacks,
            vec!["dreamCourtsDeath", "facelessHealth", "facelessThink"]
        );
        Ok(())
    }

    #[test]
    fn m26_source_levers_keep_six_soul_rooms_and_canonical_bakragore_cooldown() -> Result<(), String>
    {
        let candidates = captured_boss_levers().map_err(debug)?;
        let soul: Vec<_> = candidates
            .iter()
            .filter(|row| row.path == "data-global/lib/quests/soul_war.lua")
            .collect();
        assert_eq!(soul.len(), 6);
        assert!(soul.windows(2).all(|rows| rows[0].line < rows[1].line));
        for (row, (key, hours)) in soul.iter().zip([
            ("goshnar_s_spite", 20),
            ("goshnar_s_malice", 20),
            ("goshnar_s_greed", 20),
            ("goshnar_s_hatred", 20),
            ("goshnar_s_cruelty", 20),
            ("goshnar_s_megalomania_purple", 72),
        ]) {
            assert_eq!(
                row.creature_key.as_deref(),
                Some(format!("oteryn:creature.{key}").as_str())
            );
            assert!(row.complete_literal_layout);
            assert_eq!(row.minimum_level, Some(250));
            assert_eq!(row.minimum_players, Some(1));
            assert_eq!(row.entry_positions.len(), 5);
            assert_eq!(row.lever_positions.len(), 5);
            assert_eq!(row.cooldown_seconds, Some(hours * 3600));
            assert_eq!(row.time_limit_seconds, Some(1200));
            assert_eq!(row.after_kill_seconds, Some(60));
        }
        for (suffix, seconds) in [
            ("rotten_bakragore.lua", 244800),
            ("rotten_bakragore1.lua", 72000),
        ] {
            let row = candidates
                .iter()
                .find(|row| row.path.ends_with(suffix))
                .ok_or("source Bakragore lever")?;
            assert_eq!(row.cooldown_seconds, Some(seconds));
            assert!(row.complete_literal_layout);
        }
        Ok(())
    }
}
