//! Source-bound native death composition. Exact imported memberships, no general interpreter.
use crate::content::{ProjectV2AuthoringProfile, ProjectV2SourceIdentityBinding, WorldProject};
use crate::foundation::{
    BorethDeathLedger, BorethDeathResult, CarrierError, ChannelId, ChannelRuntimeV1, ExactActorRef,
    GameSessionId, RumBarrelDeathLedger, RumBarrelDeathResult, RuntimeScopeRefV1, RuntimeWorkStamp,
    ScopeOwnershipGeneration, ScopeRuntimeFence, WorldId,
};
use crate::weak_spot_speech::{WeakSpotSpeechError, WeakSpotSpeechMailbox};
#[derive(Debug)]
struct PinnedDeathDefinition {
    target: String,
    profile: ProjectV2AuthoringProfile,
    binding: ProjectV2SourceIdentityBinding,
}
#[derive(Debug)]
pub(crate) struct QualifiedCrystalDeaths {
    entries: Vec<PinnedDeathDefinition>,
    world: WorldId,
    channel: ChannelId,
    generation: ScopeOwnershipGeneration,
    activation: u64,
    server: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
}
#[derive(Debug)]
pub(crate) enum CrystalDeathCompositionError {
    Source(&'static str),
    Owner(CarrierError),
    Speech(WeakSpotSpeechError),
}
#[derive(Debug)]
pub(crate) enum CrystalDeathApplied {
    Boreth(BorethDeathResult),
    RumBarrel(RumBarrelDeathResult),
    WeakSpotSpeech { recipient_lines: usize },
    MapBindingRequired { actor: ExactActorRef },
    DurableOwnerRequired(&'static str),
    ServerDrawRequired,
    Unrelated,
}
impl QualifiedCrystalDeaths {
    /// Called by the native boot owner from the actual captured, selected imported project.
    /// Artifact pin only fences a qualification already proved by exact typed profile+binding
    /// membership; no supplied arbitrary digest is accepted as a source-membership proof.
    pub(crate) fn qualify(
        project: &WorldProject,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Self, CrystalDeathCompositionError> {
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("pinned-death-registry.json"))
                .map_err(|_| CrystalDeathCompositionError::Source("invalid pinned registry"))?;
        let expected: Vec<PinnedDeathDefinition> = rows
            .into_iter()
            .map(|row| {
                Ok(PinnedDeathDefinition {
                    target: row["target"]
                        .as_str()
                        .ok_or(CrystalDeathCompositionError::Source(
                            "registry target missing",
                        ))?
                        .to_owned(),
                    profile: serde_json::from_value(row["profile"].clone()).map_err(|_| {
                        CrystalDeathCompositionError::Source("registry profile invalid")
                    })?,
                    binding: serde_json::from_value(row["binding"].clone()).map_err(|_| {
                        CrystalDeathCompositionError::Source("registry binding invalid")
                    })?,
                })
            })
            .collect::<Result<_, CrystalDeathCompositionError>>()?;
        let state = project.v2().ok_or(CrystalDeathCompositionError::Source(
            "native v2 source state missing",
        ))?;
        for row in &expected {
            if !state.authoring_profiles.iter().any(|p| p == &row.profile)
                || !state
                    .source_identity_bindings
                    .iter()
                    .any(|b| b == &row.binding)
            {
                return Err(CrystalDeathCompositionError::Source(
                    "exact pinned native definition membership missing",
                ));
            }
            if !state.sources.iter().any(|s| {
                s.key == row.binding.source_key && s.revision == row.binding.source_revision
            }) {
                return Err(CrystalDeathCompositionError::Source(
                    "binding outside actual loaded source revision",
                ));
            }
        }
        let world = project
            .lower_reference_source()
            .map_err(|_| CrystalDeathCompositionError::Source("native project world invalid"))?
            .world_id;
        if world != runtime.binding().world_id() {
            return Err(CrystalDeathCompositionError::Source(
                "native project world differs from owner",
            ));
        }
        let binding = runtime.binding();
        let pin = runtime.content_pin();
        Ok(Self {
            entries: expected,
            world,
            channel: binding.channel_id(),
            generation: binding.scope_generation(),
            activation: pin.activation_sequence(),
            server: pin.server_artifact_digest(),
            client: pin.client_artifact_digest(),
            frame: pin.frame_binding_digest(),
            map: pin.map_revision_digest(),
        })
    }
    /// Reused before projection and before callbacks, so stale authority never creates a corpse.
    fn current_owner(
        &self,
        runtime: &ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
    ) -> Result<(), CrystalDeathCompositionError> {
        let pin = runtime.content_pin();
        let binding = runtime.binding();
        let scope = RuntimeScopeRefV1::channel(self.world, self.channel);
        if binding.world_id() != self.world
            || binding.channel_id() != self.channel
            || binding.scope_generation() != self.generation
            || pin.activation_sequence() != self.activation
            || pin.server_artifact_digest() != self.server
            || pin.client_artifact_digest() != self.client
            || pin.frame_binding_digest() != self.frame
            || pin.map_revision_digest() != self.map
            || !fence.is_current_for_scope(scope, self.generation)
            || !fence.accepts_stamp(stamp)
        {
            return Err(CrystalDeathCompositionError::Source(
                "stale physical source owner work",
            ));
        }
        Ok(())
    }
    /// Current owner work fence must be external and live. Sealed committed death is re-read before
    /// routing; actual consumers retain idempotence. No string callback/member from network input.
    // Keep dispatch ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn dispatch(
        &self,
        runtime: &mut ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        actor: ExactActorRef,
        boreth: &mut BorethDeathLedger,
        rum: &mut RumBarrelDeathLedger,
        speech: &mut WeakSpotSpeechMailbox,
        listeners: &[(ExactActorRef, GameSessionId)],
        server_rum_draw: Option<i64>,
    ) -> Result<CrystalDeathApplied, CrystalDeathCompositionError> {
        self.current_owner(runtime, fence, stamp)?;
        if runtime
            .native_summon_role(actor)
            .map_err(CrystalDeathCompositionError::Owner)?
            .is_some()
        {
            return Err(CrystalDeathCompositionError::Source(
                "intrinsic summon excluded from generic Crystal callback",
            ));
        }
        let identity = {
            let owner = runtime.borrow_combat_death();
            let key = owner
                .creature_target_identity(actor)
                .map_err(CrystalDeathCompositionError::Owner)?;
            let key = std::str::from_utf8(key)
                .map_err(|_| {
                    CrystalDeathCompositionError::Source("invalid native source identity")
                })?
                .to_owned();
            if !self.entries.iter().any(|e| e.target == key) {
                return Ok(CrystalDeathApplied::Unrelated);
            }
            owner
                .projected_death(actor)
                .map_err(CrystalDeathCompositionError::Owner)?;
            key
        };
        match identity.as_str() {
            "oteryn:creature.boreth" => runtime
                .commit_boreth_death(boreth, actor)
                .map(CrystalDeathApplied::Boreth)
                .map_err(CrystalDeathCompositionError::Owner),
            "oteryn:creature.rum_barrel" => match server_rum_draw {
                Some(draw) => runtime
                    .commit_rum_barrel_death(rum, actor, draw)
                    .map(CrystalDeathApplied::RumBarrel)
                    .map_err(CrystalDeathCompositionError::Owner),
                None => Ok(CrystalDeathApplied::ServerDrawRequired),
            },
            "oteryn:creature.weak_spot" => speech
                .publish(runtime, actor, listeners)
                .map(|recipient_lines| CrystalDeathApplied::WeakSpotSpeech { recipient_lines })
                .map_err(CrystalDeathCompositionError::Speech),
            "oteryn:creature.lord_retro" => Ok(CrystalDeathApplied::DurableOwnerRequired(
                "Character outfit entitlement owning writer (Lord Retro 1460 and 1461)",
            )),
            "oteryn:creature.herald_of_fire" => Ok(CrystalDeathApplied::DurableOwnerRequired(
                "QuestState writer and qualified permanent-portal Map binding",
            )),
            // Deliberately no bare commit_tentacle_next call: missing Map operations must not be silently
            // omitted. Real WorldObject source-bound line removal must precede spawning; final map
            // transform/removal is a distinct branch. Root links the actual native Map owner next.
            _ => Ok(CrystalDeathApplied::MapBindingRequired { actor }),
        }
    }
}

#[cfg(test)]
mod retained_native_source_tests {
    use super::*;
    use crate::content::*;
    use std::path::Path;
    #[test]
    #[ignore = "Requires retained actual native importer capture; run explicitly, never synthetic source membership"]
    fn actual_capture_membership_fenced_death_executes_handlers_and_blocks_map_gap()
    -> Result<(), Box<dyn std::error::Error>> {
        let limits = ProjectEvidenceLimits {
            max_documents: 11,
            max_document_bytes: 96_000_000,
            max_total_bytes: 160_000_000,
            max_json_depth: 24,
            max_decoded_fields: 2_120_000,
            max_string_bytes: 43_000_000,
            max_locator_bytes: 160,
            max_locator_segments: 8,
            max_reference_records: 70000,
            max_import_records: 16,
            max_reimport_states: 404,
        };
        let retained_native_capture_path = std::env::var_os("OTERYN_MONSTER_NATIVE_CAPTURE_ROOT")
            .filter(|value| !value.is_empty())
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| panic!("OTERYN_MONSTER_NATIVE_CAPTURE_ROOT must explicitly name the current final native eleven-document capture"));
        let path = retained_native_capture_path.as_path();
        let project = capture_world_project(
            path.parent().unwrap(),
            path.file_name().unwrap(),
            ProjectFilesystemLimits {
                project: limits,
                max_entries_per_directory_scan: 32,
                max_total_directory_entries_scanned: 201,
            },
        )?;
        let world = project.lower_reference_source()?.world_id;
        let mut runtime = crate::foundation::crystal_death_router_fixture(world);
        let registry = QualifiedCrystalDeaths::qualify(&project, &runtime)
            .expect("actual captured15 source memberships");
        let scope = RuntimeScopeRefV1::channel(world, runtime.binding().channel_id());
        let (fence, stamp) =
            crate::foundation::crystal_timer_fixture(scope, runtime.binding().scope_generation())?;
        let at = crate::foundation::MovementLocalPosition {
            x: 100,
            y: 100,
            floor: 7,
        };
        let source = runtime.crystal_router_fixture_actor("oteryn:creature.boreth", 10, at);
        let prey = runtime.crystal_router_fixture_actor(
            "oteryn:creature.plaguethrower",
            10,
            crate::foundation::MovementLocalPosition {
                x: 32936,
                y: 31474,
                floor: 1,
            },
        );
        let mut boreth = BorethDeathLedger::default();
        let mut rum = RumBarrelDeathLedger::default();
        let mut speech = WeakSpotSpeechMailbox::default();
        assert!(
            registry
                .dispatch(
                    &mut runtime,
                    &fence,
                    stamp,
                    source,
                    &mut boreth,
                    &mut rum,
                    &mut speech,
                    &[],
                    None
                )
                .is_err()
        );
        runtime.crystal_router_fixture_project_death(source, "oteryn:creature.boreth", 10);
        assert!(matches!(
            registry
                .dispatch(
                    &mut runtime,
                    &fence,
                    stamp,
                    source,
                    &mut boreth,
                    &mut rum,
                    &mut speech,
                    &[],
                    None
                )
                .unwrap(),
            CrystalDeathApplied::Boreth(_)
        ));
        assert!(!runtime.contains_live_creature(prey));
        let barrel = runtime.crystal_router_fixture_actor("oteryn:creature.rum_barrel", 10, at);
        let weak = runtime.crystal_router_fixture_actor(
            "oteryn:creature.weak_spot",
            200000,
            crate::foundation::MovementLocalPosition {
                x: 101,
                y: 100,
                floor: 7,
            },
        );
        runtime.crystal_router_fixture_project_death(barrel, "oteryn:creature.rum_barrel", 10);
        assert!(matches!(
            registry
                .dispatch(
                    &mut runtime,
                    &fence,
                    stamp,
                    barrel,
                    &mut boreth,
                    &mut rum,
                    &mut speech,
                    &[],
                    Some(70000)
                )
                .unwrap(),
            CrystalDeathApplied::RumBarrel(_)
        ));
        assert_eq!(runtime.crystal_router_fixture_health(weak), 130000);
        let tentacle = runtime.crystal_router_fixture_actor("oteryn:creature.tentacle", 10, at);
        runtime.crystal_router_fixture_project_death(tentacle, "oteryn:creature.tentacle", 10);
        assert!(matches!(
            registry
                .dispatch(
                    &mut runtime,
                    &fence,
                    stamp,
                    tentacle,
                    &mut boreth,
                    &mut rum,
                    &mut speech,
                    &[],
                    None
                )
                .unwrap(),
            CrystalDeathApplied::MapBindingRequired { .. }
        ));
        let mut missing = project.migrate_to_v2();
        missing
            .state
            .source_identity_bindings
            .retain(|b| b.target.key != "oteryn:creature.boreth");
        let bad = CanonicalProjectDocuments::from_v2_draft(missing, limits)?
            .into_snapshot(limits)?
            .parse(limits)?;
        assert!(QualifiedCrystalDeaths::qualify(&bad, &runtime).is_err());
        Ok(())
    }
}

