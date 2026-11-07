//! Current native NPC context composes with the existing Quest transaction.
//! No caller-provided NPC key or position can construct this private context.
use super::super::account_achievement::{
    AchievementCatalogueLookup, AchievementGrantRequest, AchievementSourceEvent,
    FencedGrantingCharacter, record_achievement_grant,
};
use super::super::item_transfer::CurrentCharacterItemFence;
use super::super::quest_world_callback::QuestWorldContext;
use super::*;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef};
use oteryn_gameplay_quests::quest::world_change_npcs as rules;
use oteryn_protocol_oteryn::npc_service::TALK_RANGE_TILES;
use sqlx::{Postgres, Transaction};

/// Created only by checking the current Channel owner and the real NPC slot.
/// NPC authored home cells are immutable for their actor lifetime. The durable item
/// fence and selected World day are checked again in the committing transaction.
#[derive(Debug, Clone)]
pub struct NativeWorldChangeNpc {
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    npc_key: String,
    npc_position: (u16, u16, i8),
    world: QuestWorldContext,
}
impl NativeWorldChangeNpc {
    #[allow(
        clippy::too_many_arguments,
        reason = "one coherent native conversation supplies owner, player, NPC, command and reset"
    )]
    pub fn from_current_runtime(
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        command: CommandRef,
        npc_identity: [u8; 16],
        npc_generation: u64,
        reset_epoch: u64,
        fence: CurrentCharacterItemFence,
    ) -> Result<Self> {
        runtime
            .owner_fence()
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        if command.game_session_id() != fence.game_session_id
            || runtime.binding().runtime_scope() != fence.runtime_scope
            || runtime.binding().scope_generation() != fence.scope_ownership_generation
            || !runtime
                .bound_attacker_lease(actor, command.game_session_id())
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
            || !runtime
                .player_control_facts(actor, command.game_session_id())
                .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        let visible = runtime.visible_entities();
        let player = visible
            .players
            .iter()
            .find(|p| p.actor == actor)
            .ok_or(CharacterProgressionError::AuthorityRejected)?;
        let npc = visible
            .npcs
            .iter()
            .find(|n| {
                n.actor.placement_identity() == npc_identity && n.generation == npc_generation
            })
            .ok_or(CharacterProgressionError::AuthorityRejected)?;
        let key = runtime
            .npc_key(npc.actor)
            .ok_or(CharacterProgressionError::AuthorityRejected)?;
        if player.position.floor != npc.position.floor
            || player.position.x.abs_diff(npc.position.x) > TALK_RANGE_TILES
            || player.position.y.abs_diff(npc.position.y) > TALK_RANGE_TILES
        {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        let home = runtime
            .npc_home_position(npc.actor)
            .ok_or(CharacterProgressionError::AuthorityRejected)?;
        let z = i8::try_from(home.floor).map_err(|_| CharacterProgressionError::InvalidInput)?;
        if !(0..=15).contains(&z) {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let npc_position = (
            u16::try_from(home.x).map_err(|_| CharacterProgressionError::InvalidInput)?,
            u16::try_from(home.y).map_err(|_| CharacterProgressionError::InvalidInput)?,
            z,
        );
        let hex = |bytes: [u8; 32]| {
            format!(
                "sha256:{}",
                bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
            )
        };
        let pin = runtime.content_pin();
        Ok(Self {
            fence,
            command,
            npc_key: key.into(),
            npc_position,
            world: QuestWorldContext {
                map_revision: hex(pin.map_revision_digest()),
                content_revision: hex(pin.server_artifact_digest()),
                placement_context: pin.frame_binding_digest().to_vec(),
                reset_epoch,
                actor_placement: actor.placement_identity(),
                movement_revision: player.revision,
            },
        })
    }
    pub(crate) fn npc_key(&self) -> &str {
        &self.npc_key
    }
    pub(crate) fn matches_item(
        &self,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
    ) -> bool {
        self.fence == fence && self.command == command
    }
    pub(super) fn matches(
        &self,
        fence: CurrentCharacterGameplayFence,
        request: &QuestTransitionRequest,
    ) -> bool {
        rules::for_transition(&request.transition_key)
            .is_some_and(|action| action.npc() == self.npc_key)
            && matches!(request.cause,QuestCause::Command(command) if self.matches_item(fence.item_fence(),command))
    }
    /// Source facts only; the enclosing Quest/trade owner has already checked its
    /// complete Character, node and scope fence under the same transaction.
    pub(crate) async fn source_is_current(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> std::result::Result<bool, DurabilityError> {
        let crate::foundation::RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = self.fence.runtime_scope
        else {
            return Ok(false);
        };
        let current:bool=sqlx::query_scalar("SELECT coalesce((SELECT reset_epoch=$3::text::numeric FROM game_world_reset_active WHERE world_id=encode($1,'hex')::uuid),$3='0') AND coalesce((SELECT 'sha256:'||encode(server_artifact_digest,'hex')=$4 AND frame_binding_digest=$5 FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid ORDER BY activation_sequence DESC LIMIT 1),false)")
            .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(self.world.reset_epoch.to_string())
            .bind(&self.world.content_revision).bind(&self.world.placement_context).fetch_one(&mut **tx).await?;
        Ok(current)
    }
    pub(crate) async fn selected_npc(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        change: &str,
    ) -> std::result::Result<bool, DurabilityError> {
        let world = self.fence.runtime_scope.world_id();
        let Some(day) =
            super::super::world_event::changes::load(tx, world, self.world.reset_epoch).await?
        else {
            return Ok(false);
        };
        let definitions = oteryn_gameplay_world_change::variants::definitions()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        Ok(definitions
            .iter()
            .find(|d| d.key == change)
            .and_then(|d| day.variant(change).and_then(|i| d.variants.get(i)))
            .is_some_and(|variant| {
                variant
                    .npcs
                    .iter()
                    .any(|npc| npc.key == self.npc_key && npc.position == self.npc_position)
            }))
    }
}

pub(super) fn prepare(
    request: &QuestTransitionRequest,
) -> std::result::Result<Option<AchievementGrantRequest>, DurabilityError> {
    if rules::for_transition(&request.transition_key) != Some(rules::Action::ReportQueen) {
        return Ok(None);
    }
    let entries = oteryn_gameplay_progression::achievement_catalogue::parse_shards(
        &oteryn_gameplay_progression::achievement_catalogue::SHARDS,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let entry = entries
        .get(rules::ACHIEVEMENT)
        .filter(|e| !e.retired)
        .ok_or(DurabilityError::InvalidStoredState)?;
    Ok(Some(AchievementGrantRequest {
        achievement_key: rules::ACHIEVEMENT.into(),
        catalogue: AchievementCatalogueLookup::Earnable {
            revision: entry.revision.clone(),
        },
        source: AchievementSourceEvent {
            kind: "oteryn:world-change-npc-report".into(),
            event_id: Vec::new(),
        },
    }))
}
pub(super) async fn admit(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    source: &NativeWorldChangeNpc,
) -> std::result::Result<bool, DurabilityError> {
    let Some(action) = rules::for_transition(&request.transition_key) else {
        return Ok(false);
    };
    if !source.matches(fence, request) || !source.source_is_current(tx).await? {
        return Ok(false);
    }
    if action.requires_selected_npc() {
        source.selected_npc(tx, rules::CHANGE).await
    } else {
        Ok(true)
    }
}
pub(super) async fn earn(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    changes: &[QuestTrackChange],
    achievement: Option<&AchievementGrantRequest>,
) -> std::result::Result<(), DurabilityError> {
    if request.transition_key != rules::REPORT || !rules::report_completed(changes) {
        return Ok(());
    }
    let mut grant = achievement
        .ok_or(DurabilityError::InvalidStoredState)?
        .clone();
    let QuestCause::Command(command) = request.cause else {
        return Err(DurabilityError::InvalidStoredState);
    };
    grant.source.event_id = fence.character_id.as_bytes().to_vec();
    grant
        .source
        .event_id
        .extend_from_slice(command.game_session_id().as_bytes());
    grant
        .source
        .event_id
        .extend_from_slice(&command.command_id().get().to_be_bytes());
    let granter = FencedGrantingCharacter::in_transaction(tx, fence.character_id).await?;
    record_achievement_grant(tx, granter, &grant)
        .await?
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(())
}

impl DurabilityRoot {
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_character_world_change_npc(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
        source: NativeWorldChangeNpc,
    ) -> Result<QuestTransitionOutcome> {
        self.commit_quest_inner_with_native_context::<_, 0>(
            authority,
            node,
            fence,
            request,
            catalogue,
            None,
            None,
            None,
            None,
            &grizzly::NoGrizzlyPremium,
            None,
            None,
            None,
            Some(source),
        )
        .await
    }
}
