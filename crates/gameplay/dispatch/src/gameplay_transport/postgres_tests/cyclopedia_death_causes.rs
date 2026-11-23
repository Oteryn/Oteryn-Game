use super::cyclopedia_fixtures::support::exchange;
use super::fixture::{Harness, debug, id, pvp_source_death_fixture};
use super::{
    Dependencies, TestResult, kill_reward_e2e_fixtures::scenario_thread, prepare_current_session,
    reset,
};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::GameSessionId;
use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
use crate::gameplay_transport::character_info::unix_now_ms;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::character_info::PageRequest;
use oteryn_protocol_oteryn::cyclopedia_character::{
    CAPABILITY_CYCLOPEDIA_CHARACTER_V1, CAPABILITY_CYCLOPEDIA_DEATH_DETAILS_V1,
    COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB, CharacterTab, DeathDetails, TabExtension, TabView,
};
use sqlx::{Postgres, Transaction};

#[test]
fn recent_deaths_name_final_blow_and_most_damage_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    scenario_thread::run_async(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "cyclopedia_death_causes", true, false)
            .await?;
        eprintln!(
            "POSTGRES_STARTED PAR-P12C recent_deaths_name_final_blow_and_most_damage_postgres"
        );
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        pvp_source_death_fixture::seed_root(&h.pool, 41, 60, 42).await?;
        pvp_source_death_fixture::seed_root(&h.pool, 41, 61, 42).await?;

        // Seed history only. These receipts have no unjustified points or guild war frags.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica")
            .execute(&mut *tx)
            .await?;
        seed_death_receipts(
            &mut tx,
            &[id(90).to_vec(), id(91).to_vec(), id(92).to_vec()],
            1000,
        )
        .await?;
        sqlx::query(
            "INSERT INTO game_public_death_killers
                (death_occurrence_id,ordinal,kind,character_id,creature_key,name)
             SELECT encode($1,'hex')::uuid,0,'creature',NULL::uuid,'oteryn:creature.rat','Rat'
             UNION ALL
             SELECT encode($2,'hex')::uuid, source.ordinal, 'player', root.character_id, NULL,
                root.name
             FROM (VALUES (0,encode($3,'hex')::uuid), (1,encode($4,'hex')::uuid))
                AS source(ordinal,character_id)
             JOIN game_character_roots root USING (character_id)",
        )
        .bind(id(91).as_slice())
        .bind(id(92).as_slice())
        .bind(id(61).as_slice())
        .bind(id(60).as_slice())
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO game_public_death_classifications
                (death_occurrence_id,pvp,killer_key,most_damage_ordinal)
             VALUES (encode($1,'hex')::uuid,false,'oteryn:creature.rat',0),
                    (encode($2,'hex')::uuid,true,'player',1)",
        )
        .bind(id(91).as_slice())
        .bind(id(92).as_slice())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;

        let store = FreshAdmissionStore::from_root(h.root.clone());
        let (current, _) = store
            .current_session_at(GameSessionId::decode(&id(50)).map_err(debug)?)
            .await?;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        let (view, extension) = h
            .root
            .read_cyclopedia_character_tab_extended(
                &authority,
                &h.node,
                fence,
                fence.character_id,
                CharacterTab::RecentDeaths,
                PageRequest::new(0, 3).ok_or("page")?,
                1004,
            )
            .await
            .map_err(debug)?;
        drop(store);
        drop(authority);
        drop(seal);
        h.cleanup().await?;

        assert_eq!(
            view,
            TabView::RecentDeaths(vec![
                (1003, "Died at Level 20 by Pvpcj and Pvpci".into()),
                (1002, "Died at Level 20 by Rat".into()),
                (1001, "Died at Level 20".into()),
            ]),
        );
        assert_eq!(
            extension.deaths,
            Some(vec![
                DeathDetails {
                    level: 20,
                    killer: "Pvpcj".into(),
                    most_damage: "Pvpci".into(),
                    unjustified: false
                },
                DeathDetails {
                    level: 20,
                    killer: "Rat".into(),
                    most_damage: "Rat".into(),
                    unjustified: false
                },
                DeathDetails {
                    level: 20,
                    ..DeathDetails::default()
                },
            ]),
        );
        Ok(())
    })
}

