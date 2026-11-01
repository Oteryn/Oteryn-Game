use super::*;
use oteryn_protocol_oteryn::chat::private_channel::{
    MAX_INVITED_PRIVATE_CHANNELS, MAX_PRIVATE_CHANNEL_INVITES,
};

async fn own(
    tx: &mut Transaction<'_, Postgres>,
    f: &CurrentCharacterItemFence,
) -> Result<Option<i32>> {
    Ok(sqlx::query_scalar("SELECT channel_id FROM game_private_chat_channels WHERE world_id=encode($1,'hex')::uuid AND owner_session_id=encode($2,'hex')::uuid")
        .bind(f.runtime_scope.world_id().as_bytes().as_slice()).bind(f.game_session_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?)
}

async fn cleanup(tx: &mut Transaction<'_, Postgres>, f: &CurrentCharacterItemFence) -> Result<()> {
    // The World contains at most 9,900 private channels, each with one owner and 16 invitees.
    sqlx::query("DELETE FROM game_private_chat_channels c WHERE c.world_id=encode($1,'hex')::uuid AND NOT EXISTS(SELECT 1 FROM game_private_chat_participants p JOIN game_durability_reconnect_sessions s ON s.game_session_id=p.game_session_id WHERE p.game_session_id=c.owner_session_id AND p.enabled AND s.session_state IN(1,2))")
        .bind(f.runtime_scope.world_id().as_bytes().as_slice()).execute(&mut **tx).await?;
    sqlx::query("DELETE FROM game_private_chat_members m WHERE m.world_id=encode($1,'hex')::uuid AND NOT EXISTS(SELECT 1 FROM game_private_chat_participants p JOIN game_durability_reconnect_sessions s ON s.game_session_id=p.game_session_id WHERE p.game_session_id=m.game_session_id AND p.enabled AND s.session_state IN(1,2))")
        .bind(f.runtime_scope.world_id().as_bytes().as_slice()).execute(&mut **tx).await?;
    Ok(())
}

pub(super) async fn apply(
    tx: &mut Transaction<'_, Postgres>,
    f: &CurrentCharacterItemFence,
    request: &PrivateChannelIntent,
    premium: bool,
    prepared: Option<&Publication>,
    sealed: Option<&(String, String)>,
) -> Result<ChatDisposition> {
    cleanup(tx, f).await?;
    match request {
        PrivateChannelIntent::Create => create(tx, f, premium).await,
        PrivateChannelIntent::Invite { name } => invite_or_exclude(tx, f, name, false).await,
        PrivateChannelIntent::Exclude { name } => invite_or_exclude(tx, f, name, true).await,
        PrivateChannelIntent::Join { channel_id } => join_or_leave(tx, f, *channel_id, false).await,
        PrivateChannelIntent::Leave { channel_id } => join_or_leave(tx, f, *channel_id, true).await,
        PrivateChannelIntent::Say { channel_id, .. } => {
            say(tx, f, *channel_id, prepared, sealed).await
        }
        PrivateChannelIntent::List => Ok(ChatDisposition::Ok),
    }
}

async fn create(
    tx: &mut Transaction<'_, Postgres>,
    f: &CurrentCharacterItemFence,
    premium: bool,
) -> Result<ChatDisposition> {
    if own(tx, f).await?.is_some() {
        return Ok(ChatDisposition::Rejected);
    }
    if !premium {
        return Ok(ChatDisposition::NotAllowed);
    }
    // Crystal 201189f src/creatures/interactions/chat.cpp:458-472.
    let id:Option<i32>=sqlx::query_scalar("SELECT n FROM generate_series(100,9999) n WHERE NOT EXISTS(SELECT 1 FROM game_private_chat_channels c WHERE c.world_id=encode($1,'hex')::uuid AND c.channel_id=n) ORDER BY n LIMIT 1")
        .bind(f.runtime_scope.world_id().as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(id) = id else {
        return Ok(ChatDisposition::ChatUnavailable);
    };
    sqlx::query("INSERT INTO game_private_chat_channels(world_id,channel_id,owner_session_id) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid)")
        .bind(f.runtime_scope.world_id().as_bytes().as_slice()).bind(id).bind(f.game_session_id.as_bytes().as_slice()).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_private_chat_members(world_id,channel_id,game_session_id,joined,joined_at_micros) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,true,floor(extract(epoch FROM clock_timestamp())*1000000)::bigint)")
        .bind(f.runtime_scope.world_id().as_bytes().as_slice()).bind(id).bind(f.game_session_id.as_bytes().as_slice()).execute(&mut **tx).await?;
    Ok(ChatDisposition::Ok)
}

async fn invite_or_exclude(
    tx: &mut Transaction<'_, Postgres>,
    f: &CurrentCharacterItemFence,
    name: &str,
    exclude: bool,
) -> Result<ChatDisposition> {
    let Some(channel) = own(tx, f).await? else {
        return Ok(ChatDisposition::NotAllowed);
    };
    let Ok(name) = CharacterName::parse(name) else {
        return Ok(ChatDisposition::NotOnline);
    };
    // Resolve by the same durable root/admission guards as PM. Channel and node are deliberately
    // absent from this predicate: a World invitation is valid across both boundaries.
    let target=sqlx::query("SELECT p.game_session_id::text,p.enabled FROM game_character_roots r JOIN game_durability_admission_character_guards g ON g.character_id=r.character_id AND g.eligible AND g.world_id=r.world_id JOIN game_durability_admission_account_guards a ON a.account_id=g.account_id AND a.presence_character_id=g.character_id AND a.holder_game_session_id=g.holder_game_session_id JOIN game_durability_reconnect_sessions s ON s.game_session_id=g.holder_game_session_id AND s.session_state=2 AND s.current_transport_ref IS NOT NULL AND s.control_loss_epoch IS NULL JOIN game_private_chat_participants p ON p.game_session_id=s.game_session_id AND p.connection_generation=s.current_generation WHERE r.world_id=encode($1,'hex')::uuid AND r.name_key=$2 AND r.lifecycle=1 LIMIT 1")
        .bind(f.runtime_scope.world_id().as_bytes().as_slice()).bind(name.comparison_key()).fetch_optional(&mut **tx).await?;
    let Some(target) = target else {
        return Ok(ChatDisposition::NotOnline);
    };
    let target = (
        uuid_text(&target.try_get::<String, _>("game_session_id")?)?,
        target.try_get::<bool, _>("enabled")?,
    );
    if target.0 == *f.game_session_id.as_bytes() || (!exclude && !target.1) {
        return Ok(ChatDisposition::Rejected);
    }
    if exclude {
        let removed=sqlx::query("DELETE FROM game_private_chat_members WHERE world_id=encode($1,'hex')::uuid AND channel_id=$2 AND game_session_id=encode($3,'hex')::uuid")
            .bind(f.runtime_scope.world_id().as_bytes().as_slice()).bind(channel).bind(target.0.as_slice()).execute(&mut **tx).await?.rows_affected();
        return Ok(if removed == 1 {
            ChatDisposition::Ok
        } else {
            ChatDisposition::NotInvited
        });
    }
    let existing:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_private_chat_members WHERE world_id=encode($1,'hex')::uuid AND channel_id=$2 AND game_session_id=encode($3,'hex')::uuid)")
        .bind(f.runtime_scope.world_id().as_bytes().as_slice()).bind(channel).bind(target.0.as_slice()).fetch_one(&mut **tx).await?;
    if existing {
        return Ok(ChatDisposition::Ok);
    }
    let owned:i64=sqlx::query_scalar("SELECT count(*) FROM game_private_chat_members WHERE world_id=encode($1,'hex')::uuid AND channel_id=$2 AND game_session_id<>encode($3,'hex')::uuid")
        .bind(f.runtime_scope.world_id().as_bytes().as_slice()).bind(channel).bind(f.game_session_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    let invited:i64=sqlx::query_scalar("SELECT count(*) FROM game_private_chat_members m JOIN game_private_chat_channels c ON c.world_id=m.world_id AND c.channel_id=m.channel_id WHERE m.world_id=encode($1,'hex')::uuid AND m.game_session_id=encode($2,'hex')::uuid AND c.owner_session_id<>m.game_session_id")
        .bind(f.runtime_scope.world_id().as_bytes().as_slice()).bind(target.0.as_slice()).fetch_one(&mut **tx).await?;
    if owned >= MAX_PRIVATE_CHANNEL_INVITES as i64 || invited >= MAX_INVITED_PRIVATE_CHANNELS as i64
    {
        return Ok(ChatDisposition::Rejected);
    }
    sqlx::query("INSERT INTO game_private_chat_members(world_id,channel_id,game_session_id) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid)")
        .bind(f.runtime_scope.world_id().as_bytes().as_slice()).bind(channel).bind(target.0.as_slice()).execute(&mut **tx).await?;
    Ok(ChatDisposition::Ok)
}

async fn join_or_leave(
    tx: &mut Transaction<'_, Postgres>,
    f: &CurrentCharacterItemFence,
    channel: u16,
    leave: bool,
) -> Result<ChatDisposition> {
    if leave && own(tx, f).await? == Some(i32::from(channel)) {
        sqlx::query("DELETE FROM game_private_chat_channels WHERE world_id=encode($1,'hex')::uuid AND channel_id=$2")
            .bind(f.runtime_scope.world_id().as_bytes().as_slice()).bind(i32::from(channel)).execute(&mut **tx).await?;
        return Ok(ChatDisposition::Ok);
    }
    let changed=sqlx::query("UPDATE game_private_chat_members SET joined=$4,joined_at_micros=CASE WHEN $4 AND NOT joined THEN floor(extract(epoch FROM clock_timestamp())*1000000)::bigint ELSE joined_at_micros END WHERE world_id=encode($1,'hex')::uuid AND channel_id=$2 AND game_session_id=encode($3,'hex')::uuid")
        .bind(f.runtime_scope.world_id().as_bytes().as_slice()).bind(i32::from(channel)).bind(f.game_session_id.as_bytes().as_slice()).bind(!leave).execute(&mut **tx).await?.rows_affected();
    Ok(if changed == 1 || leave {
        ChatDisposition::Ok
    } else {
        ChatDisposition::NotInvited
    })
}

async fn say(
    tx: &mut Transaction<'_, Postgres>,
    f: &CurrentCharacterItemFence,
    channel: u16,
    prepared: Option<&Publication>,
    sealed: Option<&(String, String)>,
) -> Result<ChatDisposition> {
    let joined:Option<bool>=sqlx::query_scalar("SELECT m.joined FROM game_private_chat_members m JOIN game_private_chat_channels c ON c.world_id=m.world_id AND c.channel_id=m.channel_id WHERE m.world_id=encode($1,'hex')::uuid AND m.channel_id=$2 AND m.game_session_id=encode($3,'hex')::uuid")
        .bind(f.runtime_scope.world_id().as_bytes().as_slice()).bind(i32::from(channel)).bind(f.game_session_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    match joined {
        None => return Ok(ChatDisposition::NotInvited),
        Some(false) => return Ok(ChatDisposition::RoomNotOpen),
        Some(true) => {}
    }
    let Some(current) = reads::publication(tx, f, channel).await? else {
        return Ok(ChatDisposition::NotInvited);
    };
    let Some(prepared) = prepared.filter(|p| {
        p.channel_id == current.channel_id
            && p.epoch == current.epoch
            && p.sender_name == current.sender_name
    }) else {
        return Ok(ChatDisposition::NotInvited);
    };
    let _ = prepared;
    let Some((notify_channel, ciphertext)) = sealed else {
        return Ok(ChatDisposition::ChatUnavailable);
    };
    let expected = format!(
        "oteryn_chat_{}",
        f.runtime_scope
            .world_id()
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    if *notify_channel != expected {
        return Ok(ChatDisposition::Rejected);
    }
    // Same logging prerequisite as the existing relay (0122/D1400). Pin before binding a
    // sealed line; lack of permission aborts the transaction and cannot publish or receipt it.
    sqlx::raw_sql("SET LOCAL log_parameter_max_length=0; SET LOCAL log_parameter_max_length_on_error=0; SET LOCAL log_statement='none'; SET LOCAL log_min_duration_statement=-1")
        .execute(&mut **tx).await?;
    let sent=sqlx::query("SELECT pg_notify($1,$2) WHERE current_setting('log_parameter_max_length')='0' AND current_setting('log_parameter_max_length_on_error')='0' AND current_setting('log_statement')='none' AND current_setting('log_min_duration_statement')='-1'")
        .bind(notify_channel).bind(ciphertext).fetch_optional(&mut **tx).await?;
    if sent.is_none() {
        return Err(DurabilityError::Unavailable);
    }
    Ok(ChatDisposition::Ok)
}
