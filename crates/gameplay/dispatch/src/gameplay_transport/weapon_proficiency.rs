//! PROF-EFFECT-0 native attack/death dispatch owner.
//!
//! The caller freezes the equipped weapon and contributor timestamps with the lethal commit.
//! This owner deduplicates death occurrences, accrues the existing durable track shape, and
//! applies selected attack perks before damage is handed to the combat owner. Checkpoints
//! serialize this state through the existing Character proficiency writer.

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;

use crate::durability::character_proficiency::{
    ProficiencyDefinitions, ResolvedProficiencyDefinition,
};
use oteryn_gameplay_content::content::{
    ImportedWeaponProficiencyData, ProjectV2ProficiencyPerk, ProjectV2ProficiencyThresholdClass,
    import_committed_weapon_proficiency_data,
};
use oteryn_gameplay_domain::domain::weapon_proficiency::{
    ProficiencyKillContributor, ProficiencySelectionShape, ProficiencyThresholdClass,
    proficiency_kill_credit,
};
use sha2::{Digest, Sha256};

#[path = "weapon_proficiency_shaping.rs"]
mod shaping;
pub use shaping::ProductionProficiencyShaping;

impl super::ComposedFreshAdmission<'_, '_, '_> {
    /// Equipment-owner reads initialise committed tracks even if the player never attacks.
    pub(super) async fn load_proficiency_equipment(
        &self,
        session: crate::foundation::GameSessionId,
        equipment: &crate::durability::character_equipment::EquipmentSnapshot,
    ) -> Result<(), ()> {
        let Some(catalogue) = &self.proficiency_catalogue else {
            return Ok(());
        };
        let character =
            crate::domain::CharacterId::from_bytes(equipment.character).map_err(|_| ())?;
        let (tracks, _) = self
            .root
            .read_character_proficiency_with_definitions(
                self.character,
                character,
                None,
                Arc::clone(catalogue) as Arc<dyn ProficiencyDefinitions>,
            )
            .await
            .map_err(|_| ())?;
        let mut states = self.states.proficiencies.lock().map_err(|_| ())?;
        let owner = states.entry(session).or_default();
        for stored in tracks {
            if let Some(track) = catalogue.live_track(&stored) {
                owner.install_track(stored.state.item_key(), track);
            }
        }
        Ok(())
    }

    /// The caller holds the spell lane and runtime, just as for other equipment effects.
    /// Death capture consumes this owner projection independently of the swing cache.
    pub(super) async fn install_proficiency_equipment(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
        actor: crate::foundation::ExactActorRef,
        session: crate::foundation::GameSessionId,
        equipment: &crate::durability::character_equipment::EquipmentSnapshot,
    ) -> Option<()> {
        let Some(catalogue) = &self.proficiency_catalogue else {
            return Some(());
        };
        if runtime
            .bound_attacker_lease(actor, session)
            .ok()?
            .character_id()
            .as_bytes()
            != &equipment.character
            || equipment.content_digest != runtime.content_pin().server_artifact_digest()
        {
            return None;
        }
        let key = equipment
            .items
            .iter()
            .filter(|item| matches!(item.slot, 5 | 6 | 10))
            .find_map(|item| {
                catalogue
                    .resolve_weapon(&item.definition.production_key)
                    .map(|resolved| resolved.canonical_item_key)
            });
        {
            let mut states = self.states.proficiencies.lock().ok()?;
            let owner = states.entry(session).or_default();
            if let Some(key) = &key
                && owner.track(key).is_none()
            {
                owner.install_track(key, catalogue.empty_track(key)?);
            }
        }
        self.states
            .attack
            .lock()
            .await
            .kills()
            .proficiency_equipment(actor, session, equipment.revision, key.as_deref());
        Some(())
    }
}

