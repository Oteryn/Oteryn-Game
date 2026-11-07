//! Native death descendants use the existing fenced Quest writer and immutable XP audit.
use super::*;
use oteryn_gameplay_quests::quest::kill_objectives::KillObjective;

/// Captured only from a committed native death and its coherent Character quest copy.
/// No wire command can construct the sealed death identity.
#[derive(Clone, Debug)]
pub struct NativeObjectiveDeath {
    death: Vec<u8>,
    character: CharacterId,
    position: (i32, i32, i16),
    observed_revision: u64,
    phase: i64,
}

impl NativeObjectiveDeath {
    pub fn capture(
        runtime: &mut crate::foundation::ChannelRuntimeV1,
        target: crate::foundation::ExactActorRef,
        character: CharacterId,
        observed_revision: u64,
        phase: i64,
    ) -> Option<Self> {
        if observed_revision == 0 { return None; }
        let (death, at) = runtime.borrow_combat_death().projected_death(target).ok()?;
        Some(Self {
            death: super::super::party_kill_xp::encode_death_key(death),
            character, position: (at.x, at.y, at.floor), observed_revision, phase,
        })
    }

    fn occurrence(&self) -> Result<super::super::character_progression::ExperienceRewardOccurrence> {
        let hash = Sha256::new().chain_update(b"oteryn:quest-native-death:v1")
            .chain_update(&self.death).chain_update(self.character.as_bytes()).finalize();
        let mut id = [0; 16]; id.copy_from_slice(&hash[..16]);
        id[6] = (id[6] & 15) | 0x70; id[8] = (id[8] & 63) | 0x80;
        super::super::character_progression::ExperienceRewardOccurrence::from_bytes(id)
    }

    pub(super) fn bind(&self, binding: &mut [u8; 33]) -> Result<()> {
        let hash = Sha256::new().chain_update(*binding).chain_update(&self.death)
            .chain_update(self.character.as_bytes()).chain_update(self.position.0.to_be_bytes())
            .chain_update(self.position.1.to_be_bytes()).chain_update(self.position.2.to_be_bytes())
            .chain_update(self.observed_revision.to_be_bytes()).chain_update(self.phase.to_be_bytes())
            .finalize();
        binding[1..].copy_from_slice(&hash); Ok(())
    }
}

impl DurabilityRoot {
    #[allow(clippy::too_many_arguments, reason = "joins captured native death to existing fenced Quest authority")]
    pub async fn commit_character_native_kill_objective(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        transition: &str,
        source: NativeObjectiveDeath,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome> {
        self.commit_quest_inner::<_, 0>(authority, node, fence,
            QuestTransitionRequest { transition_key: transition.into(),
                cause: QuestCause::CreatureDeath(source.occurrence()?) },
            catalogue, None, None, None, None, &grizzly::NoGrizzlyPremium,
            None, None, None, Some(source)).await
    }
}

pub(super) async fn authorize(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: &CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    rule: KillObjective,
    source: &NativeObjectiveDeath,
) -> StoredResult<bool> {
    if source.character != fence.character_id || source.phase != rule.phase_before
        || !rule.contains(source.position.0, source.position.1, source.position.2)
        || !super::super::party_kill_xp::death_matches_fence(&source.death, fence)
        || !matches!(request.cause, QuestCause::CreatureDeath(id)
            if source.occurrence().is_ok_and(|expected| id == expected))
    { return Ok(false); }
    // A shared-XP bystander is never the personal objective principal. Zero-stamina
    // deaths still have this native member audit and do not require an XP grant.
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_party_kill_xp_receipts d \
        JOIN game_party_kill_xp_members m USING(death_key) \
        JOIN game_character_quest_states q ON q.character_id=m.character_id \
        WHERE d.death_key=$1 AND d.principal_id=encode($2,'hex')::uuid \
        AND m.character_id=d.principal_id AND d.creature_key=$3 AND q.quest_key=$4 \
        AND q.started_character_revision <= $5::text::numeric)")
        .bind(&source.death).bind(fence.character_id.as_bytes().as_slice())
        .bind(rule.creature).bind(rule.quest).bind(source.observed_revision.to_string())
        .fetch_one(&mut **tx).await.map_err(DurabilityError::from)
}