async fn seed_death_receipts(
    tx: &mut Transaction<'_, Postgres>,
    occurrences: &[Vec<u8>],
    before_first_death_ms: i64,
) -> TestResult {
    sqlx::query(
        "INSERT INTO game_character_death_receipts(
            death_occurrence_id,command_binding,policy_digest,character_id,
            original_character_revision,committed_character_revision,level_before,level_after,
            experience_before,experience_after,experience_lost,blessings_before,blessings_after,
            lost_item_ids,death_world_id,death_channel_id,death_spatial_position,
            death_map_revision,respawn_position,death_policy_revision,profile_revision,
            ruleset_revision,content_revision,simulation_revision,evidence_revision,
            declaration_revision,policy_revision,reward_revision,committed_at)
         SELECT encode(source,'hex')::uuid, decode('01','hex'), decode(repeat('01',32),'hex'),
            encode($1,'hex')::uuid, n, n+1, 20, 20, 0, 0, 0, '{}', '{}', '{}',
            encode($2,'hex')::uuid, encode($3,'hex')::uuid, decode('01','hex'), 'fixture',
            decode('01','hex'), 'fixture', 'fixture', 'fixture', 'fixture', 'fixture',
            'fixture', 'fixture', 'fixture', 'fixture', $5+n
         FROM unnest($4::bytea[]) WITH ORDINALITY AS deaths(source,n)",
    )
    .bind(id(41).as_slice())
    .bind(id(42).as_slice())
    .bind(id(43).as_slice())
    .bind(occurrences)
    .bind(before_first_death_ms)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[test]
fn recent_deaths_pre_2780_preserves_pvp_name_order_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    causes_in_both_layouts(
        "recent_deaths_pre_2780_preserves_pvp_name_order_postgres",
        CauseCase::Pvp(None),
    )
}

#[test]
fn recent_deaths_current_preserves_pvp_name_order_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    causes_in_both_layouts(
        "recent_deaths_current_preserves_pvp_name_order_postgres",
        CauseCase::Pvp(Some(1)),
    )
}

#[test]
fn recent_deaths_bounds_cause_at_whole_names_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    causes_in_both_layouts(
        "recent_deaths_bounds_cause_at_whole_names_postgres",
        CauseCase::Bounded,
    )
}

#[test]
fn recent_deaths_adds_missing_captured_sources_by_identity_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    causes_in_both_layouts(
        "recent_deaths_adds_missing_captured_sources_by_identity_postgres",
        CauseCase::Mixed,
    )
}

#[test]
fn recent_deaths_pages_before_ledger_reads_with_generic_plan_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    causes_in_both_layouts(
        "recent_deaths_pages_before_ledger_reads_with_generic_plan_postgres",
        CauseCase::GenericPage,
    )
}

#[test]
fn recent_deaths_indexes_unjustified_points_by_occurrence_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    scenario_thread::run_async(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "cyclopedia_points_index", true, false)
            .await?;
        eprintln!(
            "POSTGRES_STARTED PAR-P12C recent_deaths_indexes_unjustified_points_by_occurrence_postgres"
        );
        let has_occurrence_index: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM pg_index
                WHERE indrelid = 'game_character_unjustified_points'::regclass
                  AND indisvalid AND indpred IS NULL
                  AND pg_get_indexdef(indexrelid, 1, true) = 'death_occurrence_id'
            )",
        )
        .fetch_one(&h.pool)
        .await?;
        h.cleanup().await?;
        assert!(
            has_occurrence_index,
            "ledger lookups need an index starting with the death occurrence"
        );
        Ok(())
    })
}

enum CauseCase {
    Pvp(Option<i16>),
    Bounded,
    Mixed,
    GenericPage,
}

