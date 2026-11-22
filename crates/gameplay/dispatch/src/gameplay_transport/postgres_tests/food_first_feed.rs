use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, item_look_fixtures, prepare_current_session};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_view::ItemKey;
use oteryn_protocol_oteryn::chat::{ChatLine, ChatSpeechMode};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::time::Duration;

#[test]
fn zero_second_food_retires_one_item_and_speaks_without_starting_regeneration() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(
        admin,
        "zero_second_food",
        &[("oteryn:item.tibia.i15795", 0, "Phew!")],
    )
}

#[test]
fn first_feed_accepts_the_full_duration_again_after_expiry_and_refuses_a_fed_actor() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(
        admin,
        "first_feed",
        &[
            ("oteryn:item.tibia.i48255", 1_500, "Yum."),
            ("oteryn:item.tibia.i48256", 7_260, "Yum."),
            ("oteryn:item.tibia.i48273", 2_220, "Yum."),
            ("oteryn:item.tibia.i48508", 1_500, "Yum."),
            ("oteryn:item.tibia.i48509", 1_440, "Yum."),
            ("oteryn:item.tibia.i48511", 1_440, "Yum."),
            ("oteryn:item.tibia.i48544", 1_488, "Yum."),
        ],
    )
}

type FoodCase = (&'static str, u32, &'static str);

fn run(admin: String, name: &'static str, cases: &'static [FoodCase]) -> TestResult {
    std::thread::Builder::new()
        .stack_size(128 << 20)
        .spawn(move || {
            crate::bestiary_postgres_harness::runtime()
                .map_err(|error| error.to_string())?
                .block_on(scenario(admin, name, cases))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "food dispatch thread panicked")?
        .map_err(Into::into)
}

async fn scenario(admin: String, name: &str, cases: &[FoodCase]) -> TestResult {
    let harness = Harness::create_with_admission_seed(admin, name, true, false).await?;
    eprintln!("POSTGRES_STARTED {name}");
    prepare_current_session(&harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let generation = controller.active().ok_or("active content")?;
    let room = pin.qualified_room().clone();
    let frame_digest = pin.frame_binding().digest();
    let deps = Dependencies::with_content(
        pin.into_channel_pin(),
        room,
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let mut owner = deps.owner(&harness, &authority);
    owner.active_generation = Some(generation);
    deps.initialize(&owner).await;
    let session = deps.admitted.game_session_id;
    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(harness.root.clone())
            .current_session_at(session)
            .await?
            .0;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(id(41).as_slice()).execute(&harness.pool).await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice())
        .bind(frame_digest.as_slice()).execute(&harness.pool).await?;
    owner
        .states
        .chat
        .admit(session, "Food tester", owner.owner_now());
    let entries: Vec<_> = cases
        .iter()
        .flat_map(|&(key, _, _)| [(key, 1), (key, 1)])
        .collect();
    item_look_fixtures::seed_main_backpack(&harness, &entries).await?;

    for (offset, &(key, seconds, speech)) in cases.iter().enumerate() {
        let seed = 71 + 2 * u8::try_from(offset)?;
        let command_id = 1 + 2 * u64::try_from(offset)?;
        let command = UseCommand {
            game_session_id: session,
            command_id,
            item_fence: Some(fence),
        };
        let before = owner.owner_ms();
        let result = owner
            .use_item(deps.actor, command, ItemKey::Instance(id(seed)))
            .await;
        let after = owner.owner_ms();
        assert_eq!(result.disposition, UseDisposition::Committed, "{key}");
        assert_eq!(quantity(&harness, seed).await?, 0, "{key} must retire");
        assert!(
            matches!(
                owner.states.chat.drain(session).as_slice(),
                [ChatLine::Local { mode: ChatSpeechMode::MonsterSay, text, .. }] if text == speech
            ),
            "{key} must speak once"
        );
        assert_eq!(
            owner
                .use_item(deps.actor, command, ItemKey::Instance(id(seed)))
                .await
                .disposition,
            UseDisposition::Committed,
            "{key} replay"
        );
        assert!(
            owner.states.chat.drain(session).is_empty(),
            "{key} replay must be silent"
        );
        {
            let runtime = deps.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let actor = states
                .get(&runtime, deps.actor, session)
                .ok_or("live actor")?;
            let remaining = u64::from(actor.bed_rest_snapshot(after * 1_000)[1]);
            let duration = u64::from(seconds) * 1_000;
            assert!(
                (duration.saturating_sub(after - before)..=duration).contains(&remaining),
                "{key}: {remaining}"
            );
        }

        if seconds > 0 {
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_secs(1))
                .ok_or("food cooldown")?;
            let second = owner
                .use_item(
                    deps.actor,
                    UseCommand {
                        command_id: command_id + 1,
                        ..command
                    },
                    ItemKey::Instance(id(seed + 1)),
                )
                .await;
            assert_eq!(second.disposition, UseDisposition::Full, "{key} while fed");
            assert_eq!(
                quantity(&harness, seed + 1).await?,
                1,
                "{key} refusal must preserve item"
            );
            assert!(
                owner.states.chat.drain(session).is_empty(),
                "{key} refusal must be silent"
            );
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(Duration::from_secs(u64::from(seconds)))
                .ok_or("food expiry")?;
        }
    }
    let uses: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_use_records")
        .fetch_one(&harness.pool)
        .await?;
    assert_eq!(uses, i64::try_from(cases.len())?);
    drop(owner);
    drop(authority);
    drop(seal);
    harness.cleanup().await
}

async fn quantity(harness: &Harness, seed: u8) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(seed).as_slice())
    .fetch_one(&harness.pool)
    .await?)
}
