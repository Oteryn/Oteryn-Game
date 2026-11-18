//! chsw (owner, 2026-11-17): a Party and its chat survive a voluntary Channel switch. The
//! switch's safe exit (the terminal release with `with_channel_switch`) holds the membership;
//! an ordinary logout removes it; an expired hold without a current session leaves the Party
//! through the real World Party expiry drain (migration 2859).
use crate::bestiary_postgres_harness::{
    CHANNEL, Harness, TestResult, WORLD, configured_admin, debug, id, runtime,
};
use crate::durability::fresh_admission::{ExpiredLossReleaseV1, FreshAdmissionStore};
use crate::foundation::{GameSessionId, WorldId};

const PARTY: u8 = 210;

async fn members(h: &Harness) -> TestResult<Vec<Vec<u8>>> {
    Ok(sqlx::query_scalar(
        "SELECT uuid_send(character_id) FROM game_party_members ORDER BY seq",
    )
    .fetch_all(&h.pool)
    .await?)
}

async fn holds(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_channel_switch_party_holds")
            .fetch_one(&h.pool)
            .await?,
    )
}

async fn release(h: &Harness, session: u8, switch: bool) -> TestResult<ExpiredLossReleaseV1> {
    let session = GameSessionId::decode(&id(session)).map_err(debug)?;
    let store = FreshAdmissionStore::from_root(h.root.clone());
    let store = if switch {
        store.with_channel_switch()
    } else {
        store
    };
    let current = store.current_session(session).await.map_err(debug)?;
    let transport = current.current_transport().ok_or("current transport")?;
    let account: String = sqlx::query_scalar(
        "SELECT account_id::text FROM game_durability_reconnect_sessions \
         WHERE game_session_id=encode($1,'hex')::uuid",
    )
    .bind(session.as_bytes().as_slice())
    .fetch_one(&h.pool)
    .await?;
    Ok(store
        .release_logout_session(session, &account, transport)
        .await
        .map_err(debug)?)
}

async fn scenario(h: &Harness) -> TestResult {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let mut characters = Vec::new();
    for (character, account, session) in [(60, 63, 70), (61, 64, 71), (62, 65, 72)] {
        let current = crate::world_party_runtime_postgres_cases::committed_fresh_player(
            h, character, account, session, now,
        )
        .await?;
        characters.push(*current.current_character_lease().character_id().as_bytes());
    }
    // Administrative membership fixture: the leader and two members on the harness Channel.
    let mut setup = h.pool.begin().await?;
    sqlx::query("INSERT INTO game_parties(party_id,world_id,leader_character_id,revision,next_seq) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,4)")
        .bind(id(PARTY).as_slice()).bind(id(WORLD).as_slice()).bind(characters[0].as_slice())
        .execute(&mut *setup).await?;
    for (seq, character) in characters.iter().enumerate() {
        sqlx::query("INSERT INTO game_party_members(character_id,party_id,seq,presence_state,presence_channel_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,1,encode($4,'hex')::uuid)")
            .bind(character.as_slice()).bind(id(PARTY).as_slice()).bind(i64::try_from(seq)? + 1)
            .bind(id(CHANNEL).as_slice()).execute(&mut *setup).await?;
    }
    setup.commit().await?;
    let all: Vec<Vec<u8>> = characters.iter().map(|c| c.to_vec()).collect();

    // The switch's safe exit keeps the member and holds the membership.
    assert!(matches!(
        release(h, 71, true).await?,
        ExpiredLossReleaseV1::Released { .. }
    ));
    assert_eq!(members(h).await?, all, "a switch keeps the Party membership");
    assert_eq!(holds(h).await?, 1);

    // An ordinary logout still leaves the Party.
    assert!(matches!(
        release(h, 72, false).await?,
        ExpiredLossReleaseV1::Released { .. }
    ));
    assert_eq!(members(h).await?, all[..2].to_vec(), "a logout leaves the Party");
    assert_eq!(holds(h).await?, 1);

    // A live hold is left alone by the World Party drain.
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovery = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let scope = crate::foundation::RuntimeScopeRefV1::channel(
        WorldId::decode(&id(WORLD)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
    );
    let lane = || async {
        crate::durability::spell_owner_commit::SpellLane::new(
            WorldId::decode(&id(WORLD)).map_err(debug)?,
            crate::foundation::ChannelId::decode(&id(CHANNEL)).map_err(debug)?,
        )
        .acquire()
        .await
        .map_err(|_| -> Box<dyn std::error::Error> {
            "spell lane holds an unresolved attempt".into()
        })
    };
    let drain = |permit| {
        let recovery = &recovery;
        async move {
            h.root
                .drain_world_party_expiry(&permit, recovery, &h.node, scope, 1)
                .await
                .map_err(debug)
        }
    };
    assert_eq!(drain(lane().await?).await?, 0);
    assert_eq!(members(h).await?, all[..2].to_vec());

    // No admission came back in time: the expired hold leaves the Party as a logout would.
    sqlx::query("UPDATE game_channel_switch_party_holds SET held_until_ms=1")
        .execute(&h.pool)
        .await?;
    assert_eq!(drain(lane().await?).await?, 1);
    // As with a logout, the leader left alone closes the Party.
    assert!(members(h).await?.is_empty(), "the held member left");
    let parties: i64 = sqlx::query_scalar("SELECT count(*) FROM game_parties")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(parties, 0);
    assert_eq!(holds(h).await?, 0, "the hold goes with the membership");
    assert_eq!(drain(lane().await?).await?, 0);
    Ok(())
}

#[test]
fn a_channel_switch_keeps_the_party_and_an_expired_hold_leaves_it() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            runtime().map_err(debug)?.block_on(async move {
                let h = Harness::create(admin, "channel_switch_party", true)
                    .await
                    .map_err(debug)?;
                let result = scenario(&h).await.map_err(|e| e.to_string());
                h.cleanup().await.map_err(debug)?;
                result
            })
        })?
        .join()
        .map_err(|_| "channel switch party thread panicked")??;
    Ok(())
}
