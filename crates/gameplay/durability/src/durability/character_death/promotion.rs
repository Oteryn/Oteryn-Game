//! Per-death Premium is evaluated in the fenced death transaction, never at login.
use super::*;
use std::{future::Future, pin::Pin};

pub trait DeathPremiumOwner: Sync {
    /// Immutable, source-qualified Bless selection from the exact lethal hit.
    fn charm_loss(&self) -> Option<&DeathCharmLoss> {
        None
    }

    fn current<'a>(
        &'a self,
        tx: &'a mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> Pin<Box<dyn Future<Output = std::result::Result<bool, DurabilityError>> + Send + 'a>>;
}

/// The receipt binds this retained lethal selection; it never re-reads another assignment.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeathCharmLoss {
    pub death_occurrence: [u8; 16],
    pub source_actor: [u8; 16],
    pub race_key: String,
    pub stage: u8,
    pub percent: u8,
}

impl DeathCharmLoss {
    pub(super) fn valid_for(&self, occurrence: PlayerDeathOccurrence) -> bool {
        self.death_occurrence == *occurrence.as_bytes()
            && self.source_actor != [0; 16]
            && crate::domain::charm::BestiaryRaceKey::new(&self.race_key).is_ok()
            && match self.stage {
                1 => self.percent == 6,
                2 => self.percent == 9,
                3 => self.percent == 12,
                _ => false,
            }
    }
}

const CHARM_BINDING: &[u8] = b"SRX2BL";

pub(super) fn bind_charm_loss(
    base: &[u8],
    loss: Option<&DeathCharmLoss>,
) -> std::result::Result<Vec<u8>, DurabilityError> {
    let Some(loss) = loss else {
        return Ok(base.to_vec());
    };
    let metadata = serde_json::to_vec(loss).map_err(|_| DurabilityError::InvalidStoredState)?;
    let len = u16::try_from(metadata.len()).map_err(|_| DurabilityError::InvalidStoredState)?;
    let mut binding = Vec::with_capacity(CHARM_BINDING.len() + 2 + metadata.len() + base.len());
    binding.extend_from_slice(CHARM_BINDING);
    binding.extend_from_slice(&len.to_be_bytes());
    binding.extend_from_slice(&metadata);
    binding.extend_from_slice(base);
    if binding.len() > 1024 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(binding)
}

pub(super) fn split_charm_loss(
    binding: &[u8],
) -> std::result::Result<(Option<DeathCharmLoss>, &[u8]), DurabilityError> {
    let Some(rest) = binding.strip_prefix(CHARM_BINDING) else {
        return Ok((None, binding));
    };
    let header: [u8; 2] = rest
        .get(..2)
        .ok_or(DurabilityError::InvalidStoredState)?
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let len = usize::from(u16::from_be_bytes(header));
    let metadata = rest
        .get(2..2 + len)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let base = rest
        .get(2 + len..)
        .filter(|base| !base.is_empty())
        .ok_or(DurabilityError::InvalidStoredState)?;
    let loss = serde_json::from_slice(metadata).map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok((Some(loss), base))
}

pub(super) async fn benefit(
    tx: &mut sqlx::PgConnection,
    character: CharacterId,
    premium: bool,
) -> std::result::Result<bool, DurabilityError> {
    let promoted =
        crate::durability::character_promotion::read_durable_promotion(tx, character).await?;
    Ok(crate::domain::premium::promotion_benefit_current(
        promoted, premium,
    ))
}

pub(super) async fn held_blessings(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence,
) -> Result<Vec<String>> {
    let recovery = authority
        .record_for(root)
        .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
    let node = node.clone();
    root.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
        let mut tx=begin_semantic_transaction(holder,deadline).await?;
        assert_recovery_fence(&mut tx,&recovery).await?;
        lock_admission_relations(&mut tx).await?;
        if let Err(error)=crate::durability::character_progression::assert_item_only_fence(&mut tx,&fence.item_fence(),&node).await? {return Ok(Err(error));}
        let held=sqlx::query_scalar("SELECT blessing_key COLLATE \"C\" FROM game_character_blessings WHERE character_id=encode($1,'hex')::uuid ORDER BY 1 LIMIT 33")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_all(&mut *tx).await?;
        commit_semantic_transaction(tx,deadline).await?;
        Ok(Ok(held))
    })).await?
}

#[cfg(test)]
mod charm_binding_tests {
    use super::*;

    #[test]
    fn srx2_bless_binding_retains_exact_lethal_selection_and_legacy_bytes()
    -> std::result::Result<(), DurabilityError> {
        let mut raw = [1; 16];
        raw[6] = 0x70;
        raw[8] = 0x80;
        let occurrence = PlayerDeathOccurrence::from_bytes(raw)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let loss = DeathCharmLoss {
            death_occurrence: raw,
            source_actor: [2; 16],
            race_key: "oteryn:creature.rat".into(),
            stage: 3,
            percent: 12,
        };
        assert!(loss.valid_for(occurrence));
        let base = [1; 33];
        assert_eq!(bind_charm_loss(&base, None)?, base);
        assert_eq!(split_charm_loss(&base)?, (None, base.as_slice()));
        let bound = bind_charm_loss(&base, Some(&loss))?;
        assert_eq!(
            split_charm_loss(&bound)?,
            (Some(loss.clone()), base.as_slice())
        );
        assert!(split_charm_loss(&bound[..8]).is_err());
        let mut wrong = loss.clone();
        wrong.death_occurrence[15] ^= 1;
        assert!(!wrong.valid_for(occurrence));
        let mut wrong = loss;
        wrong.percent = 6;
        assert!(!wrong.valid_for(occurrence));
        Ok(())
    }
}
