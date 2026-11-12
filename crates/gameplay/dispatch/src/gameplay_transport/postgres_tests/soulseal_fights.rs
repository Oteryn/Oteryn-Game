//! T114: actual admitted command frames reach the seal debit and real Instance handoff.
use super::{
    TestResult, debug,
    fixture::{Harness, id},
};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::{CommandStatus, task_board::soulpit as wire};
#[path = "soulseal_fights/wire.rs"]
mod transport;
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Spend,
    Capacity,
    Rollback,
    Insufficient,
}
#[test]
fn t114_soulseal_real_dispatch_spends_once_and_admits_solo() -> TestResult {
    run(Mode::Spend)
}
#[test]
fn t114_soulseal_refused_capacity_preserves_seals() -> TestResult {
    run(Mode::Capacity)
}
#[test]
fn t114_soulseal_failed_successor_rolls_back_payment() -> TestResult {
    run(Mode::Rollback)
}
#[test]
fn t114_soulseal_insufficient_balance_cannot_create_encounter() -> TestResult {
    run(Mode::Insufficient)
}
fn run(mode: Mode) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::soulpit_shared_entry::native::soulseal_scenario(mode)
}
pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    h: &Harness,
    mut admitted: AdmittedSession,
    mode: Mode,
) -> TestResult {
    use super::cyclopedia::support::exchange;
    let native = owner
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or("native")?;
    let identity = owner.active_generation.ok_or("active")?.identity();
    let frame = owner
        .runtime
        .lock()
        .await
        .content_pin()
        .frame_binding_digest();
    sqlx::query("SELECT game_world_reset_initialize(encode($1,'hex')::uuid,$2,$3,$2,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_06',1)")
        .bind(admitted.world_id.as_bytes().as_slice()).bind(identity.server_artifact_digest().as_slice()).bind("soulseal-test-map")
        .bind(identity.client_artifact_digest().as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
    let initial = if mode == Mode::Insufficient {
        0_i64
    } else {
        100
    };
    // Establish the quest revision chain through the real Task Board writer first.
    let current = crate::gameplay_transport::character_gameplay_fence::current(
        owner,
        admitted.game_session_id,
        false,
    )
    .await
    .ok_or("seed fence")?;
    let mut board = crate::durability::task_board::TaskBoardCatalogue::try_from(native)?;
    board.gameplay_content_revision=sqlx::query_scalar("SELECT content_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    let seeded = owner
        .root
        .commit_task_board(
            owner.character,
            owner.holder,
            current,
            crate::durability::task_board::TaskBoardCommand {
                cause: crate::durability::quest_state::QuestCause::Command(
                    crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(900_000)?,
                    ),
                ),
                action:
                    oteryn_gameplay_quests::quest::task_board::full::TaskBoardAction::StartWeekly {
                        difficulty: 0,
                    },
                payload_binding: vec![],
                fee_change: None,
            },
            std::sync::Arc::new(board),
            None,
        )
        .await
        .map_err(debug)?;
    assert!(
        seeded.refusal.is_none() && seeded.transition.is_some(),
        "initial quest receipt: {seeded:?}"
    );
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,'oteryn:taskboard/weekly/soulseals','oteryn:taskboard/weekly',$2,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value")
        .bind(id(41).as_slice()).bind(initial).execute(&mut *tx).await?;
    tx.commit().await?;
    let (status, bytes) = exchange(
        owner,
        &mut admitted,
        1,
        wire::COMMAND_TYPE,
        &wire::Request::Catalogue { after: 0 }
            .encode()
            .map_err(debug)?,
    )
    .await?;
    assert_eq!(status, CommandStatus::Accepted);
    let catalogue = wire::Reply::decode(&bytes).map_err(debug)?;
    assert_eq!(catalogue.status, wire::Status::Catalogue);
    assert_eq!(catalogue.balance, Some(initial.try_into()?));
    let fight = catalogue.entries.first().ok_or("configured fight")?;
    let facts = crate::durability::task_board::soulseal_fights(native)
        .into_iter()
        .find(|f| f.race == fight.race)
        .ok_or("canonical fight")?;
    assert_eq!(fight.name, facts.name);
    assert_eq!(fight.cost, (u16::from(fight.stars) + 1) * 10);
    let authorization = catalogue.authorization.ok_or("staging authorization")?;
    let request = wire::Request::Enter {
        authorization,
        race: fight.race,
    }
    .encode()
    .map_err(debug)?;
    if mode == Mode::Rollback {
        sqlx::raw_sql("CREATE SEQUENCE t114_after_debit; GRANT USAGE ON SEQUENCE t114_after_debit TO oteryn_game_runtime; CREATE FUNCTION t114_fail_successor() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM nextval('t114_after_debit'); RAISE EXCEPTION 'T114 successor failure' USING ERRCODE='23514'; END $$; CREATE CONSTRAINT TRIGGER t114_fail_successor AFTER UPDATE ON game_activity_handoff_groups DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN (NEW.state=2 AND NEW.activity_key='oteryn:activity.soulpit') EXECUTE FUNCTION t114_fail_successor();").execute(&h.pool).await?;
    }
    let source = admitted;
    let result = if mode == Mode::Spend {
        transport::enter(owner, &mut admitted, &request).await
    } else {
        exchange(owner, &mut admitted, 2, wire::COMMAND_TYPE, &request).await
    };
    let balance:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:taskboard/weekly/soulseals'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    let payments: i64 = sqlx::query_scalar("SELECT count(*) FROM game_soulseal_entries")
        .fetch_one(&h.pool)
        .await?;
    let encounters: i64 = sqlx::query_scalar("SELECT count(*) FROM game_soulpit_encounters")
        .fetch_one(&h.pool)
        .await?;
    let crafts: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_craft_records WHERE recipe='oteryn:soulpit.entry'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        crafts, 0,
        "seal entry must not manufacture an Item craft receipt"
    );
    if mode == Mode::Rollback {
        let reached: bool = sqlx::query_scalar("SELECT is_called FROM t114_after_debit")
            .fetch_one(&h.pool)
            .await?;
        assert!(
            reached,
            "failure must happen after the seal debit and successor preparation"
        );
    }
    if mode != Mode::Spend {
        assert_eq!(
            (balance, payments, encounters),
            (initial, 0, 0),
            "failed entry rolls all payment effects back"
        );
        if mode != Mode::Rollback {
            let (status, bytes) = result?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(
                wire::Reply::decode(&bytes).map_err(debug)?.status,
                wire::Status::Refused
            );
        }
        return Ok(());
    }
    let (status, bytes) = result?;
    assert_eq!(status, CommandStatus::Accepted);
    let started = wire::Reply::decode(&bytes).map_err(debug)?;
    assert_eq!(started.status, wire::Status::Started);
    assert_eq!(
        (balance, payments, encounters),
        (initial - i64::from(fight.cost), 1, 1)
    );
    assert_eq!(started.balance, Some(balance.try_into()?));
    let participants: i64 = sqlx::query_scalar("SELECT count(*) FROM game_soulpit_participants")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(participants, 1);
    assert!(matches!(
        admitted
            .item_fence
            .ok_or("destination fence")?
            .runtime_scope,
        crate::foundation::RuntimeScopeRefV1::Instance { .. }
    ));
    // The exact old command replays even after one-use authorization consumption.
    let replay = owner
        .task_board_wire(source, 2, wire::COMMAND_TYPE, &request)
        .await;
    let crate::gameplay_transport::task_board::TaskBoardReply::Payload(bytes) = replay else {
        return Err("entry replay".into());
    };
    assert_eq!(wire::Reply::decode(&bytes).map_err(debug)?, started);
    let after:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:taskboard/weekly/soulseals'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(after, balance);
    println!("<T114> POSTGRES_CASE_COMPLETED: solo entry, exact debit, real scope handoff, replay");
    Ok(())
}