/// Immutable production adapter over the promoted proficiency catalogue.
///
/// Construction validates the committed shards and bindings through the content importer.  The
/// durability writer therefore resolves requests against repository-owned content rather than
/// trusting definition identities or shapes supplied by a gameplay caller.
#[derive(Debug)]
pub struct ProductionProficiencyCatalogue {
    data: Arc<ImportedWeaponProficiencyData>,
    digest: [u8; 32],
    content_revision: String,
}

impl ProductionProficiencyCatalogue {
    /// Bind the immutable promoted catalogue to the admission's selected progression content.
    pub(super) fn for_content(&self, content_revision: &str) -> Arc<Self> {
        Arc::new(Self {
            data: Arc::clone(&self.data),
            digest: self.digest,
            content_revision: content_revision.to_owned(),
        })
    }
    pub(super) fn empty_track(&self, item_key: &str) -> Option<LiveProficiencyTrack> {
        let shape = self.resolved(item_key)?.shape;
        Some(LiveProficiencyTrack {
            shape,
            progress: 0,
            selections: vec![None; usize::from(shape.track_shape().perk_levels())],
            effects: ProficiencyAttackEffects {
                denominator: 1,
                ..Default::default()
            },
        })
    }
    pub fn embedded() -> Option<Arc<Self>> {
        let data = import_committed_weapon_proficiency_data().ok()?;
        let encoded = serde_json::to_vec(&data).ok()?;
        Some(Arc::new(Self {
            data: Arc::new(data),
            digest: Sha256::digest(encoded).into(),
            content_revision: "weapon-proficiency-progression-r1".to_owned(),
        }))
    }

    fn resolved(&self, item_key: &str) -> Option<ResolvedProficiencyDefinition> {
        let binding = self.data.bindings().get(item_key)?;
        let definition = self.data.definitions().get(&binding.profile_binding.key)?;
        let threshold = match binding.threshold_class {
            ProjectV2ProficiencyThresholdClass::Standard => ProficiencyThresholdClass::Standard,
            ProjectV2ProficiencyThresholdClass::Knight => ProficiencyThresholdClass::Knight,
            ProjectV2ProficiencyThresholdClass::Crossbow => ProficiencyThresholdClass::Crossbow,
        };
        let counts = definition
            .levels
            .iter()
            .map(|level| u8::try_from(level.perks.len()).ok())
            .collect::<Option<Vec<_>>>()?;
        Some(ResolvedProficiencyDefinition {
            canonical_item_key: binding.item.key.clone(),
            definition_key: definition.identity.key.clone(),
            definition_revision: definition.identity.revision.clone(),
            shape: ProficiencySelectionShape::new(threshold, &counts).ok()?,
        })
    }

    /// Resolve a retained durable track into the selected attack effects from the same promoted
    /// definition that supplied its shape. Unknown, locked or malformed selections fail closed.
    pub fn live_track(
        &self,
        stored: &crate::durability::character_proficiency::StoredProficiencyTrack,
    ) -> Option<LiveProficiencyTrack> {
        let resolved = self.resolved(stored.state.item_key())?;
        if stored.state.definition_key() != resolved.definition_key
            || stored.state.definition_revision() != resolved.definition_revision
        {
            return None;
        }
        let progress = u32::try_from(stored.state.progress()).ok()?;
        let selections = stored.state.selections().to_vec();
        let definition = self.data.definitions().get(&resolved.definition_key)?;
        let mut effects = ProficiencyAttackEffects {
            denominator: 1,
            ..Default::default()
        };
        for selected in resolved
            .shape
            .active_selections(progress, &selections)
            .ok()?
        {
            let perk = definition
                .levels
                .get(usize::from(selected.level))?
                .perks
                .get(usize::from(selected.perk_index))?;
            match perk {
                ProjectV2ProficiencyPerk::AttackDamage { value } => {
                    if value.denominator == 0 || value.numerator < 0 {
                        return None;
                    }
                    effects.flat_attack = effects
                        .flat_attack
                        .checked_add(value.numerator / i64::try_from(value.denominator).ok()?)?;
                }
                ProjectV2ProficiencyPerk::AlphaStrikeDamage { value } => {
                    effects.add_fraction(*value, AttackFraction::Alpha)?;
                }
                ProjectV2ProficiencyPerk::OmegaStrikeDamage { value } => {
                    effects.add_fraction(*value, AttackFraction::Omega)?;
                }
                _ => {}
            }
        }
        Some(LiveProficiencyTrack {
            shape: resolved.shape,
            progress,
            selections,
            effects,
        })
    }
}

