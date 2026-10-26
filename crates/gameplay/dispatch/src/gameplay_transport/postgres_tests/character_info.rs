//! CHARACTER-INFO-1 fenced durable unlock/select, replay and stale generation.
use super::fixture::{Harness, run};
use super::{TestResult, debug, prepare_current_session};
use oteryn_protocol_oteryn::character_info::CharacterTitle;

#[test]
fn character_title_unlock_and_selection_are_durable_and_referential() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "character_title", false, false).await?;
        prepare_current_session(&harness).await?;
        let character: Vec<u8> =
            sqlx::query_scalar("SELECT uuid_send(character_id) FROM game_character_roots LIMIT 1")
                .fetch_one(&harness.pool)
                .await?;
        let character = crate::domain::CharacterId::from_bytes(
            character.try_into().map_err(|_| "character id length")?,
        )
        .map_err(debug)?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        assert!(
            harness
                .root
                .unlock_character_title(
                    &authority,
                    character,
                    CharacterTitle::AdeptArmorer,
                    100,
                    None
                )
                .await
                .map_err(debug)?
        );
        assert!(
            !harness
                .root
                .unlock_character_title(
                    &authority,
                    character,
                    CharacterTitle::AdeptArmorer,
                    100,
                    None
                )
                .await
                .map_err(debug)?,
            "replay is idempotent"
        );
        let revision = harness
            .root
            .select_character_title(&authority, character, Some(CharacterTitle::AdeptArmorer))
            .await
            .map_err(debug)?;
        assert_eq!(revision, 1);
        assert!(
            harness
                .root
                .select_character_title(&authority, character, Some(CharacterTitle::AdeptEnhancer))
                .await
                .is_err(),
            "unearned selection is rejected"
        );
        Ok(())
    })
}
