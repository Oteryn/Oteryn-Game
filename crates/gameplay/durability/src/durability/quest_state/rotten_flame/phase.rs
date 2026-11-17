//! The source-global phase linearizes at the actual reserved FIRST think.
use super::*;

#[derive(Debug, Clone, Copy)]
pub struct BakPhaseOrigin {
    pub(super) scope: RuntimeScopeRefV1,
    pub(super) generation: crate::foundation::ScopeOwnershipGeneration,
    pub(super) epoch: u64,
    pub(super) activation: u64,
    pub(super) server: [u8; 32],
    pub(super) client: [u8; 32],
    pub(super) frame: [u8; 32],
    pub(super) map: [u8; 32],
}
impl BakPhaseOrigin {
    pub fn runtime_scope(&self) -> RuntimeScopeRefV1 {
        self.scope
    }
    pub fn generation(&self) -> u64 {
        self.generation.get()
    }
    pub fn current(&self, runtime: &ChannelRuntimeV1) -> bool {
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
}
fn digest(row: &sqlx::postgres::PgRow, key: &str) -> StoredResult<[u8; 32]> {
    row.try_get::<Vec<u8>, _>(key)?
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)
}
async fn origin_row(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    claim: &BakFirstThinkClaim,
) -> StoredResult<sqlx::postgres::PgRow> {
    let RuntimeScopeRefV1::Instance {
        world_id,
        instance_id,
    } = claim.scope
    else {
        return Err(DurabilityError::Unavailable);
    };
    sqlx::query("SELECT o.*,o.handoff_id::text AS handoff,o.source_channel_id::text AS channel,o.source_generation::text AS generation,o.reset_epoch::text AS epoch,o.activation_sequence::text AS activation FROM game_rotten_bak_phase_origins o JOIN game_activity_handoff_groups g USING(handoff_id) JOIN game_activity_runtime_entries e USING(handoff_id) WHERE g.world_id=encode($1,'hex')::uuid AND g.instance_id=encode($2,'hex')::uuid AND g.ownership_generation=$3::text::numeric(20,0) AND g.activity_key='oteryn:boss.bakragore' AND g.state=2 AND e.destination_server_artifact_digest=$4 AND e.destination_coordinate_frame_digest=$5 AND e.destination_map_revision_digest=$6 FOR SHARE OF o,g,e")
        .bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice()).bind(claim.generation.get().to_string())
        .bind(claim.server.as_slice()).bind(claim.frame.as_slice()).bind(claim.map.as_slice())
        .fetch_optional(&mut **tx).await?.ok_or(DurabilityError::Unavailable)
}
fn decode_origin(
    row: &sqlx::postgres::PgRow,
    world: crate::foundation::WorldId,
) -> StoredResult<BakPhaseOrigin> {
    Ok(BakPhaseOrigin {
        scope: RuntimeScopeRefV1::Channel {
            world_id: world,
            channel_id: crate::foundation::ChannelId::decode(&uuid_text(
                &row.try_get::<String, _>("channel")?,
            )?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        },
        generation: crate::foundation::ScopeOwnershipGeneration::new(numeric_u64(
            row,
            "generation",
        )?)
        .map_err(|_| DurabilityError::InvalidStoredState)?,
        epoch: numeric_u64(row, "epoch")?,
        activation: numeric_u64(row, "activation")?,
        server: digest(row, "server_digest")?,
        client: digest(row, "client_digest")?,
        frame: digest(row, "frame_digest")?,
        map: digest(row, "map_digest")?,
    })
}
async fn assert_origin(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    origin: BakPhaseOrigin,
) -> StoredResult<()> {
    if !super::super::super::character_progression::assert_runtime_scope_fence(
        tx,
        origin.scope,
        origin.generation,
        node,
    )
    .await?
    {
        return Err(DurabilityError::Unavailable);
    }
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = origin.scope
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let current:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_content_activations a WHERE a.world_id=encode($1,'hex')::uuid AND a.channel_id=encode($2,'hex')::uuid AND a.activation_sequence=$3::text::numeric(20,0) AND a.server_artifact_digest=$4 AND a.client_artifact_digest=$5 AND a.frame_binding_digest=$6 AND NOT EXISTS(SELECT 1 FROM game_content_activations later WHERE later.world_id=a.world_id AND later.channel_id=a.channel_id AND later.activation_sequence>a.activation_sequence)) AND coalesce((SELECT reset_epoch FROM game_world_reset_active WHERE world_id=encode($1,'hex')::uuid),0)=$7::text::numeric(20,0)")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(origin.activation.to_string())
        .bind(origin.server.as_slice()).bind(origin.client.as_slice()).bind(origin.frame.as_slice()).bind(origin.epoch.to_string())
        .fetch_one(&mut **tx).await?;
    if !current {
        return Err(DurabilityError::Unavailable);
    }
    Ok(())
}
/// Called only in the dedicated pre-drive transaction. Its caller must commit
/// before publishing HP/AI effects; the ordinary read pass always rolls back.
pub async fn capture_bak_first_think_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    claim: &BakFirstThinkClaim,
) -> StoredResult<bool> {
    if !super::super::super::character_progression::assert_runtime_scope_fence(
        tx,
        claim.scope,
        claim.generation,
        node,
    )
    .await?
    {
        return Err(DurabilityError::Unavailable);
    }
    let row = origin_row(tx, claim).await?;
    let header = uuid_text(&row.try_get::<String, _>("handoff")?)?;
    let origin = decode_origin(&row, claim.scope.world_id())?;
    assert_origin(tx, node, origin).await?;
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = origin.scope
    else {
        return Err(DurabilityError::InvalidStoredState);
    };
    lock_global_phase(
        tx,
        *world_id.as_bytes(),
        *channel_id.as_bytes(),
        origin.generation.get(),
        origin.epoch,
        origin.activation,
    )
    .await?;
    if let Some(binding)=sqlx::query_scalar::<_,Vec<u8>>("SELECT request_binding FROM game_rotten_bak_phase_preparations WHERE handoff_id=encode($1,'hex')::uuid").bind(header.as_slice()).fetch_optional(&mut **tx).await? {
        if binding!=claim.binding{return Err(DurabilityError::InvalidStoredState)}
        return Ok(false);
    }
    // Another phase cannot begin until its external fixed visuals are known.
    let pending:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_rotten_bak_phase_preparations p WHERE p.world_id=encode($1,'hex')::uuid AND p.source_channel_id=encode($2,'hex')::uuid AND p.source_generation=$3::text::numeric(20,0) AND p.reset_epoch=$4::text::numeric(20,0) AND p.activation_sequence=$5::text::numeric(20,0) AND NOT EXISTS(SELECT 1 FROM game_rotten_bak_phase_publications done WHERE done.handoff_id=p.handoff_id))")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(origin.generation.get().to_string()).bind(origin.epoch.to_string()).bind(origin.activation.to_string()).fetch_one(&mut **tx).await?;
    if pending {
        return Err(DurabilityError::Unavailable);
    }
    let written=sqlx::query("INSERT INTO game_rotten_bak_phase_preparations(handoff_id,essence_mask,source_phase_ordinal,world_id,source_channel_id,source_generation,reset_epoch,activation_sequence,think_actor_local_id,think_actor_local_generation,think_sequence,request_binding) SELECT encode($1,'hex')::uuid,coalesce((SELECT bit_or(1::integer<<p.flavor) FROM game_rotten_bak_preparations p WHERE p.world_id=encode($2,'hex')::uuid AND p.source_channel_id=encode($3,'hex')::uuid AND p.source_generation=$4::text::numeric(20,0) AND p.reset_epoch=$5::text::numeric(20,0) AND p.activation_sequence=$6::text::numeric(20,0) AND p.source_phase_ordinal=game_rotten_bak_global_phase_ordinal(encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric(20,0),$5::text::numeric(20,0),$6::text::numeric(20,0))),0)::smallint,game_rotten_bak_global_phase_ordinal(encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric(20,0),$5::text::numeric(20,0),$6::text::numeric(20,0))+1,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric(20,0),$5::text::numeric(20,0),$6::text::numeric(20,0),$7,$8::text::numeric(20,0),$9::text::numeric(20,0),$10")
        .bind(header.as_slice()).bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .bind(origin.generation.get().to_string()).bind(origin.epoch.to_string()).bind(origin.activation.to_string())
        .bind(i64::from(claim.actor.actor_local_id())).bind(claim.actor.actor_local_generation().to_string()).bind(claim.sequence.to_string()).bind(claim.binding.as_slice())
        .execute(&mut **tx).await?.rows_affected();
    if written != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    sqlx::query("INSERT INTO game_rotten_bak_preparation_claims(handoff_id,flavor,character_id,cause_id,cause_ordinal,transition_key) SELECT h.handoff_id,p.flavor,p.character_id,p.cause_id,p.cause_ordinal,p.transition_key FROM game_rotten_bak_phase_preparations h JOIN game_rotten_bak_preparations p ON (p.world_id,p.source_channel_id,p.source_generation,p.reset_epoch,p.activation_sequence,p.source_phase_ordinal)=(h.world_id,h.source_channel_id,h.source_generation,h.reset_epoch,h.activation_sequence,h.source_phase_ordinal-1) WHERE h.handoff_id=encode($1,'hex')::uuid ORDER BY p.flavor")
        .bind(header.as_slice()).execute(&mut **tx).await?;
    Ok(true)
}
pub(super) async fn read_claim(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    claim: &BakFirstThinkClaim,
) -> StoredResult<PreparedBakEssences> {
    if !super::super::super::character_progression::assert_runtime_scope_fence(
        tx,
        claim.scope,
        claim.generation,
        node,
    )
    .await?
    {
        return Err(DurabilityError::Unavailable);
    }
    let row = origin_row(tx, claim).await?;
    let header = uuid_text(&row.try_get::<String, _>("handoff")?)?;
    let origin = decode_origin(&row, claim.scope.world_id())?;
    assert_origin(tx, node, origin).await?;
    let receipt=sqlx::query("SELECT essence_mask,request_binding,EXISTS(SELECT 1 FROM game_rotten_bak_phase_publications done WHERE done.handoff_id=p.handoff_id) AS published FROM game_rotten_bak_phase_preparations p WHERE handoff_id=encode($1,'hex')::uuid")
        .bind(header.as_slice()).fetch_optional(&mut **tx).await?.ok_or(DurabilityError::Unavailable)?;
    if receipt.try_get::<Vec<u8>, _>("request_binding")? != claim.binding {
        return Err(DurabilityError::InvalidStoredState);
    }
    let mask = u8::try_from(receipt.try_get::<i16, _>("essence_mask")?)
        .ok()
        .filter(|m| *m <= 15)
        .ok_or(DurabilityError::InvalidStoredState)?;
    Ok(PreparedBakEssences {
        scope: claim.scope,
        generation: claim.generation,
        activation: claim.activation,
        server: claim.server,
        client: claim.client,
        frame: claim.frame,
        map: claim.map,
        mask,
        header,
        origin: Some(origin),
        published: receipt.try_get("published")?,
        request_binding: claim.binding,
    })
}

