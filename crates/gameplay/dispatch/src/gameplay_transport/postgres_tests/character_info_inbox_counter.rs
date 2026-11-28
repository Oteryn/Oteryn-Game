use super::TestResult;
use super::fixture::{Harness, debug, id};
use super::kill_reward_e2e_fixtures::scenario_thread::run_async;
use crate::domain::CharacterId;
use oteryn_protocol_oteryn::character_info::PageRequest;

#[test]
fn character_info_reads_inbox_counter_as_runtime_role_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run_async(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "character_info_inbox", true, false).await?;
        let result: TestResult = async {
            let character = CharacterId::from_bytes(id(41)).map_err(debug)?;
            sqlx::query(
                "INSERT INTO game_character_inbox_counters(character_id) VALUES (encode($1,'hex')::uuid)",
            )
            .bind(character.as_bytes().as_slice())
            .execute(&h.pool)
            .await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;

            h.root
                .try_issue_semantic_pass()
                .map_err(debug)?
                .run(|holder, _| {
                    Box::pin(async move {
                        sqlx::query("SET ROLE oteryn_game_runtime")
                            .execute(&mut **holder)
                            .await?;
                        let role: String = sqlx::query_scalar("SELECT current_user")
                            .fetch_one(&mut **holder)
                            .await?;
                        assert_eq!(role, "oteryn_game_runtime");
                        Ok(())
                    })
                })
                .await
                .map_err(debug)?;

            let page = PageRequest::new(0, 1).ok_or("Character Info page")?;
            let reply = h
                .root
                .read_character_info(&authority, character, character, page, 0)
                .await
                .map_err(debug)?;
            // The summary ends with the inbox count, then two u16 history counts.
            let count_end = reply.len().checked_sub(4).ok_or("history counts")?;
            let count_start = count_end.checked_sub(8).ok_or("inbox count")?;
            let inbox_count = i64::from_le_bytes(reply[count_start..count_end].try_into()?);
            assert_eq!(inbox_count, 0, "the allocated Inbox starts empty");
            eprintln!("character_info_reads_inbox_counter_as_runtime_role_postgres POSTGRES_CASE_COMPLETED");
            Ok(())
        }
        .await;
        let cleanup = h.cleanup().await;
        result?;
        cleanup
    })
}
