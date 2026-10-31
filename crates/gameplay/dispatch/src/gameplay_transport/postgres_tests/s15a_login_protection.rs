//! S15a: a fresh login hides the player from monster target selection for Crystal 201189f8
//! `loginProtectionPeriod` (`config.lua.dist:61`, 10 s; `Player::checkLoginDelay`,
//! `player.cpp:6434-6436`; read in `Monster::selectTarget`, `monster.cpp:1014`). The actor is
//! committed through the composed admission's own commit step, the one `admit_fresh_session`
//! uses, on the composed owner clock.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::GameSessionId;
use oteryn_simulation_determinism::SemanticTimeMicros;

#[test]
fn s15a_fresh_login_is_no_monster_target_for_ten_seconds_postgres() -> TestResult {
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "s15a thread")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "s15a_login", true, false).await?;
        eprintln!("POSTGRES_STARTED S15A-LOGIN-PROTECTION");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let character = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &character);
        let session = GameSessionId::decode(&id(60)).map_err(debug)?;
        let reservation = owner
            .runtime
            .lock()
            .await
            .reserve_fresh_session(session)
            .map_err(debug)?;
        let before = owner.owner_now().get();
        let actor = owner
            .commit_runtime_player(reservation)
            .await
            .map_err(debug)?;
        let after = owner.owner_now().get();
        let runtime = owner.runtime.lock().await;
        let attack = owner.states.attack.lock().await;
        let at = SemanticTimeMicros::from_micros;
        assert_eq!(
            attack.creature_target_protection(&runtime, actor, session, at(after)),
            Some(true)
        );
        assert_eq!(
            attack.creature_target_protection(&runtime, actor, session, at(before + 10_000_000)),
            Some(true)
        );
        assert_eq!(
            attack.creature_target_protection(&runtime, actor, session, at(after + 10_000_001)),
            Some(false)
        );
        // The login period does not refuse the player's own actions (reentry stays S15b's).
        assert_eq!(
            runtime.current_player_reentry_protection(actor, session, after),
            Ok(false)
        );
        // The fixture actor committed without a login clock carries no period.
        assert_eq!(
            attack.creature_target_protection(
                &runtime,
                deps.actor,
                deps.admitted.game_session_id,
                at(after)
            ),
            Some(false)
        );
        Ok(())
    })
}
