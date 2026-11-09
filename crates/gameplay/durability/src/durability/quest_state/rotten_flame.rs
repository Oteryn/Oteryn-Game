//! Literal public flame USE: storage burn and immutable source-global phase provenance.
//! No Item is minted/consumed by this action; the existing Quest owner is its writer.
pub mod phase;
pub use phase::{BakPhaseOrigin, capture_bak_first_think_in_transaction};

use super::super::quest_world_callback::QuestWorldContext;
use super::*;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementLocalPosition};
use oteryn_gameplay_quests::quest::rotten_blood::flames;

/// Borrowed current World facts, supplied by the existing selected-object owner.
/// This port resolves an actual server placement, never an acceptance flag.
#[derive(Debug, Clone, Copy)]
pub struct FlameVisualRef {
    pub key: u64,
    pub revision: u64,
    pub position: MovementLocalPosition,
    pub before: u16,
    pub after: u16,
}
pub trait RottenFlameRead {
    fn current(&self, runtime: &ChannelRuntimeV1) -> bool;
    fn selected(&self) -> Option<(u16, MovementLocalPosition, u64, u64)>;
    fn fixed_visuals(&self, source: &flames::FlameSource) -> Option<[Option<FlameVisualRef>; 2]>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlameNoChange {
    NoEssence,
    AlreadyActive,
}
#[derive(Debug, Clone)]
pub struct RottenFlameUse {
    request: QuestTransitionRequest,
    actor: ExactActorRef,
    scope: RuntimeScopeRefV1,
    generation: crate::foundation::ScopeOwnershipGeneration,
    context: QuestWorldContext,
    selected: (u16, MovementLocalPosition, u64, u64),
    flavor: u8,
    visuals: [Option<FlameVisualRef>; 2],
    no_change: Arc<std::sync::OnceLock<FlameNoChange>>,
    activation: u64,
    server: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
}
impl RottenFlameUse {
    pub fn from_current_owner<R: RottenFlameRead>(
        runtime: &ChannelRuntimeV1,
        native: &crate::content::native_gameplay::NativeGameplayState,
        actor: ExactActorRef,
        command: CommandRef,
        context: QuestWorldContext,
        reader: &R,
    ) -> Option<Self> {
        runtime.owner_fence().ok()?;
        let pin = runtime.content_pin();
        if !native.qualifies_rotten_blood_callbacks(pin.server_artifact_digest())
            || native.source_digest() != pin.server_artifact_digest()
            || !reader.current(runtime)
            || runtime
                .player_control_facts(actor, command.game_session_id())
                .ok()?
                .control_loss
                .is_some()
        {
            return None;
        }
        let movement = runtime.read_actor_position(actor).ok()?;
        let selected = reader.selected()?;
        let (flavor, source) = flames::FLAMES
            .iter()
            .enumerate()
            .find(|(_, source)| source.inactive == selected.0)?;
        let at = movement.position();
        if at.floor != selected.1.floor
            || (at.x - selected.1.x).abs().max((at.y - selected.1.y).abs()) > 1
            || context.actor_placement != actor.placement_identity()
            || context.movement_revision != movement.revision()
            || context.placement_context != pin.frame_binding_digest()
        {
            return None;
        }
        let visuals = reader.fixed_visuals(source)?;
        for (role, visual) in visuals.iter().enumerate() {
            if let Some(visual) = visual {
                let (position, before, after) = if role == 0 {
                    (source.flame, source.inactive, source.active)
                } else {
                    (
                        source.pillar,
                        flames::PILLAR_INACTIVE,
                        flames::PILLAR_ACTIVE,
                    )
                };
                if visual.position
                    != (MovementLocalPosition {
                        x: position[0],
                        y: position[1],
                        floor: i16::try_from(position[2]).ok()?,
                    })
                    || visual.before != before
                    || visual.after != after
                    || visual.revision.checked_add(1).is_none()
                {
                    return None;
                }
            }
        }
        Some(Self {
            no_change: Arc::new(std::sync::OnceLock::new()),
            visuals,
            request: QuestTransitionRequest {
                transition_key: source.key.into(),
                cause: QuestCause::Use(command),
            },
            actor,
            scope: runtime.binding().runtime_scope(),
            generation: runtime.binding().scope_generation(),
            context,
            selected,
            flavor: u8::try_from(flavor).ok()?,
            activation: pin.activation_sequence(),
            server: pin.server_artifact_digest(),
            client: pin.client_artifact_digest(),
            frame: pin.frame_binding_digest(),
            map: pin.map_revision_digest(),
        })
    }
    pub fn no_change(&self) -> Option<FlameNoChange> {
        self.no_change.get().copied()
    }
    pub fn visual_changes(&self) -> &[Option<FlameVisualRef>; 2] {
        &self.visuals
    }
    pub fn current_scope(&self, runtime: &ChannelRuntimeV1) -> bool {
        let pin = runtime.content_pin();
        runtime.owner_fence().is_ok()
            && runtime.binding().runtime_scope() == self.scope
            && runtime.binding().scope_generation() == self.generation
            && pin.activation_sequence() == self.activation
            && pin.server_artifact_digest() == self.server
            && pin.client_artifact_digest() == self.client
            && pin.frame_binding_digest() == self.frame
            && pin.map_revision_digest() == self.map
    }
    pub(super) fn request(
        &self,
        fence: &CurrentCharacterGameplayFence,
    ) -> Result<QuestTransitionRequest> {
        let QuestCause::Use(command) = self.request.cause else {
            return Err(CharacterProgressionError::InvalidInput);
        };
        if command.game_session_id() != fence.game_session_id
            || self.scope != fence.runtime_scope
            || self.generation != fence.scope_ownership_generation
            || self.actor.runtime_scope() != self.scope
            || flames::by_key(&self.request.transition_key).is_none()
        {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        Ok(self.request.clone())
    }
    pub(super) fn binding(&self, character: CharacterId) -> [u8; 33] {
        let mut result = request_binding(character, &self.request);
        let mut hash = Sha256::new()
            .chain_update(result)
            .chain_update(b"oteryn:rotten-flame-use:v1")
            .chain_update(self.actor.placement_identity())
            .chain_update(self.selected.0.to_be_bytes())
            .chain_update(self.selected.1.x.to_be_bytes())
            .chain_update(self.selected.1.y.to_be_bytes())
            .chain_update(self.selected.1.floor.to_be_bytes())
            .chain_update(self.selected.2.to_be_bytes())
            .chain_update(self.selected.3.to_be_bytes())
            .chain_update(self.activation.to_be_bytes())
            .chain_update(self.server)
            .chain_update(self.client)
            .chain_update(self.frame)
            .chain_update(self.map);
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
        for visual in &self.visuals {
            hash.update([u8::from(visual.is_some())]);
            if let Some(visual) = visual {
                hash.update(visual.key.to_be_bytes());
                hash.update(visual.revision.to_be_bytes());
                hash.update(visual.before.to_be_bytes());
                hash.update(visual.after.to_be_bytes());
            }
        }
        hash.update([self.flavor]);
        result[1..].copy_from_slice(&hash.finalize());
        result
    }
}

pub(super) async fn record(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    input: &RottenFlameUse,
    binding: &[u8; 33],
) -> StoredResult<()> {
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = input.scope
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let (cause, ordinal) = input.request.cause.key();
    let written=sqlx::query("INSERT INTO game_rotten_bak_preparations(character_id,cause_id,cause_ordinal,transition_key,request_binding,world_id,source_channel_id,source_generation,reset_epoch,activation_sequence,server_digest,client_digest,frame_digest,map_digest,flavor,flame_key,flame_revision,pillar_key,pillar_revision,committed_revision,source_phase_ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric(20,0),$4,$5,encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8::text::numeric(20,0),$9::text::numeric(20,0),$10::text::numeric(20,0),$11,$12,$13,$14,$15,$16::text::numeric(20,0),$17::text::numeric(20,0),$18::text::numeric(20,0),$19::text::numeric(20,0),(SELECT committed_character_revision FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND cause_id=encode($2,'hex')::uuid AND cause_ordinal=$3::text::numeric(20,0) AND transition_key=$4),game_rotten_bak_global_phase_ordinal(encode($6,'hex')::uuid,encode($7,'hex')::uuid,$8::text::numeric(20,0),$9::text::numeric(20,0),$10::text::numeric(20,0)))")
        .bind(fence.character_id.as_bytes().as_slice()).bind(cause.as_slice()).bind(ordinal.to_string())
        .bind(&input.request.transition_key).bind(binding.as_slice()).bind(world_id.as_bytes().as_slice())
        .bind(channel_id.as_bytes().as_slice()).bind(input.generation.get().to_string())
        .bind(input.context.reset_epoch.to_string()).bind(input.activation.to_string())
        .bind(input.server.as_slice()).bind(input.client.as_slice()).bind(input.frame.as_slice())
        .bind(input.map.as_slice()).bind(i16::from(input.flavor))
        .bind(input.visuals[0].map(|v|v.key.to_string())).bind(input.visuals[0].map(|v|(v.revision+1).to_string()))
        .bind(input.visuals[1].map(|v|v.key.to_string())).bind(input.visuals[1].map(|v|(v.revision+1).to_string())).execute(&mut **tx).await?;
    if written.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}
impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "the existing Quest owner holds its current source and world context"
    )]
    pub async fn commit_rotten_flame_use(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        input: RottenFlameUse,
        catalogue: Arc<QuestStateCatalogue>,
        permit: &super::super::spell_owner_commit::SpellLanePermit,
    ) -> Result<QuestTransitionOutcome> {
        let request = input.request(&fence)?;
        let context = input.context.clone();
        self.commit_quest_inner::<_, 0>(
            authority,
            node,
            fence,
            request,
            catalogue,
            Some((permit, context, None)),
            None,
            None,
            None,
            &grizzly::NoGrizzlyPremium,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(input),
            None,
        )
        .await
    }
}

