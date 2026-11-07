//! A real current movement step admits a source-declared cooldown through Quest.
use super::super::item_transfer::CurrentCharacterItemFence;
use super::*;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, MovementPositionSnapshot};
use oteryn_gameplay_quests::quest::world_change_entry as rules;
use oteryn_gameplay_world_change::variants;
use sqlx::{Postgres, Transaction};

#[derive(Debug, Clone)]
pub struct NativeWorldChangeEntry {
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    position: variants::Position,
    epoch: u64,
    content: [u8; 32],
    frame: [u8; 32],
}
impl NativeWorldChangeEntry {
    pub fn from_current_runtime(
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        command: CommandRef,
        entered: MovementPositionSnapshot,
        epoch: u64,
        fence: CurrentCharacterItemFence,
    ) -> Result<Option<Self>> {
        runtime
            .owner_fence()
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        if command.game_session_id() != fence.game_session_id
            || runtime.binding().runtime_scope() != fence.runtime_scope
            || runtime.binding().scope_generation() != fence.scope_ownership_generation
            || runtime.read_actor_position(actor) != Ok(entered)
            || !runtime
                .bound_attacker_lease(actor, command.game_session_id())
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
        {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        let p = entered.position();
        let position = (
            u16::try_from(p.x).map_err(|_| CharacterProgressionError::InvalidInput)?,
            u16::try_from(p.y).map_err(|_| CharacterProgressionError::InvalidInput)?,
            i8::try_from(p.floor).map_err(|_| CharacterProgressionError::InvalidInput)?,
        );
        let definitions =
            variants::definitions().map_err(|_| CharacterProgressionError::InvalidInput)?;
        if !definitions.iter().any(|d| {
            d.key == rules::CHANGE
                && d.entry_cooldown.as_deref() == Some(rules::CHANGE)
                && d.variants
                    .iter()
                    .flat_map(|v| &v.portals)
                    .any(|p| !p.exit && p.position == position)
        }) {
            return Ok(None);
        }
        Ok(Some(Self {
            fence,
            command,
            position,
            epoch,
            content: runtime.content_pin().server_artifact_digest(),
            frame: runtime.content_pin().frame_binding_digest(),
        }))
    }
    fn matches(
        &self,
        fence: CurrentCharacterGameplayFence,
        request: &QuestTransitionRequest,
    ) -> bool {
        self.fence == fence.item_fence()
            && rules::for_transition(&request.transition_key)
            && matches!(request.cause,QuestCause::Step(command) if command==self.command)
    }
}
pub(super) async fn admit(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    source: &NativeWorldChangeEntry,
) -> std::result::Result<bool, DurabilityError> {
    if !source.matches(fence, request) {
        return Ok(false);
    }
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(false);
    };
    let Some(day) = super::super::world_event::changes::load(tx, world_id, source.epoch).await?
    else {
        return Ok(false);
    };
    let current:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND activation_sequence=(SELECT max(activation_sequence) FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid) AND server_artifact_digest=$3 AND frame_binding_digest=$4)")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(source.content.as_slice()).bind(source.frame.as_slice()).fetch_one(&mut **tx).await?;
    let definitions = variants::definitions().map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(current
        && definitions
            .iter()
            .find(|d| d.key == rules::CHANGE && d.entry_cooldown.as_deref() == Some(rules::CHANGE))
            .and_then(|d| day.variant(&d.key).and_then(|i| d.variants.get(i)))
            .is_some_and(|v| {
                v.portals
                    .iter()
                    .any(|p| !p.exit && p.position == source.position)
            }))
}
impl DurabilityRoot {
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_character_world_change_entry(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: Arc<QuestStateCatalogue>,
        source: NativeWorldChangeEntry,
    ) -> Result<QuestTransitionOutcome> {
        self.commit_quest_inner_with_native_sources::<_, 0>(
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
            None,
            Some(source),
            None,
        )
        .await
    }
}
