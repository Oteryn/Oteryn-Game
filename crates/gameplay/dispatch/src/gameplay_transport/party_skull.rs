//! Observer-relative party icons, read under the existing Channel and World owners.
use super::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::durability::{DurabilityError, spell_item_transaction as item_tx, world_party};
use crate::foundation::PartySnapshot;
use oteryn_protocol_oteryn::{party_skull::PartyIcon, world_spatial_entities::EntityRef};
use sha2::{Digest, Sha256};
use sqlx::Row;
use sqlx::postgres::{PgConnectOptions, PgListener, PgPoolOptions};
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::sync::watch;

/// Transient hints keyed by the opaque tokens of live observers.
/// A subscription lives with the serving connection.
#[derive(Default)]
pub(super) struct PartyIconUpdates(std::sync::Mutex<BTreeMap<String, watch::Sender<()>>>);

impl PartyIconUpdates {
    pub(super) fn subscribe(&self, character: [u8; 16]) -> Option<watch::Receiver<()>> {
        let mut recipients = self.0.lock().ok()?;
        recipients.retain(|_, sender| sender.receiver_count() != 0);
        let token: String = Sha256::digest(character)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        Some(recipients.entry(token).or_default().subscribe())
    }

    pub(super) fn publish(&self, tokens: &str) {
        if let Ok(recipients) = self.0.lock() {
            for token in tokens.split(',') {
                if let Some(recipient) = recipients.get(token) {
                    recipient.send_replace(());
                }
            }
        }
    }

    pub(super) async fn receive(&self, listener: &mut PgListener) -> Result<bool, sqlx::Error> {
        let Some(notice) = listener.try_recv().await? else {
            return Ok(false);
        };
        self.publish(notice.payload());
        Ok(true)
    }

