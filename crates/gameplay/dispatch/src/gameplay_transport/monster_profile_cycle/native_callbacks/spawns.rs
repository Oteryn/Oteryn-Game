//! Source-selected monster creations; the existing source owner retains the
//! callback clock/count/replay and the canonical carrier owns every admission.
use super::*;
use crate::content::bosses::activity_template::PinnedActivitySourceRoom as RoomKind;
use crate::content::{QualifiedCallbackSourceContext, QualifiedPublishedWorldContext};
use crate::foundation::{
    RuntimeWorkStamp,
    owner_timer::{OwnerClock, SemanticTimeMicros},
};
use crate::movement::source_placement::{SourcePlacementCell, SourcePlacementPlan};
use oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_spawn::{
    CallbackSpawnOwner, Kind, SourceCallbackSpawn, SourceSpawnFacts, SourceSpawnPlacement,
};

#[derive(Default)]
pub(super) struct NativeSpawns {
    owner: Option<CallbackSpawnOwner>,
    contexts: Vec<(RoomKind, Option<QualifiedCallbackSourceContext>)>,
    world: Option<QualifiedPublishedWorldContext>,
    world_checked: bool,
}
impl std::fmt::Debug for NativeSpawns {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeSpawns")
            .field(
                "pending",
                &self.owner.as_ref().map_or(0, |owner| owner.pending.len()),
            )
            .finish_non_exhaustive()
    }
}
struct Now(u64);
impl OwnerClock for Now {
    fn now(&self) -> SemanticTimeMicros {
        SemanticTimeMicros::from_micros(self.0)
    }
}
fn spawn_proposal(proposal: &ProfileAbilityProposal) -> bool {
    matches!(
        proposal.ability.key.as_str(),
        "oteryn:ability.spell.gaz_haragoth_summon"
            | "oteryn:ability.spell.glooth_generator_summon"
            | "oteryn:ability.spell.tenebris_summon"
            | "oteryn:ability.spell.mazoran_fire"
            | "oteryn:ability.spell.thorn_summon"
            | "oteryn:ability.spell.plagirath_summon"
            | "oteryn:ability.spell.generator"
            | "oteryn:ability.spell.razzagorn_summon"
            | "oteryn:ability.spell.shulgrax_summon"
            | "oteryn:ability.spell.soulcatcher_summon"
            | "oteryn:ability.spell.time_guardian_lost_time"
            | "oteryn:ability.spell.destruction_summon"
            | "oteryn:ability.spell.hunger_summon"
            | "oteryn:ability.spell.rage_summon"
            | "oteryn:ability.spell.devourer_summon"
    )
}
fn source_room(kind: Kind) -> Option<RoomKind> {
    Some(match kind {
        Kind::Tenebris => RoomKind::CallbackLadyTenebris,
        Kind::Mazoran => RoomKind::CallbackMazoran,
        Kind::Plagirath => RoomKind::CallbackPlagirath,
        Kind::Maxxen => RoomKind::CallbackProfessorMaxxen,
        Kind::Razzagorn => RoomKind::CallbackRazzagorn,
        Kind::Shulgrax => RoomKind::CallbackShulgrax,
        Kind::Blazing | Kind::Freezing => RoomKind::CallbackTimeGuardian,
        _ => return None,
    })
}
fn attempts(kind: Kind) -> usize {
    match kind {
        Kind::Gaz => 7,
        Kind::Mazoran | Kind::Plagirath | Kind::Razzagorn => 4,
        Kind::Shulgrax => 8,
        _ => 1,
    }
}
fn locations(source: &SourceCallbackSpawn, draw: u64) -> Result<Vec<SourceSpawnPlacement>, ()> {
    let anchor = |key: &str, area: bool| -> Result<SourceSpawnPlacement, ()> {
        let definition = source
            .anchors()
            .iter()
            .find(|anchor| anchor["key"].as_str() == Some(key))
            .cloned()
            .ok_or(())?;
        Ok(if area {
            SourceSpawnPlacement::Room {
                key: key.into(),
                definition,
            }
        } else {
            SourceSpawnPlacement::Anchor {
                key: key.into(),
                definition,
            }
        })
    };
    Ok(vec![match source.kind {
        Kind::Gaz | Kind::GeneratorRelease => SourceSpawnPlacement::Subject,
        Kind::Tenebris => anchor("tenebris_room", true)?,
        Kind::Mazoran => anchor("mazoran_summon_room", true)?,
        Kind::Thorn => SourceSpawnPlacement::Offset(3),
        Kind::Plagirath => anchor("plagirath_room", true)?,
        Kind::Maxxen => anchor(&format!("generator_landing_{}", draw % 4), false)?,
        Kind::Razzagorn => anchor("razzagorn_arena", true)?,
        Kind::Shulgrax => anchor("shulgrax_arena", true)?,
        Kind::Soulcatcher => SourceSpawnPlacement::Relative(0, -1),
        Kind::Blazing | Kind::Freezing => {
            SourceSpawnPlacement::Relative((draw % 5) as i32 - 2, ((draw / 5) % 5) as i32 - 2)
        }
        Kind::Destruction | Kind::Hunger | Kind::Rage | Kind::Devourer => {
            SourceSpawnPlacement::Offset(1)
        }
    }])
}
#[derive(PartialEq, Eq)]
enum LocationKey {
    Subject,
    Offset(u32),
    Relative(i32, i32),
    Anchor(String),
    Room(String),
}
fn key(location: &SourceSpawnPlacement) -> LocationKey {
    match location {
        SourceSpawnPlacement::Subject => LocationKey::Subject,
        SourceSpawnPlacement::Offset(radius) => LocationKey::Offset(*radius),
        SourceSpawnPlacement::Relative(x, y) => LocationKey::Relative(*x, *y),
        SourceSpawnPlacement::Anchor { key, .. } => LocationKey::Anchor(key.clone()),
        SourceSpawnPlacement::Room { key, .. } => LocationKey::Room(key.clone()),
    }
}
fn random(
    seed: u64,
    ordinal: usize,
    label: &[u8],
    counter: u64,
    min: i32,
    max: i32,
) -> Option<i32> {
    if min > max {
        return None;
    }
    let digest = Sha256::new()
        .chain_update(b"oteryn:native-source-placement:v1")
        .chain_update(seed.to_le_bytes())
        .chain_update((ordinal as u64).to_le_bytes())
        .chain_update(label)
        .chain_update(counter.to_le_bytes())
        .finalize();
    let mut bytes = [0; 8];
    bytes.copy_from_slice(&digest[..8]);
    Some(
        crate::spell::uniform_draw(u64::from_le_bytes(bytes), i64::from(min), i64::from(max))
            as i32,
    )
}
fn center(
    runtime: &ChannelRuntimeV1,
    parent: crate::foundation::ExactActorRef,
    location: &SourceSpawnPlacement,
    draw: u64,
    ordinal: usize,
    context: Option<&QualifiedCallbackSourceContext>,
) -> Option<MovementLocalPosition> {
    let at = runtime.read_actor_position(parent).ok()?.position();
    let relative = |dx: i32, dy: i32| {
        Some(MovementLocalPosition {
            x: at.x.checked_add(dx)?,
            y: at.y.checked_add(dy)?,
            ..at
        })
    };
    match location {
        SourceSpawnPlacement::Subject => Some(at),
        SourceSpawnPlacement::Offset(radius) => {
            let radius = i32::try_from(*radius).ok()?;
            relative(
                random(draw, ordinal, b"offset-x", 0, -radius, radius)?,
                random(draw, ordinal, b"offset-y", 0, -radius, radius)?,
            )
        }
        SourceSpawnPlacement::Relative(x, y) => relative(*x, *y),
        SourceSpawnPlacement::Anchor { definition, .. }
        | SourceSpawnPlacement::Room { definition, .. } => {
            let location = &definition["location"];
            let integer = |value: &serde_json::Value| i32::try_from(value.as_i64()?).ok();
            let (x, y, z) = if location["kind"].as_str() == Some("point") {
                (
                    integer(&location["x"])?,
                    integer(&location["y"])?,
                    integer(&location["floor"])?,
                )
            } else {
                let boxes = location["boxes"].as_array()?;
                if boxes.len() != 1 {
                    return None;
                }
                let area = &boxes[0];
                (
                    random(
                        draw,
                        ordinal,
                        b"room-x",
                        0,
                        integer(&area["x"][0])?,
                        integer(&area["x"][1])?,
                    )?,
                    random(
                        draw,
                        ordinal,
                        b"room-y",
                        0,
                        integer(&area["y"][0])?,
                        integer(&area["y"][1])?,
                    )?,
                    integer(&area["floor"])?,
                )
            };
            let (x, y, floor) = context?.source_frame().physical_position([x, y, z])?;
            Some(MovementLocalPosition { x, y, floor })
        }
    }
}
struct Prepared {
    actor: crate::foundation::ExactActorRef,
    child: Ref,
    location: LocationKey,
    draw: u64,
    ordinal: usize,
    center: MovementLocalPosition,
    plan: SourcePlacementPlan,
    context: Option<QualifiedCallbackSourceContext>,
    forced: bool,
    sight: Vec<(MovementLocalPosition, bool)>,
}
struct Reader<'a, 'map, 'db> {
    native: &'a NativeGameplayState,
    map: &'a CurrentMonsterMap<'map, 'db>,
    world: Option<&'a QualifiedPublishedWorldContext>,
    prepared: &'a [Prepared],
    selected: Option<(
        crate::foundation::ExactActorRef,
        Ref,
        MovementLocalPosition,
        bool,
    )>,
}
impl Reader<'_, '_, '_> {
    fn cell(
        &self,
        runtime: &ChannelRuntimeV1,
        child: &Ref,
        at: MovementLocalPosition,
        context: Option<&QualifiedCallbackSourceContext>,
    ) -> Option<SourcePlacementCell> {
        match (context, self.world) {
            (_, Some(world)) => self
                .map
                .source_placement_cell_in_world(runtime, &child.key, at, world),
            (Some(context), _) => self
                .map
                .source_placement_cell_in_callback(runtime, &child.key, at, context),
            _ => self.map.source_placement_cell(runtime, &child.key, at),
        }
    }
    fn exists(
        &self,
        runtime: &ChannelRuntimeV1,
        at: MovementLocalPosition,
        context: Option<&QualifiedCallbackSourceContext>,
    ) -> Option<bool> {
        match (context, self.world) {
            (_, Some(world)) => self.map.source_tile_exists_in_world(runtime, at, world),
            (Some(context), _) => self
                .map
                .source_tile_exists_in_callback(runtime, at, context),
            _ => self.map.source_tile_exists(runtime, at),
        }
    }
}
impl SourceSpawnFacts for Reader<'_, '_, '_> {
    fn current_native(&self) -> Option<&NativeGameplayState> {
        Some(self.native)
    }
    fn source_spawn_position(
        &mut self,
        _: &ChannelRuntimeV1,
        _: crate::foundation::ExactActorRef,
        _: &SourceSpawnPlacement,
        _: u64,
        _: usize,
        _: RuntimeWorkStamp,
    ) -> Option<MovementLocalPosition> {
        None
    }
    fn source_spawn_tile_allowed(
        &mut self,
        _: &ChannelRuntimeV1,
        _: crate::foundation::ExactActorRef,
        _: MovementLocalPosition,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        None
    }
    fn source_spawn_child_requested_center(
        &mut self,
        _: &ChannelRuntimeV1,
        parent: crate::foundation::ExactActorRef,
        child: &Ref,
        location: &SourceSpawnPlacement,
        draw: u64,
        ordinal: usize,
        _: RuntimeWorkStamp,
    ) -> Option<MovementLocalPosition> {
        let location_key = key(location);
        self.prepared
            .iter()
            .find(|p| {
                p.actor == parent
                    && p.child == *child
                    && p.location == location_key
                    && p.draw == draw
                    && p.ordinal == ordinal
            })
            .map(|p| p.center)
    }
    fn source_spawn_child_position(
        &mut self,
        runtime: &ChannelRuntimeV1,
        parent: crate::foundation::ExactActorRef,
        child: &Ref,
        location: &SourceSpawnPlacement,
        draw: u64,
        ordinal: usize,
        _: RuntimeWorkStamp,
    ) -> Option<MovementLocalPosition> {
        let location_key = key(location);
        let p = self.prepared.iter().find(|p| {
            p.actor == parent
                && p.child == *child
                && p.location == location_key
                && p.draw == draw
                && p.ordinal == ordinal
        })?;
        let selected = if p.forced && self.exists(runtime, p.center, p.context.as_ref())? {
            Some(p.center)
        } else {
            p.plan
                .select(
                    |at| self.cell(runtime, child, at, p.context.as_ref()),
                    |_, at| {
                        p.sight
                            .iter()
                            .find(|(candidate, _)| *candidate == at)
                            .map(|(_, clear)| *clear)
                    },
                )
                .ok()?
        };
        let (at, allowed) = match selected {
            Some(at) => {
                let allowed = if p.forced && at == p.center {
                    self.exists(runtime, at, p.context.as_ref())?
                } else {
                    self.cell(runtime, child, at, p.context.as_ref())?.query_add
                };
                (at, allowed)
            }
            None => (p.center, false),
        };
        self.selected = Some((parent, child.clone(), at, allowed));
        Some(at)
    }
    fn source_spawn_child_tile_allowed(
        &mut self,
        _: &ChannelRuntimeV1,
        parent: crate::foundation::ExactActorRef,
        child: &Ref,
        at: MovementLocalPosition,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        self.selected
            .as_ref()
            .filter(|(actor, key, position, _)| *actor == parent && key == child && *position == at)
            .map(|(_, _, _, allowed)| *allowed)
    }
    fn source_spawn_anchor_contains(
        &mut self,
        runtime: &ChannelRuntimeV1,
        parent: crate::foundation::ExactActorRef,
        anchor: &serde_json::Value,
        _: RuntimeWorkStamp,
    ) -> Option<bool> {
        let context = self
            .prepared
            .iter()
            .find(|p| p.actor == parent)?
            .context
            .as_ref()?;
        let at = runtime.read_actor_position(parent).ok()?.position();
        let source = context
            .source_frame()
            .source_position((at.x, at.y, at.floor))?;
        let boxes = anchor["location"]["boxes"].as_array()?;
        Some(boxes.iter().any(|area| {
            area["floor"].as_i64() == Some(i64::from(source[2]))
                && area["x"][0]
                    .as_i64()
                    .is_some_and(|min| i64::from(source[0]) >= min)
                && area["x"][1]
                    .as_i64()
                    .is_some_and(|max| i64::from(source[0]) <= max)
                && area["y"][0]
                    .as_i64()
                    .is_some_and(|min| i64::from(source[1]) >= min)
                && area["y"][1]
                    .as_i64()
                    .is_some_and(|max| i64::from(source[1]) <= max)
        }))
    }
}
impl NativeSpawns {
    pub(super) fn next_due(&self) -> Option<u64> {
        self.owner
            .as_ref()
            .and_then(CallbackSpawnOwner::next_due_time)
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "one existing native source owner pass"
    )]
    pub(super) async fn drive(
        &mut self,
        active: &crate::content::ActiveGeneration,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        proposals: &[ProfileAbilityProposal],
        now: u64,
    ) -> Result<(), ()> {
        self.drive_inner(active, runtime, room, native, map, proposals, now, now)
            .await
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "qualified source, current runtime, map, proposals and original cutoff require independent authority checks"
    )]
    pub(super) async fn drive_due_through(
        &mut self,
        active: &crate::content::ActiveGeneration,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        eligible_through_us: u64,
        now: u64,
    ) -> Result<(), ()> {
        self.drive_inner(
            active,
            runtime,
            room,
            native,
            map,
            &[],
            eligible_through_us,
            now,
        )
        .await
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "qualified source, current runtime, map, proposals and original cutoff require independent authority checks"
    )]
    async fn drive_inner(
        &mut self,
        active: &crate::content::ActiveGeneration,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        native: &NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        proposals: &[ProfileAbilityProposal],
        eligible_through_us: u64,
        now: u64,
    ) -> Result<(), ()> {
        if self.owner.is_none() && !proposals.iter().any(spawn_proposal) {
            return Ok(());
        }
        let stamp = runtime.issue_owner_work().map_err(|_| ())?;
        if !self.world_checked {
            self.world = active.qualify_published_world_context(room, runtime.content_pin());
            self.world_checked = true;
        }
        for proposal in proposals.iter().filter(|proposal| spawn_proposal(proposal)) {
            if !matches!(
                current_source_creature_role(runtime, proposal.issuer),
                Ok(CurrentSourceCreatureRole::Ordinary
                    | CurrentSourceCreatureRole::IntrinsicMonsterSummon(_))
            ) {
                continue;
            }
            let snapshot = runtime
                .companion_snapshot(proposal.issuer)
                .map_err(|_| ())?;
            let Some(record) = native.creature_profiles().records.iter().find(|record| {
                record.profile.target.key == snapshot.state.policy.definition_key
                    && record.profile.target.revision == snapshot.state.policy.definition_revision
            }) else {
                continue;
            };
            let Some(source) = SourceCallbackSpawn::qualify_native(
                native,
                &record.profile.target,
                proposal.entry_index,
                proposal.list,
            )
            .map_err(|_| ())?
            else {
                continue;
            };
            if let Some(kind) = source_room(source.kind) {
                if !self.contexts.iter().any(|(key, _)| *key == kind) {
                    self.contexts.push((
                        kind,
                        active.qualify_callback_source_context(room, runtime.content_pin(), kind),
                    ));
                }
                if !self.contexts.iter().any(|(key, proof)| {
                    *key == kind
                        && proof
                            .as_ref()
                            .is_some_and(|proof| proof.matches(room, runtime.content_pin()))
                }) {
                    return Err(());
                }
            }
            if self.owner.is_none() {
                self.owner = Some(CallbackSpawnOwner::new_current(runtime).map_err(|_| ())?);
            }
            self.owner
                .as_mut()
                .ok_or(())?
                .schedule_current(
                    runtime,
                    stamp,
                    &source,
                    proposal,
                    SemanticTimeMicros::from_micros(now),
                )
                .map_err(|_| ())?;
        }
        let Some(owner) = self.owner.as_ref() else {
            return Ok(());
        };
        if owner
            .next_due_time()
            .is_none_or(|due| due > now || due > eligible_through_us)
        {
            return Ok(());
        }
        let work = owner
            .pending
            .iter()
            .filter(|pending| {
                owner
                    .pending_due(pending)
                    .is_some_and(|due| due <= now && due <= eligible_through_us)
            })
            .map(|pending| (pending.source().clone(), pending.proposal().clone()))
            .collect::<Vec<_>>();
        let mut prepared = Vec::new();
        for (source, proposal) in work {
            if !runtime.matches_live_creature_identity(proposal.issuer, source.creature.as_bytes())
            {
                continue;
            }
            let context = source_room(source.kind)
                .and_then(|kind| self.contexts.iter().find(|(key, _)| *key == kind))
                .and_then(|(_, context)| context.clone());
            let draw =
                CallbackSpawnOwner::draw(source.content, &proposal, "selection").map_err(|_| ())?;
            for location in locations(&source, draw)? {
                for child in &source.children {
                    let token = child.native_source().ok_or(())?;
                    for ordinal in 0..attempts(source.kind) {
                        let center = center(
                            runtime,
                            proposal.issuer,
                            &location,
                            draw,
                            ordinal,
                            context.as_ref(),
                        )
                        .ok_or(())?;
                        let mut index = 0_u64;
                        let plan = SourcePlacementPlan::prepare(
                            center,
                            token.extended(),
                            token.forced(),
                            |bound| {
                                let value = random(
                                    draw,
                                    ordinal,
                                    b"placement-shuffle",
                                    index,
                                    0,
                                    (bound - 1) as i32,
                                )
                                .map(|value| value as u32)
                                .unwrap_or(bound);
                                index = index.saturating_add(1);
                                value
                            },
                        )
                        .map_err(|_| ())?;
                        let mut sight = Vec::new();
                        let force_existing = token.forced() && {
                            let reader = Reader {
                                native,
                                map,
                                world: self.world.as_ref(),
                                prepared: &[],
                                selected: None,
                            };
                            reader.exists(runtime, center, context.as_ref()).ok_or(())?
                        };
                        if !force_existing {
                            for at in plan.candidates() {
                                map.read(runtime, *at).await.ok_or(())?;
                                // Canary map.cpp:956-958: placeCreature passes
                                // floorCheck=false; same-floor0 needs no ray.
                                if center.floor == 0 {
                                    sight.push((*at, true));
                                    continue;
                                }
                                if token.extended() {
                                    let steps =
                                        crate::durability::native_world_read::source_sight_steps(
                                            center, *at,
                                        )
                                        .map_err(|_| ())?;
                                    for (step, exempt) in steps {
                                        if !exempt {
                                            map.read(runtime, step).await.ok_or(())?;
                                        }
                                    }
                                }
                                let reader = Reader {
                                    native,
                                    map,
                                    world: self.world.as_ref(),
                                    prepared: &[],
                                    selected: None,
                                };
                                let mut clear = true;
                                for (step, exempt) in
                                    crate::durability::native_world_read::source_sight_steps(
                                        center, *at,
                                    )
                                    .map_err(|_| ())?
                                {
                                    if !exempt
                                        && reader
                                            .exists(runtime, step, context.as_ref())
                                            .ok_or(())?
                                        && map.view().blocks_projectile(step).ok_or(())?
                                    {
                                        clear = false;
                                        break;
                                    }
                                }
                                sight.push((*at, clear));
                            }
                        }
                        prepared.push(Prepared {
                            actor: proposal.issuer,
                            child: child.creature.clone(),
                            location: key(&location),
                            draw,
                            ordinal,
                            center,
                            plan,
                            context: context.clone(),
                            forced: token.forced(),
                            sight,
                        });
                    }
                }
            }
        }
        if !map.validate_transaction(runtime).await {
            return Err(());
        }
        let mut reader = Reader {
            native,
            map,
            world: self.world.as_ref(),
            prepared: &prepared,
            selected: None,
        };
        // Validate all cached queryAdd/coverage facts before the first source
        // callback can physically publish a child. Selection re-reads occupancy.
        for p in &prepared {
            if p.forced
                && reader
                    .exists(runtime, p.center, p.context.as_ref())
                    .ok_or(())?
            {
                continue;
            }
            p.plan
                .select(
                    |at| reader.cell(runtime, &p.child, at, p.context.as_ref()),
                    |_, at| {
                        p.sight
                            .iter()
                            .find(|(candidate, _)| *candidate == at)
                            .map(|(_, clear)| *clear)
                    },
                )
                .map_err(|_| ())?;
        }
        let owner = self.owner.as_mut().ok_or(())?;
        let receipts = owner
            .drain_current_through(runtime, stamp, &mut reader, &Now(now), eligible_through_us)
            .map_err(|_| ())?;
        emit_speech(room, owner.take_source_speech());
        emit_effects(runtime, room, stamp, &receipts);
        if receipts.iter().any(|receipt| {
            !matches!(
                receipt,
                Ok(_) | Err(crate::creature_auto_attack::AttackError::StaleIssuer)
            )
        }) {
            return Err(());
        }
        Ok(())
    }
}
pub(super) fn emit_speech(
    room: &QualifiedNativeEntryRoom,
    lines: Vec<
        oteryn_gameplay_encounters::bosses::source_callbacks::callback_speech::CallbackSpeech,
    >,
) {
    use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
    use crate::map::boot::BundleCollisionIndexExt;
    use oteryn_protocol_oteryn::chat::{ChatLine, ChatSpeechMode};
    let NativeMovementCollisionIndex::Bundle(map) = room.movement_cells().index() else {
        return;
    };
    for speech in lines {
        let ChatLine::Local {
            speaker_name,
            text,
            mode,
            ..
        } = speech.line()
        else {
            continue;
        };
        let Some(position) = super::visual_position(speech.position()) else {
            continue;
        };
        let Some(name) =
            oteryn_protocol_oteryn::entity_names::EntityDisplayName::new(speaker_name).ok()
        else {
            continue;
        };
        map.map()
            .facts
            .emit_movement_speech(crate::map::facts::MovementSpeech {
                speaker: speech.actor(),
                position,
                talk_type: if *mode == ChatSpeechMode::Yell {
                    37
                } else {
                    36
                },
                text: text.clone(),
                captured_name: Some(name),
            });
    }
}

