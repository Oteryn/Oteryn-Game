//! Shared atomic settlement seam; each physical event retains its own admission rules.
use super::*;
use crate::content::native_gameplay::NativeGameplayState;

#[derive(Clone, Debug)]
pub(crate) enum NativeQuestEvent<'a> {
    Door(native_doors::NativeQuestDoorOpening),
    Death(native_kills::NativeQuestDeath, Option<&'a NativeGameplayState>),
    Hit(native_hits::NativeQuestHit),
}
impl NativeQuestEvent<'_> {
    pub(super) fn evaluation_time(&self, catalogue: &QuestStateCatalogue, now: i64) -> i64 {
        match self {
            Self::Death(death, _) => death.evaluation_time(catalogue, now),
            Self::Door(_) | Self::Hit(_) => now,
        }
    }

    pub(super) fn permits_scope(&self, fence: &CurrentCharacterGameplayFence) -> bool {
        match self {
            Self::Death(death, _) => death.permits_scope(fence),
            Self::Hit(_) | Self::Door(_) => matches!(fence.runtime_scope, crate::foundation::RuntimeScopeRefV1::Channel { .. }),
        }
    }

    pub(super) fn valid(
        &self,
        request: &QuestTransitionRequest,
        catalogue: &QuestStateCatalogue,
    ) -> bool {
        match self {
            Self::Death(death, _) => death.valid(request, catalogue),
            Self::Hit(hit) => hit.valid(request, catalogue),
            Self::Door(door) => door.valid(request, catalogue),
        }
    }
    pub(super) fn bind(&self, binding: &mut [u8; 33]) -> Result<()> {
        match self {
            Self::Death(death, _) => death.bind(binding),
            Self::Door(door) => { door.bind(binding); Ok(()) },
            Self::Hit(hit) => {
                hit.bind(binding);
                Ok(())
            }
        }
    }
    pub(super) fn party(&self) -> Option<&crate::foundation::PartySnapshot> {
        match self {
            Self::Death(death, _) => death.party(),
            Self::Hit(_) | Self::Door(_) => None,
        }
    }
    pub(super) async fn admit(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        fence: CurrentCharacterGameplayFence,
        catalogue: &QuestStateCatalogue,
    ) -> StoredResult<bool> {
        match self {
            Self::Death(death, _) => death.admit(tx, fence, catalogue).await,
            Self::Hit(hit) => hit.admit(tx, fence, catalogue).await,
            Self::Door(door) => door.admit(tx, fence, catalogue).await,
        }
    }
}

impl From<native_kills::NativeQuestDeath> for NativeQuestEvent<'_> {
    fn from(death: native_kills::NativeQuestDeath) -> Self { Self::Death(death, None) }
}
