//! Shared atomic settlement seam; each physical event retains its own admission rules.
use super::*;

#[derive(Clone, Debug)]
pub(crate) enum NativeQuestEvent {
    Door(native_doors::NativeQuestDoorOpening),
    Death(native_kills::NativeQuestDeath),
    Hit(native_hits::NativeQuestHit),
}
impl NativeQuestEvent {
    pub(super) fn permits_scope(&self, fence: &CurrentCharacterGameplayFence) -> bool {
        match self {
            Self::Death(death) => death.permits_scope(fence),
            Self::Hit(_) | Self::Door(_) => matches!(fence.runtime_scope, crate::foundation::RuntimeScopeRefV1::Channel { .. }),
        }
    }

    pub(super) fn valid(
        &self,
        request: &QuestTransitionRequest,
        catalogue: &QuestStateCatalogue,
    ) -> bool {
        match self {
            Self::Death(death) => death.valid(request, catalogue),
            Self::Hit(hit) => hit.valid(request, catalogue),
            Self::Door(door) => door.valid(request, catalogue),
        }
    }
    pub(super) fn bind(&self, binding: &mut [u8; 33]) -> Result<()> {
        match self {
            Self::Death(death) => death.bind(binding),
            Self::Door(door) => { door.bind(binding); Ok(()) },
            Self::Hit(hit) => {
                hit.bind(binding);
                Ok(())
            }
        }
    }
    pub(super) fn party(&self) -> Option<&crate::foundation::PartySnapshot> {
        match self {
            Self::Death(death) => death.party(),
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
            Self::Death(death) => death.admit(tx, fence, catalogue).await,
            Self::Hit(hit) => hit.admit(tx, fence, catalogue).await,
            Self::Door(door) => door.admit(tx, fence, catalogue).await,
        }
    }
}

impl From<native_kills::NativeQuestDeath> for NativeQuestEvent {
    fn from(death: native_kills::NativeQuestDeath) -> Self { Self::Death(death) }
}