/// Called only after the existing entry owner has locked the entire real roster
/// and inserted its provenance, in that same Type2 transaction.
pub(in crate::durability) async fn record_entry_origin(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    request: &super::super::activity_handoff::ActivityEnterRequest,
) -> StoredResult<()> {
    if request.activity_key != "oteryn:boss.bakragore" {
        return Ok(());
    }
    let written=sqlx::query("INSERT INTO game_rotten_bak_phase_origins(handoff_id,world_id,source_channel_id,source_generation,reset_epoch,activation_sequence,server_digest,client_digest,frame_digest,map_digest) SELECT g.handoff_id,m.world_id,m.origin_channel_id,m.source_ownership_generation,coalesce((SELECT reset_epoch FROM game_world_reset_active WHERE world_id=m.world_id),0),a.activation_sequence,a.server_artifact_digest,a.client_artifact_digest,a.frame_binding_digest,m.map_revision_digest FROM game_activity_handoff_groups g JOIN game_activity_runtime_entries e USING(handoff_id) JOIN game_activity_handoff_participants m ON m.handoff_id=g.handoff_id AND m.character_id=e.initiating_character_id JOIN LATERAL(SELECT * FROM game_content_activations a WHERE a.world_id=m.world_id AND a.channel_id=m.origin_channel_id ORDER BY activation_sequence DESC LIMIT 1)a ON true WHERE g.handoff_id=encode($1,'hex')::uuid AND g.state=1 AND a.server_artifact_digest=m.server_artifact_digest AND a.frame_binding_digest=m.coordinate_frame_digest")
      .bind(request.handoff_id.as_slice()).execute(&mut ***tx).await?;
    if written.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

/// Current DATA from one real committed handoff, not an essence/taint grant.
#[derive(Debug, Clone, Copy)]
pub struct PreparedBakEssences {
    scope: RuntimeScopeRefV1,
    generation: crate::foundation::ScopeOwnershipGeneration,
    activation: u64,
    server: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
    mask: u8,
    header: [u8; 16],
    origin: Option<BakPhaseOrigin>,
    published: bool,
    request_binding: [u8; 32],
}
impl PreparedBakEssences {
    pub fn origin(&self) -> Option<BakPhaseOrigin> {
        self.origin
    }
    pub fn publication_pending(&self) -> bool {
        !self.published
    }
    pub fn current_mask(&self, runtime: &ChannelRuntimeV1) -> Option<u8> {
        let pin = runtime.content_pin();
        (self.published
            && runtime.owner_fence().is_ok()
            && runtime.binding().runtime_scope() == self.scope
            && runtime.binding().scope_generation() == self.generation
            && pin.activation_sequence() == self.activation
            && pin.server_artifact_digest() == self.server
            && pin.client_artifact_digest() == self.client
            && pin.frame_binding_digest() == self.frame
            && pin.map_revision_digest() == self.map)
            .then_some(self.mask)
    }
}
/// Reuses the existing fenced scope read transaction. Missing header is unknown,
/// while the actual header0 is explicit evidence of an unprepared entry.
pub async fn read_prepared_bak_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    authority: &super::super::spell_item_transaction::SpellItemScopeAuthority,
    runtime: &ChannelRuntimeV1,
) -> StoredResult<Option<PreparedBakEssences>> {
    use super::super::spell_item_transaction::check_scope_transaction;
    if authority.runtime_scope() != runtime.binding().runtime_scope()
        || authority.generation() != runtime.binding().scope_generation().get()
        || runtime.owner_fence().is_err()
    {
        return Err(DurabilityError::Unavailable);
    }
    check_scope_transaction(tx, authority)
        .await
        .map_err(|_| DurabilityError::Unavailable)?;
    let RuntimeScopeRefV1::Instance {
        world_id,
        instance_id,
    } = runtime.binding().runtime_scope()
    else {
        return Ok(None);
    };
    let pin = runtime.content_pin();
    let mask:Option<i16>=sqlx::query_scalar("SELECT p.essence_mask FROM game_rotten_bak_phase_preparations p JOIN game_activity_handoff_groups g USING(handoff_id) JOIN game_activity_runtime_entries e USING(handoff_id) WHERE g.world_id=encode($1,'hex')::uuid AND g.instance_id=encode($2,'hex')::uuid AND g.ownership_generation=$3::text::numeric(20,0) AND g.activity_key='oteryn:boss.bakragore' AND g.state=2 AND e.destination_server_artifact_digest=$4 AND e.destination_coordinate_frame_digest=$5 AND e.destination_map_revision_digest=$6 AND EXISTS(SELECT 1 FROM game_rotten_bak_phase_publications done WHERE done.handoff_id=p.handoff_id)")
        .bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice())
        .bind(runtime.binding().scope_generation().get().to_string())
        .bind(pin.server_artifact_digest().as_slice()).bind(pin.frame_binding_digest().as_slice())
        .bind(pin.map_revision_digest().as_slice()).fetch_optional(&mut **tx).await?;
    check_scope_transaction(tx, authority)
        .await
        .map_err(|_| DurabilityError::Unavailable)?;
    let Some(mask) = mask else {
        return Ok(None);
    };
    let mask = u8::try_from(mask)
        .ok()
        .filter(|m| *m <= 15)
        .ok_or(DurabilityError::InvalidStoredState)?;
    Ok(Some(PreparedBakEssences {
        scope: runtime.binding().runtime_scope(),
        generation: runtime.binding().scope_generation(),
        activation: pin.activation_sequence(),
        server: pin.server_artifact_digest(),
        client: pin.client_artifact_digest(),
        frame: pin.frame_binding_digest(),
        map: pin.map_revision_digest(),
        mask,
        header: [0; 16],
        origin: None,
        published: true,
        request_binding: [0; 32],
    }))
}