fn causes_in_both_layouts(case: &'static str, scenario: CauseCase) -> TestResult {
    scenario_thread::run_async(async move |admin| {
        reset();
        // Distinct short names isolate concurrent scenarios even when their clocks tie.
        let database_tag = match scenario {
            CauseCase::Pvp(None) => "cyclopedia_old_pvp",
            CauseCase::Pvp(Some(_)) => "cyclopedia_current_pvp",
            CauseCase::Bounded => "cyclopedia_cause_bound",
            CauseCase::Mixed => "cyclopedia_cause_mixed",
            CauseCase::GenericPage => "cyclopedia_cause_page",
        };
        let mut h = Harness::create_with_admission_seed(admin, database_tag, true, false).await?;
        eprintln!("POSTGRES_STARTED PAR-P12C {case}");
        let generic_plan = matches!(scenario, CauseCase::GenericPage);
        if generic_plan {
            let database: String = sqlx::query_scalar("SELECT quote_ident(current_database())")
                .fetch_one(&h.pool)
                .await?;
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "ALTER DATABASE {database} SET plan_cache_mode = force_generic_plan"
            )))
            .execute(&h.pool)
            .await?;
            // The database setting applies to new connections on the runtime owner.
            h.root = h.independent_root()?;
            assert!(h.root.maintain_ready_once().await?);
        }
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        let at = unix_now_ms();
        let (view, extension) = match scenario {
            CauseCase::Pvp(ordinal) => seed_pvp_history(&h, at, ordinal).await?,
            CauseCase::Bounded => seed_long_causes(&h, at).await?,
            CauseCase::Mixed => seed_mixed_causes(&h, at).await?,
            CauseCase::GenericPage => seed_generic_plan_history(&h, at).await?,
        };

        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        deps.initialize(&owner).await;
        let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(deps.admitted.game_session_id)
            .await?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        let offers = [
            OfferedCapability {
                id: CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
                requires: &[],
            },
            OfferedCapability {
                id: CAPABILITY_CYCLOPEDIA_DEATH_DETAILS_V1,
                requires: &[CAPABILITY_CYCLOPEDIA_CHARACTER_V1],
            },
        ];
        let tab = COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB;
        let limit = if generic_plan { 1 } else { 2 };
        let request = [CharacterTab::RecentDeaths as u8, 0, 0, limit];
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[CAPABILITY_CYCLOPEDIA_CHARACTER_V1])
                .ok_or("cyclopedia character capability")?;
        let ledger_scans_before = if generic_plan {
            Some(generic_plan_ledger_scans(&h).await?)
        } else {
            None
        };
        let plain = exchange(&owner, &mut admitted, 1, tab, &request).await?;
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            &offers,
            &[
                CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
                CAPABILITY_CYCLOPEDIA_DEATH_DETAILS_V1,
            ],
        )
        .ok_or("cyclopedia death details capability")?;
        let detailed = exchange(&owner, &mut admitted, 2, tab, &request).await?;
        let ledger_scans = if let Some((seq_before, index_before)) = ledger_scans_before {
            let (seq_after, index_after) = generic_plan_ledger_scans(&h).await?;
            Some((seq_after - seq_before, index_after - index_before))
        } else {
            None
        };
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;

        assert_eq!(
            (plain, detailed),
            (
                (CommandStatus::Accepted, view.encode().map_err(debug)?),
                (
                    CommandStatus::Accepted,
                    view.encode_extended(&extension).map_err(debug)?
                ),
            ),
            "both replies preserve the name-ordered cause and capability 488 appends the facts"
        );
        if let Some((seq, index)) = ledger_scans {
            // Each layout requests one receipt and needs at most an existence probe
            // and a contributor lookup on that receipt.
            assert!(
                seq == 0 && (1..=4).contains(&index),
                "two one-row replies performed {seq} sequential and {index} index ledger scans"
            );
        }
        Ok(())
    })
}

async fn generic_plan_ledger_scans(h: &Harness) -> TestResult<(i64, i64)> {
    let (plan_mode, seq, index): (String, i64, i64) = h
        .root
        .try_issue_semantic_pass()
        .map_err(debug)?
        .run(|holder, _| {
            Box::pin(async move {
                sqlx::query("SELECT pg_stat_force_next_flush()")
                    .execute(&mut **holder)
                    .await?;
                Ok(sqlx::query_as(
                    "SELECT current_setting('plan_cache_mode'), seq_scan, COALESCE(idx_scan, 0)
                     FROM pg_stat_user_tables WHERE relname='game_character_unjustified_points'",
                )
                .fetch_one(&mut **holder)
                .await?)
            })
        })
        .await
        .map_err(debug)?;
    assert_eq!(plan_mode, "force_generic_plan");
    Ok((seq, index))
}