impl ProficiencyDefinitions for ProductionProficiencyCatalogue {
    fn content_revision(&self) -> &str {
        &self.content_revision
    }

    fn digest(&self) -> [u8; 32] {
        self.digest
    }

    fn active_weapon_count(&self) -> usize {
        self.data.bindings().len()
    }

    fn resolve_weapon(&self, item_key: &str) -> Option<ResolvedProficiencyDefinition> {
        self.resolved(item_key)
    }

    fn retained_definition(
        &self,
        item_key: &str,
        definition_key: &str,
        revision: &str,
    ) -> Option<ProficiencySelectionShape> {
        let resolved = self.resolved(item_key)?;
        (resolved.definition_key == definition_key && resolved.definition_revision == revision)
            .then_some(resolved.shape)
    }
}

/// Exact aggregate of selected perks read by the existing combat/stat stages.
/// Fractions use one denominator and are truncated once after summation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProficiencyAttackEffects {
    pub flat_attack: i64,
    pub damage_numerator: i64,
    pub denominator: i64,
    pub alpha_numerator: i64,
    pub omega_numerator: i64,
}

#[derive(Debug, Clone, Copy)]
enum AttackFraction {
    Alpha,
    Omega,
}

impl ProficiencyAttackEffects {
    fn add_fraction(
        &mut self,
        value: oteryn_gameplay_content::content::ProjectV2ExactRatio,
        target: AttackFraction,
    ) -> Option<()> {
        let denominator = i64::try_from(value.denominator).ok()?;
        if denominator <= 0 || value.numerator < 0 {
            return None;
        }
        let common = self.denominator.checked_mul(denominator)?;
        self.damage_numerator = self.damage_numerator.checked_mul(denominator)?;
        self.alpha_numerator = self.alpha_numerator.checked_mul(denominator)?;
        self.omega_numerator = self.omega_numerator.checked_mul(denominator)?;
        let scaled = value.numerator.checked_mul(self.denominator)?;
        match target {
            AttackFraction::Alpha => {
                self.alpha_numerator = self.alpha_numerator.checked_add(scaled)?
            }
            AttackFraction::Omega => {
                self.omega_numerator = self.omega_numerator.checked_add(scaled)?
            }
        }
        self.denominator = common;
        Some(())
    }

