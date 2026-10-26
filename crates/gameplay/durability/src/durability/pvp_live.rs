//! Current same-transaction PvP observations and write-ahead harm, shared by swings and spells.
use super::admission_journal::party_target_binding::current_colocated_party_character_in_transaction;
use super::character_progression::CharacterProgressionError;
use super::spell_field_policy::{FieldWorldType, read_world_field_policy_in_transaction};
use super::spell_item_transaction::{SpellItemAuthority, SpellItemError, check_transaction};
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{ChannelRuntimeV1, CommandRef, ExactActorRef, GameSessionId};
use oteryn_gameplay_combat_rules::pvp::{PvpRelation, WorldPvpType};
use sqlx::{Postgres, Row, Transaction};

#[path = "pvp_live_ledger.rs"]
mod ledger;

type Result<T> = std::result::Result<T, SpellItemError>;
fn refused() -> SpellItemError {
    SpellItemError::Rejected("current PvP facts unavailable")
}

/// Current World contract and skull for an expert fight-mode intent, under its session fence.
pub async fn read_expert_pvp_context_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    authority: &SpellItemAuthority,
) -> Result<(WorldPvpType, bool)> {
    check_transaction(tx, authority).await?;
    let policy = read_world_field_policy_in_transaction(
        tx,
        root,
        authority.runtime_scope(),
        authority.scope_generation(),
    )
    .await?
    .ok_or_else(refused)?;
    let world = match policy.bind(tx).mode() {
        FieldWorldType::NoPvp => WorldPvpType::Optional,
        FieldWorldType::Pvp => WorldPvpType::Open,
        FieldWorldType::PvpEnforced => WorldPvpType::Hardcore,
    };
    let black = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pvp_state WHERE character_id=encode($1,'hex')::uuid AND skull=2 AND skull_until>clock_timestamp())")
        .bind(authority.character_id_bytes().as_slice()).fetch_one(&mut **tx).await?;
    Ok((world, black))
}

/// Data sampled under current session, lease, scope, World policy and physical owner guards.
/// This is not a reusable authority token; each harm must re-read it in its own transaction.
#[derive(Debug)]
pub struct LivePvpFacts {
    pub world: WorldPvpType,
    pub relation: PvpRelation,
    pub actor_black: bool,
    pub target_black: bool,
    pub marked: bool,
    pub post_login_immune: bool,
    attacker: [u8; 16],
    target: [u8; 16],
    world_id: [u8; 16],
    actor_placement: [u8; 16],
    target_placement: [u8; 16],
}