/// One retained current-Channel composition. Registration comes only from actual loaded project;
/// no callback metadata creates reward authority. The caller removes the actor only after handling
/// the returned callback/dependency outcome, and may retry while the native projection is retained.
pub(crate) struct CrystalDeathOwner {
    registry: QualifiedCrystalDeaths,
    boreth: BorethDeathLedger,
    rum: RumBarrelDeathLedger,
    speech: WeakSpotSpeechMailbox,
}
#[derive(Debug)]
pub(crate) struct CrystalSecondaryDeath {
    pub(crate) actor: ExactActorRef,
    pub(crate) result: Result<
        (
            crate::foundation::CreatureDeathOccurrenceKey,
            crate::foundation::MovementLocalPosition,
            usize,
        ),
        CrystalDeathCompositionError,
    >,
}
#[derive(Debug)]
pub(crate) struct CrystalProjectedDeath {
    pub(crate) death: crate::foundation::CreatureDeathOccurrenceKey,
    pub(crate) position: crate::foundation::MovementLocalPosition,
    /// Callback failure after a successful projection remains explicit; it is not rolled back or
    /// confused with a failed lethal transition. Missing durable/map owners remain typed outcomes.
    pub(crate) callback: Result<CrystalDeathApplied, CrystalDeathCompositionError>,
    pub(crate) secondary: Vec<CrystalSecondaryDeath>,
}
impl CrystalDeathOwner {
    pub(crate) fn bind(
        project: &WorldProject,
        runtime: &ChannelRuntimeV1,
    ) -> Result<Self, CrystalDeathCompositionError> {
        Ok(Self {
            registry: QualifiedCrystalDeaths::qualify(project, runtime)?,
            boreth: BorethDeathLedger::default(),
            rum: RumBarrelDeathLedger::default(),
            speech: WeakSpotSpeechMailbox::default(),
        })
    }
    /// Existing Combat projection followed immediately by source-qualified native callbacks.
    /// Never removes actors, grants quests/account rewards, or manufactures a Map operation.
    pub(crate) fn project_and_dispatch(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        actor: ExactActorRef,
        listeners: &[(ExactActorRef, GameSessionId)],
        server_rum_draw: Option<i64>,
    ) -> Result<CrystalProjectedDeath, CrystalDeathCompositionError> {
        self.registry.current_owner(runtime, fence, stamp)?;
        // Intrinsic physical role, including HP0-present children; stale getter errors cannot default to
        // ordinary creature. The native summon owner handles children with no generic corpse/rewards.
        if runtime
            .native_summon_role(actor)
            .map_err(CrystalDeathCompositionError::Owner)?
            .is_some()
        {
            return Err(CrystalDeathCompositionError::Source(
                "intrinsic summon requires summon lifecycle owner",
            ));
        }
        let mut secondary = Vec::new();
        secondary
            .try_reserve(9)
            .map_err(|_| CrystalDeathCompositionError::Owner(CarrierError::AllocationFailed))?;
        let (death, position) = {
            let mut owner = runtime.borrow_combat_death();
            let corpse = crate::combat::project_fixed_one_creature_death(&mut owner, actor)
                .map_err(CrystalDeathCompositionError::Owner)?;
            (corpse.occurrence().death_key(), corpse.position())
        };
        let callback = self.registry.dispatch(
            runtime,
            fence,
            stamp,
            actor,
            &mut self.boreth,
            &mut self.rum,
            &mut self.speech,
            listeners,
            server_rum_draw,
        );
        // Source Rum lifedrain may lethally damage WeakSpot. Native real HP receipts select only
        // committed HP0 targets; callback chain is fixed at <=9, never a general event interpreter.
        if let Ok(CrystalDeathApplied::RumBarrel(result)) = &callback {
            for &(child, damage) in &result.targets {
                if damage.health_after == 0 {
                    let result = (|| {
                        self.registry.current_owner(runtime, fence, stamp)?;
                        if runtime
                            .native_summon_role(child)
                            .map_err(CrystalDeathCompositionError::Owner)?
                            .is_some()
                        {
                            return Err(CrystalDeathCompositionError::Source(
                                "intrinsic summoned weakspot excluded",
                            ));
                        }
                        let (death, position) = {
                            let mut owner = runtime.borrow_combat_death();
                            let corpse =
                                crate::combat::project_fixed_one_creature_death(&mut owner, child)
                                    .map_err(CrystalDeathCompositionError::Owner)?;
                            (corpse.occurrence().death_key(), corpse.position())
                        };
                        let lines = self
                            .speech
                            .publish(runtime, child, listeners)
                            .map_err(CrystalDeathCompositionError::Speech)?;
                        Ok((death, position, lines))
                    })();
                    secondary.push(CrystalSecondaryDeath {
                        actor: child,
                        result,
                    });
                }
            }
        }
        Ok(CrystalProjectedDeath {
            death,
            position,
            callback,
            secondary,
        })
    }
    /// Real protocol ChatLine bytes consumed by existing connection transport; current recipients
    /// and audibility are revalidated by the mailbox. No synthetic success or new wire identifiers.
    pub(crate) fn drain_speech(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Result<Vec<Vec<u8>>, WeakSpotSpeechError> {
        self.speech.drain(runtime, actor, session)
    }
    pub(crate) fn clear_session(&mut self, session: GameSessionId) {
        self.speech.clear_session(session)
    }
}

#[cfg(test)]
mod retained_projection_owner_tests {
    use super::*;
    use crate::content::*;
    use std::path::Path;
    #[test]
    #[ignore = "Requires retained actual native importer capture and intrinsic native summon owner"]
    fn actual_capture_projection_hook_executes_boreth_rum_secondary_once_and_refuses_stale_authority()
    -> Result<(), Box<dyn std::error::Error>> {
        let limits = ProjectEvidenceLimits {
            max_documents: 11,
            max_document_bytes: 96_000_000,
            max_total_bytes: 160_000_000,
            max_json_depth: 24,
            max_decoded_fields: 2_120_000,
            max_string_bytes: 43_000_000,
            max_locator_bytes: 160,
            max_locator_segments: 8,
            max_reference_records: 70000,
            max_import_records: 16,
            max_reimport_states: 404,
        };
        let retained_native_capture_path = std::env::var_os("OTERYN_MONSTER_NATIVE_CAPTURE_ROOT")
            .filter(|value| !value.is_empty())
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| panic!("OTERYN_MONSTER_NATIVE_CAPTURE_ROOT must explicitly name the current final native eleven-document capture"));
        let path = retained_native_capture_path.as_path();
        let project = capture_world_project(
            path.parent().unwrap(),
            path.file_name().unwrap(),
            ProjectFilesystemLimits {
                project: limits,
                max_entries_per_directory_scan: 32,
                max_total_directory_entries_scanned: 201,
            },
        )?;
        let world = project.lower_reference_source()?.world_id;
        let mut runtime = crate::foundation::crystal_death_router_fixture(world);
        let mut composition = CrystalDeathOwner::bind(&project, &runtime).unwrap();
        let scope = RuntimeScopeRefV1::channel(world, runtime.binding().channel_id());
        let (fence, stamp) =
            crate::foundation::crystal_timer_fixture(scope, runtime.binding().scope_generation())?;
        let at = crate::foundation::MovementLocalPosition {
            x: 100,
            y: 100,
            floor: 7,
        };
        let source = runtime.crystal_router_fixture_actor("oteryn:creature.boreth", 10, at);
        let prey = runtime.crystal_router_fixture_actor(
            "oteryn:creature.plaguethrower",
            10,
            crate::foundation::MovementLocalPosition {
                x: 32936,
                y: 31474,
                floor: 1,
            },
        );
        runtime.crystal_router_fixture_commit_lethal(source, "oteryn:creature.boreth", 10);
        let mut other = [0u8; 16];
        other[6] = 0x70;
        other[8] = 0x80;
        other[15] = 94;
        let wrongscope = RuntimeScopeRefV1::channel(world, ChannelId::decode(&other)?);
        let (wrongfence, wrongstamp) = crate::foundation::crystal_timer_fixture(
            wrongscope,
            runtime.binding().scope_generation(),
        )?;
        assert!(
            composition
                .project_and_dispatch(&mut runtime, &wrongfence, wrongstamp, source, &[], None)
                .is_err()
        );
        assert!(
            runtime
                .borrow_combat_death()
                .projected_death(source)
                .is_err()
        );
        assert!(runtime.contains_live_creature(prey));
        let committed = composition
            .project_and_dispatch(&mut runtime, &fence, stamp, source, &[], None)
            .unwrap();
        assert!(matches!(
            committed.callback,
            Ok(CrystalDeathApplied::Boreth(_))
        ));
        assert!(!runtime.contains_live_creature(prey));
        assert_eq!(
            runtime
                .borrow_combat_death()
                .projected_death(source)
                .unwrap()
                .0,
            committed.death
        );
        let replacement = runtime.crystal_router_fixture_actor(
            "oteryn:creature.plaguethrower",
            10,
            crate::foundation::MovementLocalPosition {
                x: 32936,
                y: 31474,
                floor: 1,
            },
        );
        let retry = composition
            .project_and_dispatch(&mut runtime, &fence, stamp, source, &[], None)
            .unwrap();
        assert_eq!(retry.death, committed.death);
        assert!(runtime.contains_live_creature(replacement));
        let barrel = runtime.crystal_router_fixture_actor("oteryn:creature.rum_barrel", 10, at);
        let weak = runtime.crystal_router_fixture_actor(
            "oteryn:creature.weak_spot",
            50000,
            crate::foundation::MovementLocalPosition {
                x: 101,
                y: 100,
                floor: 7,
            },
        );
        runtime.crystal_router_fixture_commit_lethal(barrel, "oteryn:creature.rum_barrel", 10);
        let rum = composition
            .project_and_dispatch(&mut runtime, &fence, stamp, barrel, &[], Some(70000))
            .unwrap();
        assert!(matches!(
            rum.callback,
            Ok(CrystalDeathApplied::RumBarrel(_))
        ));
        assert_eq!(runtime.crystal_router_fixture_health(weak), 0);
        assert_eq!(rum.secondary.len(), 1);
        let secondary = rum.secondary[0].result.as_ref().unwrap();
        assert_eq!(
            runtime
                .borrow_combat_death()
                .projected_death(weak)
                .unwrap()
                .0,
            secondary.0
        );
        assert_ne!(rum.death, secondary.0);
        let repeated = composition
            .project_and_dispatch(&mut runtime, &fence, stamp, barrel, &[], Some(70000))
            .unwrap();
        assert_eq!(repeated.death, rum.death);
        assert_eq!(
            repeated.secondary[0].result.as_ref().unwrap().0,
            secondary.0
        );
        assert_eq!(runtime.crystal_router_fixture_health(weak), 0);
        Ok(())
    }
}