/// Captured ONLY from the actual reserved native FIRST Bakragore think.
/// The original actor/sequence and full current owner stamp survive async retry.
#[derive(Debug, Clone)]
pub struct BakFirstThinkClaim {
    actor: ExactActorRef,
    sequence: u64,
    stamp: crate::foundation::RuntimeWorkStamp,
    scope: RuntimeScopeRefV1,
    generation: crate::foundation::ScopeOwnershipGeneration,
    activation: u64,
    server: [u8; 32],
    client: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
    binding: [u8; 32],
}
impl BakFirstThinkClaim {
    pub fn from_current_owner(
        runtime: &mut ChannelRuntimeV1,
        native: &crate::content::native_gameplay::NativeGameplayState,
        occurrence: crate::foundation::ThinkOccurrence,
    ) -> Option<Self> {
        if !matches!(
            runtime.binding().runtime_scope(),
            RuntimeScopeRefV1::Instance { .. }
        ) || !native
            .qualifies_rotten_blood_callbacks(runtime.content_pin().server_artifact_digest())
            || native.source_digest() != runtime.content_pin().server_artifact_digest()
            || !runtime
                .matches_live_creature_identity(occurrence.actor, b"oteryn:creature.bakragore")
            || runtime.native_summon_master(occurrence.actor).is_some()
        {
            return None;
        }
        let snapshot = runtime.companion_snapshot(occurrence.actor).ok()?;
        if snapshot.state.master.is_some() || snapshot.health <= 0 {
            return None;
        }
        let stamp = runtime.issue_owner_work().ok()?;
        let pin = runtime.content_pin();
        let binding = Sha256::new()
            .chain_update(b"oteryn:bak-first-think-global-phase:v1")
            .chain_update(occurrence.actor.placement_identity())
            .chain_update(occurrence.actor.actor_local_generation().to_be_bytes())
            .chain_update(occurrence.sequence.to_be_bytes())
            .chain_update(pin.activation_sequence().to_be_bytes())
            .chain_update(pin.server_artifact_digest())
            .chain_update(pin.client_artifact_digest())
            .chain_update(pin.frame_binding_digest())
            .chain_update(pin.map_revision_digest())
            .finalize()
            .into();
        Some(Self {
            actor: occurrence.actor,
            sequence: occurrence.sequence,
            stamp,
            scope: runtime.binding().runtime_scope(),
            generation: runtime.binding().scope_generation(),
            activation: pin.activation_sequence(),
            server: pin.server_artifact_digest(),
            client: pin.client_artifact_digest(),
            frame: pin.frame_binding_digest(),
            map: pin.map_revision_digest(),
            binding,
        })
    }
    pub fn current(&self, runtime: &ChannelRuntimeV1) -> bool {
        let pin = runtime.content_pin();
        runtime
            .owner_fence()
            .is_ok_and(|f| f.accepts_stamp(self.stamp))
            && runtime.binding().runtime_scope() == self.scope
            && runtime.binding().scope_generation() == self.generation
            && pin.activation_sequence() == self.activation
            && pin.server_artifact_digest() == self.server
            && pin.client_artifact_digest() == self.client
            && pin.frame_binding_digest() == self.frame
            && pin.map_revision_digest() == self.map
    }
}
impl DurabilityRoot {
    /// Closed source-event owner. No Character recovery or fake Item cause is
    /// substituted for the existing native Instance/source-Channel node fences.
    pub async fn commit_bak_first_think(
        &self,
        node: &NodeIncarnationProof,
        input: &BakFirstThinkClaim,
    ) -> Result<PreparedBakEssences> {
        let input = input.clone();
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        super::super::db::begin_semantic_transaction(holder, deadline).await?;
                    let result = phase::read_claim(&mut tx, &node, &input).await?;
                    super::super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(result))
                })
            })
            .await?
    }
}