/// Both identities come from independently current admitted sessions, never from command bytes.
#[allow(clippy::too_many_arguments)]
pub async fn read_live_pvp_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    authority: &SpellItemAuthority,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    target: ExactActorRef,
    target_session: GameSessionId,
) -> Result<LivePvpFacts> {
    check_transaction(tx, authority).await?;
    let attacker = current_colocated_party_character_in_transaction(
        tx,
        root,
        authority,
        runtime,
        actor,
        authority.game_session_id(),
    )
    .await?
    .ok_or_else(refused)?;
    let target_id = current_colocated_party_character_in_transaction(
        tx,
        root,
        authority,
        runtime,
        target,
        target_session,
    )
    .await?
    .ok_or_else(refused)?;
    if attacker != authority.character_id_bytes() || attacker == target_id {
        return Err(refused());
    }
    let source_lease = runtime
        .bound_attacker_lease(actor, authority.game_session_id())
        .map_err(|_| refused())?;
    let target_lease = runtime
        .bound_attacker_lease(target, target_session)
        .map_err(|_| refused())?;
    let current = super::fresh_admission::FreshAdmissionStore::from_root(root.clone())
        .current_session_in_transaction(tx, target_session)
        .await?;
    if source_lease.character_id().as_bytes() != &attacker
        || source_lease.generation() != authority.character_lease_generation()
        || target_lease.character_id().as_bytes() != &target_id
        || target_lease.generation() != current.current_character_lease().generation()
    {
        return Err(refused());
    }

    let policy = read_world_field_policy_in_transaction(
        tx,
        root,
        authority.runtime_scope(),
        authority.scope_generation(),
    )
    .await?
    .ok_or_else(refused)?;
    let policy = policy.bind(tx);
    // The published World mode is shared with fields. No row/default/ready bit grants harm.
    let world = match policy.mode() {
        FieldWorldType::NoPvp => WorldPvpType::Optional,
        FieldWorldType::Pvp => WorldPvpType::Open,
        FieldWorldType::PvpEnforced => WorldPvpType::Hardcore,
    };
    let world_id = *policy.world().as_bytes();
    // PARTY-1's World serialization also qualifies explicit absence of membership.
    super::world_party::read_world_party_in_transaction(tx, authority).await?;
    let group: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_party_members a JOIN game_party_members b USING(party_id) WHERE a.character_id=encode($1,'hex')::uuid AND b.character_id=encode($2,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_guild_members a JOIN game_guild_members b USING(guild_id) WHERE a.character_id=encode($1,'hex')::uuid AND b.character_id=encode($2,'hex')::uuid)")
        .bind(attacker.as_slice()).bind(target_id.as_slice()).fetch_one(&mut **tx).await?;
    let retaliation:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_pvp_aggression WHERE attacker=encode($2,'hex')::uuid AND target=encode($1,'hex')::uuid AND until_at>clock_timestamp() AND scope_generation=$3::text::numeric AND attacker_placement=$4 AND target_placement=$5)")
        .bind(attacker.as_slice()).bind(target_id.as_slice()).bind(authority.scope_generation().to_string()).bind(target.placement_identity().as_slice()).bind(actor.placement_identity().as_slice()).fetch_one(&mut **tx).await?;
    // A protected ally must still occupy its current placement in this Channel. Membership
    // comes from the same durable party/guild rows as rule 5, never a client-supplied id.
    let placements: Vec<Vec<u8>> = runtime
        .visible_entities()
        .players
        .iter()
        .map(|p| p.actor.placement_identity().to_vec())
        .collect();
    let group_aggressor: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_pvp_aggression ag \
         WHERE ag.attacker=encode($2,'hex')::uuid AND ag.until_at>clock_timestamp() \
         AND ag.scope_generation=$3::text::numeric AND ag.attacker_placement=$4 \
         AND ag.target_placement=ANY($5::bytea[]) AND (\
         EXISTS(SELECT 1 FROM game_party_members a JOIN game_party_members b USING(party_id) \
         WHERE a.character_id=encode($1,'hex')::uuid AND b.character_id=ag.target) OR \
         EXISTS(SELECT 1 FROM game_guild_members a JOIN game_guild_members b USING(guild_id) \
         WHERE a.character_id=encode($1,'hex')::uuid AND b.character_id=ag.target)))",
    )
    .bind(attacker.as_slice())
    .bind(target_id.as_slice())
    .bind(authority.scope_generation().to_string())
    .bind(target.placement_identity().as_slice())
    .bind(&placements)
    .fetch_one(&mut **tx)
    .await?;
    let row=sqlx::query("SELECT COALESCE((SELECT skull=2 AND skull_until>clock_timestamp() FROM game_character_pvp_state WHERE character_id=encode($1,'hex')::uuid),false) AS actor_black, COALESCE((SELECT skull=2 AND skull_until>clock_timestamp() FROM game_character_pvp_state WHERE character_id=encode($2,'hex')::uuid),false) AS target_black, COALESCE((SELECT (skull>0 AND skull_until>clock_timestamp()) OR white_skull_until>clock_timestamp() FROM game_character_pvp_state WHERE character_id=encode($2,'hex')::uuid),false) OR EXISTS(SELECT 1 FROM game_character_revenge_marks WHERE victim_character_id=encode($1,'hex')::uuid AND killer_character_id=encode($2,'hex')::uuid AND avenged_at IS NULL AND expires_at>clock_timestamp()) AS marked, (SELECT authorization_decided_at>floor(extract(epoch FROM clock_timestamp()))::bigint-10 FROM game_durability_fresh_admission_receipts r JOIN game_durability_reconnect_sessions s ON COALESCE(s.fresh_replay_key,s.initial_fresh_replay_key)=r.replay_key WHERE s.game_session_id=encode($3,'hex')::uuid) AS immune")
        .bind(attacker.as_slice()).bind(target_id.as_slice()).bind(authority.game_session_id().as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    Ok(LivePvpFacts {
        world,
        relation: PvpRelation {
            war_between: false,
            same_party_or_guild: Some(group),
            friendly_fire: false,
            target_aggressor_of_actor: retaliation,
            target_aggressor_of_actor_group: group_aggressor,
        },
        actor_black: row.try_get("actor_black")?,
        target_black: row.try_get("target_black")?,
        marked: retaliation || row.try_get::<bool, _>("marked")?,
        post_login_immune: row.try_get::<Option<bool>, _>("immune")?.unwrap_or(true),
        attacker,
        target: target_id,
        world_id,
        actor_placement: actor.placement_identity(),
        target_placement: target.placement_identity(),
    })
}

