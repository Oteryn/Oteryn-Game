//! Boss room views issued by the existing Content activation owner.
//!
//! Source capture is inert authoring. A template requires the current artifact,
//! its already qualified map, a complete native creature closure and every
//! source hook. Unknown map cells or custom hooks never become a vanilla boss.
use crate::content::{
    ActiveGeneration, ContentActivationController, LogicalCell, QualifiedNativeEntryRoom,
    channel_content_pin::ChannelContentPin, digest::sha256, project::ProjectV2AuthoringProfileData,
};
use serde::{Deserialize, Serialize};

pub const CRYSTAL_REVISION: &str = "201189f871d23597b82caf88fd611454a2ddefb8";

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityTemplateDiagnostic {
    pub path: String,
    pub reason: ActivityTemplateRefusal,
}

/// Produced only by the active Content owner, never by a client or SQL catalogue
/// row. Its bounded room keeps the original frame/map/artifact identity and only
/// exposes the admitted encounter region. Entry/exit facts retain authored order.
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
    exit_position: LogicalCell,
    minimum_players: u32,
    minimum_level: u32,
    cooldown_seconds: u32,
    time_limit_seconds: u32,
    after_kill_seconds: u32,
    activation_sequence: u64,
    server_digest: [u8; 32],
    client_digest: [u8; 32],
}

impl QualifiedActivityRoomTemplate {
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
    pub fn exit_position(&self) -> LogicalCell {
        self.exit_position
    }
    pub fn minimum_players(&self) -> u32 {
        self.minimum_players
    }
    pub fn maximum_players(&self) -> usize {
        self.entries.len()
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
        || document.records.windows(2).any(|w| w[0].path >= w[1].path)
    {
        return Err(ActivityTemplateRefusal::InvalidSourceCapture);
    }
    Ok(document.records)
}

impl ContentActivationController {
    /// Deterministic initial selection authorized by PR2769 ANSWER6543189919.
    /// Every earlier refusal is retained; only the first complete qualified
    /// template can be registered by the gameplay seam.
    pub fn qualify_initial_activity_template(
        &self,
        room: &QualifiedNativeEntryRoom,
        content_revision: &str,
        candidates: &[BossLeverCandidate],
    ) -> (
        Vec<QualifiedActivityRoomTemplate>,
        Vec<ActivityTemplateDiagnostic>,
    ) {
        let mut diagnostics = Vec::new();
        if !self.is_ready() || self.active().is_none() {
            diagnostics.push(ActivityTemplateDiagnostic {
                path: String::new(),
                reason: ActivityTemplateRefusal::NoActiveGeneration,
            });
            return (Vec::new(), diagnostics);
        }
        let mut ordered: Vec<_> = candidates.iter().collect();
        ordered.sort_by(|a, b| a.path.cmp(&b.path));
        for candidate in ordered {
            match self.qualify_activity_template(room, content_revision, candidate) {
                Ok(template) => return (vec![template], diagnostics),
                Err(reason) => diagnostics.push(ActivityTemplateDiagnostic {
                    path: candidate.path.clone(),
                    reason,
                }),
            }
        }
        (Vec::new(), diagnostics)
    }

