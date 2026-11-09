//! Explicit M49 arrivals own admission; reset rolls never admit these clocks.
use super::*;

pub(in crate::durability) struct ElementalClockSource {
    pub world: WorldId,
    pub channel: ChannelId,
    pub generation: ScopeOwnershipGeneration,
    pub transaction: [u8; 16],
    pub map: [u8; 32],
    pub reset_epoch: u64,
}

/// Re-prove the live arrival lease and actual owner in the claiming transaction.
/// A result is a pending clock snapshot, not an actor installation capability.
pub(in crate::durability) async fn claim_elemental_clock(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    source: ElementalClockSource,
) -> Result<Option<(BossClockKey, BossClock)>> {
    assert_channel_owner(
        tx,
        recovery,
        node,
        source.world,
        source.channel,
        source.generation,
    )
    .await?;
    let reset: i64 = source
        .reset_epoch
        .try_into()
        .map_err(|_| BossClockError::InvalidInput)?;
    let row: Option<(String, i64)> = sqlx::query_as(
        "SELECT 'qn1.elemental:'||replace(b.transaction_id::text,'-',''), \
           floor(extract(epoch FROM clock_timestamp())*1000)::bigint \
         FROM game_elemental_boss_leases l JOIN game_elemental_summon_intents b USING(transaction_id) \
         JOIN LATERAL (SELECT server_artifact_digest,frame_binding_digest FROM game_content_activations \
           WHERE world_id=l.world_id AND channel_id=l.channel_id ORDER BY activation_sequence DESC LIMIT 1) a ON true \
         WHERE l.transaction_id=encode($1,'hex')::uuid AND l.world_id=encode($2,'hex')::uuid \
           AND l.channel_id=encode($3,'hex')::uuid AND l.reset_epoch=$4 \
           AND game_world_reset_map_epoch_current(l.world_id,$4::text::numeric) \
           AND b.map_revision='sha256:'||encode($5,'hex') \
           AND b.content_revision='sha256:'||encode(a.server_artifact_digest,'hex') \
           AND b.placement_context=a.frame_binding_digest"
    ).bind(source.transaction.as_slice()).bind(source.world.as_bytes().as_slice())
        .bind(source.channel.as_bytes().as_slice()).bind(reset).bind(source.map.as_slice())
        .fetch_optional(&mut **tx).await?;
    let Some((spawn, now)) = row else {
        return Ok(None);
    };
    let key = BossClockKey {
        world: source.world,
        channel: source.channel,
        spawn,
    };
    let Some(mut clock) = read_clock(tx, &key).await? else {
        return Err(BossClockError::MissingClock);
    };
    if clock.state != BossClockState::Alive || clock.reset_epoch != source.reset_epoch {
        return Ok(None);
    }
    if let Some(next) = plan_claim(&clock, source.generation, now)? {
        if !compare_and_set_clock(tx, &key, &clock, &next).await? {
            return Ok(None);
        }
        clock = next;
    }
    if clock.owner_generation != Some(source.generation) {
        return Ok(None);
    }
    Ok(Some((key, clock)))
}