/// Basic unarmoured target qualification. Unknown or equipped mitigation fails closed.
/// Advanced equipment mitigation is deliberately not approximated by creature resistance.
pub async fn unarmoured_pvp_target_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    facts: &LivePvpFacts,
) -> Result<bool> {
    let present:Option<String>=sqlx::query_scalar("SELECT revision::text FROM game_character_equipment_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(facts.target.as_slice()).fetch_optional(&mut **tx).await?;
    let empty:bool=sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM game_character_equipment_slots WHERE character_id=encode($1,'hex')::uuid)")
        .bind(facts.target.as_slice()).fetch_one(&mut **tx).await?;
    Ok(present.is_some() && empty)
}

/// Write the immutable outcome, compact damage ledger and conservative locks in one transaction.
/// The caller already staged its exact physical successor; replay never refreshes aggression.
pub async fn record_live_pvp_harm_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    facts: &LivePvpFacts,
    ordinal: u16,
    binding: [u8; 32],
    damage: u32,
) -> Result<()> {
    check_transaction(tx, authority).await?;
    if facts.attacker != authority.character_id_bytes() {
        return Err(refused());
    }
    let command = authority.command();
    if let Some(old) = read_receipt(tx, command, ordinal, facts.target).await? {
        if old != binding.to_vec() {
            return Err(refused());
        }
        return Ok(());
    }
    let mut ids = [facts.attacker, facts.target];
    ids.sort();
    for id in ids {
        super::party_pz_fact::lock_in_transaction(tx, &id).await?;
        sqlx::query("INSERT INTO game_character_pvp_state(character_id,world_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid) ON CONFLICT DO NOTHING")
            .bind(id.as_slice()).bind(facts.world_id.as_slice()).execute(&mut **tx).await?;
        let valid:bool=sqlx::query_scalar("SELECT world_id=encode($2,'hex')::uuid FROM game_character_pvp_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(id.as_slice()).bind(facts.world_id.as_slice()).fetch_one(&mut **tx).await?;
        if !valid {
            return Err(refused());
        }
    }
    // Only actual HP/MP harm gets a contributor. Zero draws may still hold logout/PZ locks.
    if damage > 0 {
        ledger::record(tx, facts, damage).await?;
    }
    let unjustified = facts.world == WorldPvpType::Open
        && !facts.marked
        && !facts.relation.target_aggressor_of_actor;
    for id in ids {
        sqlx::query("UPDATE game_character_pvp_state SET logout_block_until=GREATEST(logout_block_until,clock_timestamp()+interval '70 seconds'), pz_block_until=CASE WHEN character_id=encode($2,'hex')::uuid AND NOT $4 THEN GREATEST(pz_block_until,clock_timestamp()+interval '70 seconds') ELSE pz_block_until END, white_skull_until=CASE WHEN character_id=encode($2,'hex')::uuid AND $3 THEN GREATEST(white_skull_until,clock_timestamp()+interval '70 seconds') ELSE white_skull_until END, adventurer_forfeited=adventurer_forfeited OR (character_id=encode($2,'hex')::uuid AND NOT $4), revision=revision+1 WHERE character_id=encode($1,'hex')::uuid")
            .bind(id.as_slice()).bind(facts.attacker.as_slice()).bind(unjustified).bind(facts.relation.target_aggressor_of_actor).execute(&mut **tx).await?;
    }
    sqlx::query("DELETE FROM game_character_pvp_aggression WHERE attacker=encode($1,'hex')::uuid AND target IN(SELECT target FROM game_character_pvp_aggression WHERE attacker=encode($1,'hex')::uuid AND (until_at<=clock_timestamp() OR attacker_placement<>$2 OR scope_generation<>$3::text::numeric) ORDER BY until_at LIMIT 64 FOR UPDATE)")
        .bind(facts.attacker.as_slice()).bind(facts.actor_placement.as_slice()).bind(authority.scope_generation().to_string()).execute(&mut **tx).await?;
    let full:bool=sqlx::query_scalar("SELECT (SELECT count(*) FROM game_character_pvp_aggression WHERE attacker=encode($1,'hex')::uuid)>=64 AND NOT EXISTS(SELECT 1 FROM game_character_pvp_aggression WHERE attacker=encode($1,'hex')::uuid AND target=encode($2,'hex')::uuid)")
        .bind(facts.attacker.as_slice()).bind(facts.target.as_slice()).fetch_one(&mut **tx).await?;
    if full {
        return Err(refused());
    }
    sqlx::query("INSERT INTO game_character_pvp_aggression(attacker,target,until_at,scope_generation,attacker_placement,target_placement) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,clock_timestamp()+interval '70 seconds',$3::text::numeric,$4,$5) ON CONFLICT(attacker,target) DO UPDATE SET until_at=GREATEST(game_character_pvp_aggression.until_at,EXCLUDED.until_at),scope_generation=EXCLUDED.scope_generation,attacker_placement=EXCLUDED.attacker_placement,target_placement=EXCLUDED.target_placement")
        .bind(facts.attacker.as_slice()).bind(facts.target.as_slice()).bind(authority.scope_generation().to_string()).bind(facts.actor_placement.as_slice()).bind(facts.target_placement.as_slice()).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_character_pvp_harm(game_session_id,command_id,ordinal,victim,attacker,binding,damage,unjustified) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6,$7,$8)")
        .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(i32::from(ordinal)).bind(facts.target.as_slice()).bind(facts.attacker.as_slice()).bind(binding.as_slice()).bind(i32::try_from(damage).map_err(|_|refused())?).bind(unjustified).execute(&mut **tx).await?;
    Ok(())
}