/// All public Players and private first-think consumers serialize this SAME
/// external phase key. Hashes serialize work only; full columns prove identity.
pub(super) async fn lock_global_phase(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    world: [u8; 16],
    channel: [u8; 16],
    generation: u64,
    epoch: u64,
    activation: u64,
) -> StoredResult<()> {
    let encode = |id: [u8; 16]| id.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let key = format!(
        "oteryn:bak-global-phase:{}:{}:{generation}:{epoch}:{activation}",
        encode(world),
        encode(channel)
    );
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(key)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
pub(super) async fn admit(
    tx: &mut super::super::item_mint_audit::Type2Transaction<'_>,
    fence: CurrentCharacterGameplayFence,
    input: &RottenFlameUse,
) -> StoredResult<bool> {
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = input.scope
    else {
        return Ok(false);
    };
    if input.no_change.get().is_some() {
        return Ok(false);
    }
    let source = flames::FLAMES
        .get(usize::from(input.flavor))
        .ok_or(DurabilityError::InvalidStoredState)?;
    let value:Option<i64>=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2 FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).bind(source.storage).fetch_optional(&mut ***tx).await?;
    if value.unwrap_or(0) < 1 {
        let _ = input.no_change.set(FlameNoChange::NoEssence);
        #[cfg(any(test, feature = "test-support"))]
        eprintln!(
            "SRX3_FLAME_SOURCE_NO_ESSENCE_REACHED flavor={}",
            input.flavor
        );
        return Ok(false);
    }
    lock_global_phase(
        &mut *tx,
        *world_id.as_bytes(),
        *channel_id.as_bytes(),
        input.generation.get(),
        input.context.reset_epoch,
        input.activation,
    )
    .await?;
    let pending:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_rotten_bak_phase_preparations p WHERE p.world_id=encode($1,'hex')::uuid AND p.source_channel_id=encode($2,'hex')::uuid AND p.source_generation=$3::text::numeric(20,0) AND p.reset_epoch=$4::text::numeric(20,0) AND p.activation_sequence=$5::text::numeric(20,0) AND NOT EXISTS(SELECT 1 FROM game_rotten_bak_phase_publications done WHERE done.handoff_id=p.handoff_id))")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(input.generation.get().to_string()).bind(input.context.reset_epoch.to_string()).bind(input.activation.to_string()).fetch_one(&mut ***tx).await?;
    if pending {
        return Err(DurabilityError::Unavailable);
    }
    let active:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_rotten_bak_preparations WHERE world_id=encode($1,'hex')::uuid AND source_channel_id=encode($2,'hex')::uuid AND source_generation=$3::text::numeric(20,0) AND reset_epoch=$4::text::numeric(20,0) AND activation_sequence=$5::text::numeric(20,0) AND source_phase_ordinal=game_rotten_bak_global_phase_ordinal(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric(20,0),$4::text::numeric(20,0),$5::text::numeric(20,0)) AND flavor=$6)")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .bind(input.generation.get().to_string()).bind(input.context.reset_epoch.to_string()).bind(input.activation.to_string())
        .bind(i16::from(input.flavor)).fetch_one(&mut ***tx).await?;
    #[cfg(any(test, feature = "test-support"))]
    if active {
        eprintln!(
            "SRX3_FLAME_SOURCE_ALREADY_GLOBAL_REACHED flavor={}",
            input.flavor
        );
    }
    if active {
        let _ = input.no_change.set(FlameNoChange::AlreadyActive);
    }
    Ok(!active)
}
