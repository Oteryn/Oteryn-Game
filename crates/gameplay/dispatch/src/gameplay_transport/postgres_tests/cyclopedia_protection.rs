//! The pre-death view reads current owners through the admitted protocol caller.
use super::fixture::{Harness, id, run};
use super::{Dependencies, TestResult, debug, prepare_current_session, reset};
use crate::gameplay_transport::{
    capabilities::{OfferedCapability, SelectedCapabilities},
    connection::FirstEntryOutcome,
};
use oteryn_protocol_oteryn::{
    CommandStatus,
    cyclopedia_character::{COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB, protection},
};
#[test]
fn cyclopedia_protection_forecast_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "death forecast dispatch thread failed")??;
    Ok(())
}
fn scenario() -> TestResult {
    use super::cyclopedia::support::exchange;
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "cyclopedia_protection", true, false)
            .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        let command = COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB;
        assert_eq!(
            exchange(&owner, &mut admitted, 1, command, &[protection::SELECTOR])
                .await?
                .0,
            CommandStatus::Rejected
        );
        let offers = [OfferedCapability {
            id: 199,
            requires: &[],
        }];
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[199]).ok_or("select cyclopedia")?;
        assert_eq!(
            exchange(
                &owner,
                &mut admitted,
                2,
                command,
                &[protection::SELECTOR, 0]
            )
            .await?
            .0,
            CommandStatus::Rejected
        );
        let receipts_before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_death_receipts WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        let (status, bytes) =
            exchange(&owner, &mut admitted, 3, command, &[protection::SELECTOR]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let value = protection::Forecast::decode(&bytes).ok_or("forecast codec")?;
        let stored:i64=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(u64::from(value.level), u64::try_from(stored)?);
        let expected = crate::domain::death::death_experience_loss(
            value.level,
            value.regular_blessings,
            value.promotion_benefit,
        )
        .map_err(debug)?
        .min(u128::from(value.experience));
        assert_eq!(u128::from(value.pve.experience_lost), expected);
        assert!(value.observed_at_unix_ms > 0);
        if let Some(pvp) = value.pvp {
            assert_eq!(
                pvp.experience_lost,
                if value.adventurer {
                    0
                } else {
                    value.pve.experience_lost
                }
            );
        }
        assert!(!value.adventurer || value.pvp.is_some());
        let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_death_receipts WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(
            receipts, receipts_before,
            "a forecast never commits a death"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
