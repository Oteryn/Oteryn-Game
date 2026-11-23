//! P13 first entry installs account loyalty for live weapon and shielding skills.
use super::fixture::{Harness, debug, id, run};
use super::tutorial_vitals_fixture::{admit, native_asset_revisions, publish_native_activation};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::ability::creature_bite::CreatureBiteVitals;
use crate::durability::character_build::BuildFormula;
use crate::foundation::owner_timer::SemanticTimeMicros;
use crate::foundation::{ChannelContentPin, WorldId};
use crate::gameplay_transport::attack::player_block::PlayerBiteVitals;
use crate::spell::cast::SwingSkills;
use oteryn_gameplay_combat_rules::attack::formulas::DefenceSource;
use oteryn_gameplay_combat_rules::attack::{AttackConstants, FightMode};
use oteryn_gameplay_combat_rules::resolution::player_defence::{
    hand_defence, player_defence_bounds,
};

#[test]
fn loyalty_combat_projects_weapon_and_shielding_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("loyalty-combat", || scenario(false))
}

#[test]
fn loyalty_combat_refreshes_platform_corrections_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("loyalty-correction", || scenario(true))
}

fn scenario(check_correction: bool) -> TestResult {
    run(async |admin| {
        let started = std::time::Instant::now();
        let h = Harness::create_with_admission_seed(admin, "loyalty_combat", true, false).await?;
        let request = prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let active = controller.active().ok_or("active content")?;
        publish_native_activation(&h, &pin).await?;
        native_asset_revisions(&h, active).await?;

        let native = active.native_gameplay().ok_or("native gameplay")?;
        let formula = native.training_formula().ok_or("native training formula")?;
        let constants = AttackConstants::checked_in().ok_or("attack constants")?;
        let shield = "oteryn:item.tibia.i3412";
        let revision = native
            .item_index()
            .and_then(|items| items.revision(shield))
            .ok_or("shield definition")?;
        let hand = hand_defence(native, constants, |slot| {
            (slot == 6).then_some((shield, revision))
        })
        .ok_or("native shield defence")?;
        assert_eq!(hand.source, DefenceSource::Shield);
        // One try short of the next level. Omitting the remainder would give 102/104.
        let sword_tries = formula.required("paladin", 3, 101).ok_or("sword curve")? - 1;
        let shielding_tries = formula
            .required("paladin", 6, 101)
            .ok_or("shielding curve")?
            - 1;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO game_character_build_state (
                character_id, vocation, magic_level, mana_spent, fist_level, fist_tries,
                club_level, club_tries, sword_level, sword_tries, axe_level, axe_tries,
                distance_level, distance_tries, shielding_level, shielding_tries,
                fishing_level, fishing_tries, committed_character_revision, last_build_occurrence_id
            ) VALUES (
                encode($1,'hex')::uuid, 'paladin', 0, 0, 10, 0,
                10, 0, 100, $2, 10, 0, 10, 0, 100, $3, 10, 0, 2, encode($4,'hex')::uuid
            )",
        )
        .bind(id(41).as_slice())
        .bind(i64::try_from(sword_tries)?)
        .bind(i64::try_from(shielding_tries)?)
        .bind(id(30).as_slice())
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE game_character_roots SET character_revision=2 WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        let read_build = async || {
            sqlx::query_scalar::<_, serde_json::Value>(
                "SELECT to_jsonb(b) FROM game_character_build_state b WHERE character_id=encode($1,'hex')::uuid",
            ).bind(id(41).as_slice()).fetch_one(&h.pool).await
        };
        let before = read_build().await?;

        let room = pin.qualified_room().clone();
        let start = room.entry_start();
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            room.map_revision_digest(),
            (start.x, start.y, start.floor),
        );
        for (points, attack, shielding) in [(0, 100, 100), (3600, 103, 105)] {
            sqlx::query(
                "INSERT INTO game_account_loyalty_projections (account_id, loyalty_points, platform_revision)
                 VALUES (encode($1,'hex')::uuid, $2, $3)
                 ON CONFLICT (account_id) DO UPDATE
                 SET loyalty_points=EXCLUDED.loyalty_points, platform_revision=EXCLUDED.platform_revision",
            )
            .bind(id(40).as_slice())
            .bind(i64::from(points))
            .bind(if points == 0 { 1_i64 } else { 2_i64 })
            .execute(&h.pool)
            .await?;
            let deps = Dependencies::with_content(
                channel_pin.clone(),
                room.clone(),
                crate::spell::cast::v1_spell_book().map_err(debug)?,
            )?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(active);
            admit(&owner, &deps, &request).await?;
            let runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let actor = states
                .get_mut(&runtime, deps.actor, deps.admitted.game_session_id)
                .ok_or("admitted combat actor")?;
            let now = owner.owner_now().get();
            let swing = actor.swing_skills(2, now);
            // The starter shield's extra-defence field is unknown. Use a fully
            // qualified shield so this test isolates training from equipment admission.
            actor.install_equipped_defence(Some(hand));
            let expected =
                player_defence_bounds(constants, hand, shielding, FightMode::Balanced, false)
                    .ok_or("expected defence range")?;
            let mut attack_states = owner.states.attack.lock().await;
            let mut trained = Vec::new();
            let mut vitals = PlayerBiteVitals {
                quest_sessions: None,
                conditions: None,
                charm: None,
                states: &mut states,
                attack: &mut attack_states,
                formula: None,
                trained: &mut trained,
            };
            let defence = vitals.target_defence(
                &runtime,
                deps.actor,
                deps.admitted.game_session_id,
                SemanticTimeMicros::from_micros(now),
            );
            assert_eq!(
                (swing, defence),
                (
                    Some(Some(SwingSkills { attack, shielding })),
                    Some(expected)
                ),
                "combat skills with {points} loyalty points"
            );
            drop(attack_states);
            drop(states);
            drop(runtime);
            if check_correction && points == 3600 {
                let applied: i16 = sqlx::query_scalar(
                    "SELECT game_account_loyalty_project_points(encode($1,'hex')::uuid,0,0,3)",
                )
                .bind(id(40).as_slice())
                .fetch_one(&h.pool)
                .await?;
                assert_eq!(applied, 1);
                Box::pin(owner.tick_timed_items()).await;
                let runtime = deps.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                let state = states
                    .get(&runtime, deps.actor, deps.admitted.game_session_id)
                    .ok_or("combat actor after correction")?;
                assert_eq!(
                    state.swing_skills(2, owner.owner_now().get()),
                    Some(Some(SwingSkills {
                        attack: 100,
                        shielding: 100
                    })),
                    "the next combat cadence applies a newer Platform revision of zero",
                );
            }
        }
        assert_eq!(
            read_build().await?,
            before,
            "admission preserves the committed build"
        );
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        eprintln!(
            "LOYALTY_COMBAT POSTGRES_CASE_COMPLETED elapsed={:?}",
            started.elapsed()
        );
        Ok(())
    })
}