impl CrystalDeathOwner {
    /// Exact source-qualified sealed-death -> existing Map-owner operation.
    /// This slice preserves real REMOVE/TRANSFORM; source successor spawn stays a separate
    /// native actor-owner operation after successful Map publication. Map selection/occupancy
    /// and publisher come from independently current Map owner, never actor metadata.
    // Keep commit_tentacle_source_map ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn commit_tentacle_source_map<E>(
        &self,
        runtime: &mut ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        actor: ExactActorRef,
        map: &crate::content::CanonicalReferencePlayableContent,
        object: &mut crate::world_runtime::LocalObjectRuntime,
        operation: &crate::world_runtime::ScopeLocalObjectOperation,
        occupied: &std::collections::BTreeSet<crate::content::LogicalCell>,
        publish: impl FnOnce(&crate::world_runtime::ScopePublish<'_>) -> Result<(), E>,
    ) -> Result<Result<crate::foundation::TerminalSemanticOutcome, E>, CrystalDeathCompositionError>
    {
        self.registry.current_owner(runtime, fence, stamp)?;
        if runtime
            .native_summon_role(actor)
            .map_err(CrystalDeathCompositionError::Owner)?
            .is_some()
        {
            return Err(CrystalDeathCompositionError::Source(
                "summon is not a source Tentacle death",
            ));
        }
        let (key, death) = {
            let owner = runtime.borrow_combat_death();
            let key = std::str::from_utf8(
                owner
                    .creature_target_identity(actor)
                    .map_err(CrystalDeathCompositionError::Owner)?,
            )
            .map_err(|_| CrystalDeathCompositionError::Source("invalid exact creature source key"))?
            .to_owned();
            let (_, position) = owner
                .projected_death(actor)
                .map_err(CrystalDeathCompositionError::Owner)?;
            (key, position)
        };
        if !self.registry.entries.iter().any(|e| e.target == key) {
            return Err(CrystalDeathCompositionError::Source(
                "Tentacle profile outside current qualified source registry",
            ));
        }
        if !object
            .tentacle_source_map_binding_matches(map, operation, &key, death)
            .map_err(|_| {
                CrystalDeathCompositionError::Source("invalid current Tentacle Map binding")
            })?
        {
            return Err(CrystalDeathCompositionError::Source(
                "source item or tile differs from exact callback",
            ));
        }
        let binding = runtime.binding();
        object
            .apply_scope_operation(
                RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id()),
                binding.scope_generation(),
                operation,
                occupied,
                None,
                publish,
            )
            .map_err(|_| {
                CrystalDeathCompositionError::Source(
                    "native Map owner refused source callback operation",
                )
            })
    }
}