    pub fn qualify_activity_template(
        &self,
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
        let source = room.source_world().ok_or(R::SourceMapUnavailable)?;
        let frame: SourceFrameDocument = serde_json::from_slice(
            active
                .native_source_world()
                .ok_or(R::SourceMapUnavailable)?,
        )
        .map_err(|_| R::SourceMapUnavailable)?;
        if frame.native_frame.floor_transform != "native_floor=-source_z" {
            return Err(R::SourceMapUnavailable);
        }
        let project = |[x, y, z]: [i32; 3]| -> Result<LogicalCell, R> {
            let cell = LogicalCell {
                x: x.checked_sub(frame.native_frame.origin_x)
                    .ok_or(R::OutsideQualifiedMap)?,
                y: y.checked_sub(frame.native_frame.origin_y)
                    .ok_or(R::OutsideQualifiedMap)?,
                z: z.checked_neg().ok_or(R::OutsideQualifiedMap)?,
            };
            if i16::try_from(cell.z).is_err() || !source.contains_capture_cell(cell) {
                return Err(R::OutsideQualifiedMap);
            }
            Ok(cell)
        };
        let from = project(low)?;
        let to = project(high)?;
        let bounds = [from.x, from.y, to.x, to.y, to.z, from.z];
        let boss = project(boss)?;
        let exit = project(exit)?;
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
            if source.lookup(source.scope(), position).ok()
                != Some(crate::content::CollisionClass::Walkable)
            {
                return Err(R::UnknownPlacement);
            }
        }
        if !candidate.blockers.is_empty()
            || !candidate.callbacks_known
            || candidate
                .required_callbacks
                .iter()
                .any(|hook| hook != "BossLeverOnDeath")
        {
            return Err(R::SourceHookUnavailable);
        }
        let native = active
            .native_gameplay()
            .ok_or(R::CreatureClosureUnavailable)?;
        let record = native
            .creature_profiles()
            .records
            .iter()
            .find(|record| record.profile.target.key == creature)
            .ok_or(R::CreatureClosureUnavailable)?;
        let value = serde_json::to_value(record).map_err(|_| R::SourceProfileChanged)?;
        let fingerprint = sha256(&serde_json::to_vec(&value).map_err(|_| R::SourceProfileChanged)?);
        let fingerprint: String = fingerprint
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        if candidate.native_profile_sha256.as_deref() != Some(fingerprint.as_str()) {
            return Err(R::SourceProfileChanged);
        }
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
        let cropped = source
            .activity_region(bounds)
            .ok_or(R::OutsideQualifiedMap)?;
        let first = entries[0];
        let start = crate::content::NativeEntryStart {
            x: first.x,
            y: first.y,
            floor: i16::try_from(first.z).map_err(|_| R::IncompleteLayout)?,
        };
        let view = room
            .activity_view(cropped, start)
            .ok_or(R::SourceMapUnavailable)?;
        let suffix = creature
            .strip_prefix("oteryn:creature.")
            .ok_or(R::InvalidSourceCapture)?;
        Ok(QualifiedActivityRoomTemplate {
            boss_key: format!("oteryn:boss.{suffix}"),
            template_key: format!("oteryn:room.boss.{suffix}"),
            template_revision: format!("crystal-{}", candidate.sha256),
            content_revision: content_revision.into(),
            creature_key: creature.into(),
            source_path: candidate.path.clone(),
            source_sha256: candidate.sha256.clone(),
            room: view,
            source_levers: levers,
            entries,
            boss_position: boss,
            exit_position: exit,
            minimum_players: minimum,
            minimum_level: level,
            cooldown_seconds: cooldown,
            time_limit_seconds: timeout,
            after_kill_seconds: after,
            activation_sequence: active.activation_sequence(),
            server_digest: active.identity().server_artifact_digest(),
            client_digest: active.identity().client_artifact_digest(),
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
fn hex_digest(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn current_qualified_source_map_cannot_activate_any_captured_crystal_room() {
        let world = oteryn_protocol_oteryn::WorldId::decode(&[42; 16]).unwrap();
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
        let candidates = captured_boss_levers().unwrap();
        let (templates, diagnostics) = controller.qualify_initial_activity_template(
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
                    .unwrap()
                    .reason,
                ActivityTemplateRefusal::OutsideQualifiedMap,
                "{}",
                candidate.path
            );
        }
        let mut forged = candidates
            .iter()
            .find(|candidate| candidate.complete_literal_layout)
            .unwrap()
            .clone();
        forged.boss_position = Some([5854, 5298, 5]);
        assert!(matches!(
            controller.qualify_activity_template(pin.qualified_room(), "rev-1", &forged),
            Err(ActivityTemplateRefusal::InvalidSourceCapture)
        ));
    }

    #[test]
    fn bounded_activity_map_never_exposes_an_origin_cell_outside_the_authored_zone() {
        let world = oteryn_protocol_oteryn::WorldId::decode(&[42; 16]).unwrap();
        let (_, pin) =
            crate::content::native_gameplay::test_support::activated_loop_monster(world, false);
        let source = pin.qualified_room().source_world().unwrap();
        let at = LogicalCell { x: 0, y: 0, z: -5 };
        let view = source
            .activity_region([at.x, at.y, at.x, at.y, at.z, at.z])
            .unwrap();
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
    }

    #[test]
    fn captured_first_complete_crystal_room_is_preserved_without_spacing_or_callback_loss() {
        let candidates = captured_boss_levers().unwrap();
        let first = candidates
            .iter()
            .find(|candidate| candidate.complete_literal_layout)
            .unwrap();
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
    }
}
