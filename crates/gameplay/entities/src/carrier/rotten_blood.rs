//! Exact Rotten native source children under the existing Instance physical owner.
use super::*;

const MURCION: &str = "oteryn:creature.murcion";
const ICHGAHAL: &str = "oteryn:creature.ichgahal";
const VEMIATH: &str = "oteryn:creature.vemiath";
const CHAGORZ: &str = "oteryn:creature.chagorz";
const BAKRAGORE: &str = "oteryn:creature.bakragore";

/// A source continuation retained by the Instance callback owner. Constructed
/// from a registered source parent; it is not an actor reference or a placement grant.
#[derive(Debug)]
pub struct RottenBloodSpawnSource {
    stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
    content: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
    family: &'static str,
}
fn rotten_blood_child_allowed(parent: &str, child: &str) -> bool {
    match parent {
        MURCION | ICHGAHAL => matches!(
            child,
            "oteryn:creature.mushroom" | "oteryn:creature.elder_bloodjaw"
        ),
        VEMIATH | CHAGORZ => matches!(
            child,
            "oteryn:creature.pillar_of_dark_energy" | "oteryn:creature.elder_bloodjaw"
        ),
        BAKRAGORE => matches!(
            child,
            "oteryn:creature.mushroom"
                | "oteryn:creature.elder_bloodjaw"
                | "oteryn:creature.pillar_of_dark_energy"
                | "oteryn:creature.echo_of_murcion"
                | "oteryn:creature.echo_of_ichgahal"
                | "oteryn:creature.echo_of_chagorz"
                | "oteryn:creature.echo_of_vemiath"
        ),
        _ => false,
    }
}

impl ChannelRuntimeV1 {
    fn require_rotten_blood_scope(&self) -> Result<(), CarrierError> {
        self.owner_fence()?;
        if !matches!(
            self.binding().runtime_scope(),
            RuntimeScopeRefV1::Instance { .. }
        ) {
            return Err(CarrierError::WrongScope);
        }
        Ok(())
    }

