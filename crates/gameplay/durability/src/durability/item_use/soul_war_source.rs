//! Closed source Item family; current target/Quest/room proof belongs to its fused caller.
use super::*;
pub const SOURCE_USE_CAUSE: u32 = 5;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceUse {
    Sorrow,
    MortalEssence,
    CleansedSanity,
}
pub fn source_use(definition: &TypedDefinitionRef) -> Option<SourceUse> {
    if definition.family != "Item" || definition.revision_ref != "definition-r1" {
        return None;
    }
    Some(match definition.production_key.as_str() {
        "oteryn:item.tibia.i33793" => SourceUse::Sorrow,
        "oteryn:item.tibia.i33891" => SourceUse::MortalEssence,
        "oteryn:item.tibia.i33950" => SourceUse::CleansedSanity,
        _ => return None,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn srx3_soul_war_source_use_five_has_only_three_literal_source_items() {
        for (id, kind) in [
            (33793, SourceUse::Sorrow),
            (33891, SourceUse::MortalEssence),
            (33950, SourceUse::CleansedSanity),
        ] {
            let definition = TypedDefinitionRef {
                family: "Item".into(),
                production_key: format!("oteryn:item.tibia.i{id}"),
                revision_ref: "definition-r1".into(),
            };
            assert_eq!(source_use(&definition), Some(kind));
            assert_eq!(
                source_use(&TypedDefinitionRef {
                    revision_ref: "unqualified-r2".into(),
                    ..definition.clone()
                }),
                None
            );
        }
        assert_eq!(
            source_use(&TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i43501".into(),
                revision_ref: "definition-r1".into()
            }),
            None
        );
    }
}

/// Data from the current target resolver, never a client-provided acceptance flag.
#[derive(Debug, Clone)]
pub enum SelectedSoulWarUseTarget {
    Item {
        definition: TypedDefinitionRef,
        position: crate::foundation::MovementLocalPosition,
    },
    Creature(crate::foundation::ExactActorRef),
    OtherMapEntry(crate::foundation::MovementLocalPosition),
}
/// The World adapter retains its actual map/entity owner borrow while resolving
/// the original USE. Like NativeWorldObjectRead, this port supplies facts, not a
/// second map store or a bool granting one of the source actions.
pub trait SoulWarUseTargetRead {
    /// Actual fenced inventory resolution for a handle source. A caller's hinted
    /// definition alone never qualifies a handle's source subtype.
    fn source_definition(
        &self,
        _intent: &oteryn_protocol_oteryn::world_object::UseIntent,
    ) -> Option<TypedDefinitionRef> {
        None
    }
    fn current(&self, runtime: &crate::foundation::ChannelRuntimeV1) -> bool;
    fn resolve(
        &self,
        intent: &oteryn_protocol_oteryn::world_object::UseIntent,
    ) -> Option<SelectedSoulWarUseTarget>;
}

/// Immutable selected-target proof retained across the existing Item/Quest await.
/// Its fields cannot be manufactured by an ItemUse request or a bare bool.
#[derive(Debug, Clone)]
pub struct QualifiedSoulWarUseTarget {
    kind: SourceUse,
    accepted: bool,
    item_target_position: Option<crate::foundation::MovementLocalPosition>,
    wire: [u8; 32],
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    scope: crate::foundation::RuntimeScopeRefV1,
    generation: crate::foundation::ScopeOwnershipGeneration,
    activation: u64,
    server: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
}
impl QualifiedSoulWarUseTarget {
    pub fn from_current_owner<R: SoulWarUseTargetRead>(
        runtime: &crate::foundation::ChannelRuntimeV1,
        native: &crate::content::native_gameplay::NativeGameplayState,
        actor: crate::foundation::ExactActorRef,
        session: crate::foundation::GameSessionId,
        definition: &TypedDefinitionRef,
        intent: &oteryn_protocol_oteryn::world_object::UseIntent,
        reader: &R,
    ) -> Option<Self> {
        use oteryn_protocol_oteryn::world_object::UseTarget;
        runtime.owner_fence().ok()?;
        let pin = runtime.content_pin();
        if native.source_digest() != pin.server_artifact_digest()
            || !native.qualifies_soul_war_callbacks(pin.server_artifact_digest())
            || !reader.current(runtime)
        {
            return None;
        }
        let kind = source_use(definition)?;
        let index = native.item_index()?;
        match &intent.target {
            UseTarget::ItemByDefinition(reference) => {
                let (key, revision) = index.definition_at(*reference)?;
                if definition.family != "Item"
                    || key != definition.production_key
                    || revision != definition.revision_ref
                {
                    return None;
                }
            }
            UseTarget::Item(_) => {
                if reader.source_definition(intent).as_ref() != Some(definition) {
                    return None;
                }
                index.definition_ref(
                    "Item",
                    &definition.production_key,
                    &definition.revision_ref,
                )?;
            }
            _ => return None,
        }
        let (scope, generation) = if let Ok(house) = runtime.house_actor_state(actor, session) {
            if runtime
                .house_player_control_facts(actor, session)
                .ok()?
                .control_loss
                .is_some()
            {
                return None;
            }
            (house.scope, house.generation)
        } else {
            if runtime
                .player_control_facts(actor, session)
                .ok()?
                .control_loss
                .is_some()
            {
                return None;
            }
            (
                runtime.binding().runtime_scope(),
                runtime.binding().scope_generation(),
            )
        };
        let selected = reader.resolve(intent)?;
        let item_target_position = match &selected {
            SelectedSoulWarUseTarget::Item { position, .. } => Some(*position),
            _ => None,
        };
        let accepted = match (kind, selected) {
            (SourceUse::Sorrow, SelectedSoulWarUseTarget::Creature(target)) => runtime
                .companion_snapshot(target)
                .ok()
                .is_some_and(|snapshot| {
                    snapshot.health > 0
                        && matches!(
                            snapshot.state.policy.definition_key.as_str(),
                            "oteryn:creature.ashes_of_burning_hatred"
                                | "oteryn:creature.spark_of_burning_hatred"
                                | "oteryn:creature.flame_of_burning_hatred"
                                | "oteryn:creature.blaze_of_burning_hatred"
                        )
                }),
            (
                SourceUse::MortalEssence | SourceUse::CleansedSanity,
                SelectedSoulWarUseTarget::Item { definition, .. },
            ) => {
                definition.family == "Item"
                    && definition.production_key == "oteryn:item.tibia.i33890"
                    && definition.revision_ref == "definition-r1"
                    && index
                        .definition_ref(
                            "Item",
                            &definition.production_key,
                            &definition.revision_ref,
                        )
                        .is_some()
            }
            _ => false,
        };
        let encoded = oteryn_protocol_oteryn::world_object::encode_item_use_intent(intent).ok()?;
        let mut hash = Sha256::new();
        hash.update(b"oteryn:soul-war-source-use-wire:v1");
        hash.update(encoded);
        Some(Self {
            kind,
            accepted,
            item_target_position,
            wire: hash.finalize().into(),
            actor,
            session,
            scope,
            generation,
            activation: pin.activation_sequence(),
            server: pin.server_artifact_digest(),
            client: pin.client_artifact_digest(),
            frame: pin.frame_binding_digest(),
            map: runtime.map_revision_digest(),
        })
    }
    pub const fn source_use(&self) -> SourceUse {
        self.kind
    }
    pub const fn target_accepted(&self) -> bool {
        self.accepted
    }
    pub const fn item_target_position(&self) -> Option<crate::foundation::MovementLocalPosition> {
        self.item_target_position
    }
    pub const fn wire_binding(&self) -> [u8; 32] {
        self.wire
    }
    pub const fn actor(&self) -> crate::foundation::ExactActorRef {
        self.actor
    }
    pub const fn game_session_id(&self) -> crate::foundation::GameSessionId {
        self.session
    }
    pub const fn runtime_scope(&self) -> crate::foundation::RuntimeScopeRefV1 {
        self.scope
    }
    pub const fn scope_generation(&self) -> crate::foundation::ScopeOwnershipGeneration {
        self.generation
    }
    pub const fn activation_sequence(&self) -> u64 {
        self.activation
    }
    pub const fn server_artifact_digest(&self) -> [u8; 32] {
        self.server
    }
    pub const fn client_artifact_digest(&self) -> [u8; 32] {
        self.client
    }
    pub const fn frame_binding_digest(&self) -> [u8; 32] {
        self.frame
    }
    pub const fn map_revision_digest(&self) -> [u8; 32] {
        self.map
    }
}

#[cfg(test)]
mod target_tests {
    use super::*;
    use crate::content::native_gameplay::test_support::activated_with_loot_tables;
    use crate::foundation::{
        ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition, NodeId,
        RuntimeScopeRefV1, WorldId,
    };
    use oteryn_protocol_oteryn::world_object::{
        MapUseTarget, UseIntent, UseTarget, WorldObjectTarget,
    };
    struct CurrentTarget {
        scope: RuntimeScopeRefV1,
        digest: [u8; 32],
        expected: UseIntent,
        selected: SelectedSoulWarUseTarget,
    }
    impl SoulWarUseTargetRead for CurrentTarget {
        fn current(&self, runtime: &ChannelRuntimeV1) -> bool {
            runtime.binding().runtime_scope() == self.scope
                && runtime.content_pin().server_artifact_digest() == self.digest
        }
        fn resolve(&self, intent: &UseIntent) -> Option<SelectedSoulWarUseTarget> {
            (intent == &self.expected).then(|| self.selected.clone())
        }
    }
    fn id(tag: u8) -> [u8; 16] {
        let mut id = [tag; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id
    }
    fn definition(id: u32) -> TypedDefinitionRef {
        TypedDefinitionRef {
            family: "Item".into(),
            production_key: format!("oteryn:item.tibia.i{id}"),
            revision_ref: "definition-r1".into(),
        }
    }
    #[test]
    fn srx3_soul_war_source_target_requires_current_typed_selection_and_keeps_wrong_sanity_target()
    -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let world = WorldId::decode(&id(80)).map_err(|e| format!("{e:?}"))?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let native = controller
            .active()
            .and_then(|a| a.native_gameplay())
            .ok_or("actual native generation")?;
        let scope = RuntimeScopeRefV1::instance(world, id(81)).map_err(|e| format!("{e:?}"))?;
        let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
            scope,
            NodeId::decode(&id(82)).map_err(|e| format!("{e:?}"))?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            pin.into_channel_pin(),
        )?;
        let session = GameSessionId::decode(&id(83)).map_err(|e| format!("{e:?}"))?;
        let reservation = runtime.reserve_fresh_session(session)?;
        let actor: ExactActorRef = runtime.commit_fresh_session(reservation)?;
        runtime.initialize_pinned_test_position(
            actor,
            MovementLocalPosition {
                x: 33856,
                y: 31873,
                floor: 7,
            },
        )?;
        let index = native.item_index().ok_or("actual native Item identities")?;
        let source = index
            .definition_ref("Item", "oteryn:item.tibia.i33891", "definition-r1")
            .ok_or("actual source MortalEssence")?;
        assert!(
            index
                .definition_ref("Item", "oteryn:item.tibia.i33890", "definition-r1")
                .is_some()
        );
        // This unit supplies the current target-reader port explicitly. The real
        // World adapter must independently resolve the original Map17 object.
        let intent = UseIntent {
            target: UseTarget::ItemByDefinition(source),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                placement: vec![1; 40],
                expected_revision: 0,
            })),
            use_with_position: None,
        };
        let mut reader = CurrentTarget {
            scope,
            digest: native.source_digest(),
            expected: intent.clone(),
            selected: SelectedSoulWarUseTarget::Item {
                definition: definition(33890),
                position: MovementLocalPosition {
                    x: 33856,
                    y: 31874,
                    floor: 7,
                },
            },
        };
        let target = QualifiedSoulWarUseTarget::from_current_owner(
            &runtime,
            native,
            actor,
            session,
            &definition(33891),
            &intent,
            &reader,
        )
        .ok_or("current actual typed Maw target must qualify")?;
        assert!(target.target_accepted());
        assert_eq!(target.actor(), actor);
        assert_eq!(target.game_session_id(), session);
        assert_eq!(target.runtime_scope(), scope);
        assert_eq!(target.server_artifact_digest(), native.source_digest());
        reader.selected = SelectedSoulWarUseTarget::Item {
            definition: definition(33793),
            position: MovementLocalPosition {
                x: 33856,
                y: 31874,
                floor: 7,
            },
        };
        let sanity_intent = UseIntent {
            target: UseTarget::ItemByDefinition(
                index
                    .definition_ref("Item", "oteryn:item.tibia.i33950", "definition-r1")
                    .ok_or("actual Sanity identity")?,
            ),
            ..intent.clone()
        };
        reader.expected = sanity_intent.clone();
        let wrong = QualifiedSoulWarUseTarget::from_current_owner(
            &runtime,
            native,
            actor,
            session,
            &definition(33950),
            &sanity_intent,
            &reader,
        )
        .ok_or("wrong target remains a qualified Sanity callback so its cooldown commits")?;
        assert!(!wrong.target_accepted());
        reader.digest = [0; 32];
        assert!(
            QualifiedSoulWarUseTarget::from_current_owner(
                &runtime,
                native,
                actor,
                session,
                &definition(33950),
                &intent,
                &reader
            )
            .is_none()
        );
        Ok(())
    }
}

/// Validate a non-consuming source action under the same canonical carried-Item
/// locks as a consuming USE. Retaining an Item never grants stale custody.
pub(in crate::durability) async fn assert_retained_source(
    connection: &mut sqlx::PgConnection,
    fence: &CurrentCharacterItemFence,
    request: &ItemUseRequest,
) -> Result<()> {
    request.validate()?;
    if !matches!(
        request.kind,
        ItemUseKind::SoulWarSourceUse { .. } | ItemUseKind::RottenEssence
    ) {
        return Err(ItemUseError::InvalidInput);
    }
    let holder = *fence.character_id.as_bytes();
    let world = *fence.runtime_scope.world_id().as_bytes();
    let backpack: Option<String> = sqlx::query_scalar(
        "SELECT item_instance_id::text FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid",
    ).bind(holder.as_slice()).fetch_optional(&mut *connection).await?;
    let backpack = uuid_text(&backpack.ok_or(ItemUseError::StaleState)?)?;
    super::lock_use_path(connection, &holder, &backpack, &world, request).await?;
    super::lock_stack(
        connection,
        &holder,
        &world,
        &request.source_item_instance_id,
        &request.source_definition,
        request.source_quantity_before,
    )
    .await?;
    Ok(())
}
