//! Explicit account/world Friends consent, separate from character VIP consent.
use super::*;
pub(super) async fn replay_account(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    command: u64,
    request: &Value,
) -> Result<Option<Value>> {
    let row=sqlx::query("SELECT uuid_send(character_id) AS actor,request,result FROM game_account_friend_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.to_string()).fetch_optional(&mut **tx).await?;
    row.map(|row| {
        Ok(
            if row.try_get::<Vec<u8>, _>("actor")? == fence.character_id.as_bytes().as_slice()
                && row.try_get::<Value, _>("request")? == *request
            {
                row.try_get("result")?
            } else {
                rejected()
            },
        )
    })
    .transpose()
}
pub(super) async fn execute(
    tx: &mut Transaction<'_, Postgres>,
    write: &VipWrite,
    unavailable: bool,
) -> Result<Value> {
    if write.request["valid"] != true {
        return Ok(rejected());
    }
    let actor = write.fence.character_id.as_bytes().as_slice();
    let world = write.fence.runtime_scope.world_id();
    // Resolve current ownership from the fenced actor, never from VipWrite.account.
    let row=sqlx::query("SELECT uuid_send(account_id) AS account,name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid").bind(actor).bind(world.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    let account: Vec<u8> = row.try_get("account")?;
    let name: String = row.try_get("name")?;
    let number = write.request["number"].as_u64().unwrap_or(0);
    let text = write.request["text"].as_str().unwrap_or("").trim();
    let mut target = None;
    let mut target_name = String::new();
    if write.operation == Operation::FriendInvite {
        if let Some((peer, resolved_name)) = sqlx::query_as::<_, (Vec<u8>, String)>("SELECT uuid_send(account_id),name FROM game_character_roots WHERE lower(name)=lower($1) AND world_id=encode($2,'hex')::uuid AND lifecycle=1 LIMIT 1").bind(text).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await? {
            target = Some(peer);
            target_name = resolved_name;
        }
    } else if matches!(
        write.operation,
        Operation::FriendAccept | Operation::FriendRevoke | Operation::FriendReject
    ) {
        target=sqlx::query_scalar("SELECT CASE WHEN r.left_account=encode($3,'hex')::uuid THEN uuid_send(r.right_account) ELSE uuid_send(r.left_account) END FROM game_account_friend_handles h JOIN game_account_friends r ON r.id=h.relation_id WHERE h.game_session_id=encode($1,'hex')::uuid AND h.character_id=encode($2,'hex')::uuid AND h.handle=$4::text::bigint AND r.world_id=encode($5,'hex')::uuid AND encode($3,'hex')::uuid IN(r.left_account,r.right_account)")
            .bind(write.fence.game_session_id.as_bytes().as_slice()).bind(actor).bind(&account).bind(number.to_string()).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    }
    sqlx::query("SELECT account_id FROM game_character_account_guards WHERE account_id=encode($1,'hex')::uuid OR account_id=encode($2,'hex')::uuid ORDER BY account_id FOR UPDATE")
        .bind(&account).bind(target.as_deref()).fetch_all(&mut **tx).await?;
    if write.operation == Operation::FriendsQuery {
        return query(tx, write, &account, number).await;
    }
    let mut code = if unavailable { 9 } else { 1 };
    if code == 1 {
        sqlx::query("INSERT INTO game_account_social_profiles(account_id,world_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid) ON CONFLICT DO NOTHING").bind(&account).bind(world.as_bytes().as_slice()).execute(&mut **tx).await?;
        let tokens: i16=sqlx::query_scalar("UPDATE game_account_social_profiles SET tokens=least(20,tokens+greatest(0,(floor(extract(epoch FROM clock_timestamp()))::bigint-rate_at)/3)),rate_at=rate_at+greatest(0,(floor(extract(epoch FROM clock_timestamp()))::bigint-rate_at)/3)*3 WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid RETURNING tokens").bind(&account).bind(world.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        if tokens == 0 {
            code = 8;
        } else {
            sqlx::query("UPDATE game_account_social_profiles SET tokens=tokens-1 WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid").bind(&account).bind(world.as_bytes().as_slice()).execute(&mut **tx).await?;
            match write.operation {
                Operation::MainCharacter => {
                    sqlx::query("UPDATE game_account_social_profiles SET main_character=encode($3,'hex')::uuid WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid").bind(&account).bind(world.as_bytes().as_slice()).bind(actor).execute(&mut **tx).await?;
                }
                Operation::FriendInvite => {
                    if let Some(peer) = target.as_ref().filter(|peer| **peer != account) {
                        let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_friends WHERE world_id=encode($2,'hex')::uuid AND (left_account IN(encode($1,'hex')::uuid,encode($3,'hex')::uuid) OR right_account IN(encode($1,'hex')::uuid,encode($3,'hex')::uuid))").bind(&account).bind(world.as_bytes().as_slice()).bind(peer).fetch_one(&mut **tx).await?;
                        if count >= 200 {
                            code = 4;
                        } else {
                            let n=sqlx::query("INSERT INTO game_account_friends(world_id,left_account,right_account,requester,requester_name,target_name) VALUES(encode($1,'hex')::uuid,least(encode($2,'hex')::uuid,encode($3,'hex')::uuid),greatest(encode($2,'hex')::uuid,encode($3,'hex')::uuid),encode($2,'hex')::uuid,$4,$5) ON CONFLICT DO NOTHING").bind(world.as_bytes().as_slice()).bind(&account).bind(peer).bind(&name).bind(&target_name).execute(&mut **tx).await?.rows_affected();
                            if n == 0 {
                                code = 3;
                            }
                        }
                    } else {
                        code = 2;
                    }
                }
                Operation::FriendAccept | Operation::FriendReject | Operation::FriendRevoke => {
                    let relation:Option<(i64,bool,bool)>=sqlx::query_as("SELECT r.id,r.accepted,r.requester=encode($3,'hex')::uuid FROM game_account_friend_handles h JOIN game_account_friends r ON r.id=h.relation_id WHERE h.game_session_id=encode($1,'hex')::uuid AND h.character_id=encode($2,'hex')::uuid AND h.handle=$4::text::bigint AND r.world_id=encode($5,'hex')::uuid AND encode($3,'hex')::uuid IN(r.left_account,r.right_account) FOR UPDATE OF r")
                        .bind(write.fence.game_session_id.as_bytes().as_slice()).bind(actor).bind(&account).bind(number.to_string()).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
                    if let Some((id, accepted, sender)) = relation {
                        if write.operation != Operation::FriendRevoke && (sender || accepted) {
                            code = 10;
                        } else if write.operation == Operation::FriendAccept {
                            sqlx::query(
                                "UPDATE game_account_friends SET accepted=true WHERE id=$1",
                            )
                            .bind(id)
                            .execute(&mut **tx)
                            .await?;
                        } else {
                            sqlx::query("DELETE FROM game_account_friends WHERE id=$1")
                                .bind(id)
                                .execute(&mut **tx)
                                .await?;
                        }
                    } else {
                        code = 7;
                    }
                }
                _ => code = 10,
            }
        }
    }
    let result = json!({"code":code,"revision":0,"number":0,"wait":if code==8 {3}else{0}});
    sqlx::query("INSERT INTO game_account_friend_receipts(game_session_id,command_id,character_id,request,result) VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,$4,$5)").bind(write.fence.game_session_id.as_bytes().as_slice()).bind(write.command.to_string()).bind(actor).bind(&write.request).bind(&result).execute(&mut **tx).await?;
    Ok(result)
}
async fn query(
    tx: &mut Transaction<'_, Postgres>,
    write: &VipWrite,
    account: &[u8],
    after: u64,
) -> Result<Value> {
    let world = write.fence.runtime_scope.world_id();
    let rows=sqlx::query("SELECT id FROM game_account_friends WHERE world_id=encode($2,'hex')::uuid AND encode($1,'hex')::uuid IN(left_account,right_account) ORDER BY id LIMIT 201").bind(account).bind(world.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
    if rows.len() > 200 {
        return Ok(json!({"code":9,"revision":0,"number":0,"wait":0}));
    }
    sqlx::query("INSERT INTO game_account_friend_handle_counters(game_session_id) VALUES(encode($1,'hex')::uuid) ON CONFLICT DO NOTHING").bind(write.fence.game_session_id.as_bytes().as_slice()).execute(&mut **tx).await?;
    for row in rows {
        let id: i64 = row.try_get("id")?;
        let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_account_friend_handles WHERE game_session_id=encode($1,'hex')::uuid AND relation_id=$2)").bind(write.fence.game_session_id.as_bytes().as_slice()).bind(id).fetch_one(&mut **tx).await?;
        if !exists {
            let handle:i64=sqlx::query_scalar("UPDATE game_account_friend_handle_counters SET next_handle=next_handle+1 WHERE game_session_id=encode($1,'hex')::uuid RETURNING next_handle-1").bind(write.fence.game_session_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            sqlx::query("INSERT INTO game_account_friend_handles(game_session_id,character_id,handle,relation_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4)")
                .bind(write.fence.game_session_id.as_bytes().as_slice()).bind(write.fence.character_id.as_bytes().as_slice()).bind(handle).bind(id).execute(&mut **tx).await?;
        }
    }
    let main:Option<String>=sqlx::query_scalar("SELECT c.name FROM game_account_social_profiles p JOIN game_character_roots c ON c.character_id=p.main_character AND c.account_id=p.account_id AND c.world_id=p.world_id WHERE p.account_id=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid AND c.lifecycle=1").bind(account).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let rows=sqlx::query("SELECT h.handle,r.accepted,r.requester=encode($3,'hex')::uuid AS sender,CASE WHEN r.requester=encode($3,'hex')::uuid THEN r.target_name ELSE r.requester_name END AS name,CASE WHEN r.accepted THEN c.name ELSE NULL END AS main FROM game_account_friend_handles h JOIN game_account_friends r ON r.id=h.relation_id LEFT JOIN game_account_social_profiles p ON p.account_id=CASE WHEN r.left_account=encode($3,'hex')::uuid THEN r.right_account ELSE r.left_account END AND p.world_id=r.world_id LEFT JOIN game_character_roots c ON c.character_id=p.main_character AND c.account_id=p.account_id AND c.world_id=p.world_id AND c.lifecycle=1 WHERE h.game_session_id=encode($1,'hex')::uuid AND h.character_id=encode($2,'hex')::uuid AND r.world_id=encode($4,'hex')::uuid AND encode($3,'hex')::uuid IN(r.left_account,r.right_account) AND h.handle>$5::text::bigint ORDER BY h.handle LIMIT 26")
        .bind(write.fence.game_session_id.as_bytes().as_slice()).bind(write.fence.character_id.as_bytes().as_slice()).bind(account).bind(world.as_bytes().as_slice()).bind(after.to_string()).fetch_all(&mut **tx).await?;
    let next = if rows.len() == 26 {
        rows[24].try_get::<i64, _>("handle")?
    } else {
        0
    };
    let mut friends = Vec::new();
    for row in rows.into_iter().take(25) {
        friends.push(json!({"handle":row.try_get::<i64,_>("handle")?,"name":row.try_get::<String,_>("name")?,"main_character":row.try_get::<Option<String>,_>("main")?,"state":if row.try_get::<bool,_>("accepted")? {"friend"}else if row.try_get::<bool,_>("sender")? {"outgoing"}else{"incoming"}}));
    }
    Ok(
        json!({"code":1,"revision":0,"number":0,"wait":0,"view":{"account_friends":friends,"main_character":main,"next_friend":next}}),
    )
}
