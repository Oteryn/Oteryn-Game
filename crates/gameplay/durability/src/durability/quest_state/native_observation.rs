//! One internal source slot for native observations; public receiver APIs stay distinct.
use super::*;

#[derive(Clone, Debug)]
pub(crate) enum NativeQuestObservation {
    Death(super::native_kills::NativeQuestDeath),
    Door(super::native_doors::NativeQuestDoorOpening),
}

impl From<super::native_kills::NativeQuestDeath> for NativeQuestObservation {
    fn from(death: super::native_kills::NativeQuestDeath) -> Self {
        Self::Death(death)
    }
}
impl NativeQuestObservation {
    pub(super) fn valid(
        &self,
        request: &QuestTransitionRequest,
        catalogue: &QuestStateCatalogue,
    ) -> bool {
        match self {
            Self::Death(death) => death.valid(request, catalogue),
            Self::Door(opening) => opening.valid(request, catalogue),
        }
    }
    pub(super) fn bind(&self, binding: &mut [u8; 33]) -> Result<()> {
        match self {
            Self::Death(death) => death.bind(binding),
            Self::Door(opening) => {
                opening.bind(binding);
                Ok(())
            }
        }
    }
    pub(super) fn party(&self) -> Option<&crate::foundation::PartySnapshot> {
        match self {
            Self::Death(death) => death.party(),
            Self::Door(_) => None,
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
            Self::Door(opening) => opening.admit(tx, fence, catalogue).await,
        }
    }
}
