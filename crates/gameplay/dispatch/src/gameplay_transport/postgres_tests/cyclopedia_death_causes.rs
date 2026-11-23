use super::fixture::{Harness, debug, id, pvp_source_death_fixture};
use super::{TestResult, kill_reward_e2e_fixtures::scenario_thread, prepare_current_session};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::GameSessionId;
use oteryn_protocol_oteryn::character_info::PageRequest;
use oteryn_protocol_oteryn::cyclopedia_character::{CharacterTab, DeathDetails, TabView};

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
                'fixture', 'fixture', 'fixture', 'fixture', 1000+n
             FROM unnest($4::bytea[]) WITH ORDINALITY AS deaths(source,n)",
        )
        .bind(id(41).as_slice())
        .bind(id(42).as_slice())
        .bind(id(43).as_slice())
        .bind([id(90).to_vec(), id(91).to_vec(), id(92).to_vec()].as_slice())
        .execute(&mut *tx)
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
