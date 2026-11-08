//! Crystal movements_arena_pit.lua:52-64: advancement is a native STEP.
use super::*;
use crate::bestiary_postgres_harness as h;

#[test]
fn m51_postgres_portal_step_origin_cannot_replay_as_a_client_command() -> h::TestResult {
    let Some(admin) = h::configured_admin() else {
        return Ok(());
    };
    h::runtime()?.block_on(async move {
        let started = std::time::Instant::now();
        let fixture = h::Harness::create(admin, "srvenc_m51_native_origin", true).await?;
        let result: h::TestResult = async {
            let fence = h::fence(1)?;
            let command = crate::foundation::CommandRef::new(
                fence.game_session_id,
                crate::foundation::CommandId::new(91).map_err(h::debug)?,
            );
            let payload = oteryn_protocol_oteryn::arena::encode_match_request(
                oteryn_protocol_oteryn::arena::ArenaAction::Advance,
            )
            .map_err(h::debug)?;
            let state = MatchState::default();
            let reply = oteryn_protocol_oteryn::arena::encode_match_result(&ArenaResult {
                disposition: ArenaDisposition::Busy,
                state: state.wire(0, [0; 3]),
            })
            .map_err(h::debug)?;
            let mut tx = fixture.pool.begin().await?;
            save_origin(
                &mut tx,
                fence,
                command,
                &payload,
                &state,
                &state,
                &reply,
                Some(b"native-portal-step-a"),
            )
            .await?;
            tx.commit().await?;
            let mut read = fixture.pool.begin().await?;
            assert_eq!(
                replay_origin(
                    &mut read,
                    fence,
                    command,
                    &payload,
                    Some(b"native-portal-step-a")
                )
                .await?,
                Some(reply)
            );
            assert!(
                replay(&mut read, fence, command, &payload).await.is_err(),
                "a native STEP is never a client command replay"
            );
            assert!(
                replay_origin(
                    &mut read,
                    fence,
                    command,
                    &payload,
                    Some(b"native-portal-step-b")
                )
                .await
                .is_err(),
                "compact-id collisions must compare the complete native binding"
            );
            read.rollback().await?;
            Ok(())
        }
        .await;
        fixture.cleanup().await?;
        result?;
        eprintln!(
            "M51 REAL POSTGRES native portal origin duration={:?}",
            started.elapsed()
        );
        assert!(started.elapsed().as_secs_f64() > 0.01);
        Ok(())
    })
}
