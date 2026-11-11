//! Source callback placement DATA and current physical occupancy. These facts
//! grant no callback, map location, force flag or actor admission by themselves.
use super::*;
use oteryn_gameplay_content::content::ProjectV2AuthoringProfileData;
use oteryn_gameplay_content::content::native_gameplay::NativeGameplayState;
use oteryn_gameplay_content::content::native_monster_callbacks::QualifiedNativeCallbackSpawn;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeSourcePlacementOccupant {
    PlayerOrNpc,
    Monster {
        pushable: bool,
        player_owned_summon: bool,
    },
    IgnoredGhost,
}
pub fn native_source_monster_occupancy_admits(
    push_creatures: bool,
    occupants: &[NativeSourcePlacementOccupant],
) -> bool {
    occupants.iter().all(|occupant| match occupant {
        NativeSourcePlacementOccupant::IgnoredGhost => true,
        NativeSourcePlacementOccupant::Monster {
            pushable,
            player_owned_summon,
        } => push_creatures && *pushable && !*player_owned_summon,
        NativeSourcePlacementOccupant::PlayerOrNpc => false,
    })
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeSourceOccupancyFacts {
    pub admits_occupants: bool,
    pub familiar: bool,
}
impl ChannelRuntimeV1 {
    /// Actual native queryAdd occupancy, read anew for each child. `removed`
    /// describes a pending source removal for read-only preflight; physical
    /// admission always recomputes without ignoring an actor.
    pub fn current_native_source_occupancy(
        &self,
        native: &NativeGameplayState,
        child: &str,
        at: MovementLocalPosition,
        removed: Option<ExactActorRef>,
    ) -> Result<NativeSourceOccupancyFacts, CarrierError> {
        self.owner_fence()?;
        if native.source_digest() != self.content_pin().server_artifact_digest()
            || removed.is_some_and(|actor| !self.contains_live_creature(actor))
        {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        let movement = |key: &str, revision: &str| {
            let record = native.creature_profiles().records.iter().find(|record| {
                record.profile.target.key == key && record.profile.target.revision == revision
            })?;
            let ProjectV2AuthoringProfileData::Behavior(profile) = &record.behavior.as_ref()?.data
            else {
                return None;
            };
            Some(&profile.movement)
        };
        let child_policy = self.companion_policy(child)?;
        if child_policy.definition_key != child {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let child_movement = movement(child, &child_policy.definition_revision)
            .ok_or(CarrierError::InvalidCreatureTarget)?;
        let mut occupants = Vec::new();
        for (actor, position, session) in self.positioned_actor_census()? {
            if position.position() != at || Some(actor) == removed {
                continue;
            }
            if session.is_some() {
                occupants.push(NativeSourcePlacementOccupant::PlayerOrNpc);
                continue;
            }
            let Ok(snapshot) = self.companion_snapshot(actor) else {
                occupants.push(NativeSourcePlacementOccupant::PlayerOrNpc);
                continue;
            };
            let policy = &snapshot.state.policy;
            if self.companion_policy(&policy.definition_key)?.as_ref() != policy.as_ref() {
                return Err(CarrierError::ContentPinWorldMismatch);
            }
            let occupant = movement(&policy.definition_key, &policy.definition_revision)
                .ok_or(CarrierError::InvalidCreatureTarget)?;
            occupants.push(NativeSourcePlacementOccupant::Monster {
                pushable: occupant.pushable,
                player_owned_summon: snapshot.state.master.is_some(),
            });
        }
        Ok(NativeSourceOccupancyFacts {
            admits_occupants: native_source_monster_occupancy_admits(
                child_movement.push_creatures,
                &occupants,
            ),
            familiar: child_policy.is_familiar,
        })
    }
    pub(super) fn validate_native_callback_spawn_source(
        &self,
        native: &NativeGameplayState,
        parent: ExactActorRef,
        source: &QualifiedNativeCallbackSpawn,
    ) -> Result<(), CarrierError> {
        self.owner_fence()?;
        let digest = self.content_pin().server_artifact_digest();
        if native.source_digest() != digest || source.source_digest() != digest {
            return Err(CarrierError::ContentPinWorldMismatch);
        }
        // These sixteen source entrypoints require a live caster at creation.
        // Parent-independent retained births use their separate source owners.
        let snapshot = self.companion_snapshot(parent)?;
        let policy = &snapshot.state.policy;
        if policy.definition_key != source.parent().key
            || policy.definition_revision != source.parent().revision
            || self.companion_policy(&policy.definition_key)?.as_ref() != policy.as_ref()
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        let child = self.companion_policy(&source.child().key)?;
        if child.definition_key != source.child().key
            || child.definition_revision != source.child().revision
        {
            return Err(CarrierError::CreatureTargetMismatch);
        }
        Ok(())
    }
}
