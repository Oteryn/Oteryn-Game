//! Base outfit rewards from Crystal 201189f8's native boss death callbacks.

use crate::content::native_gameplay::NativeGameplayState;
use crate::domain::CharacterId;
use crate::durability::character_appearance::quest_grants::{
    grant_validated_content_outfit, outfit_compatible,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::{GameSessionId, RuntimeScopeRefV1, ScopeOwnershipGeneration};
use oteryn_gameplay_quests::quest::reward_callbacks::OutfitReward;
use sqlx::{Postgres, Transaction, types::Uuid};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossOutfit {
    Poltergeist,
    Rascoohan,
    FiendSlayer,
    FormalDress,
}

impl BossOutfit {
    pub fn for_death(creature: &str) -> Option<Self> {
        Some(match creature {
            "oteryn:creature.the_pale_worm" => Self::Poltergeist,
            "oteryn:creature.ratmiral_blackwhiskers" => Self::Rascoohan,
            "oteryn:creature.arbaziloth" | "oteryn:creature.enraged_arbaziloth" => {
                Self::FiendSlayer
            }
            "oteryn:creature.lord_retro" => Self::FormalDress,
            _ => return None,
        })
    }

    fn reward(self) -> OutfitReward {
        let outfit = match self {
            Self::Poltergeist => "oteryn:outfit.poltergeist",
            Self::Rascoohan => "oteryn:outfit.rascoohan",
            Self::FiendSlayer => "oteryn:outfit.fiend_slayer",
            Self::FormalDress => "oteryn:outfit.formal_dress",
        };
        OutfitReward {
            outfit: outfit.into(),
            addons: 0,
            sex: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BossOutfitRecipient {
    pub character: CharacterId,
    pub session: GameSessionId,
    pub lease_generation: u64,
}

/// The dispatch owner freezes these exact players on the lethal turn. The
/// settlement owner independently proves their retained issuance before granting.
#[derive(Clone, Debug)]
pub struct BossOutfitDeath {
    outfit: BossOutfit,
    recipients: Vec<BossOutfitRecipient>,
}

impl BossOutfitDeath {
    pub fn admit(
        outfit: BossOutfit,
        mut recipients: Vec<BossOutfitRecipient>,
        native: &NativeGameplayState,
    ) -> Option<Self> {
        if recipients.is_empty() || !outfit_compatible(native, &outfit.reward()) {
            return None;
        }
        recipients.sort_by_key(|recipient| *recipient.character.as_bytes());
        Some(Self { outfit, recipients })
    }

    pub(super) fn matches_settlement(&self, boss_key: &str) -> bool {
        BossOutfit::for_death(boss_key) == Some(self.outfit)
    }

    pub(super) async fn grant(
        &self,
        root: &DurabilityRoot,
        tx: &mut Transaction<'_, Postgres>,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        death: [u8; 16],
    ) -> Result<(), DurabilityError> {
        // Read the existing room owner's accepted victory, including the hard form.
        // A late death or a wild Arbaziloth supplies no room receipt.
        let room = match (self.outfit, scope) {
            (BossOutfit::FiendSlayer, RuntimeScopeRefV1::Instance { instance_id, .. }) => {
                Some(instance_id)
            }
            (BossOutfit::FiendSlayer, _) => return Ok(()),
            _ => None,
        };
        let world = scope.world_id();
        let (channel, instance) = match scope {
            RuntimeScopeRefV1::Channel { channel_id, .. } => (Some(*channel_id.as_bytes()), None),
            RuntimeScopeRefV1::Instance { instance_id, .. } => (None, Some(instance_id)),
        };
        let mut accounts = BTreeMap::<Uuid, CharacterId>::new();
        let admissions = FreshAdmissionStore::from_root(root.clone());
        for recipient in &self.recipients {
            let character = crate::foundation::CharacterId::decode(recipient.character.as_bytes())
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if !admissions
                .committed_session_use(tx, character, recipient.session)
                .await?
            {
                return Err(DurabilityError::CommitRejected);
            }
            let account: Option<Uuid> = sqlx::query_scalar(
                "SELECT r.account_id FROM game_character_roots r
                 JOIN game_durability_reconnect_sessions s USING(character_id)
                 WHERE r.character_id=encode($1,'hex')::uuid AND r.world_id=encode($2,'hex')::uuid
                   AND s.world_id=r.world_id AND s.game_session_id=encode($3,'hex')::uuid
                   AND s.account_id=r.account_id AND s.character_lease_generation=$4::text::numeric
                   AND s.runtime_scope_world_id=r.world_id AND s.scope_ownership_generation=$5::text::numeric
                   AND s.runtime_scope_channel_id IS NOT DISTINCT FROM encode($6,'hex')::uuid
                   AND s.runtime_scope_instance_id IS NOT DISTINCT FROM encode($7,'hex')::uuid",
            )
            .bind(recipient.character.as_bytes().as_slice())
            .bind(world.as_bytes().as_slice())
            .bind(recipient.session.as_bytes().as_slice())
            .bind(recipient.lease_generation.to_string())
            .bind(generation.get().to_string())
            .bind(channel.as_ref().map(|id| id.as_slice()))
            .bind(instance.as_ref().map(|id| id.as_slice()))
            .fetch_optional(&mut **tx)
            .await?;
            let Some(account) = account else {
                return Err(DurabilityError::CommitRejected);
            };
            if let Some(instance) = room {
                let victorious: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM game_boss_room_activities a
                     JOIN game_activity_handoff_groups g ON g.handoff_id=a.admission
                     JOIN game_boss_room_participants p ON p.admission=a.admission
                     WHERE a.world_id=encode($1,'hex')::uuid AND a.instance_id=encode($2,'hex')::uuid
                       AND g.ownership_generation=$3::text::numeric AND a.defeat_id=encode($4,'hex')::uuid
                       AND a.state IN (2,4) AND p.character_id=encode($5,'hex')::uuid
                       AND a.boss_creature_key IN ('oteryn:creature.arbaziloth','oteryn:creature.enraged_arbaziloth'))",
                )
                .bind(world.as_bytes().as_slice()).bind(instance.as_slice())
                .bind(generation.get().to_string()).bind(death.as_slice())
                .bind(recipient.character.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                if !victorious {
                    continue;
                }
            }
            accounts.entry(account).or_insert(recipient.character);
        }
        // Parallel deaths take account appearance locks in the same order. The
        // appearance owner keeps the original receipt across repeat victories.
        let occurrence = [b"native-boss-outfit\0".as_slice(), world.as_bytes(), &death].concat();
        let reward = self.outfit.reward();
        for character in accounts.into_values() {
            grant_validated_content_outfit(tx, character, &occurrence, &reward).await?;
        }
        Ok(())
    }
}
