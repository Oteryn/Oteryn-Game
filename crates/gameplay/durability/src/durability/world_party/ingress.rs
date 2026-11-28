//! Basic server ingress reads and the sealed leave proof. The dispatch owner retains the
//! runtime, spell and attack guards until commit; SQL locks protect absent PvP state too.
use super::*;
use crate::foundation::{CommandRef, ExactActorRef};
use oteryn_gameplay_combat_rules::attack::AttackState;
use oteryn_simulation_determinism::SemanticTimeMicros;

#[derive(Debug)]
pub struct PartyIngressView {
    pub source: PartySnapshot,
    pub members: Vec<([u8; 16], String)>,
    pub invitations: Vec<([u8; 16], String, u64)>,
}
/// Read only the acting character's own party and pending invitations under the World owner.
pub async fn read_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
) -> Result<PartyIngressView> {
    let source = read_world_party_in_transaction(tx, authority).await?;
    if !source.is_well_formed() {
        return Err(reject("party ingress shape"));
    }
    let locals = std::collections::BTreeMap::from([(source.character, source.party_id)]);
    let views = source
        .party_id
        .map(|party| (party, source.members.iter().map(|member| member.character)));
    oteryn_gameplay_social_rules::party::validate_memberships(&locals, views)
        .map_err(|_| reject("party membership rules"))?;
    let members = if let Some(party) = source.party_id {
        let rows = sqlx::query("SELECT r.character_id::text,r.name FROM game_party_members m JOIN game_character_roots r USING(character_id) WHERE m.party_id=encode($1,'hex')::uuid ORDER BY m.seq LIMIT 50")
            .bind(party.as_slice()).fetch_all(&mut **tx).await?;
        rows.into_iter()
            .map(|row| Ok((uuid(row.try_get("character_id")?)?, row.try_get("name")?)))
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };
    let rows = sqlx::query("SELECT i.party_id::text,r.name,ceil(extract(epoch FROM i.expires_at))::bigint AS expires FROM game_party_invitations i JOIN game_parties p USING(party_id) JOIN game_character_roots r ON r.character_id=p.leader_character_id WHERE i.invitee=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid AND i.expires_at>clock_timestamp() ORDER BY i.created_at,i.party_id LIMIT 20")
        .bind(authority.character_id_bytes().as_slice()).bind(source.world.as_slice()).fetch_all(&mut **tx).await?;
    let invitations = rows
        .into_iter()
        .map(|row| {
            Ok((
                uuid(row.try_get("party_id")?)?,
                row.try_get("name")?,
                u64::try_from(row.try_get::<i64, _>("expires")?)
                    .map_err(|_| reject("party expiry"))?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(PartyIngressView {
        source,
        members,
        invitations,
    })
}

pub fn party_icon_topic(world: [u8; 16]) -> String {
    let world: String = world.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("oteryn_party_icons_{world}")
}

/// Publish a presentation hint as part of the party command transaction. PostgreSQL
/// releases the hint at commit. Its payload contains opaque recipient tokens,
/// and receivers must re-read the existing scoped party projection.
pub async fn notify_party_icons_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    party: [u8; 16],
) -> Result<String> {
    let (world, _) = owner(tx, authority).await?;
    // The World party invariant permits 50 members and 50 live invitations.
    // 100 hex SHA-256 tokens plus separators fit one PostgreSQL notification.
    let recipients: String = sqlx::query_scalar(
        "SELECT COALESCE(string_agg(encode(sha256(uuid_send(recipient)), 'hex'), ',' ORDER BY recipient), '')
         FROM (
             SELECT m.character_id AS recipient FROM game_party_members m JOIN game_parties p USING(party_id)
             WHERE p.party_id=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid
             UNION
             SELECT i.invitee FROM game_party_invitations i JOIN game_parties p USING(party_id)
             WHERE p.party_id=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid
                 AND i.expires_at>clock_timestamp()
         ) AS recipients",
    )
    .bind(party.as_slice())
    .bind(world.as_slice())
    .fetch_one(&mut **tx)
    .await?;
    sqlx::query("SELECT pg_notify($1, $2)")
        .bind(party_icon_topic(world))
        .bind(&recipients)
        .execute(&mut **tx)
        .await?;
    Ok(recipients)
}

struct CurrentLogoutProof {
    character: [u8; 16],
    command: CommandRef,
    transaction: String,
    clear: bool,
}
impl super::logout_owner_seal::Sealed for CurrentLogoutProof {}
impl PartyLogoutEligibility for CurrentLogoutProof {
    fn matches(&self, authority: &SpellItemAuthority, transaction: &str) -> bool {
        self.character == authority.character_id_bytes()
            && self.command == authority.command()
            && self.transaction == transaction
    }
    fn locks_clear(&self) -> bool {
        self.clear
    }
}
/// Only the physical runtime owner calls this with its borrowed attack state while retaining
/// all owning guards. The independently fenced SQL authority is required on every retry.
pub async fn leave_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    runtime: &crate::foundation::ChannelRuntimeV1,
    actor: ExactActorRef,
    attack: Option<&AttackState<ExactActorRef, CommandRef>>,
    now: SemanticTimeMicros,
) -> Result<PreparedWorldPartyReceipt> {
    check_transaction(tx, authority).await?;
    let binding = runtime.binding();
    if runtime.owner_fence().is_err()
        || authority.runtime_scope() != binding.runtime_scope()
        || authority.scope_generation() != binding.scope_generation().get()
        || !runtime
            .player_control_facts(actor, authority.game_session_id())
            .is_ok_and(|p| p.control_loss.is_none())
    {
        return Err(reject("party leave runtime owner"));
    }
    super::super::party_pz_fact::lock_in_transaction(tx, &authority.character_id_bytes()).await?;
    let stored = super::super::pvp_state::load_pvp_state(
        tx,
        crate::domain::CharacterId::from_bytes(authority.character_id_bytes())
            .map_err(|_| reject("party character"))?,
    )
    .await?;
    let pvp_blocked = stored.is_some_and(|s| s.readmission_clock.has_running_lock());
    let proof = CurrentLogoutProof {
        character: authority.character_id_bytes(),
        command: authority.command(),
        transaction: sqlx::query_scalar("SELECT pg_current_xact_id()::text")
            .fetch_one(&mut **tx)
            .await?,
        clear: !pvp_blocked && !attack.is_some_and(|a| a.in_fight(now)),
    };
    apply_world_party_command_with_logout_in_transaction(
        tx,
        authority,
        PartyAction::Leave,
        None,
        &proof,
    )
    .await
}

/// A price list is stored with qualified identities, never generation-local wire references.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct HuntPrice {
    pub key: String,
    pub revision: String,
    pub loot: Option<u64>,
    pub supply: Option<u64>,
}

pub enum HuntChange {
    Reset,
    Market,
    Leader(Vec<HuntPrice>),
}

pub struct HuntSettings {
    pub interval: u64,
    pub elapsed_seconds: u64,
    pub leader_prices: Option<Vec<HuntPrice>>,
}

pub async fn hunt_settings_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    party: &PartySnapshot,
    change: Option<HuntChange>,
) -> Result<HuntSettings> {
    check_transaction(tx, authority).await?;
    let character = authority.character_id_bytes();
    let world = *authority.runtime_scope().world_id().as_bytes();
    if party.character != character || party.world != world {
        return Err(reject("party hunt authority"));
    }
    let id = party.party_id.ok_or(reject("party hunt membership"))?;
    if let Some(change) = change {
        // The SQL predicate independently proves current leadership. The caller obtained its
        // snapshot in this transaction under the World party owner lock.
        let (sql, prices) = match change {
            HuntChange::Reset => (
                "UPDATE game_parties SET hunt_interval=hunt_interval+1,hunt_started_at=clock_timestamp() WHERE party_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND leader_character_id=encode($3,'hex')::uuid AND $4::jsonb IS NULL",
                None,
            ),
            HuntChange::Market => (
                "UPDATE game_parties SET hunt_uses_leader_prices=false WHERE party_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND leader_character_id=encode($3,'hex')::uuid AND $4::jsonb IS NULL",
                None,
            ),
            HuntChange::Leader(prices) => (
                "UPDATE game_parties SET hunt_uses_leader_prices=true,hunt_price_owner=leader_character_id,hunt_leader_prices=$4 WHERE party_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND leader_character_id=encode($3,'hex')::uuid",
                Some(sqlx::types::Json(prices)),
            ),
        };
        let updated = sqlx::query(sql)
            .bind(id.as_slice()).bind(world.as_slice()).bind(character.as_slice()).bind(prices)
            .execute(&mut **tx).await?;
        if updated.rows_affected() != 1 {
            return Err(reject("party hunt leadership"));
        }
    }
    let row = sqlx::query("SELECT hunt_interval, coalesce(greatest(0,floor(extract(epoch FROM clock_timestamp()-hunt_started_at))),0)::bigint AS elapsed, CASE WHEN hunt_uses_leader_prices THEN CASE WHEN hunt_price_owner=leader_character_id THEN hunt_leader_prices ELSE '[]'::jsonb END END AS prices FROM game_parties WHERE party_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
        .bind(id.as_slice()).bind(world.as_slice()).fetch_one(&mut **tx).await?;
    Ok(HuntSettings {
        interval: u64::try_from(row.try_get::<i64,_>("hunt_interval")?).map_err(|_| reject("party hunt interval"))?,
        elapsed_seconds: u64::try_from(row.try_get::<i64,_>("elapsed")?).map_err(|_| reject("party hunt elapsed"))?,
        leader_prices: row.try_get::<Option<sqlx::types::Json<Vec<HuntPrice>>>,_>("prices")?.map(|prices| prices.0),
    })
}

/// Bounded batch for one party page. Match both definition key and revision and count
/// one history row per eligible settlement, using the market statistics' 30-day horizon.
pub async fn hunt_market_prices_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    definitions: &[(String, String)],
) -> Result<Vec<(String, String, u64)>> {
    if definitions.is_empty() {
        return Ok(Vec::new());
    }
    if definitions.len() > 16384 {
        return Err(reject("party hunt market batch"));
    }
    check_transaction(tx, authority).await?;
    let (keys, revisions): (Vec<_>, Vec<_>) = definitions.iter().cloned().unzip();
    let rows = sqlx::query(
        "WITH stamp AS (SELECT floor(extract(epoch FROM statement_timestamp())*1000000)::bigint AS now_us)
         SELECT d.key, d.revision, floor(avg(trades.price))::bigint AS price
         FROM unnest($2::text[], $3::text[]) AS d(key,revision)
         CROSS JOIN stamp
         CROSS JOIN LATERAL (
            SELECT DISTINCT ON (s.transaction_id) h.price
            FROM game_market_offers o JOIN game_market_settlements s ON s.offer_id=o.offer_id
            JOIN game_market_history h ON h.transaction_id=s.transaction_id
            WHERE o.world_id=encode($1,'hex')::uuid AND o.definition_key=d.key
              AND o.definition_revision=d.revision AND s.world_id=o.world_id AND s.kind IN (2,5)
              AND s.occurred_at_us BETWEEN stamp.now_us-2592000000000 AND stamp.now_us
            ORDER BY s.transaction_id,h.character_id
         ) trades GROUP BY d.key,d.revision",
    )
    .bind(authority.runtime_scope().world_id().as_bytes().as_slice())
    .bind(keys).bind(revisions).fetch_all(&mut **tx).await?;
    rows.into_iter().map(|row| Ok((
        row.try_get("key")?,
        row.try_get("revision")?,
        u64::try_from(row.try_get::<i64,_>("price")?).map_err(|_| reject("party hunt market price"))?,
    ))).collect()
}
