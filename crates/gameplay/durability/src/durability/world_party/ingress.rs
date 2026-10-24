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
        || authority.runtime_scope()
            != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
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
