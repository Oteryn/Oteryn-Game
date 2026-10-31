//! PAR-P12 Cyclopedia character tabs (capability 199) through real dispatch.
use super::fixture::{Harness, fence, id, pvp_source_death_fixture, run};
use super::{Dependencies, TestResult, debug, prepare_current_session, reset};
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::domain::CharacterId;
use crate::durability::pvp_death_consequences::{PvpContributorKind, PvpDeathContributor};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::cyclopedia_character::{
    CAPABILITY_CYCLOPEDIA_CHARACTER_V1, COMMAND_TYPE_CYCLOPEDIA_BADGES,
    COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
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


#[test]
fn cyclopedia_character_tabs_postgres_real_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| tabs_scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "cyclopedia character tabs scenario panicked")??;
    Ok(())
}

/// `tab:u8, count:u16`, then `at:i64` and a status byte when `status`, and `len:u8` text.
fn history(reply: &[u8], status: bool) -> TestResult<Vec<(u8, String)>> {
    let count = usize::from(u16::from_le_bytes([reply[1], reply[2]]));
    let mut at = 3;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        at += 8;
        let kind = if status {
            at += 1;
            reply[at - 1]
        } else {
            0
        };
        let len = usize::from(reply[at]);
        entries.push((kind, String::from_utf8(reply[at + 1..at + 1 + len].to_vec())?));
        at += 1 + len;
    }
    assert_eq!(at, reply.len(), "the page has no trailing bytes");
    Ok(entries)
}

fn tabs_scenario() -> TestResult {
    use super::cyclopedia::support::exchange;
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "cyclopedia_tabs", true, false).await?;
        eprintln!("POSTGRES_STARTED PAR-P12C cyclopedia_character_tabs_postgres_real_dispatch");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        // A committed PvP death of the session Character by a second Character (PVP-1a fixture).
        pvp_source_death_fixture::seed_root(&h.pool, 41, 60, 42).await?;
        sqlx::query("INSERT INTO game_character_pvp_state (character_id, world_id) VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(42).as_slice()).execute(&h.pool).await?;
        pvp_source_death_fixture::seed_ledger(&h.pool, 41, 60, true, Some(1), "0 seconds").await?;
        let mut request = crate::gameplay_transport::death_ml_loss_postgres::death(90)?;
        request.pvp_contributors = vec![PvpDeathContributor {
            character_id: CharacterId::from_bytes(id(60)).map_err(debug)?,
            kind: PvpContributorKind::Damage,
            unjustified: true,
        }];
        let mut current = fence(1)?;
        current.character_lease_generation = 2;
        owner
            .settle_source_player_death(current, request, None, None)
            .await
            .map_err(debug)?;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        deps.initialize(&owner).await;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        let tab = COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB;
        assert_eq!(
            exchange(&owner, &mut admitted, 1, tab, &[1, 0, 0, 1]).await?.0,
            CommandStatus::Rejected,
            "the unselected capability never reaches the projection"
        );
        let offers = [OfferedCapability {
            id: CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
            requires: &[],
        }];
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[CAPABILITY_CYCLOPEDIA_CHARACTER_V1])
                .ok_or("cyclopedia character capability")?;
        for (command, invalid) in [(2, &[8, 0, 0, 1][..]), (3, &[2, 0, 0, 0]), (4, &[2, 0, 0])] {
            assert_eq!(
                exchange(&owner, &mut admitted, command, tab, invalid).await?.0,
                CommandStatus::Rejected
            );
        }
        let (level, victim): (i64, String) = sqlx::query_as(
            "SELECT d.level_before,r.name FROM game_character_death_receipts d JOIN game_character_roots r ON r.character_id=d.character_id WHERE d.character_id=encode($1,'hex')::uuid",
        )
        .bind(id(41).as_slice())
        .fetch_one(&h.pool)
        .await?;
        let (status, deaths) = exchange(&owner, &mut admitted, 5, tab, &[2, 0, 0, 50]).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(
            history(&deaths, false)?,
            [(0, format!("Died at Level {level} by Pvpci"))],
            "the cause names the Character killer the PvP owner recorded"
        );
        // The build owner's row, as the energy ring case seeds it: magic level 7, sword 42.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_build_state(character_id,vocation,magic_level,mana_spent,fist_level,fist_tries,club_level,club_tries,sword_level,sword_tries,axe_level,axe_tries,distance_level,distance_tries,shielding_level,shielding_tries,fishing_level,fishing_tries,committed_character_revision,last_build_occurrence_id) VALUES(encode($1,'hex')::uuid,'knight',7,0,10,0,10,0,42,0,10,0,10,0,10,0,10,0,2,encode($2,'hex')::uuid) ON CONFLICT(character_id) DO UPDATE SET magic_level=7,sword_level=42")
            .bind(id(41).as_slice()).bind(id(30).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let (_, stats) = exchange(&owner, &mut admitted, 6, tab, &[1, 0, 0, 1]).await?;
        let stored: i64 = sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        let word = |at: usize| u16::from_le_bytes([stats[at], stats[at + 1]]);
        assert_eq!(stats.len(), 42);
        assert_eq!(
            (stats[0], i64::from(u32::from_le_bytes([stats[9], stats[10], stats[11], stats[12]])), word(13), word(19)),
            (1, stored, 7, 42),
            "level, magic level and sword come from the progression and build owners"
        );
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_blessings(character_id,blessing_key,provenance) VALUES(encode($1,'hex')::uuid,'oteryn:blessing.twist_of_fate','fixture')")
            .bind(id(41).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let (_, store) = exchange(&owner, &mut admitted, 7, tab, &[6, 0, 0, 1]).await?;
        let mut expected = vec![6, 1, 29];
        expected.extend_from_slice(b"oteryn:blessing.twist_of_fate");
        expected.extend([0, 0, 0]);
        assert_eq!(store, expected);
        // The killer's PvP kills are disclosed only under the viewer's accepted VIP consent.
        let mut killer = vec![3, 0, 0, 50];
        killer.extend_from_slice(&id(60));
        assert_eq!(
            exchange(&owner, &mut admitted, 8, tab, &killer).await?.0,
            CommandStatus::Rejected
        );
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_vip_consents(account_id,world_id,inviter_character_id,character_id,consent,invite_expires_at) SELECT account_id,world_id,character_id,encode($2,'hex')::uuid,2,now()+interval '1 day' FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(id(60).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let (status, kills) = exchange(&owner, &mut admitted, 9, tab, &killer).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(history(&kills, true)?, [(1, format!("Killed {victim}."))]);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