    fn valid(self) -> bool {
        self.denominator > 0
            && self.damage_numerator >= 0
            && self.alpha_numerator >= 0
            && self.omega_numerator >= 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveProficiencyTrack {
    pub shape: ProficiencySelectionShape,
    pub progress: u32,
    pub selections: Vec<Option<u8>>,
    pub effects: ProficiencyAttackEffects,
}

/// Session-local state. `seen_deaths` implements PROF-EFFECT0-RL-01; both it and
/// uncheckpointed progress disappear on restart by design. `modification_cap` is the session's
/// `PROF1B-RL-03` window.
#[derive(Debug, Default)]
pub struct WeaponProficiencyDispatch {
    pub(super) tracks: BTreeMap<String, LiveProficiencyTrack>,
    pub(super) pending_progress: BTreeMap<String, u32>,
    pub(super) dirty_since: Option<u64>,
    pub(super) checkpoint: Option<super::proficiency_checkpoint::ProficiencyCheckpoint>,
    pub(super) modification_cap:
        oteryn_gameplay_domain::domain::weapon_proficiency::ProficiencyModificationRateCap,
    seen_deaths: VecDeque<[u8; 16]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KillDispatch<'a> {
    pub death_occurrence: [u8; 16],
    pub death_at_ms: u64,
    pub creature_is_summon: bool,
    pub points: Option<u32>,
    pub contributor: ProficiencyKillContributor<'a>,
}

impl WeaponProficiencyDispatch {
    /// Refresh committed values while retaining this session's uncheckpointed delta.
    pub fn install_track(&mut self, item_key: impl Into<String>, mut track: LiveProficiencyTrack) {
        let item_key = item_key.into();
        // Until reconciliation acknowledges a retained receipt, a durable refresh may already
        // contain its delta. Keep the live predecessor rather than adding that delta twice.
        if self.checkpoint.is_some() && self.tracks.contains_key(&item_key) {
            return;
        }
        let pending = self.pending_progress.get(&item_key).copied().unwrap_or(0);
        if let Some(existing) = self.tracks.get(&item_key) {
            track.progress = track
                .progress
                .max(existing.progress.saturating_sub(pending));
        }
        if let Some(delta) = self.pending_progress.get(&item_key) {
            track.progress = track.shape.track_shape().credited(track.progress, *delta);
        }
        self.tracks.insert(item_key, track);
    }

    pub fn track(&self, item_key: &str) -> Option<&LiveProficiencyTrack> {
        self.tracks.get(item_key)
    }

    /// Death-dispatch entry point. Duplicate occurrences and absent tracks are no-ops.
    pub fn dispatch_kill(&mut self, input: KillDispatch<'_>) -> Option<u32> {
        if self.seen_deaths.contains(&input.death_occurrence) {
            return None;
        }
        let credit = proficiency_kill_credit(
            input.death_at_ms,
            input.creature_is_summon,
            input.points,
            input.contributor,
        )?;
        let track = self.tracks.get_mut(credit.weapon_item_key)?;
        // PROFEFF0-RL-01 retains the latest 64 occurrences in arrival order.
        if self.seen_deaths.len() == 64 {
            self.seen_deaths.pop_front();
        }
        self.seen_deaths.push_back(input.death_occurrence);
        let before = track.progress;
        track.progress = track
            .shape
            .track_shape()
            .credited(track.progress, credit.points);
        let delta = track.progress - before;
        if delta != 0 {
            self.dirty_since
                .get_or_insert(input.death_at_ms.saturating_mul(1000));
            let pending = self
                .pending_progress
                .entry(credit.weapon_item_key.to_owned())
                .or_default();
            *pending = pending.saturating_add(delta);
        }
        Some(track.progress)
    }

    /// Auto-attack dispatch entry point. Alpha is strictly above 95% health and Omega is
    /// strictly below 30%; percentage contributions are summed and truncated once.
    pub fn dispatch_attack(
        &self,
        equipped_weapon_item_key: &str,
        base_damage: i64,
        target_health: u64,
        target_max_health: u64,
    ) -> i64 {
        let Some(track) = self.tracks.get(equipped_weapon_item_key) else {
            return base_damage;
        };
        let effects = track.effects;
        let has_active_selection = track
            .shape
            .active_selections(track.progress, &track.selections)
            .is_ok_and(|selections| !selections.is_empty());
        if !has_active_selection || !effects.valid() || base_damage <= 0 || target_max_health == 0 {
            return base_damage;
        }
        let mut numerator = effects.damage_numerator;
        if u128::from(target_health) * 100 > u128::from(target_max_health) * 95 {
            numerator = numerator.saturating_add(effects.alpha_numerator);
        }
        if u128::from(target_health) * 100 < u128::from(target_max_health) * 30 {
            numerator = numerator.saturating_add(effects.omega_numerator);
        }
        base_damage
            .saturating_add(effects.flat_attack)
            .saturating_add(base_damage.saturating_mul(numerator) / effects.denominator)
            .max(0)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures require valid tracks")]
#[path = "weapon_proficiency_refresh_tests.rs"]
mod refresh_tests;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use oteryn_gameplay_domain::domain::weapon_proficiency::ProficiencyThresholdClass;

    fn track() -> LiveProficiencyTrack {
        LiveProficiencyTrack {
            shape: ProficiencySelectionShape::new(
                ProficiencyThresholdClass::Standard,
                &[1, 1, 1, 1, 1, 1, 1],
            )
            .expect("shape"),
            progress: 1_750,
            selections: vec![Some(0), None, None, None, None, None, None],
            effects: ProficiencyAttackEffects {
                flat_attack: 2,
                damage_numerator: 5,
                denominator: 100,
                alpha_numerator: 10,
                omega_numerator: 20,
            },
        }
    }

    #[test]
    fn production_catalogue_resolves_promoted_weapon_for_durable_writer() {
        let catalogue = ProductionProficiencyCatalogue::embedded().expect("committed catalogue");
        let binding = catalogue.data.bindings().values().next().expect("binding");
        let resolved = catalogue
            .resolve_weapon(&binding.item.key)
            .expect("promoted definition");
        assert_eq!(resolved.canonical_item_key, binding.item.key);
        assert_eq!(
            catalogue.retained_definition(
                &resolved.canonical_item_key,
                &resolved.definition_key,
                &resolved.definition_revision,
            ),
            Some(resolved.shape)
        );
    }

    #[test]
    fn promoted_selected_attack_perk_reaches_live_dispatch() {
        use crate::domain::CharacterRevision;
        use crate::durability::character_proficiency::{
            DurableProficiencyState, StoredProficiencyTrack,
        };

        let catalogue = ProductionProficiencyCatalogue::embedded().expect("committed catalogue");
        let stored = StoredProficiencyTrack {
            state: DurableProficiencyState::new(
                "oteryn:item.tibia.i43864",
                "oteryn:proficiency.tibia.p6",
                "definition-r1",
                2_000_000,
                vec![None, None, None, None, Some(2), None, None],
            )
            .expect("durable track"),
            committed_character_revision: CharacterRevision::new(7).expect("revision"),
            last_proficiency_occurrence_id: [0; 16],
        };
        let live = catalogue.live_track(&stored).expect("resolved live track");
        assert_eq!(live.effects.flat_attack, 1);
        let mut owner = WeaponProficiencyDispatch::default();
        owner.install_track(stored.state.item_key(), live);
        assert_eq!(
            owner.dispatch_attack(stored.state.item_key(), 100, 50, 100),
            101
        );
    }

    #[test]
    fn real_kill_dispatch_credits_every_recent_contributor_full_points_once() {
        let mut first = WeaponProficiencyDispatch::default();
        let mut party_peer = WeaponProficiencyDispatch::default();
        first.install_track("weapon", track());
        party_peer.install_track("weapon", track());
        let input = |last_damage_at_ms| KillDispatch {
            death_occurrence: [7; 16],
            death_at_ms: 400_000,
            creature_is_summon: false,
            points: Some(100),
            contributor: ProficiencyKillContributor {
                character_in_channel: true,
                last_damage_at_ms,
                equipped_weapon_item_key: Some("weapon"),
                weapon_has_promoted_proficiency: true,
            },
        };
        assert_eq!(first.dispatch_kill(input(399_999)), Some(1_850));
        assert_eq!(party_peer.dispatch_kill(input(100_000)), Some(1_850));
        assert_eq!(first.dispatch_kill(input(399_999)), None);
        assert_eq!(first.track("weapon").expect("track").progress, 1_850);
    }

    #[test]
    fn real_attack_dispatch_applies_selected_threshold_effects_once() {
        let mut owner = WeaponProficiencyDispatch::default();
        owner.install_track("weapon", track());
        assert_eq!(owner.dispatch_attack("weapon", 100, 96, 100), 117);
        assert_eq!(owner.dispatch_attack("weapon", 100, 95, 100), 107);
        assert_eq!(owner.dispatch_attack("weapon", 100, 29, 100), 127);
        assert_eq!(owner.dispatch_attack("weapon", 100, 30, 100), 107);
    }
}
