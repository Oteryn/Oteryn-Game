//! Settings travel through the real admitted transport; Find Person consumes the result.
use super::*;
use crate::foundation::ExactActorRef;
use oteryn_protocol_oteryn::exiva::COMMAND_TYPE_EXIVA_SETTINGS;

#[test]
fn exiva_settings_allow_all_postgres() -> TestResult {
    scenario(7)
}
#[test]
fn exiva_settings_player_list_postgres() -> TestResult {
    scenario(8)
}
#[test]
fn exiva_settings_guild_postgres() -> TestResult {
    scenario(9)
}
#[test]
fn exiva_settings_party_postgres() -> TestResult {
    scenario(10)
}
#[test]
fn exiva_settings_vip_postgres() -> TestResult {
    scenario(11)
}
#[test]
fn exiva_settings_denied_postgres() -> TestResult {
    scenario(12)
}

#[test]
fn exiva_settings_own_guild_postgres() -> TestResult {
    scenario(13)
}

pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut caster: AdmittedSession,
    target_actor: ExactActorRef,
    case: u8,
) -> TestResult {
    let target_session = GameSessionId::decode(&id(110)).map_err(debug)?;
    let current =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(target_session)
            .await?
            .0;
    let account: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(101).as_slice()).fetch_one(&h.pool).await?;
    let mut target = AdmittedSession {
        game_session_id: target_session,
        runtime_actor: Some(target_actor),
        item_fence: item_fence_of(current),
        controller: Some(crate::gameplay_transport::connection::ControllerBinding {
            account_id: account.try_into().map_err(|_| "target account UUID")?,
            transport: current.current_transport().ok_or("target transport")?,
        }),
        ..caster
    };
    target.continuity = crate::gameplay_transport::connection::SessionContinuity::FRESH;
    target.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[6, 7, 147])
            .ok_or("Exiva capability")?;
    let modes = match case {
        7 => 1,
        8 => 2,
        9 | 13 => 4,
        10 => 8,
        11 => 16,
        _ => 0,
    };
    // Seed only relationship-owner fixtures. Privacy edits and casts use production dispatch.
    if matches!(case, 9..=11 | 13) {
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        match case {
            9 | 13 => {
                sqlx::query("INSERT INTO game_guilds(guild_id,world_id,name,state,formation_deadline,revision,created_at) VALUES(encode($1,'hex')::uuid,encode($3,'hex')::uuid,'Caster Guild',2,9999999999,1,1),(encode($2,'hex')::uuid,encode($3,'hex')::uuid,'Target Guild',2,9999999999,1,1)")
                    .bind(id(120).as_slice()).bind(id(121).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_guild_members(character_id,guild_id,rank_level,joined_at) VALUES(encode($1,'hex')::uuid,encode($3,'hex')::uuid,1,1),(encode($2,'hex')::uuid,encode($4,'hex')::uuid,1,1)")
                    .bind(id(41).as_slice()).bind(id(101).as_slice()).bind(id(120).as_slice()).bind(id(121).as_slice()).execute(&mut *tx).await?;
            }
            10 => {
                sqlx::query("INSERT INTO game_parties(party_id,world_id,leader_character_id,revision,next_seq) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,3)")
                    .bind(id(120).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_party_members(character_id,party_id,seq,presence_state) VALUES(encode($1,'hex')::uuid,encode($3,'hex')::uuid,1,2),(encode($2,'hex')::uuid,encode($3,'hex')::uuid,2,2)")
                    .bind(id(41).as_slice()).bind(id(101).as_slice()).bind(id(120).as_slice()).execute(&mut *tx).await?;
            }
            11 => {
                sqlx::query("INSERT INTO game_vip_lists(account_id,world_id) SELECT account_id,world_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                    .bind(id(101).as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_vip_entries(account_id,world_id,character_id,entry_no) SELECT account_id,world_id,encode($2,'hex')::uuid,1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                    .bind(id(101).as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
            }
            _ => unreachable!(),
        }
        if case == 13 {
            sqlx::query("UPDATE game_guild_members SET guild_id=encode($1,'hex')::uuid,rank_level=3 WHERE character_id=encode($2,'hex')::uuid")
                .bind(id(120).as_slice()).bind(id(101).as_slice()).execute(&mut *tx).await?;
        }
        tx.commit().await?;
    }
    let mut commands = vec![(COMMAND_TYPE_EXIVA_SETTINGS, vec![0, modes])];
    if case == 8 {
        let mut add = vec![1];
        add.extend(id(41));
        commands.push((COMMAND_TYPE_EXIVA_SETTINGS, add));
    } else if case == 9 {
        let mut add = vec![3];
        add.extend(id(120));
        commands.push((COMMAND_TYPE_EXIVA_SETTINGS, add));
    }
    let (results, _) = exchange(owner, &mut target, &commands, false, false).await?;
    for result in &results {
        assert_eq!(result, &(CommandStatus::Accepted, vec![1]));
    }
    // A replay on a new transport must not create a second receipt/revision.
    // The ingress window retains the latest result; older committed commands
    // are tested separately against the durable receipt below.
    let latest = commands.len() - 1;
    let (results, _) = exchange_from(
        owner,
        &mut target,
        &commands[latest..],
        false,
        false,
        u64::try_from(latest + 1)?,
    )
    .await?;
    assert_eq!(results[0].0, CommandStatus::DuplicateReplay);
    // A process with no retained ingress result still reconciles the committed
    // receipt under the current fence before touching preferences.
    assert_eq!(
        owner
            .exiva_settings(
                target,
                1,
                oteryn_protocol_oteryn::exiva::Request::decode(&commands[0].1)
                    .ok_or("settings request")?
            )
            .await,
        Some(true)
    );
    let revision: String = sqlx::query_scalar("SELECT revision::text FROM game_character_spell_privacy WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(101).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(revision, if matches!(case, 8 | 9) { "2" } else { "1" });
    let payload = chat::encode_chat_intent(&chat::ChatIntent::Say {
        mode: chat::ChatSpeechMode::Say,
        text: "exiva \"Fixture Watcher\"".into(),
    })
    .map_err(debug)?;
    let (results, feedback) = exchange(
        owner,
        &mut caster,
        &[(chat::COMMAND_TYPE_CHAT_INTENT, payload)],
        false,
        true,
    )
    .await?;
    let disposition = chat::decode_chat_intent_result(&results[0].1)
        .map_err(debug)?
        .disposition;
    if case == 12 {
        assert_eq!(disposition, chat::ChatDisposition::Rejected);
        assert!(feedback.iter().any(|line| matches!(line, chat::ChatLine::Private { speaker_name, text } if speaker_name == "Game" && text == "The character you are trying to find with Exiva is currently protected from your spell.")), "{feedback:?}");
    } else {
        assert_eq!(disposition, chat::ChatDisposition::Ok);
        assert!(feedback.iter().any(|line| matches!(line, chat::ChatLine::Private { speaker_name, text } if speaker_name == "Game" && text.starts_with("Fixture Watcher "))), "{feedback:?}");
    }
    // Remove every relationship / allow-list entry and prove the next cast refuses.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for query in [
        "DELETE FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid",
        "DELETE FROM game_party_members WHERE character_id=encode($1,'hex')::uuid",
        "DELETE FROM game_vip_entries WHERE character_id=encode($1,'hex')::uuid",
    ] {
        sqlx::query(query)
            .bind(id(41).as_slice())
            .execute(&mut *tx)
            .await?;
    }
    if case == 11 {
        // The reverse VIP relation must not grant permission.
        sqlx::query("INSERT INTO game_vip_lists(account_id,world_id) SELECT account_id,world_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT DO NOTHING")
            .bind(id(41).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_vip_entries(account_id,world_id,character_id,entry_no) SELECT account_id,world_id,encode($2,'hex')::uuid,1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).bind(id(101).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    let payload = if case == 8 {
        let mut remove = vec![2];
        remove.extend(id(41));
        remove
    } else if case == 9 {
        let mut remove = vec![4];
        remove.extend(id(120));
        remove
    } else {
        vec![
            0,
            if matches!(case, 9..=11 | 13) {
                modes
            } else {
                0
            },
        ]
    };
    let (results, _) = exchange(
        owner,
        &mut target,
        &[(COMMAND_TYPE_EXIVA_SETTINGS, payload)],
        false,
        false,
    )
    .await?;
    assert_eq!(results[0], (CommandStatus::Accepted, vec![1]));
    // Privacy denial does not consume mana or start the spell cooldown. Successful
    // casts above do, so advance the owner's deterministic clock past that cooldown.
    tokio::time::sleep(Duration::from_millis(2100)).await;
    let payload = chat::encode_chat_intent(&chat::ChatIntent::Say {
        mode: chat::ChatSpeechMode::Say,
        text: "exiva \"Fixture Watcher\"".into(),
    })
    .map_err(debug)?;
    let (results, feedback) = exchange(
        owner,
        &mut caster,
        &[(chat::COMMAND_TYPE_CHAT_INTENT, payload)],
        false,
        true,
    )
    .await?;
    assert_eq!(
        chat::decode_chat_intent_result(&results[0].1)
            .map_err(debug)?
            .disposition,
        chat::ChatDisposition::Rejected
    );
    assert!(feedback.iter().any(|line| matches!(line, chat::ChatLine::Private { speaker_name, text } if speaker_name == "Game" && text == "The character you are trying to find with Exiva is currently protected from your spell.")), "{feedback:?}");
    if case == 12 {
        let before: String = sqlx::query_scalar("SELECT revision::text FROM game_character_spell_privacy WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(101).as_slice()).fetch_one(&h.pool).await?;
        let mut unknown = vec![1];
        unknown.extend(id(125));
        let (results, _) = exchange(
            owner,
            &mut target,
            &[
                (COMMAND_TYPE_EXIVA_SETTINGS, vec![0, 32]),
                (COMMAND_TYPE_EXIVA_SETTINGS, unknown),
            ],
            false,
            false,
        )
        .await?;
        assert!(
            results
                .iter()
                .all(|result| *result == (CommandStatus::Rejected, vec![0]))
        );
        target.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[6, 7])
                .ok_or("no Exiva capability")?;
        let (results, _) = exchange(
            owner,
            &mut target,
            &[(COMMAND_TYPE_EXIVA_SETTINGS, vec![0, 1])],
            false,
            false,
        )
        .await?;
        assert_eq!(results[0].0, CommandStatus::Rejected);
        let after: String = sqlx::query_scalar("SELECT revision::text FROM game_character_spell_privacy WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(101).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(before, after);
    }
    Ok(())
}
