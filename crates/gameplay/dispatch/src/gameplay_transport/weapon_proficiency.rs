//! PROF-EFFECT-0 native attack/death dispatch owner.
//!
//! The caller freezes the equipped weapon and contributor timestamps with the lethal commit.
//! This owner deduplicates death occurrences, accrues the existing durable track shape, and
//! applies selected attack perks before damage is handed to the combat owner. Checkpoints
//! serialize this state through the existing Character proficiency writer.

use std::collections::{BTreeMap, BTreeSet};
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

use super::{ComposedFreshAdmission, capabilities::OfferedCapability, connection::AdmittedSession};
use oteryn_protocol_oteryn::weapon_proficiency as wire;

pub(super) fn offered(base: &[OfferedCapability], available: bool) -> Vec<OfferedCapability> {
    let mut result: Vec<_> = base
        .iter()
        .copied()
        .filter(|entry| entry.id != wire::CAPABILITY_WEAPON_PROFICIENCY_V1)
        .collect();
    if available {
        let at = result.partition_point(|entry| entry.id < wire::CAPABILITY_WEAPON_PROFICIENCY_V1);
        result.insert(
            at,
            OfferedCapability {
                id: wire::CAPABILITY_WEAPON_PROFICIENCY_V1,
                requires: &[],
            },
        );
    }
    result
}

macro_rules! authority_methods {
    () => {
        fn weapon_proficiency_available(&self) -> bool {
            false
        }
        fn weapon_proficiency_command(
            &self,
            _admitted: $crate::gameplay_transport::connection::AdmittedSession,
            _request: oteryn_protocol_oteryn::weapon_proficiency::Request,
        ) -> impl std::future::Future<Output = Option<Vec<u8>>> {
            async { None }
        }
    };
}
pub(super) use authority_methods;

macro_rules! composed_methods {
    () => {
        fn weapon_proficiency_available(&self) -> bool {
            self.bundle_map().is_some() && self.proficiency_catalogue.is_some()
        }
        async fn weapon_proficiency_command(
            &self,
            admitted: connection::AdmittedSession,
            request: oteryn_protocol_oteryn::weapon_proficiency::Request,
        ) -> Option<Vec<u8>> {
            self.weapon_proficiency_wire(admitted, request).await
        }
    };
}
pub(super) use composed_methods;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn weapon_proficiency_wire(
        &self,
        admitted: AdmittedSession,
        request: wire::Request,
    ) -> Option<Vec<u8>> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(wire::CAPABILITY_WEAPON_PROFICIENCY_V1)
        {
            return None;
        }
        let fence = admitted.item_fence?;
        if fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return None;
        }
        // WEAPPROF4 step 1 deliberately exposes the inert durable query first. Mutations are
        // decoded and reach this owner, but remain typed refusals until their atomic writers are
        // composed in the next step; no partial or session-local mutation is possible.
        let wire::Request::Query = request else {
            let revision = match request {
                wire::Request::Select { track_revision, .. }
                | wire::Request::Refine { track_revision, .. }
                | wire::Request::Reshape { track_revision, .. } => track_revision,
                wire::Request::Query => unreachable!(),
            };
            return wire::encode_result(1, revision, &[]);
        };
        let catalogue = self.proficiency_catalogue.as_ref()?;
        let (tracks, _) = self
            .root
            .read_character_proficiency_with_definitions(
                self.character,
                fence.character_id,
                None,
                std::sync::Arc::clone(catalogue)
                    as std::sync::Arc<
                        dyn crate::durability::character_proficiency::ProficiencyDefinitions,
                    >,
            )
            .await
            .ok()?;
        let tracks = tracks
            .into_iter()
            .map(|track| wire::Track {
                item_key: track.state.item_key().to_owned(),
                revision: track.committed_character_revision.get(),
                progress: track.state.progress(),
                selections: track.state.selections().to_vec(),
            })
            .collect::<Vec<_>>();
        let body = wire::encode_tracks(&tracks)?;
        wire::encode_result(0, 0, &body)
    }
}

/// Immutable production adapter over the promoted proficiency catalogue.
///
/// Construction validates the committed shards and bindings through the content importer.  The
/// durability writer therefore resolves requests against repository-owned content rather than
/// trusting definition identities or shapes supplied by a gameplay caller.
#[derive(Debug)]
pub struct ProductionProficiencyCatalogue {
    data: ImportedWeaponProficiencyData,
    digest: [u8; 32],
}

impl ProductionProficiencyCatalogue {
    pub fn embedded() -> Option<Arc<Self>> {
        let data = import_committed_weapon_proficiency_data().ok()?;
        let encoded = serde_json::to_vec(&data).ok()?;
        Some(Arc::new(Self {
            data,
            digest: Sha256::digest(encoded).into(),
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
        "weapon-proficiency-progression-r1"
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
/// uncheckpointed progress disappear on restart by design.
#[derive(Debug, Default)]
pub struct WeaponProficiencyDispatch {
    tracks: BTreeMap<String, LiveProficiencyTrack>,
    seen_deaths: BTreeSet<[u8; 16]>,
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
    /// Install a track loaded by the existing durable proficiency reader.
    pub fn install_track(&mut self, item_key: impl Into<String>, track: LiveProficiencyTrack) {
        self.tracks.insert(item_key.into(), track);
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
        self.seen_deaths.insert(input.death_occurrence);
        track.progress = track
            .shape
            .track_shape()
            .credited(track.progress, credit.points);
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
