//! Coherent identity, badges and inspection through production dispatch and VIP privacy.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::gameplay_transport::{
    capabilities::{OfferedCapability, SelectedCapabilities},
    connection::FirstEntryOutcome,
};
use oteryn_protocol_oteryn::{
    CommandStatus,
    cyclopedia_character::{
        CAPABILITY_CYCLOPEDIA_CHARACTER_V1 as CAP, COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB as TAB,
        profile::{Profile, request},
    },
};
#[test]
fn character_profile_postgres_real_dispatch_and_privacy() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run_scenario(false)
}

#[cfg(feature = "qualification-clock")]
#[test]
fn p12b_badges_use_world_clock_at_admission_and_on_request() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    const PROBE: &str = "OTERYN_P12B_BADGE_CLOCK_PROBE";
    if std::env::var_os(PROBE).is_some() {
        return run_scenario(true);
    }

    // The offset is process-wide. Keep it out of the other dispatch tests.
    let mut account_time = [0; 8];
    account_time[2..].copy_from_slice(&id(40)[..6]);
    let two_years_ms = 2 * 365 * 86_400_000i64;
    let wall = crate::gameplay_transport::character_info::unix_now_ms();
    let offset = i64::from_be_bytes(account_time) + two_years_ms - wall;
    let path = std::env::temp_dir().join(format!("oteryn-p12b-clock-{}", std::process::id()));
    std::fs::write(&path, offset.to_string())?;
    let (_, module) = module_path!().split_once("::").ok_or("test module")?;
    let case = format!("{module}::p12b_badges_use_world_clock_at_admission_and_on_request");
    let status = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", &case, "--nocapture"])
        .env(PROBE, "1")
        .env("OTERYN_QUALIFICATION_CLOCK_FILE", &path)
        .status();
    std::fs::remove_file(path)?;
    assert!(status?.success(), "qualification-clock badges");
    Ok(())
}