async fn seed_pvp_history(
    h: &Harness,
    at: i64,
    most_damage_ordinal: Option<i16>,
) -> TestResult<(TabView, TabExtension)> {
    // Store the final blow first and, for current deaths, the most damage second.
    // The PvP ledger still contains every contributor and the cause uses name order.
    let seeds: &[u8] = if most_damage_ordinal.is_some() {
        &[62, 61, 60]
    } else {
        &[61, 60]
    };
    for &seed in seeds {
        pvp_source_death_fixture::seed_root(&h.pool, 41, seed, 42).await?;
    }
    let contributors: Vec<_> = seeds.iter().map(|&seed| id(seed).to_vec()).collect();
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    seed_death_receipts(&mut tx, &[id(90).to_vec()], at - 1).await?;
    sqlx::query(
        "INSERT INTO game_public_death_killers
            (death_occurrence_id,ordinal,kind,character_id,name)
         SELECT encode($1,'hex')::uuid, roster.position-1, 'player', root.character_id, root.name
         FROM unnest($2::bytea[]) WITH ORDINALITY AS roster(source,position)
         JOIN game_character_roots root ON root.character_id=encode(roster.source,'hex')::uuid",
    )
    .bind(id(90).as_slice())
    .bind(&contributors)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_public_death_classifications
            (death_occurrence_id,pvp,killer_key,most_damage_ordinal)
         VALUES (encode($1,'hex')::uuid,true,'player',$2)",
    )
    .bind(id(90).as_slice())
    .bind(most_damage_ordinal)
    .execute(&mut *tx)
    .await?;
    seed_unjustified_points(&mut tx, 90, &contributors).await?;
    tx.commit().await?;

    let (cause, final_blow, most_damage) = if most_damage_ordinal.is_some() {
        (
            "Died at Level 20 by Pvpci and Pvpcj and Pvpck",
            "Pvpck",
            "Pvpcj",
        )
    } else {
        ("Died at Level 20 by Pvpci and Pvpcj", "Pvpcj", "")
    };
    let view = TabView::RecentDeaths(vec![(at, cause.into())]);
    let extension = TabExtension {
        vocation_id: None,
        deaths: Some(vec![DeathDetails {
            level: 20,
            killer: final_blow.into(),
            most_damage: most_damage.into(),
            unjustified: true,
        }]),
    };
    Ok((view, extension))
}

async fn seed_unjustified_points(
    tx: &mut Transaction<'_, Postgres>,
    occurrence: u8,
    contributors: &[Vec<u8>],
) -> TestResult {
    sqlx::query(
        "INSERT INTO game_character_unjustified_points
            (character_id,death_occurrence_id,victim_character_id,points_milli)
         SELECT encode(source,'hex')::uuid, encode($1,'hex')::uuid, encode($2,'hex')::uuid,
            1000 / cardinality($3::bytea[])
         FROM unnest($3::bytea[]) AS roster(source)",
    )
    .bind(id(occurrence).as_slice())
    .bind(id(41).as_slice())
    .bind(contributors)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn seed_long_causes(h: &Harness, at: i64) -> TestResult<(TabView, TabExtension)> {
    let seeds = 60..68;
    for seed in seeds.clone() {
        pvp_source_death_fixture::seed_root(&h.pool, 41, seed, 42).await?;
    }
    let contributors: Vec<_> = seeds.map(|seed| id(seed).to_vec()).collect();
    let names: Vec<_> = (b'A'..=b'H')
        .map(|letter| format!("{}{}", char::from(letter), "x".repeat(28)))
        .collect();
    let multibyte_name = "é".repeat(16);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE game_character_roots root SET name=roster.name
         FROM unnest($1::bytea[], $2::text[]) AS roster(source,name)
         WHERE root.character_id=encode(roster.source,'hex')::uuid",
    )
    .bind(&contributors)
    .bind(&names)
    .execute(&mut *tx)
    .await?;
    seed_death_receipts(&mut tx, &[id(90).to_vec(), id(91).to_vec()], at - 2).await?;
    seed_unjustified_points(&mut tx, 90, &contributors).await?;
    seed_unjustified_points(&mut tx, 91, &contributors[..6]).await?;
    // Six 29-byte names leave 36 bytes. The separator and this UTF-8 name need 37.
    sqlx::query(
        "INSERT INTO game_public_death_killers
            (death_occurrence_id,ordinal,kind,creature_key,name)
         VALUES (encode($1,'hex')::uuid,0,'creature','oteryn:creature.rat',$2)",
    )
    .bind(id(91).as_slice())
    .bind(&multibyte_name)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_public_death_classifications
            (death_occurrence_id,pvp,killer_key,most_damage_ordinal)
         VALUES (encode($1,'hex')::uuid,true,'oteryn:creature.rat',0)",
    )
    .bind(id(91).as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let view = TabView::RecentDeaths(vec![
        (
            at,
            format!("Died at Level 20 by {}", names[..6].join(" and ")),
        ),
        (
            at - 1,
            format!("Died at Level 20 by {}", names[..7].join(" and ")),
        ),
    ]);
    let extension = TabExtension {
        vocation_id: None,
        deaths: Some(vec![
            DeathDetails {
                level: 20,
                killer: multibyte_name.clone(),
                most_damage: multibyte_name,
                unjustified: true,
            },
            DeathDetails {
                level: 20,
                unjustified: true,
                ..DeathDetails::default()
            },
        ]),
    };
    Ok((view, extension))
}