    pub(super) async fn listen(&self, world: [u8; 16], options: PgConnectOptions) {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(2))
            .max_lifetime(None)
            .idle_timeout(None)
            .connect_lazy_with(options);
        let topic = world_party::ingress::party_icon_topic(world);
        loop {
            let subscription = tokio::time::timeout(Duration::from_secs(2), async {
                let mut listener = PgListener::connect_with(&pool).await?;
                listener.eager_reconnect(false);
                listener.listen(&topic).await?;
                Ok::<_, sqlx::Error>(listener)
            });
            if let Ok(Ok(mut listener)) = subscription.await {
                // Catch a change committed while the listener was disconnected.
                if let Ok(recipients) = self.0.lock() {
                    for recipient in recipients.values() {
                        recipient.send_replace(());
                    }
                }
                while matches!(self.receive(&mut listener).await, Ok(true)) {}
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}

fn shield(
    same: bool,
    leader: bool,
    enabled: bool,
    all: bool,
    eligible: bool,
    outsider: bool,
) -> u8 {
    if !same {
        return if outsider { 11 } else { 0 };
    }
    match (leader, enabled, all, eligible) {
        (false, false, _, _) => 3,
        (true, false, _, _) => 4,
        (false, true, true, _) => 5,
        (true, true, true, _) => 6,
        (false, true, false, false) => 7,
        (true, true, false, false) => 8,
        (false, true, false, true) => 9,
        (true, true, false, true) => 10,
    }
}

struct Member {
    entity: EntityRef,
    level: u32,
    x: i32,
    y: i32,
    active: bool,
    pz: bool,
    skull: u8,
    aggression: bool,
    revenge: bool,
    invited_by_observer: bool,
    invites_observer: bool,
}

fn eligible(
    source: &PartySnapshot,
    character: &[u8; 16],
    members: &BTreeMap<[u8; 16], Member>,
) -> bool {
    if source.members.len() < 2 {
        return false;
    }
    let Some(member) = members.get(character) else {
        return false;
    };
    let Some(leader) = source.leader.and_then(|id| members.get(&id)) else {
        return false;
    };
    let highest = source
        .members
        .iter()
        .filter_map(|m| members.get(&m.character))
        .map(|m| m.level)
        .max()
        .unwrap_or(0);
    member.active
        && u64::from(member.level) * 3 >= u64::from(highest) * 2
        && member.x.abs_diff(leader.x) <= 30
        && member.y.abs_diff(leader.y) <= 30
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn observe_party_skull(
        &self,
        admitted: AdmittedSession,
    ) -> Result<Vec<PartyIcon>, DurabilityError> {
        self.read_party_skull(admitted, false).await
    }

    pub(super) async fn observe_party_skull_for_push(
        &self,
        admitted: AdmittedSession,
    ) -> Result<Vec<PartyIcon>, DurabilityError> {
        self.read_party_skull(admitted, true).await
    }

    async fn read_party_skull(
        &self,
        admitted: AdmittedSession,
        push: bool,
    ) -> Result<Vec<PartyIcon>, DurabilityError> {
        use oteryn_protocol_oteryn::party_skull::CAPABILITY_PARTY_SKULL_V1;
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_PARTY_SKULL_V1)
            && !admitted
                .continuity
                .selected_capabilities
                .contains(oteryn_protocol_oteryn::party_effects::CAPABILITY_PARTY_EFFECTS_V1)
            && !admitted
                .continuity
                .selected_capabilities
                .contains(oteryn_protocol_oteryn::party::CAPABILITY_PARTY_LIST_V1)
        {
            return Err(DurabilityError::Unavailable);
        }
        let actor = admitted.runtime_actor.ok_or(DurabilityError::Unavailable)?;
        let fence = admitted.item_fence.ok_or(DurabilityError::Unavailable)?;
        if fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return Err(DurabilityError::Unavailable);
        }
        let (permit, runtime) = if push {
            let permit = self
                .spell_lane
                .try_acquire_for_observation()
                .ok_or(DurabilityError::RootUnavailable)?;
            let runtime = self
                .runtime
                .try_lock()
                .map_err(|_| DurabilityError::RootUnavailable)?;
            (permit, runtime)
        } else {
            (
                self.try_spell_lane_permit().await?,
                self.runtime.lock().await,
            )
        };
        runtime
            .assert_actor_spell_unreserved(actor)
            .map_err(|_| DurabilityError::Unavailable)?;
        let lease = runtime
            .bound_attacker_lease(actor, admitted.game_session_id)
            .map_err(|_| DurabilityError::Unavailable)?;
        if lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || lease.generation() != fence.character_lease_generation
        {
            return Err(DurabilityError::Unavailable);
        }
        if runtime
            .player_control_facts(actor, admitted.game_session_id)
            .map_err(|_| DurabilityError::Unavailable)?
            .control_loss
            .is_some()
        {
            return Err(DurabilityError::Unavailable);
        }
        let (states, attack) = if push {
            let states = self
                .states
                .spell_states
                .try_lock()
                .map_err(|_| DurabilityError::RootUnavailable)?;
            let attack = self
                .states
                .attack
                .try_lock()
                .map_err(|_| DurabilityError::RootUnavailable)?;
            (states, attack)
        } else {
            (
                self.states.spell_states.lock().await,
                self.states.attack.lock().await,
            )
        };
        let pass = self.root.try_issue_semantic_pass()?;
        let mut context = (self, &*runtime, &*states, &*attack, &permit);
        let result = pass.run_with_context(&mut context, move |holder, deadline, ctx| {
            Box::pin(async move {
                let (owner, runtime, states, attack, permit) = ctx;
                let unavailable = || DurabilityError::Unavailable;
                let binding = runtime.binding();
                let scope = binding.runtime_scope();
                let mut tx = item_tx::begin_spell_owner_transaction(holder, deadline).await?;
                let authority = item_tx::assert_spell_item_scope_in_transaction(&mut tx, permit, owner.root, owner.character, owner.holder, scope, binding.scope_generation().get()).await.map_err(|_|unavailable())?;
                let observer = crate::durability::admission_journal::party_target_binding::current_colocated_party_character_for_scope_in_transaction(&mut tx,owner.root,&authority,runtime,actor,admitted.game_session_id).await.map_err(|_|unavailable())?.ok_or_else(unavailable)?;
                let connection = crate::durability::admission_journal::party_target_binding::read_connection_generation_for_share_in_transaction(&mut tx, admitted.game_session_id).await?;
                if connection != admitted.continuity.connection_generation.to_string() { return Err(unavailable()); }
                let sources = world_party::read_world_party_views_for_scope_in_transaction(&mut tx,owner.root,&authority,runtime).await.map_err(|_|unavailable())?;
                let source = sources.iter().find(|s|s.character == observer).ok_or_else(unavailable)?;
                let policy = crate::durability::spell_field_policy::read_world_field_policy_in_transaction(&mut tx,owner.root,scope,binding.scope_generation().get()).await.map_err(|_|unavailable())?.ok_or_else(unavailable)?;
                let open = policy.bind(&tx).mode() == crate::durability::spell_field_policy::FieldWorldType::Pvp;
                let visible = runtime.visible_entities();
                let now = owner.owner_now();
                let mut members = BTreeMap::new();
                for (current, position, session) in runtime.positioned_actor_census().map_err(|_|unavailable())? {
                    let Some(session) = session else { continue };
                    let lease = runtime.bound_attacker_lease(current,session).map_err(|_|unavailable())?;
                    let id = *lease.character_id().as_bytes();
                    let row = sqlx::query("SELECT COALESCE((SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid),$5) AS level, COALESCE((SELECT CASE WHEN skull=2 AND skull_until>clock_timestamp() THEN 5 WHEN skull=1 AND skull_until>clock_timestamp() THEN 4 WHEN white_skull_until>clock_timestamp() THEN 3 ELSE 0 END FROM game_character_pvp_state WHERE character_id=encode($1,'hex')::uuid),0) AS skull, COALESCE((SELECT pz_block_until>clock_timestamp() FROM game_character_pvp_state WHERE character_id=encode($1,'hex')::uuid),false) AS pz, EXISTS(SELECT 1 FROM game_character_pvp_aggression WHERE attacker=encode($1,'hex')::uuid AND target=encode($2,'hex')::uuid AND until_at>clock_timestamp() AND scope_generation=$3::text::numeric AND attacker_placement=$4 AND target_placement=$6) AS aggression, EXISTS(SELECT 1 FROM game_character_revenge_marks WHERE (victim_character_id=encode($2,'hex')::uuid OR encode($1,'hex')::uuid=encode($2,'hex')::uuid) AND killer_character_id=encode($1,'hex')::uuid AND avenged_at IS NULL AND expires_at>clock_timestamp()) AS revenge, EXISTS(SELECT 1 FROM game_party_invitations i JOIN game_parties p USING(party_id) WHERE p.leader_character_id=encode($2,'hex')::uuid AND i.invitee=encode($1,'hex')::uuid AND i.expires_at>clock_timestamp()) AS invited, EXISTS(SELECT 1 FROM game_party_invitations i JOIN game_parties p USING(party_id) WHERE p.leader_character_id=encode($1,'hex')::uuid AND i.invitee=encode($2,'hex')::uuid AND i.expires_at>clock_timestamp()) AS inviting")
                        .bind(id.as_slice()).bind(observer.as_slice()).bind(binding.scope_generation().get().to_string()).bind(current.placement_identity().as_slice()).bind(i64::from(states.get(runtime,current,session).map_or(1,|s|s.character_facts().level))).bind(actor.placement_identity().as_slice()).fetch_one(&mut *tx).await?;
                    let Some(entity) = visible.players.iter().find(|p|p.actor==current) else { continue };
                    let pos = position.position();
                    members.insert(id,Member { entity:EntityRef { identity:current.placement_identity(),generation:entity.generation },level:u32::try_from(row.try_get::<i64,_>("level")?).map_err(|_|unavailable())?,x:pos.x,y:pos.y,active:attack.shared_experience_active(runtime,current,session,now),pz:row.try_get("pz")?,skull:u8::try_from(row.try_get::<i32,_>("skull")?).map_err(|_|unavailable())?,aggression:row.try_get("aggression")?,revenge:row.try_get("revenge")?,invited_by_observer:row.try_get("invited")?,invites_observer:row.try_get("inviting")? });
                }
                let mut icons = Vec::new();
                for target in &sources {
                    let Some(member) = members.get(&target.character) else { continue };
                    let same = source.party_id.is_some() && source.party_id == target.party_id;
                    // Use the same Channel-local membership and eligibility policy as PARTY-XP-1.
                    let all = target.members.iter().filter(|m|members.contains_key(&m.character) || authority.channel_bytes().is_some_and(|channel| m.channel==Some(channel))).all(|m|eligible(target,&m.character,&members))
                        && target.leader.and_then(|id|members.get(&id)).is_some_and(|leader|!leader.pz);
                    let mut shield = shield(same,target.leader==Some(target.character),target.shared_xp_enabled,all,eligible(target,&target.character,&members),target.party_id.is_some());
                    if !same {
                        if member.invited_by_observer { shield=2; }
                        else if member.invites_observer { shield=1; }
                    }
                    let skull = if !open { 0 } else if member.skull!=0 { member.skull } else if member.revenge { 6 } else if same { 2 } else if member.aggression { 1 } else { 0 };
                    // The wire child applies the session's actual interest set before sending.
                    icons.push(PartyIcon { entity:member.entity,shield,skull });
                }
                tx.rollback().await?;
                Ok(icons)
            })
        }).await;
        #[cfg(test)]
        if let Err(error) = &result {
            eprintln!("PARTY-SKULL-1 observation unavailable: {error:?}");
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn party_skull_all_shield_states() {
        for (leader, plain, shared, blink, inactive) in [(false, 3, 5, 7, 9), (true, 4, 6, 8, 10)] {
            assert_eq!(shield(true, leader, false, false, false, true), plain);
            assert_eq!(shield(true, leader, true, true, true, true), shared);
            assert_eq!(shield(true, leader, true, false, false, true), blink);
            assert_eq!(shield(true, leader, true, false, true, true), inactive);
        }
        assert_eq!(shield(false, false, true, true, true, true), 11);
        assert_eq!(shield(false, false, true, true, true, false), 0);
    }
}

#[path = "party_skull/status.rs"]
mod status;
