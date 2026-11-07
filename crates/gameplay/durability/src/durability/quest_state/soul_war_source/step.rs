//! Actual per-Item committed-movement causes over the existing Quest/Item owner.
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::quest_state::{QuestCause, QuestTransitionRequest, SourceStepOccurrence};
use crate::durability::spell_items_abi::SpellGroundTarget;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition,
    MovementPositionSnapshot, RuntimeScopeRefV1, ScopeOwnershipGeneration,
};
use sha2::{Digest, Sha256};

type Result<T> = std::result::Result<T, CharacterProgressionError>;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceStepKind {
    Remorse,
    SpiteFire,
    Energy,
}
impl SourceStepKind {
    pub const fn item_key(self) -> &'static str {
        match self {
            Self::Remorse => "oteryn:item.tibia.i33792",
            Self::SpiteFire => "oteryn:item.tibia.i33877",
            Self::Energy => "oteryn:item.tibia.i34005",
        }
    }
    fn transition(self) -> String {
        format!(
            "{}{}",
            oteryn_gameplay_quests::quest::soul_war::source_items::TRANSITION_PREFIX,
            match self {
                Self::Remorse => "remorse",
                Self::SpiteFire => "spite-fire",
                Self::Energy => "energy",
            }
        )
    }
}
#[derive(Debug, Clone)]
struct SourceStepGroundItem {
    pub item: [u8; 16],
    pub definition: TypedDefinitionRef,
    /// The current runtime's physical coordinate frame, not a wire projection.
    pub position: MovementLocalPosition,
}
trait SoulWarStepItemRead {
    fn current(&self, runtime: &ChannelRuntimeV1) -> bool;
    fn items(&self, position: MovementLocalPosition) -> Vec<SourceStepGroundItem>;
}
#[derive(Debug, Clone)]
pub struct SoulWarSourceStep {
    kind: SourceStepKind,
    actor: ExactActorRef,
    session: GameSessionId,
    movement: MovementPositionSnapshot,
    epoch: u64,
    item: SourceStepGroundItem,
    target: SpellGroundTarget,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    activation: u64,
    server: [u8; 32],
    client: [u8; 32],
    map: [u8; 32],
    frame: [u8; 32],
    cause: SourceStepOccurrence,
    cause_binding: [u8; 32],
}
impl SoulWarSourceStep {
    #[allow(
        clippy::too_many_arguments,
        reason = "the original committed movement and current Item owner independently prove this callback"
    )]
    fn from_current_owner<R: SoulWarStepItemRead>(
        runtime: &ChannelRuntimeV1,
        native: &crate::content::native_gameplay::NativeGameplayState,
        room: &crate::content::QualifiedNativeEntryRoom,
        actor: ExactActorRef,
        session: GameSessionId,
        movement: MovementPositionSnapshot,
        reset_epoch: u64,
        item: [u8; 16],
        reader: &R,
    ) -> Option<Self> {
        runtime.owner_fence().ok()?;
        let pin = runtime.content_pin();
        if native.source_digest() != pin.server_artifact_digest()
            || !native.qualifies_soul_war_callbacks(pin.server_artifact_digest())
            || runtime
                .player_control_facts(actor, session)
                .ok()?
                .control_loss
                .is_some()
            || runtime.read_actor_position(actor).ok()? != movement
            || movement.last_translation_revision() == 0
            || movement.revision() != movement.last_translation_revision()
            || !reader.current(runtime)
            || !crate::durability::spell_item_transaction::uuid_v7(&item)
        {
            return None;
        }
        let at = movement.position();
        let current = reader.items(at);
        let mut selected = current.into_iter().filter(|entry| entry.item == item);
        let selected_item = selected.next()?;
        if selected.next().is_some()
            || selected_item.position != at
            || selected_item.definition.family != "Item"
            || selected_item.definition.revision_ref != "definition-r1"
        {
            return None;
        }
        let kind = match selected_item.definition.production_key.as_str() {
            "oteryn:item.tibia.i33792" => SourceStepKind::Remorse,
            "oteryn:item.tibia.i33877" => SourceStepKind::SpiteFire,
            "oteryn:item.tibia.i34005" => SourceStepKind::Energy,
            _ => return None,
        };
        // Crystal soul_war_mechanics.lua:476-478 registers this callback
        // by position; an identical field elsewhere is not a Spite event.
        if kind == SourceStepKind::SpiteFire
            && ![
                (33743, 31628, 14),
                (33736, 31632, 14),
                (33750, 31632, 14),
                (33742, 31637, 14),
            ]
            .contains(&(at.x, at.y, at.floor))
        {
            return None;
        }
        native.item_index()?.definition_ref(
            "Item",
            &selected_item.definition.production_key,
            "definition-r1",
        )?;
        let target = SpellGroundTarget::for_native_tile_read(room, runtime, at).ok()?;
        let mut hash = Sha256::new();
        hash.update(b"oteryn:quest-world-step-item:v1");
        hash.update(session.as_bytes());
        hash.update(actor.placement_identity());
        hash.update(actor.actor_local_generation().to_be_bytes());
        hash.update(movement.revision().to_be_bytes());
        hash.update(movement.last_translation_revision().to_be_bytes());
        hash.update(reset_epoch.to_be_bytes());
        hash.update(item);
        hash.update(selected_item.definition.production_key.as_bytes());
        hash.update(pin.server_artifact_digest());
        hash.update(pin.frame_binding_digest());
        hash.update(runtime.map_revision_digest());
        hash.update(pin.client_artifact_digest());
        hash.update(pin.activation_sequence().to_be_bytes());
        hash.update(runtime.binding().scope_generation().get().to_be_bytes());
        hash.update(runtime.binding().runtime_scope().world_id().as_bytes());
        match runtime.binding().runtime_scope() {
            RuntimeScopeRefV1::Channel { channel_id, .. } => {
                hash.update([1]);
                hash.update(channel_id.as_bytes());
            }
            RuntimeScopeRefV1::Instance { instance_id, .. } => {
                hash.update([2]);
                hash.update(instance_id);
            }
        }
        let cause_binding: [u8; 32] = hash.finalize().into();
        let cause = SourceStepOccurrence::from_binding_digest(cause_binding);
        Some(Self {
            kind,
            actor,
            session,
            movement,
            epoch: reset_epoch,
            item: selected_item,
            target,
            scope: runtime.binding().runtime_scope(),
            generation: runtime.binding().scope_generation(),
            activation: pin.activation_sequence(),
            server: pin.server_artifact_digest(),
            client: pin.client_artifact_digest(),
            frame: pin.frame_binding_digest(),
            map: runtime.map_revision_digest(),
            cause,
            cause_binding,
        })
    }
    pub(super) fn request(
        &self,
        fence: &CurrentCharacterGameplayFence,
    ) -> Result<QuestTransitionRequest> {
        if !self.current_fence(fence) {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        Ok(QuestTransitionRequest {
            transition_key: self.kind.transition(),
            cause: QuestCause::SourceStep(self.cause),
        })
    }
    pub(super) fn current_fence(&self, fence: &CurrentCharacterGameplayFence) -> bool {
        self.session == fence.game_session_id
            && self.scope == fence.runtime_scope
            && self.generation == fence.scope_ownership_generation
    }
    pub(super) fn binding(
        &self,
        character: crate::domain::CharacterId,
        request: &QuestTransitionRequest,
    ) -> [u8; 33] {
        let mut result = crate::durability::quest_state::request_binding(character, request);
        let mut hash = Sha256::new();
        hash.update(result);
        hash.update(b"oteryn:soul-war-source-step-binding:v1");
        hash.update(self.cause_binding);
        hash.update(self.actor.placement_identity());
        hash.update(self.movement.revision().to_be_bytes());
        hash.update(self.epoch.to_be_bytes());
        hash.update(self.item.item);
        hash.update(self.item.definition.family.as_bytes());
        hash.update([0]);
        hash.update(self.item.definition.production_key.as_bytes());
        hash.update([0]);
        hash.update(self.item.definition.revision_ref.as_bytes());
        hash.update(self.scope.world_id().as_bytes());
        match self.scope {
            RuntimeScopeRefV1::Channel { channel_id, .. } => {
                hash.update([1]);
                hash.update(channel_id.as_bytes());
            }
            RuntimeScopeRefV1::Instance { instance_id, .. } => {
                hash.update([2]);
                hash.update(instance_id);
            }
        }
        hash.update(self.generation.get().to_be_bytes());
        hash.update(self.activation.to_be_bytes());
        hash.update(self.server);
        hash.update(self.client);
        hash.update(self.map);
        hash.update(self.frame);
        hash.update(&self.target.spatial_position);
        result[1..].copy_from_slice(&hash.finalize());
        result
    }
    pub fn kind(&self) -> SourceStepKind {
        self.kind
    }
    pub fn actor(&self) -> ExactActorRef {
        self.actor
    }
    pub fn game_session_id(&self) -> GameSessionId {
        self.session
    }
    pub fn item_instance_id(&self) -> [u8; 16] {
        self.item.item
    }
    pub fn source_definition(&self) -> &TypedDefinitionRef {
        &self.item.definition
    }
    pub fn target(&self) -> &SpellGroundTarget {
        &self.target
    }
    pub fn runtime_scope(&self) -> RuntimeScopeRefV1 {
        self.scope
    }
    pub fn scope_generation(&self) -> ScopeOwnershipGeneration {
        self.generation
    }
    pub fn activation_sequence(&self) -> u64 {
        self.activation
    }
    pub fn server_artifact_digest(&self) -> [u8; 32] {
        self.server
    }
    pub fn client_artifact_digest(&self) -> [u8; 32] {
        self.client
    }
    pub fn frame_binding_digest(&self) -> [u8; 32] {
        self.frame
    }
    pub fn map_revision_digest(&self) -> [u8; 32] {
        self.map
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::native_gameplay::test_support::activated_with_loot_tables;
    use crate::foundation::{ChannelId, NodeId, WorldId};
    struct Ground {
        scope: RuntimeScopeRefV1,
        items: Vec<SourceStepGroundItem>,
    }
    impl SoulWarStepItemRead for Ground {
        fn current(&self, runtime: &ChannelRuntimeV1) -> bool {
            self.scope == runtime.binding().runtime_scope()
        }
        fn items(&self, at: MovementLocalPosition) -> Vec<SourceStepGroundItem> {
            self.items
                .iter()
                .filter(|item| item.position == at)
                .cloned()
                .collect()
        }
    }
    fn id(tag: u8) -> [u8; 16] {
        let mut id = [tag; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id
    }
    #[test]
    fn srx3_soul_war_source_step_binds_real_committed_movement_and_each_item()
    -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let world = WorldId::decode(&id(85)).map_err(|e| format!("{e:?}"))?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let native = controller
            .active()
            .and_then(|active| active.native_gameplay())
            .ok_or("actual native generation")?;
        let room = pin.qualified_room().clone();
        let channel = ChannelId::decode(&id(86)).map_err(|e| format!("{e:?}"))?;
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            NodeId::decode(&id(87)).map_err(|e| format!("{e:?}"))?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            pin.into_channel_pin(),
        )?;
        let session = GameSessionId::decode(&id(88)).map_err(|e| format!("{e:?}"))?;
        let reservation = runtime.reserve_fresh_session(session)?;
        let actor = runtime.commit_fresh_session(reservation)?;
        runtime.initialize_first_entry_position(actor)?;
        let before = runtime.read_actor_position(actor)?;
        let origin = before.position();
        let next = [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .into_iter()
            .map(|(dx, dy)| MovementLocalPosition {
                x: origin.x + dx,
                y: origin.y + dy,
                floor: origin.floor,
            })
            .find(|at| {
                room.movement_cells().index().lookup(
                    room.movement_cells().scope(),
                    crate::content::LogicalCell {
                        x: at.x,
                        y: at.y,
                        z: i32::from(at.floor),
                    },
                ) == Ok(crate::content::CollisionClass::Walkable)
            })
            .ok_or("actual adjacent walkable native cell")?;
        let committed = runtime
            .borrow_movement_position()
            .commit_cardinal(before, next)?;
        assert!(committed.last_translation_revision() > before.last_translation_revision());
        let definition = TypedDefinitionRef {
            family: "Item".into(),
            production_key: SourceStepKind::Remorse.item_key().into(),
            revision_ref: "definition-r1".into(),
        };
        let reader = Ground {
            scope: runtime.binding().runtime_scope(),
            items: vec![
                SourceStepGroundItem {
                    item: id(89),
                    definition: definition.clone(),
                    position: next,
                },
                SourceStepGroundItem {
                    item: id(90),
                    definition,
                    position: next,
                },
            ],
        };
        let first = SoulWarSourceStep::from_current_owner(
            &runtime,
            native,
            &room,
            actor,
            session,
            committed,
            1,
            id(89),
            &reader,
        )
        .ok_or("actual committed movement source callback must qualify")?;
        let second = SoulWarSourceStep::from_current_owner(
            &runtime,
            native,
            &room,
            actor,
            session,
            committed,
            1,
            id(90),
            &reader,
        )
        .ok_or("second Item on the same actual step must qualify independently")?;
        let fence = CurrentCharacterGameplayFence {
            character_id: crate::domain::CharacterId::from_bytes(id(91))
                .map_err(|e| format!("{e:?}"))?,
            game_session_id: session,
            connection_generation: crate::foundation::ConnectionGeneration::new(1)
                .map_err(|e| format!("{e:?}"))?,
            character_lease_generation: 1,
            runtime_scope: runtime.binding().runtime_scope(),
            scope_ownership_generation: runtime.binding().scope_generation(),
            expected_character_revision: crate::domain::CharacterRevision::new(1)
                .map_err(|e| format!("{e:?}"))?,
        };
        let a = first.request(&fence)?;
        let b = second.request(&fence)?;
        assert!(matches!(a.cause, QuestCause::SourceStep(_)));
        assert_ne!(a.cause.key(), b.cause.key());
        assert_ne!(
            first.binding(fence.character_id, &a),
            second.binding(fence.character_id, &b)
        );
        assert_eq!(first.movement.revision(), committed.revision());
        assert!(
            SoulWarSourceStep::from_current_owner(
                &runtime,
                native,
                &room,
                actor,
                session,
                before,
                1,
                id(89),
                &reader
            )
            .is_none(),
            "a stale or uncommitted movement cannot mint a new source callback"
        );
        // Crystal mechanics.lua:476-478 registers Spite fire by four
        // positions, unlike Remorse/Energy's Item-id registrations.
        assert!(
            ![
                (33743, 31628, 14),
                (33736, 31632, 14),
                (33750, 31632, 14),
                (33742, 31637, 14)
            ]
            .contains(&(next.x, next.y, next.floor))
        );
        let misplaced_fire = Ground {
            scope: runtime.binding().runtime_scope(),
            items: vec![SourceStepGroundItem {
                item: id(92),
                definition: TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: SourceStepKind::SpiteFire.item_key().into(),
                    revision_ref: "definition-r1".into(),
                },
                position: next,
            }],
        };
        assert!(
            SoulWarSourceStep::from_current_owner(
                &runtime,
                native,
                &room,
                actor,
                session,
                committed,
                1,
                id(92),
                &misplaced_fire
            )
            .is_none(),
            "Spite's position-only MoveEvent must not activate on an arbitrary 33877 tile"
        );
        let turned = runtime
            .borrow_movement_position()
            .commit_turn(committed, crate::foundation::MovementFacing::North)?;
        assert_eq!(turned.position(), committed.position());
        assert_eq!(
            turned.last_translation_revision(),
            committed.last_translation_revision()
        );
        assert!(turned.revision() > turned.last_translation_revision());
        assert!(
            SoulWarSourceStep::from_current_owner(
                &runtime,
                native,
                &room,
                actor,
                session,
                turned,
                1,
                id(89),
                &reader
            )
            .is_none(),
            "a current facing-only commit is not a new MoveEvent translation"
        );
        Ok(())
    }
}

#[path = "step/standing_read.rs"]
mod standing_read;