    /// Capture the bounded native source closure under the actual Instance
    /// physical owner. The driver separately checks its pinned room and think.
    pub fn qualify_rotten_blood_spawn_source(
        &mut self,
        parent: ExactActorRef,
        native: &oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
    ) -> Result<RottenBloodSpawnSource, CarrierError> {
        self.require_rotten_blood_scope()?;
        if !native.qualifies_rotten_blood_callbacks(self.content_pin().server_artifact_digest()) {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        if !self.source_creature_is_registered(parent)? {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let snapshot = self.companion_snapshot_including_dead(parent)?;
        let family = match snapshot.state.policy.definition_key.as_str() {
            MURCION => MURCION,
            ICHGAHAL => ICHGAHAL,
            VEMIATH => VEMIATH,
            CHAGORZ => CHAGORZ,
            BAKRAGORE => BAKRAGORE,
            _ => return Err(CarrierError::CreatureTargetMismatch),
        };
        if snapshot.state.master.is_some() || self.native_summon_master(parent).is_some() {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        self.assert_actor_spell_unreserved(parent)?;
        let stamp = self.issue_owner_work()?;
        let pin = self.content_pin();
        Ok(RottenBloodSpawnSource {
            stamp,
            family,
            content: pin.server_artifact_digest(),
            client: pin.client_artifact_digest(),
            frame: pin.frame_binding_digest(),
            map: self.map_revision_digest(),
        })
    }

    /// Global Lua callbacks survive their original boss's death, but never their
    /// Instance owner. Current map admission and actual native policies remain mandatory.
    pub fn spawn_rotten_blood_child_from_source(
        &mut self,
        source: &RottenBloodSpawnSource,
        child: &str,
        at: MovementLocalPosition,
        map_admits: impl FnOnce(&ChannelRuntimeV1, MovementLocalPosition) -> Option<bool>,
    ) -> Result<ExactActorRef, EncounterSpawnError> {
        self.require_rotten_blood_scope()
            .map_err(EncounterSpawnError::Native)?;
        if !self
            .positioned_actor_census()
            .map_err(EncounterSpawnError::Native)?
            .iter()
            .any(|(_, _, session)| session.is_some())
        {
            return Err(EncounterSpawnError::Native(
                CarrierError::PlayerReservationMismatch,
            ));
        }
        let pin = self.content_pin();
        if !self
            .owner_fence()
            .map_err(EncounterSpawnError::Native)?
            .accepts_stamp(source.stamp)
            || pin.server_artifact_digest() != source.content
            || pin.client_artifact_digest() != source.client
            || pin.frame_binding_digest() != source.frame
            || self.map_revision_digest() != source.map
            || !rotten_blood_child_allowed(source.family, child)
        {
            return Err(EncounterSpawnError::Native(CarrierError::WrongScope));
        }
        let revision = self
            .companion_policy(child)
            .map_err(EncounterSpawnError::Native)?
            .definition_revision
            .clone();
        self.admit_native_masterless_with_placement(
            child,
            &revision,
            source.content,
            at,
            false,
            map_admits,
        )
    }
}

pub use super::rotten_blood_presentation::RottenBloodIconKey;

#[derive(Debug, Clone, Copy)]
enum IconSourceRole {
    Area,
    Agony,
}

/// Exact source Creature continuation for presentation writes on a live Player.
/// It does not carry HP, a condition, positions or a mutable second actor store.
#[derive(Debug)]
pub struct RottenBloodIconSource {
    parent: ExactActorRef,
    parent_key: String,
    role: IconSourceRole,
    stamp: oteryn_gameplay_foundation::foundation::RuntimeWorkStamp,
    content: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
}

fn icon_role(key: &str) -> Option<IconSourceRole> {
    match key {
        BAKRAGORE | CHAGORZ | VEMIATH => Some(IconSourceRole::Agony),
        // These fifteen bodies call onRottenMonsterThink, verified against the
        // full pinned source witness rather than bestiary classes or names.
        "oteryn:creature.bloated_man_maggot"
        | "oteryn:creature.converter"
        | "oteryn:creature.darklight_construct"
        | "oteryn:creature.darklight_emitter"
        | "oteryn:creature.darklight_matter"
        | "oteryn:creature.darklight_source"
        | "oteryn:creature.darklight_striker"
        | "oteryn:creature.meandering_mushroom"
        | "oteryn:creature.mycobiontic_beetle"
        | "oteryn:creature.oozing_carcass"
        | "oteryn:creature.rotten_man_maggot"
        | "oteryn:creature.sopping_carcass"
        | "oteryn:creature.sopping_corpus"
        | "oteryn:creature.walking_pillar"
        | "oteryn:creature.wandering_pillar" => Some(IconSourceRole::Area),
        _ => None,
    }
}

impl ChannelRuntimeV1 {
    pub fn qualify_rotten_blood_icon_source(
        &mut self,
        parent: ExactActorRef,
        native: &oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
    ) -> Result<RottenBloodIconSource, CarrierError> {
        self.owner_fence()?;
        if !native.qualifies_rotten_blood_callbacks(self.content_pin().server_artifact_digest()) {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let snapshot = self.companion_snapshot(parent)?;
        if snapshot.state.master.is_some()
            || self.native_summon_master(parent).is_some()
            || snapshot.health <= 0
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let role = icon_role(&snapshot.state.policy.definition_key)
            .ok_or(CarrierError::CreatureTargetMismatch)?;
        if matches!(role, IconSourceRole::Agony)
            && !matches!(
                self.binding().runtime_scope(),
                RuntimeScopeRefV1::Instance { .. }
            )
        {
            return Err(CarrierError::WrongScope);
        }
        let parent_key = snapshot.state.policy.definition_key.clone();
        self.assert_actor_spell_unreserved(parent)?;
        let stamp = self.issue_owner_work()?;
        let pin = self.content_pin();
        Ok(RottenBloodIconSource {
            parent,
            parent_key,
            role,
            stamp,
            content: pin.server_artifact_digest(),
            client: pin.client_artifact_digest(),
            frame: pin.frame_binding_digest(),
            map: self.map_revision_digest(),
        })
    }

    /// Existing sealed onThink source for the three bosses' COMBAT_AGONYDAMAGE.
    /// This grants no arbitrary element, magnitude, Item or source actor.
    pub fn rotten_blood_agony_source_current(&self, source: &RottenBloodIconSource) -> bool {
        self.owner_fence()
            .is_ok_and(|fence| fence.accepts_stamp(source.stamp))
            && matches!(source.role, IconSourceRole::Agony)
            && self.content_pin().server_artifact_digest() == source.content
            && self.content_pin().client_artifact_digest() == source.client
            && self.content_pin().frame_binding_digest() == source.frame
            && self.map_revision_digest() == source.map
            && self.companion_snapshot(source.parent).is_ok_and(|parent| {
                parent.health > 0
                    && parent.state.policy.definition_key == source.parent_key
                    && parent.state.master.is_none()
                    && self.native_summon_master(source.parent).is_none()
            })
    }

    pub fn set_rotten_blood_player_icon(
        &mut self,
        source: &RottenBloodIconSource,
        actor: ExactActorRef,
        session: GameSessionId,
        key: RottenBloodIconKey,
        count: Option<u16>,
    ) -> Result<(), CarrierError> {
        let fence = self.owner_fence()?;
        let pin = self.content_pin();
        let role = match key {
            RottenBloodIconKey::AgonyStacks => matches!(source.role, IconSourceRole::Agony),
            _ => matches!(source.role, IconSourceRole::Area),
        };
        if !role
            || !fence.accepts_stamp(source.stamp)
            || pin.server_artifact_digest() != source.content
            || pin.client_artifact_digest() != source.client
            || pin.frame_binding_digest() != source.frame
            || self.map_revision_digest() != source.map
            || !self.companion_snapshot(source.parent).is_ok_and(|parent| {
                parent.health > 0
                    && parent.state.policy.definition_key == source.parent_key
                    && parent.state.master.is_none()
                    && self.native_summon_master(source.parent).is_none()
            })
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        self.assert_actor_spell_unreserved(actor)?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        match &mut self.carrier.slots[index] {
            Slot::Occupied {
                lifecycle,
                committed: true,
                game_session_id: Some(current),
                generation,
                ..
            } if *current == session && *generation == actor.0.actor_local_generation.0 => {
                lifecycle.rotten_presentation.set(key, count)
            }
            _ => Err(CarrierError::PlayerReservationMismatch),
        }
    }
}

impl ChannelRuntimeV1 {
    /// The native leave callback removes exactly the three area keys. The caller
    /// owns the actual movement/return boundary; Agony remains Player metadata.
    pub fn clear_rotten_blood_area_icons(
        &mut self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        self.assert_actor_spell_unreserved(actor)?;
        let index = self.carrier.validate_ref(&self.continuity, actor.0)?;
        match &mut self.carrier.slots[index] {
            Slot::Occupied {
                lifecycle,
                committed: true,
                game_session_id: Some(current),
                generation,
                ..
            } if *current == session && *generation == actor.0.actor_local_generation.0 => {
                lifecycle.rotten_presentation.leave_area();
                Ok(())
            }
            _ => Err(CarrierError::PlayerReservationMismatch),
        }
    }
}
