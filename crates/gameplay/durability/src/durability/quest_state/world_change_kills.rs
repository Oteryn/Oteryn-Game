//! Native credited kills compose through the existing Quest writer and receipt.
use super::super::account_achievement::{
    AchievementCatalogueLookup, AchievementGrantRequest, AchievementSourceEvent,
    FencedGrantingCharacter, record_achievement_grant,
};
use super::super::spell_owner_commit::SpellLanePermit;
use super::*;
use oteryn_gameplay_quests::quest::world_change_kills as rules;
use sqlx::{Postgres, Transaction};

/// The dispatch adapter builds this only from the existing sealed native death
/// and its frozen reward principal. A live successor session cannot inherit it.
#[derive(Debug, Clone)]
pub struct NativeWorldChangeKill {
    proof: crate::foundation::NativeCreditedKillProof,
    death: crate::foundation::CreatureDeathOccurrenceKey,
    occurrence: [u8; 16],
    creature: String,
    character: CharacterId,
    session: GameSessionId,
    lease_generation: u64,
    world: super::super::quest_world_callback::QuestWorldContext,
    corpse: (u16, u16, i8),
}

impl NativeWorldChangeKill {
    pub fn from_native_proof(
        proof: crate::foundation::NativeCreditedKillProof,
        reset_epoch: u64,
    ) -> Result<Self> {
        let principal = proof.principal();
        let character = CharacterId::from_bytes(*principal.character.as_bytes())
            .map_err(|_| CharacterProgressionError::InvalidInput)?;
        let p = proof.position();
        let corpse = (
            u16::try_from(p.x).map_err(|_| CharacterProgressionError::InvalidInput)?,
            u16::try_from(p.y).map_err(|_| CharacterProgressionError::InvalidInput)?,
            i8::try_from(p.floor).map_err(|_| CharacterProgressionError::InvalidInput)?,
        );
        let hex = |bytes: [u8; 32]| {
            format!(
                "sha256:{}",
                bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
            )
        };
        Ok(Self {
            death: proof.death(),
            occurrence: proof.occurrence(),
            creature: proof.creature().into(),
            character,
            session: principal.session,
            lease_generation: principal.lease_generation,
            world: super::super::quest_world_callback::QuestWorldContext {
                map_revision: hex(proof.map_digest()),
                content_revision: hex(proof.content_digest()),
                placement_context: proof.frame_digest().to_vec(),
                reset_epoch,
                actor_placement: [0; 16],
                movement_revision: 0,
            },
            corpse,
            proof,
        })
    }
    pub const fn death(&self) -> crate::foundation::CreatureDeathOccurrenceKey {
        self.death
    }
    pub const fn character(&self) -> CharacterId {
        self.character
    }
    pub const fn session(&self) -> GameSessionId {
        self.session
    }
    pub const fn lease_generation(&self) -> u64 {
        self.lease_generation
    }
    pub fn creature(&self) -> &str {
        self.proof.creature()
    }

    pub(super) fn binds(
        &self,
        fence: CurrentCharacterGameplayFence,
        request: &QuestTransitionRequest,
    ) -> bool {
        let Some(rule) = rules::for_creature(&self.creature) else {
            return false;
        };
        self.character == fence.character_id
            && self.session == fence.game_session_id
            && self.lease_generation == fence.character_lease_generation
            && self.death.runtime_scope() == fence.runtime_scope
            && self.death.scope_ownership_generation() == fence.scope_ownership_generation
            && request.transition_key == rule.transition
            && matches!(request.cause, QuestCause::CreatureDeath(id) if id.as_bytes() == &self.occurrence)
    }

    pub(super) fn bind(&self, binding: &mut [u8; 33]) -> Result<()> {
        let world =
            serde_json::to_vec(&self.world).map_err(|_| CharacterProgressionError::InvalidInput)?;
        let digest = Sha256::new()
            .chain_update(binding.as_slice())
            .chain_update(super::super::party_kill_xp::encode_death_key(self.death))
            .chain_update(self.character.as_bytes())
            .chain_update(self.session.as_bytes())
            .chain_update(self.lease_generation.to_be_bytes())
            .chain_update(self.creature.as_bytes())
            .chain_update(self.corpse.0.to_be_bytes())
            .chain_update(self.corpse.1.to_be_bytes())
            .chain_update(self.corpse.2.to_be_bytes())
            .chain_update(world)
            .finalize();
        binding[1..].copy_from_slice(&digest);
        Ok(())
    }
}