fn run_scenario(check_badge_clock: bool) -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(check_badge_clock).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Character profile scenario panicked")??;
    Ok(())
}
fn scenario(check_badge_clock: bool) -> TestResult {
    use super::cyclopedia_fixtures::support::exchange;
    use oteryn_protocol_oteryn::cyclopedia_character::{Badge, COMMAND_TYPE_CYCLOPEDIA_BADGES};
    run(async move |admin| {
        super::reset();
        let h =
            Harness::create_with_admission_seed(admin, "character_profile", true, false).await?;
        eprintln!("POSTGRES_STARTED character_profile_postgres_real_dispatch_and_privacy");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        deps.initialize(&owner).await;
        let current = FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(deps.admitted.game_session_id)
            .await?
            .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        let mut admitted = deps.admitted;
        admitted.item_fence = Some(fence);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let query = request(None);
        assert!(
            owner
                .cyclopedia_character_wire(admitted, 0, TAB, &query)
                .await
                .is_err()
        );
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            &[OfferedCapability {
                id: CAP,
                requires: &[],
            }],
            &[CAP],
        )
        .ok_or("capability")?;
        let mut next_command = 1;
        if check_badge_clock {
            let clock = || -> Result<i64, Box<dyn std::error::Error>> {
                Ok(i64::try_from(crate::world_clock::now_utc_ms().ok_or("World clock")?)?)
            };
            let before_login = clock()?;
            owner.refresh_session_titles(admitted.game_session_id).await;
            let earned_at = "SELECT earned_at_unix_ms FROM game_account_badges
                WHERE account_id=encode($1,'hex')::uuid AND badge=$2";
            let age: i64 = sqlx::query_scalar(earned_at)
                .bind(id(40).as_slice())
                .bind(i16::from(Badge::FledgelingHero.id()))
                .fetch_one(&h.pool)
                .await?;
            assert!((before_login..=clock()?).contains(&age));
            sqlx::query("INSERT INTO game_account_loyalty_projections
                         (account_id, loyalty_points, platform_revision)
                         VALUES(encode($1,'hex')::uuid, 101, 1)")
                .bind(id(40).as_slice())
                .execute(&h.pool)
                .await?;
            let before_request = clock()?;
            let (status, badges) = exchange(
                &owner,
                &mut admitted,
                next_command,
                COMMAND_TYPE_CYCLOPEDIA_BADGES,
                &[],
            )
            .await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(badges, [2, Badge::FledgelingHero.id(), Badge::TibiaLoyalistGrade1.id()]);
            let loyalty: i64 = sqlx::query_scalar(earned_at)
                .bind(id(40).as_slice())
                .bind(i16::from(Badge::TibiaLoyalistGrade1.id()))
                .fetch_one(&h.pool)
                .await?;
            assert!((before_request..=clock()?).contains(&loyalty));
            next_command += 1;
        }
        let (status, own) = exchange(&owner, &mut admitted, next_command, TAB, &query).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(Profile::decode(&own).map_err(debug)?.character, id(41));
        sqlx::query(
            "INSERT INTO game_character_account_guards(account_id) VALUES(encode($1,'hex')::uuid)",
        )
        .bind(id(61).as_slice())
        .execute(&h.pool)
        .await?;
        sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) SELECT encode($2,'hex')::uuid,encode($3,'hex')::uuid,world_id,lifecycle,1,profile_revision,ruleset_revision,content_revision,starter_template_revision,'Peer Hero' FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(id(60).as_slice()).bind(id(61).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_account_badges(account_id,badge,earned_at_unix_ms) VALUES(encode($1,'hex')::uuid,1,100)").bind(id(61).as_slice()).execute(&h.pool).await?;
        let foreign = request(Some(id(60)));
        assert!(
            owner
                .cyclopedia_character_wire(admitted, 0, TAB, &foreign)
                .await
                .is_err()
        );
        sqlx::query("INSERT INTO game_vip_lists(account_id,world_id) SELECT account_id,world_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_vip_entries(account_id,world_id,character_id,entry_no) SELECT account_id,world_id,encode($2,'hex')::uuid,1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).bind(id(60).as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_vip_consents(account_id,world_id,inviter_character_id,character_id,consent,invite_expires_at) SELECT account_id,world_id,character_id,encode($2,'hex')::uuid,2,now()+interval '1 day' FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(id(60).as_slice()).execute(&h.pool).await?;
        let permitted = owner
            .cyclopedia_character_wire(admitted, 0, TAB, &foreign)
            .await
            .map_err(|_| "permitted profile")?;
        let permitted = Profile::decode(&permitted).map_err(debug)?;
        assert_eq!(permitted.character, id(60));
        assert_eq!(permitted.name, "Peer Hero");
        assert_eq!(
            permitted.badges,
            vec![oteryn_protocol_oteryn::cyclopedia_character::Badge::FledgelingHero]
        );
        assert!(permitted.equipment.is_empty());
        let mut stale = admitted;
        let mut stale_fence = fence;
        stale_fence.character_lease_generation += 1;
        stale.item_fence = Some(stale_fence);
        assert!(
            owner
                .cyclopedia_character_wire(stale, 0, TAB, &foreign)
                .await
                .is_err()
        );
        sqlx::query(
            "UPDATE game_vip_consents SET consent=1 WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(60).as_slice())
        .execute(&h.pool)
        .await?;
        assert!(
            owner
                .cyclopedia_character_wire(admitted, 0, TAB, &foreign)
                .await
                .is_err()
        );
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_account_badges WHERE account_id=encode($1,'hex')::uuid",
        )
        .bind(id(61).as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(count, 1, "foreign reads cannot grant badges");
        eprintln!("POSTGRES_EXECUTED character_profile_postgres_real_dispatch_and_privacy");
        h.pool.close().await;
        Ok(())
    })
}