async fn seed_mixed_causes(h: &Harness, at: i64) -> TestResult<(TabView, TabExtension)> {
    for seed in [60, 61, 62] {
        pvp_source_death_fixture::seed_root(&h.pool, 41, seed, 42).await?;
    }
    let contributors = [id(60).to_vec(), id(61).to_vec()];
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    // The renamed player and creature share a display name but are different sources.
    sqlx::query(
        "UPDATE game_character_roots SET name='Rat' WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(60).as_slice())
    .execute(&mut *tx)
    .await?;
    seed_death_receipts(&mut tx, &[id(90).to_vec(), id(91).to_vec()], at - 2).await?;
    seed_unjustified_points(&mut tx, 90, &contributors).await?;
    seed_unjustified_points(&mut tx, 91, &contributors).await?;
    sqlx::query(
        "INSERT INTO game_public_death_killers
            (death_occurrence_id,ordinal,kind,character_id,creature_key,name)
         VALUES
            (encode($1,'hex')::uuid,0,'player',encode($3,'hex')::uuid,NULL,'Pvpci'),
            (encode($1,'hex')::uuid,1,'player',encode($4,'hex')::uuid,NULL,'Pvpck'),
            (encode($2,'hex')::uuid,0,'creature',NULL,'oteryn:creature.rat','Rat'),
            (encode($2,'hex')::uuid,1,'player',encode($4,'hex')::uuid,NULL,'Pvpck')",
    )
    .bind(id(90).as_slice())
    .bind(id(91).as_slice())
    .bind(id(60).as_slice())
    .bind(id(62).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_public_death_classifications
            (death_occurrence_id,pvp,killer_key,most_damage_ordinal)
         VALUES (encode($1,'hex')::uuid,true,'player',1),
                (encode($2,'hex')::uuid,true,'oteryn:creature.rat',1)",
    )
    .bind(id(90).as_slice())
    .bind(id(91).as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let view = TabView::RecentDeaths(vec![
        (
            at,
            "Died at Level 20 by Pvpcj and Rat and Rat and Pvpck".into(),
        ),
        (at - 1, "Died at Level 20 by Pvpcj and Rat and Pvpck".into()),
    ]);
    let extension = TabExtension {
        vocation_id: None,
        deaths: Some(
            ["Rat", "Pvpci"]
                .into_iter()
                .map(|killer| DeathDetails {
                    level: 20,
                    killer: killer.into(),
                    most_damage: "Pvpck".into(),
                    unjustified: true,
                })
                .collect(),
        ),
    };
    Ok((view, extension))
}

async fn seed_generic_plan_history(h: &Harness, at: i64) -> TestResult<(TabView, TabExtension)> {
    pvp_source_death_fixture::seed_root(&h.pool, 41, 59, 42).await?;
    for seed in 60..110 {
        pvp_source_death_fixture::seed_root(&h.pool, 41, seed, 42).await?;
    }
    let contributors: Vec<_> = (60..110).map(|seed| id(seed).to_vec()).collect();
    let occurrences: Vec<Vec<u8>> =
        sqlx::query_scalar("SELECT uuid_send(uuidv7()) FROM generate_series(1, 20500)")
            .fetch_all(&h.pool)
            .await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *tx)
        .await?;
    seed_death_receipts(&mut tx, &occurrences, at - 20500).await?;
    // The other victim contributes one million retained ledger rows. The viewed
    // character still has 500 in-window receipts, with one ledger row on the latest.
    sqlx::query(
        "UPDATE game_character_death_receipts SET character_id=encode($1,'hex')::uuid
         WHERE original_character_revision<=20000",
    )
    .bind(id(59).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_character_unjustified_points
            (character_id,death_occurrence_id,victim_character_id,points_milli)
         SELECT encode(source,'hex')::uuid, d.death_occurrence_id,d.character_id,20
         FROM game_character_death_receipts d CROSS JOIN unnest($1::bytea[]) AS roster(source)
         WHERE d.character_id=encode($2,'hex')::uuid",
    )
    .bind(&contributors)
    .bind(id(59).as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_character_unjustified_points
            (character_id,death_occurrence_id,victim_character_id,points_milli)
         VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1000)",
    )
    .bind(id(60).as_slice())
    .bind(occurrences.last().ok_or("latest occurrence")?)
    .bind(id(41).as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    sqlx::query("ANALYZE game_character_death_receipts, game_character_unjustified_points")
        .execute(&h.pool)
        .await?;

    let view = TabView::RecentDeaths(vec![(at, "Died at Level 20 by Pvpci".into())]);
    let extension = TabExtension {
        vocation_id: None,
        deaths: Some(vec![DeathDetails {
            level: 20,
            unjustified: true,
            ..DeathDetails::default()
        }]),
    };
    Ok((view, extension))
}