/// The actual current origin map owner supplies eight literal-tile results;
/// missing source coverage is None for the whole read, never a nil Lua Item.
pub trait BakPublicResetRead {
    fn current(&self, runtime: &ChannelRuntimeV1) -> bool;
    fn reset_slots(&self) -> Option<[Option<FlameVisualRef>; 8]>;
    fn matches_after(&self, slots: &[Option<FlameVisualRef>; 8]) -> bool;
}
#[derive(Debug, Clone)]
pub struct BakPublicReset {
    prepared: PreparedBakEssences,
    slots: [Option<FlameVisualRef>; 8],
}
impl BakPublicReset {
    pub fn from_current_owner<R: BakPublicResetRead>(
        runtime: &ChannelRuntimeV1,
        prepared: PreparedBakEssences,
        read: &R,
    ) -> Option<Self> {
        let origin = prepared.origin?;
        if !prepared.publication_pending() || !origin.current(runtime) || !read.current(runtime) {
            return None;
        }
        let slots = read.reset_slots()?;
        for (i, slot) in slots.iter().enumerate() {
            let source = flames::FLAMES.get(i / 2)?;
            let (position, before, after) = if i % 2 == 0 {
                (source.flame, source.active, source.inactive)
            } else {
                (
                    source.pillar,
                    flames::PILLAR_ACTIVE,
                    flames::PILLAR_INACTIVE,
                )
            };
            if let Some(slot) = slot
                && (slot.before != before
                    || slot.after != after
                    || slot.position.x != position[0]
                    || slot.position.y != position[1]
                    || slot.position.floor != i16::try_from(position[2]).ok()?
                    || slot.revision == u64::MAX)
            {
                return None;
            }
        }
        Some(Self { prepared, slots })
    }
    pub fn visual_changes(&self) -> &[Option<FlameVisualRef>; 8] {
        &self.slots
    }
    pub fn published<R: BakPublicResetRead>(
        &self,
        runtime: &ChannelRuntimeV1,
        read: &R,
    ) -> Option<BakPhasePublication> {
        if !self.prepared.origin?.current(runtime)
            || !read.current(runtime)
            || !read.matches_after(&self.slots)
        {
            return None;
        }
        let mut results = [0_u8; 168];
        for (i, slot) in self.slots.iter().enumerate() {
            let source = flames::FLAMES.get(i / 2)?;
            let (before, after) = if i % 2 == 0 {
                (source.active, source.inactive)
            } else {
                (flames::PILLAR_ACTIVE, flames::PILLAR_INACTIVE)
            };
            let offset = i * 21;
            if let Some(slot) = slot {
                results[offset] = 1;
                results[offset + 1..offset + 9].copy_from_slice(&slot.key.to_be_bytes());
                results[offset + 9..offset + 17]
                    .copy_from_slice(&(slot.revision + 1).to_be_bytes());
            }
            results[offset + 17..offset + 19].copy_from_slice(&before.to_be_bytes());
            results[offset + 19..offset + 21].copy_from_slice(&after.to_be_bytes());
        }
        Some(BakPhasePublication {
            prepared: self.prepared,
            results,
        })
    }
}
#[derive(Debug, Clone, Copy)]
pub struct BakPhasePublication {
    prepared: PreparedBakEssences,
    results: [u8; 168],
}
impl DurabilityRoot {
    pub async fn commit_bak_phase_publication(
        &self,
        node: &NodeIncarnationProof,
        input: &BakPhasePublication,
    ) -> Result<PreparedBakEssences> {
        let input = *input;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=super::super::super::db::begin_semantic_transaction(holder,deadline).await?;
            let origin=input.prepared.origin.ok_or(DurabilityError::InvalidStoredState)?;
            assert_origin(&mut tx,&node,origin).await?;
            if !super::super::super::character_progression::assert_runtime_scope_fence(&mut tx,input.prepared.scope,input.prepared.generation,&node).await?{return Ok(Err(CharacterProgressionError::AuthorityRejected))}
            let RuntimeScopeRefV1::Channel{world_id,channel_id}=origin.scope else{return Err(DurabilityError::InvalidStoredState)};
            lock_global_phase(&mut tx,*world_id.as_bytes(),*channel_id.as_bytes(),origin.generation.get(),origin.epoch,origin.activation).await?;
            let fact=node.fact();
            let inserted=sqlx::query("INSERT INTO game_rotten_bak_phase_publications(handoff_id,request_binding,reset_results,node_id,node_registration_revision) VALUES(encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid,$5::text::numeric(20,0)) ON CONFLICT(handoff_id) DO NOTHING")
                .bind(input.prepared.header.as_slice()).bind(input.prepared.request_binding.as_slice()).bind(input.results.as_slice())
                .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).execute(&mut *tx).await?.rows_affected();
            if inserted==0 {
                let exact:bool=sqlx::query_scalar("SELECT request_binding=$2 AND reset_results=$3 FROM game_rotten_bak_phase_publications WHERE handoff_id=encode($1,'hex')::uuid")
                    .bind(input.prepared.header.as_slice()).bind(input.prepared.request_binding.as_slice()).bind(input.results.as_slice()).fetch_one(&mut *tx).await?;
                if !exact{return Err(DurabilityError::InvalidStoredState)}
            }
            super::super::super::db::commit_semantic_transaction(tx,deadline).await?;
            let mut prepared=input.prepared;prepared.published=true;
            Ok(Ok(prepared))
        })).await?
    }
}