impl QualifiedCrystalDeaths {
    /// Real source producer. The opaque request is frozen under current physical death
    /// authority; its consumer awaits the existing SQL writer after dropping runtime locks.
    /// PROJECT: only top-damage Character receives progress, not donor party fan-out.
    // Keep prepare_herald_quest_request ABI explicit: current runtime owner, independent fence/stamp, exact actor/session/source and occurrence/policy facts are separate admission inputs.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_herald_quest_request(
        &self,
        runtime: &mut ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
        actor: ExactActorRef,
        character_fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
        copy: &crate::durability::quest_state::QuestStateCopy,
        catalogue: std::sync::Arc<crate::durability::quest_state::quest::QuestStateCatalogue>,
    ) -> Result<Option<crate::crystal_herald_quest::HeraldQuestRequest>, CrystalDeathCompositionError>
    {
        self.current_owner(runtime, fence, stamp)?;
        if character_fence.runtime_scope != RuntimeScopeRefV1::channel(self.world, self.channel)
            || character_fence.scope_ownership_generation != self.generation
            || !self
                .entries
                .iter()
                .any(|entry| entry.target == "oteryn:creature.herald_of_fire")
            || runtime
                .native_summon_role(actor)
                .map_err(CrystalDeathCompositionError::Owner)?
                .is_some()
        {
            return Err(CrystalDeathCompositionError::Source(
                "Herald source scope or intrinsic-role mismatch",
            ));
        }
        let mut owner = runtime.borrow_combat_death();
        if owner
            .creature_target_identity(actor)
            .map_err(CrystalDeathCompositionError::Owner)?
            != b"oteryn:creature.herald_of_fire"
        {
            return Err(CrystalDeathCompositionError::Source(
                "Herald actual physical source mismatch",
            ));
        }
        owner
            .projected_death(actor)
            .map_err(CrystalDeathCompositionError::Owner)?;
        let Some(top) = owner
            .top_damage_character(actor)
            .map_err(CrystalDeathCompositionError::Owner)?
        else {
            return Ok(None);
        };
        if top.as_bytes() != character_fence.character_id.as_bytes() {
            return Err(CrystalDeathCompositionError::Source(
                "Herald recipient is not physical top-damage Character",
            ));
        }
        let Some(transition) =
            crate::crystal_herald_quest::select_transition(&catalogue, copy.tracks()).map_err(
                |_| {
                    CrystalDeathCompositionError::Source(
                        "Herald exact grouped quest definition unavailable",
                    )
                },
            )?
        else {
            return Ok(None);
        };
        let (bytes, _) = owner
            .reward_occurrence(actor, *top.as_bytes())
            .map_err(CrystalDeathCompositionError::Owner)?;
        let occurrence =
            crate::durability::character_progression::ExperienceRewardOccurrence::from_bytes(bytes)
                .map_err(|_| {
                    CrystalDeathCompositionError::Source("Herald actual owner occurrence invalid")
                })?;
        Ok(Some(
            crate::crystal_herald_quest::HeraldQuestRequest::from_owner(
                character_fence,
                occurrence,
                transition,
                catalogue,
                stamp,
            ),
        ))
    }
}

