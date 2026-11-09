//! Registered native door receiver. Client quest commands cannot create this source.
use super::*;

/// Server-captured opening, analogous to the native death receiver context. The
/// dispatcher constructs it only from DoorBook's opaque successful observation.
#[derive(Clone, Debug)]
pub struct NativeQuestDoorOpening {
    pub transition: String,
    pub placement: [u8; 40],
    pub key_instance: [u8; 16],
    pub key_number: u16,
    pub door_revision: u64,
    pub observed_character_revision: u64,
    pub definition_hash: [u8; 32],
}

impl NativeQuestDoorOpening {
    fn usage<'a>(
        &self,
        catalogue: &'a QuestStateCatalogue,
    ) -> Option<(
        &'a oteryn_gameplay_quests::quest::authored_bindings::BoundQuest,
        &'a oteryn_gameplay_quests::quest::authored_bindings::door_use::DoorUse,
    )> {
        catalogue
            .authored_bindings()?
            .quests
            .iter()
            .find_map(|quest| {
                quest.stages.iter().find_map(|stage| {
                    let oteryn_gameplay_quests::quest::authored_bindings::Trigger::DoorUse {
                        usage,
                    } = &stage.trigger
                    else {
                        return None;
                    };
                    (usage.key == self.transition).then_some((quest, usage.as_ref()))
                })
            })
    }
    pub(super) fn valid(
        &self,
        request: &QuestTransitionRequest,
        catalogue: &QuestStateCatalogue,
    ) -> bool {
        self.transition == request.transition_key
            && matches!(request.cause, QuestCause::Command(_))
            && self.observed_character_revision > 0
            && (1..=(u64::MAX >> 2)).contains(&self.door_revision)
            && self.usage(catalogue).is_some_and(|(quest, usage)| {
                usage.matches_opening(&self.placement, self.key_number)
                    && catalogue.definition_hash(&quest.quest) == Some(self.definition_hash)
            })
    }
    pub(super) fn bind(&self, binding: &mut [u8; 33]) {
        let digest = Sha256::new()
            .chain_update(*binding)
            .chain_update(b"native-quest-door-v1")
            .chain_update(self.placement)
            .chain_update(self.key_instance)
            .chain_update(self.key_number.to_be_bytes())
            .chain_update(self.door_revision.to_be_bytes())
            .chain_update(self.observed_character_revision.to_be_bytes())
            .chain_update(self.definition_hash)
            .finalize();
        binding[1..].copy_from_slice(&digest);
    }
    pub(super) async fn admit(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        fence: CurrentCharacterGameplayFence,
        catalogue: &QuestStateCatalogue,
    ) -> StoredResult<bool> {
        if self.observed_character_revision > fence.expected_character_revision.get() {
            return Ok(false);
        }
        // One exact key instance, read under the Character root lock. The native
        // World receiver does not grant custody or trust a client-supplied key id.
        let owned:Option<Vec<u8>>=sqlx::query_scalar("SELECT uuid_send(i.item_instance_id) FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item JOIN game_item_keys k ON k.item_instance_id=i.item_instance_id WHERE i.item_instance_id=encode($3,'hex')::uuid AND i.lifecycle=1 AND i.quantity=1 AND i.definition_family='Item' AND k.key_number=$4 FOR UPDATE OF i")
            .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
            .bind(self.key_instance.as_slice()).bind(i32::from(self.key_number)).fetch_optional(&mut **tx).await?;
        if owned.is_none() {
            return Ok(false);
        }
        let (_, usage) = self
            .usage(catalogue)
            .ok_or(DurabilityError::InvalidStoredState)?;
        let tracks: Vec<_> = usage
            .storage
            .iter()
            .map(|guard| guard.track.as_str())
            .collect();
        // A later mission start/reset must not consume an earlier captured opening.
        // The binding bounds this indexed track set to the eight owned effects.
        let changed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=ANY($2::text[]) AND committed_character_revision>$3::text::numeric)")
            .bind(fence.character_id.as_bytes().as_slice()).bind(tracks).bind(self.observed_character_revision.to_string()).fetch_one(&mut **tx).await?;
        Ok(!changed)
    }
}
impl DurabilityRoot {
    pub async fn commit_character_native_quest_door(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        cause: QuestCause,
        opening: NativeQuestDoorOpening,
        catalogue: Arc<QuestStateCatalogue>,
    ) -> Result<QuestTransitionOutcome> {
        let request = QuestTransitionRequest {
            transition_key: opening.transition.clone(),
            cause,
        };
        self.commit_quest_inner::<_, 0>(
            authority,
            node,
            fence,
            request,
            catalogue,
            None,
            None,
            None,
            None,
            &super::grizzly::NoGrizzlyPremium,
            None,
            None,
            None,
            Some(super::native_observation::NativeQuestObservation::Door(
                opening,
            )),
            None,
            None,
            None,
            None,
        )
        .await
    }
}
