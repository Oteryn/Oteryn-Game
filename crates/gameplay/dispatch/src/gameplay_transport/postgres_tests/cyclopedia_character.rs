//! PAR-P12 Cyclopedia character tabs (capability 199) through real dispatch.
use super::fixture::{Harness, id, run};
use super::{Dependencies, TestResult, debug, prepare_current_session, reset};
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::cyclopedia_character::{
    CAPABILITY_CYCLOPEDIA_CHARACTER_V1, COMMAND_TYPE_CYCLOPEDIA_BADGES,
};

#[test]
fn cyclopedia_badges_postgres_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| badges_scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "cyclopedia badges scenario panicked")??;
    Ok(())
}

fn badges_scenario() -> TestResult {
    use super::cyclopedia::support::exchange;
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "cyclopedia_badges", true, false)
            .await?;
        eprintln!("POSTGRES_STARTED PAR-P12B cyclopedia_badges_postgres_real_dispatch");
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
        let badges = COMMAND_TYPE_CYCLOPEDIA_BADGES;
        assert_eq!(
            exchange(&owner, &mut admitted, 1, badges, &[]).await?.0,
            CommandStatus::Rejected,
            "the unselected capability never reaches the badge write"
        );
        let offers = [OfferedCapability {
            id: CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            requires: &[],
        }];
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[CAPABILITY_CYCLOPEDIA_CHARACTER_V1])
                .ok_or("cyclopedia character capability")?;
        assert_eq!(
            exchange(&owner, &mut admitted, 2, badges, &[0]).await?.0,
            CommandStatus::Rejected,
            "the request is empty"
        );
        let account: Vec<u8> = sqlx::query_scalar(
            "SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice())
        .fetch_one(&h.pool)
        .await?;
        let stored = async || -> TestResult<Vec<i16>> {
            Ok(sqlx::query_scalar("SELECT badge FROM game_account_badges WHERE account_id=encode($1,'hex')::uuid ORDER BY badge")
                .bind(account.as_slice()).fetch_all(&h.pool).await?)
        };
        let (status, before) = exchange(&owner, &mut admitted, 3, badges, &[]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let earned_before = stored().await?;
        assert_eq!(usize::from(before[0]), earned_before.len());
        assert!(
            !earned_before.iter().any(|badge| (6..=8).contains(badge)),
            "no loyalty badge without loyalty points"
        );
        // Platform projects 1001 loyalty points: more than 100 and more than 1000, not 5000.
        sqlx::query("INSERT INTO game_account_loyalty_projections(account_id,loyalty_points,platform_revision) VALUES(encode($1,'hex')::uuid,1001,1)")
            .bind(account.as_slice()).execute(&h.pool).await?;
        let (status, after) = exchange(&owner, &mut admitted, 4, badges, &[]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let earned_after = stored().await?;
        let mut expected = earned_before.clone();
        expected.extend([6, 7]);
        expected.sort_unstable();
        assert_eq!(earned_after, expected);
        let wire: Vec<i16> = after[1..].iter().map(|badge| i16::from(*badge)).collect();
        assert_eq!(wire, earned_after, "the reply lists the stored badges in id order");
        // Badges are permanent: losing the points keeps them.
        sqlx::query("UPDATE game_account_loyalty_projections SET loyalty_points=0,platform_revision=2 WHERE account_id=encode($1,'hex')::uuid")
            .bind(account.as_slice()).execute(&h.pool).await?;
        assert_eq!(
            exchange(&owner, &mut admitted, 5, badges, &[]).await?.1,
            after,
            "badges are never revoked"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