impl CrystalDeathOwner {
    /// Read-only source qualification; does not mint mutation or reward authority.
    pub(crate) fn qualified_registry(&self) -> &QualifiedCrystalDeaths {
        &self.registry
    }
}

impl QualifiedCrystalDeaths {
    /// Pure current-owner check reused after the async Character slot wait. Full native
    /// activation/server/client/frame/map pins are required, not just a scope generation.
    pub(crate) fn validate_current_work(
        &self,
        runtime: &ChannelRuntimeV1,
        fence: &ScopeRuntimeFence,
        stamp: RuntimeWorkStamp,
    ) -> Result<(), CrystalDeathCompositionError> {
        self.current_owner(runtime, fence, stamp)
    }
}
#[cfg(test)]
mod herald_actual_source_tests {
    use super::*;
    use crate::content::*;
    use crate::crystal_herald_quest::{MISSION, QUEST};
    use crate::durability::quest_state::quest::loader::load_embedded_quest_state;
    use crate::durability::quest_state::{CommittedQuestTransition, QuestCause, QuestStateCopy};
    use crate::foundation::{AttackerCommand, CommandId, CommandRef, OwnerDamageCommand};
    #[test]
    #[ignore = "Retained actual eleven-document native source capture; PROJECT Character/runtime positions, not shipping map admission"]
    fn actual_herald_death_source_produces_stable_request_only_after_projection_and_for_top_character()
    -> Result<(), Box<dyn std::error::Error>> {
        let p = std::path::PathBuf::from(
            std::env::var_os("OTERYN_MONSTER_NATIVE_CAPTURE_ROOT")
                .ok_or("actual native capture environment required")?,
        );
        let project = capture_world_project(
            p.parent().ok_or("parent")?,
            p.file_name().ok_or("name")?,
            ProjectFilesystemLimits {
                project: ProjectEvidenceLimits {
                    max_documents: 11,
                    max_document_bytes: 96_000_000,
                    max_total_bytes: 160_000_000,
                    max_json_depth: 24,
                    max_decoded_fields: 2_120_000,
                    max_string_bytes: 43_000_000,
                    max_locator_bytes: 160,
                    max_locator_segments: 8,
                    max_reference_records: 70_000,
                    max_import_records: 16,
                    max_reimport_states: 404,
                },
                max_entries_per_directory_scan: 32,
                max_total_directory_entries_scanned: 201,
            },
        )?;
        let world = project.lower_reference_source()?.world_id;
        let mut runtime = crate::foundation::crystal_death_router_fixture(world);
        let mut owner =
            CrystalDeathOwner::bind(&project, &runtime).map_err(|e| format!("{e:?}"))?;
        let scope = RuntimeScopeRefV1::channel(world, runtime.binding().channel_id());
        let (fence, stamp) =
            crate::foundation::crystal_timer_fixture(scope, runtime.binding().scope_generation())?;
        let mut id = [41u8; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        let character = crate::domain::CharacterId::from_bytes(id)?;
        id[15] = 50;
        let session = GameSessionId::decode(&id)?;
        let character_fence =
            crate::durability::character_progression::CurrentCharacterGameplayFence {
                character_id: character,
                game_session_id: session,
                connection_generation: crate::foundation::ConnectionGeneration::new(1)?,
                character_lease_generation: 1,
                runtime_scope: scope,
                scope_ownership_generation: runtime.binding().scope_generation(),
                expected_character_revision: crate::domain::CharacterRevision::new(2)?,
            };
        let catalogue = std::sync::Arc::new(
            load_embedded_quest_state("source-herald-crystal00ce-r1")
                .map_err(|e| format!("{e:?}"))?
                .catalogue()
                .clone(),
        );
        let start = "oteryn:quest-transition/crystalserver/quest/u15_24/targuna/burning_heart/mission/npc_1";
        let t = catalogue
            .transition(start)
            .ok_or("source mission bootstrap")?;
        let changes = catalogue
            .evaluate(t, &Default::default(), 1)
            .map_err(|e| format!("{e:?}"))?;
        // Explicit local admission-copy fixture; actual SQL receipt generation is separately
        // tested on PostgreSQL. No fabricated receipt is used by production composition.
        let mut copy = QuestStateCopy::default();
        assert!(copy.apply(&CommittedQuestTransition {
            character_id: character,
            cause: QuestCause::Command(CommandRef::new(session, CommandId::new(1)?)),
            transition_key: start.into(),
            quest_key: QUEST.into(),
            original_character_revision: crate::domain::CharacterRevision::new(1)?,
            committed_character_revision: crate::domain::CharacterRevision::new(2)?,
            pinned_content_revision: "source-herald-crystal00ce-r1".into(),
            definition_hash: catalogue.definition_hash(QUEST).ok_or("quest hash")?,
            completes: false,
            changes,
            experience: None
        }));
        assert_eq!(copy.tracks().get(MISSION), Some(&1));
        let actor = runtime.crystal_router_fixture_actor(
            "oteryn:creature.herald_of_fire",
            10,
            crate::foundation::MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
        );
        // Attributed test owner commit uses the same bounded native contributor accumulator.
        runtime
            .borrow_exact_actor_commit()
            .commit_damage_for_attacker(
                actor,
                AttackerCommand::new(
                    crate::foundation::CharacterId::decode(character.as_bytes())?,
                    1,
                    CommandRef::new(session, CommandId::new(2)?),
                    0,
                ),
                OwnerDamageCommand {
                    target: b"oteryn:creature.herald_of_fire",
                    occurrence: b"ignored-by-attributed-owner",
                    binding: b"source-herald-test",
                    damage: 10,
                },
            )
            .map_err(|e| format!("{e:?}"))?;
        assert!(
            owner
                .qualified_registry()
                .prepare_herald_quest_request(
                    &mut runtime,
                    &fence,
                    stamp,
                    actor,
                    character_fence,
                    &copy,
                    std::sync::Arc::clone(&catalogue)
                )
                .is_err()
        );
        owner
            .project_and_dispatch(&mut runtime, &fence, stamp, actor, &[], None)
            .map_err(|e| format!("{e:?}"))?;
        let prepared = owner
            .qualified_registry()
            .prepare_herald_quest_request(
                &mut runtime,
                &fence,
                stamp,
                actor,
                character_fence,
                &copy,
                std::sync::Arc::clone(&catalogue),
            )
            .map_err(|e| format!("{e:?}"))?
            .ok_or("eligible source request")?;
        assert_eq!(
            prepared.selection_policy(),
            "PROJECT_TOP_DAMAGE_CHARACTER_ONLY_NOT_GLOBAL_PARTY_PARITY"
        );
        let occurrence = runtime
            .borrow_combat_death()
            .reward_occurrence(actor, *character.as_bytes())
            .map_err(|e| format!("{e:?}"))?;
        assert!(!occurrence.1);
        let repeated = owner
            .qualified_registry()
            .prepare_herald_quest_request(
                &mut runtime,
                &fence,
                stamp,
                actor,
                character_fence,
                &copy,
                std::sync::Arc::clone(&catalogue),
            )
            .map_err(|e| format!("{e:?}"))?
            .ok_or("same frozen source")?;
        assert_eq!(format!("{prepared:?}"), format!("{repeated:?}"));
        let mut wrong = character_fence;
        id[15] = 77;
        wrong.character_id = crate::domain::CharacterId::from_bytes(id)?;
        assert!(
            owner
                .qualified_registry()
                .prepare_herald_quest_request(
                    &mut runtime,
                    &fence,
                    stamp,
                    actor,
                    wrong,
                    &copy,
                    std::sync::Arc::clone(&catalogue)
                )
                .is_err()
        );
        let empty = QuestStateCopy::default();
        assert!(
            owner
                .qualified_registry()
                .prepare_herald_quest_request(
                    &mut runtime,
                    &fence,
                    stamp,
                    actor,
                    character_fence,
                    &empty,
                    std::sync::Arc::clone(&catalogue)
                )
                .map_err(|e| format!("{e:?}"))?
                .is_none()
        );
        let mut stale = character_fence;
        stale.runtime_scope =
            RuntimeScopeRefV1::channel(world, crate::foundation::ChannelId::decode(&id)?);
        assert!(
            owner
                .qualified_registry()
                .prepare_herald_quest_request(
                    &mut runtime,
                    &fence,
                    stamp,
                    actor,
                    stale,
                    &copy,
                    catalogue
                )
                .is_err()
        );
        Ok(())
    }
}
