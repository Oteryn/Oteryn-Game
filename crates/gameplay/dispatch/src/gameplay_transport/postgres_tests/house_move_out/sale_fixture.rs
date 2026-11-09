use super::*;
use crate::durability::{guild_chat_fixture as history, item_transfer::CurrentCharacterItemFence};
pub(super) fn buyer() -> [u8; 16] {
    history::sid(90, 1)
}
pub(super) fn account() -> [u8; 16] {
    history::sid(90, 2)
}
pub(super) async fn prepare(h: &Harness) -> TestResult<CurrentCharacterItemFence> {
    history::historical_character(
        &h.pool,
        90,
        crate::domain::CharacterId::from_bytes(buyer()).map_err(debug)?,
        account(),
        id(WORLD),
        "House Recipient",
    )
    .await?;
    let fence = history::historical_session(
        &h.pool,
        90,
        super::super::super::fixture::fence(1)?.item_fence(),
    )
    .await?;
    // This is a real vocation write, not a forged eligible build snapshot.
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(WORLD)).map_err(debug)?;
    let (generation, _) =
        super::super::super::death_ml_loss_composed::production_generation_with_source(
            world,
            "content-1",
            None,
        )
        .map_err(debug)?;
    let active = generation.active().ok_or("active native generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let formula = native.training_formula().ok_or("native training formula")?;
    use crate::durability::character_build::{
        BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
    };
    use crate::durability::character_progression::CurrentCharacterGameplayFence;
    let before = DurableBuildState::default();
    let gameplay = CurrentCharacterGameplayFence {
        character_id: fence.character_id,
        game_session_id: fence.game_session_id,
        connection_generation: fence.connection_generation,
        character_lease_generation: fence.character_lease_generation,
        runtime_scope: fence.runtime_scope,
        scope_ownership_generation: fence.scope_ownership_generation,
        expected_character_revision: crate::domain::CharacterRevision::new(1).map_err(debug)?,
    };
    use crate::domain::progression::{FiniteProgressionPolicy, LevelThreshold};
    use crate::durability::character_progression::ProgressionInitializationRequest;
    use oteryn_simulation_determinism::{ExactI64, RoundingMode};
    let context = crate::bestiary_postgres_harness::context();
    h.root
        .initialize_character_progression(
            &authority,
            &h.node,
            gameplay,
            ProgressionInitializationRequest {
                context: context.clone(),
                policy_revision: "policy-1".into(),
                reward_revision: "reward-1".into(),
                policy: FiniteProgressionPolicy {
                    context,
                    policy_revision: "policy-1".into(),
                    reward_revision: "reward-1".into(),
                    death_policy_revision: "death-1".into(),
                    declared_difference_revision: "declaration-1".into(),
                    thresholds: [
                        LevelThreshold {
                            level: 1,
                            minimum_experience: ExactI64::new(0),
                        },
                        LevelThreshold {
                            level: 2,
                            minimum_experience: ExactI64::new(100),
                        },
                    ],
                    terminal_exclusive_experience: ExactI64::new(200),
                    death_loss_numerator: 1,
                    death_loss_denominator: 10,
                    death_loss_rounding: RoundingMode::Floor,
                },
            },
        )
        .await
        .map_err(debug)?;
    h.root
        .commit_character_build(
            &authority,
            &h.node,
            gameplay,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(150)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                after: convert_vocation(formula, &before, "knight").map_err(debug)?,
                before,
                pruned_stance: None,
            },
            formula,
        )
        .await
        .map_err(debug)?;
    super::super::super::guild_bank::bank_seed::balance(h, account(), buyer(), 1000, 180).await?;
    super::super::super::guild_bank::bank_seed::balance(h, id(40), id(41), 100, 160).await?;
    Ok(fence)
}
pub(super) async fn balance(h: &Harness, a: [u8; 16]) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
        .bind(a.as_slice()).bind(id(WORLD).as_slice()).fetch_one(&h.pool).await?)
}
pub(super) async fn advance_calendar(h: &Harness) -> TestResult {
    // Advance only the private House business calendar. Node possession, session
    // generations, recovery, Premium clocks and physical-transaction proofs stay real.
    sqlx::query("CREATE OR REPLACE FUNCTION game_house_calendar_milliseconds(t TIMESTAMPTZ) RETURNS BIGINT LANGUAGE sql IMMUTABLE STRICT AS $$ SELECT floor(extract(epoch FROM t)*1000)::bigint+259200000 $$")
        .execute(&h.pool).await?;
    Ok(())
}