fn emit_effects(
    runtime: &ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    stamp: RuntimeWorkStamp,
    receipts:&[Result<oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_spawn::CallbackSpawnReceipt,crate::creature_auto_attack::AttackError>],
) {
    use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
    use crate::map::boot::BundleCollisionIndexExt;
    let NativeMovementCollisionIndex::Bundle(map) = room.movement_cells().index() else {
        return;
    };
    for receipt in receipts.iter().filter_map(|receipt| receipt.as_ref().ok()) {
        // Canary47df utils_definitions.hpp:71,74,80: TELEPORT11,
        // MAGIC_RED14,SOUND_RED20. This closed callback rule follows the
        // source script; it does not add aliases to the separate casting corpus.
        let effect = match receipt.ability.key.as_str() {
            "oteryn:ability.spell.gaz_haragoth_summon" if receipt.requested > 0 => 20,
            "oteryn:ability.spell.soulcatcher_summon" if receipt.requested > 0 => 11,
            "oteryn:ability.spell.tenebris_summon" | "oteryn:ability.spell.thorn_summon"
                if receipt.children.iter().any(|child| child.result.is_ok()) =>
            {
                14
            }
            _ => continue,
        };
        let Some(position) = runtime
            .read_actor_position(receipt.actor)
            .ok()
            .and_then(|position| super::visual_position(position.position()))
        else {
            continue;
        };
        if let Some(cue) = crate::map::facts::CreatureVisualCue::effect(
            runtime,
            stamp,
            receipt.actor,
            position,
            effect,
        ) {
            map.map().facts.emit_creature_visual(runtime, cue);
        }
    }
}