/// A Channel decision captured at boot is not sufficient: recheck current reset,
/// activation and selected spawn membership under the Quest transaction fence.
pub(super) async fn admit(
    tx: &mut Transaction<'_, Postgres>,
    permit: &SpellLanePermit,
    fence: CurrentCharacterGameplayFence,
    context: &NativeWorldChangeKill,
) -> std::result::Result<bool, DurabilityError> {
    if !super::super::quest_world_callback::context_is_current(tx, permit, fence, &context.world)
        .await?
    {
        return Ok(false);
    }
    let Some(day) = super::super::world_event::changes::load(
        tx,
        context.death.world_id(),
        context.world.reset_epoch,
    )
    .await?
    else {
        return Ok(false);
    };
    let Some(rule) = rules::for_creature(&context.creature) else {
        return Ok(false);
    };
    // Water Elementals outside the selected dungeon cannot acquire its counter.
    // The bounded dungeon area is derived only from the selected variant's spawn
    // points, with a conservative 16-tile movement allowance around those points.
    let definitions = oteryn_gameplay_world_change::variants::definitions()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let selected_variant = day.variant(rule.change).and_then(|index| {
        definitions
            .iter()
            .find(|d| d.key == rule.change)
            .and_then(|d| d.variants.get(index))
    });
    let selected = selected_variant.is_some_and(|variant| {
        variant.spawns.iter().any(|spawn| {
            spawn.creature == rule.creature
                && (rule != rules::DOWN_THE_DRAIN
                    || (spawn.position.2 == context.corpse.2
                        && spawn.position.0.abs_diff(context.corpse.0) <= 16
                        && spawn.position.1.abs_diff(context.corpse.1) <= 16))
        })
    });
    Ok(selected)
}

pub(super) fn prepare(
    request: &QuestTransitionRequest,
) -> std::result::Result<Option<AchievementGrantRequest>, DurabilityError> {
    let Some(rule) = rules::for_transition(&request.transition_key) else {
        return Ok(None);
    };
    let entries = oteryn_gameplay_progression::achievement_catalogue::parse_shards(
        &oteryn_gameplay_progression::achievement_catalogue::SHARDS,
    )
    .map_err(|_| DurabilityError::InvalidStoredState)?;
    let entry = entries
        .get(rule.achievement)
        .filter(|e| !e.retired)
        .ok_or(DurabilityError::InvalidStoredState)?;
    Ok(Some(AchievementGrantRequest {
        achievement_key: rule.achievement.into(),
        catalogue: AchievementCatalogueLookup::Earnable {
            revision: entry.revision.clone(),
        },
        source: AchievementSourceEvent {
            kind: "oteryn:world-change-credited-kill".into(),
            event_id: Vec::new(),
        },
    }))
}

pub(super) async fn earn(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    changes: &[QuestTrackChange],
    achievement: Option<&AchievementGrantRequest>,
) -> std::result::Result<(), DurabilityError> {
    let Some(rule) = rules::for_transition(&request.transition_key) else {
        return Ok(());
    };
    if !changes
        .iter()
        .any(|c| c.track == rule.track && c.before == rule.maximum - 1 && c.after == rule.maximum)
    {
        return Ok(());
    }
    let mut grant = achievement
        .ok_or(DurabilityError::InvalidStoredState)?
        .clone();
    grant.source.event_id = fence.character_id.as_bytes().to_vec();
    let granter = FencedGrantingCharacter::in_transaction(tx, fence.character_id).await?;
    record_achievement_grant(tx, granter, &grant)
        .await?
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(())
}

impl DurabilityRoot {
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_character_world_change_kill(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        catalogue: Arc<QuestStateCatalogue>,
        permit: &SpellLanePermit,
        context: NativeWorldChangeKill,
    ) -> Result<QuestTransitionOutcome> {
        let rule = rules::for_creature(&context.creature)
            .ok_or(CharacterProgressionError::InvalidInput)?;
        let cause = QuestCause::CreatureDeath(
            ExperienceRewardOccurrence::from_bytes(context.occurrence)
                .map_err(|_| CharacterProgressionError::InvalidInput)?,
        );
        self.commit_quest_inner_with_world_kill::<_, 0>(
            authority,
            node,
            fence,
            QuestTransitionRequest {
                transition_key: rule.transition.into(),
                cause,
            },
            catalogue,
            None,
            None,
            None,
            None,
            &grizzly::NoGrizzlyPremium,
            None,
            None,
            Some((permit, context)),
        )
        .await
    }
}
