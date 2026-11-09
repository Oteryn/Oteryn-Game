//! M03: evaluate chosen recipe prerequisites under the quest writer's Character lock.
//! Evidence: content/quests/definitions/quests-00200-00299.json:1, record 67,
//! requirements. Promotion uses the permanent purchase owner, not a session vocation.
use crate::{domain::CharacterId, durability::DurabilityError};
use oteryn_gameplay_quests::quest::world_callbacks::CharacterRequirements;

pub(super) async fn satisfied(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    requirements: &CharacterRequirements,
) -> Result<bool, DurabilityError> {
    if requirements.promoted
        && !crate::durability::character_promotion::read_durable_promotion(tx, character).await?
    {
        return Ok(false);
    }
    let held: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_blessings WHERE character_id=encode($1,'hex')::uuid AND blessing_key=ANY($2::text[])")
        .bind(character.as_bytes().as_slice())
        .bind(&requirements.blessings)
        .fetch_one(&mut **tx)
        .await?;
    Ok(usize::try_from(held).ok() == Some(requirements.blessings.len()))
}

/// The existing Premium owner evaluates its native account fence inside this transaction.
/// This is shared with reward admission; no cached session boolean is authoritative.
pub(crate) async fn premium_is_current<
    G: crate::durability::quest_state::grizzly::GrizzlyPremiumGate,
>(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
    gate: &G,
) -> Result<bool, DurabilityError> {
    let account: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    gate.task_permitted(tx, *account.as_bytes()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bestiary_postgres_harness as harness;

    struct PremiumProbe {
        allowed: bool,
        transaction: String,
        calls: std::sync::atomic::AtomicUsize,
    }
    impl crate::durability::quest_state::grizzly::GrizzlyPremiumGate for PremiumProbe {
        async fn task_permitted(
            &self,
            tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
            account: [u8; 16],
        ) -> Result<bool, DurabilityError> {
            assert_eq!(
                account,
                harness::id(40),
                "Premium must bind the durable account, not the character or a caller-supplied identity"
            );
            let alive: i32 = sqlx::query_scalar("SELECT 1").fetch_one(&mut **tx).await?;
            assert_eq!(alive, 1);
            let transaction: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
                .fetch_one(&mut **tx)
                .await?;
            assert_eq!(
                transaction, self.transaction,
                "Premium must share the quest transaction"
            );
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(self.allowed)
        }
    }

    #[test]
    fn m03_postgres_world_premium_uses_current_account_and_transaction() -> harness::TestResult {
        let Some(admin) = harness::configured_admin() else {
            return Ok(());
        };
        harness::runtime()?.block_on(async move {
            let h = harness::Harness::create(admin, "srvenc_m03_premium", true).await?;
            let result: harness::TestResult = async {
                let character = CharacterId::from_bytes(harness::id(harness::CHARACTER)).map_err(harness::debug)?;
                let mut tx = h.pool.begin().await?;
                let transaction: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text").fetch_one(&mut *tx).await?;
                let gate = PremiumProbe { allowed: true, transaction: transaction.clone(), calls: std::sync::atomic::AtomicUsize::new(0) };
                assert!(premium_is_current(&mut tx, character, &gate).await?);
                let refused = PremiumProbe { allowed: false, transaction, calls: std::sync::atomic::AtomicUsize::new(0) };
                assert!(!premium_is_current(&mut tx, character, &refused).await?);
                assert!(!premium_is_current(&mut tx, character, &crate::durability::quest_state::grizzly::NoGrizzlyPremium).await?);
                let absent: Option<&PremiumProbe> = None;
                assert!(!premium_is_current(&mut tx, character, &absent).await?);
                assert_eq!(gate.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
                assert_eq!(refused.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
                tx.rollback().await?;
                eprintln!("M03 REAL POSTGRES Premium: durable account + same transaction; current/refused/missing owner: PASS");
                Ok(())
            }.await;
            h.cleanup().await?;
            result
        })
    }

    #[test]
    fn m03_postgres_character_requirements_read_current_transaction() -> harness::TestResult {
        let Some(admin) = harness::configured_admin() else {
            return Ok(());
        };
        harness::runtime()?.block_on(async move {
            let h = harness::Harness::create(admin, "srvenc_m03_character", true).await?;
            let result: harness::TestResult = async {
                let character = CharacterId::from_bytes(harness::id(harness::CHARACTER)).map_err(harness::debug)?;
                let mut tx = h.pool.begin().await?;
                // The production quest writer already owns this root before consume().
                sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                    .bind(character.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                let mut requirements = CharacterRequirements { premium: false, promoted: true, blessings: vec![] };
                assert!(!satisfied(&mut tx, character, &requirements).await?);
                // A deferred purchase FK is deliberately never committed: this fixture
                // tests observation, not the independent promotion purchase owner.
                sqlx::query("INSERT INTO game_character_promotions(character_id,purchase_occurrence_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
                    .bind(character.as_bytes().as_slice()).bind(harness::id(201).as_slice()).execute(&mut *tx).await?;
                assert!(satisfied(&mut tx, character, &requirements).await?);
                requirements.blessings = oteryn_gameplay_quests::quest::world_callbacks::load()?.into_iter()
                    .find(|callback| callback.key == "oteryn:quest-callback/qm03/anniversary-25095")
                    .and_then(|callback| callback.character_requirements).ok_or("registered time machine requirements")?.blessings;
                assert_eq!(requirements.blessings.len(), 5);
                assert!(!satisfied(&mut tx, character, &requirements).await?);
                for key in &requirements.blessings {
                    sqlx::query("INSERT INTO game_character_blessings(character_id,blessing_key,provenance) VALUES(encode($1,'hex')::uuid,$2,'m03-fixture')")
                        .bind(character.as_bytes().as_slice()).bind(key).execute(&mut *tx).await?;
                }
                assert!(satisfied(&mut tx, character, &requirements).await?);
                sqlx::query("DELETE FROM game_character_blessings WHERE character_id=encode($1,'hex')::uuid AND blessing_key=$2")
                    .bind(character.as_bytes().as_slice()).bind(&requirements.blessings[0]).execute(&mut *tx).await?;
                assert!(!satisfied(&mut tx, character, &requirements).await?, "death-consumed blessing must not survive in a cached admission");
                tx.rollback().await?;
                eprintln!("M03 REAL POSTGRES: missing promotion, acquired promotion, missing blessings, all held, consumed blessing: PASS");
                Ok(())
            }.await;
            h.cleanup().await?;
            result
        })
    }
}
