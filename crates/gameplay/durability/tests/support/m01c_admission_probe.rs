//! Compile the exact private production read path, with no public test API.
use oteryn_gameplay_durability::durability::DurabilityError;
use oteryn_gameplay_quests::quest::QuestStateCatalogue;
type StoredResult<T> = Result<T, DurabilityError>;
#[allow(dead_code)]
#[path = "../../src/durability/quest_state/authored_admission.rs"]
mod authored_admission;
#[allow(dead_code)]
#[path = "../../src/durability/quest_state/native_prerequisites.rs"]
mod native_prerequisites;

pub async fn prerequisites_hold(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: &[u8],
    catalogue: &QuestStateCatalogue,
    quest: &str,
) -> StoredResult<bool> {
    authored_admission::prerequisites_hold(tx, character, catalogue, quest).await
}