async fn read_receipt(
    tx: &mut Transaction<'_, Postgres>,
    command: CommandRef,
    ordinal: u16,
    target: [u8; 16],
) -> Result<Option<Vec<u8>>> {
    Ok(sqlx::query_scalar("SELECT binding FROM game_character_pvp_harm WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric AND ordinal=$3 AND victim=encode($4,'hex')::uuid")
        .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(i32::from(ordinal)).bind(target.as_slice()).fetch_optional(&mut **tx).await?)
}

/// The commit window retains the staged physical attempt across cancellation/unknown COMMIT.
pub async fn commit_live_pvp_transaction<T: Send + 'static>(
    tx: Transaction<'_, Postgres>,
    deadline: std::time::Instant,
    window: &mut super::spell_owner_commit::SpellCommitWindow<'_, T>,
) -> std::result::Result<(), DurabilityError> {
    window.mark_already_committed();
    super::db::commit_semantic_transaction(tx, deadline).await
}

/// Reconciliation waits for the original transaction's session/command locks before this read.
pub async fn live_pvp_receipt_matches_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    ordinal: u16,
    target: [u8; 16],
    binding: [u8; 32],
) -> Result<bool> {
    check_transaction(tx, authority).await?;
    match read_receipt(tx, authority.command(), ordinal, target).await? {
        Some(old) if old == binding => Ok(true),
        Some(_) => Err(refused()),
        None => Ok(false),
    }
}

/// Derive only from live harm receipts in the victim's fenced death transaction.
/// A live ledger without source receipts needs explicit consequences; an empty request is
/// invalid input, not a storage outage. Database failures retain their unavailable error.
pub(super) async fn death_contributors_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    victim: crate::domain::CharacterId,
) -> std::result::Result<
    Vec<super::pvp_death_request::PvpDeathContributor>,
    CharacterProgressionError,
> {
    let rows=sqlx::query("SELECT encode(uuid_send(l.contributor_character_id),'hex') AS contributor, (SELECT h.unjustified FROM game_character_pvp_harm h WHERE h.victim=l.victim_character_id AND h.attacker=l.contributor_character_id AND h.damage>0 AND h.created_at>=clock_timestamp()-interval '5 minutes' ORDER BY h.created_at DESC LIMIT 1) AS unjustified FROM game_character_pvp_ledger l WHERE l.victim_character_id=encode($1,'hex')::uuid AND l.last_at>=clock_timestamp()-interval '5 minutes' ORDER BY l.contributor_character_id LIMIT 81")
        .bind(victim.as_bytes().as_slice()).fetch_all(&mut **tx).await.map_err(DurabilityError::from)?;
    if rows.len() > 80 {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    let mut out = Vec::new();
    for row in rows {
        let Some(unjustified) = row
            .try_get::<Option<bool>, _>("unjustified")
            .map_err(DurabilityError::from)?
        else {
            return Err(CharacterProgressionError::InvalidInput);
        };
        let hex: String = row.try_get("contributor").map_err(DurabilityError::from)?;
        let mut bytes = [0; 16];
        for (i, b) in bytes.iter_mut().enumerate() {
            *b = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
        }
        out.push(super::pvp_death_request::PvpDeathContributor {
            character_id: crate::domain::CharacterId::from_bytes(bytes)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            kind: super::pvp_death_request::PvpContributorKind::Damage,
            unjustified,
        });
    }
    Ok(out)
}
