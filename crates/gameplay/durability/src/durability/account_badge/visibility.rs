use super::*;
use oteryn_protocol_oteryn::cyclopedia_character::badge_visibility as wire;

impl DurabilityRoot {
    pub async fn account_badge_visibility(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: u64,
        request: wire::Request,
    ) -> Result<wire::Reply, CharacterAuthorityError> {
        let recovery = authority.record_for(self)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let bytes=request.encode();
            if matches!(request, wire::Request::Set { .. }) {
                if command == 0 { return Ok(Err(CharacterAuthorityError::Rejected)); }
                let mut key=fence.game_session_id.as_bytes().to_vec();
                key.extend(command.to_be_bytes());
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:badge-visibility:' || encode($1,'hex'),0))")
                    .bind(key).execute(&mut *tx).await?;
                if let Some(row)=sqlx::query("SELECT uuid_send(character_id) AS actor,request,result FROM game_account_badge_visibility_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                    .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.to_string()).fetch_optional(&mut *tx).await? {
                    if row.try_get::<Vec<u8>,_>("actor")? != fence.character_id.as_bytes().as_slice() || row.try_get::<Vec<u8>,_>("request")? != bytes { return Ok(Err(CharacterAuthorityError::Rejected)); }
                    let reply=wire::Reply::decode(&row.try_get::<Vec<u8>,_>("result")?).map_err(|_| super::super::DurabilityError::InvalidStoredState)?;
                    commit_semantic_transaction(tx,deadline).await?;
                    return Ok(Ok(reply));
                }
            }
            super::super::db::lock_admission_relations(&mut tx).await?;
            if super::super::character_progression::assert_session_fence(&mut tx,&fence,&node).await?.is_err() { return Ok(Err(CharacterAuthorityError::Rejected)); }
            let target=match request {
                wire::Request::Query(Some(bytes)) => match CharacterId::from_bytes(bytes) { Ok(id)=>id,Err(_)=>return Ok(Err(CharacterAuthorityError::Rejected)) },
                _=>fence.character_id,
            };
            // Match the accepted Cyclopedia character disclosure rule, including target consent.
            let allowed: bool=sqlx::query_scalar("SELECT $1=$2 OR EXISTS(SELECT 1 FROM game_vip_consents c JOIN game_character_roots v ON v.account_id=c.account_id AND v.world_id=c.world_id WHERE v.character_id=encode($1,'hex')::uuid AND c.character_id=encode($2,'hex')::uuid AND c.consent=2)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(target.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            if !allowed { return Ok(Err(CharacterAuthorityError::Rejected)); }
            if let wire::Request::Set { badge, public }=request {
                sqlx::query("SELECT 1 FROM game_character_account_guards WHERE account_id=(SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid) FOR UPDATE")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                let earned: bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_account_badges WHERE account_id=(SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid) AND badge=$2)")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(i16::from(badge.id())).fetch_one(&mut *tx).await?;
                if !earned { return Ok(Err(CharacterAuthorityError::Rejected)); }
                sqlx::query("INSERT INTO game_account_badge_visibility(account_id,badge,is_public) SELECT account_id,$2,$3 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(account_id,badge) DO UPDATE SET is_public=EXCLUDED.is_public")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(i16::from(badge.id())).bind(public).execute(&mut *tx).await?;
            }
            let own=target==fence.character_id;
            let rows=sqlx::query("SELECT b.badge,COALESCE(v.is_public,true) AS public FROM game_account_badges b LEFT JOIN game_account_badge_visibility v USING(account_id,badge) WHERE b.account_id=(SELECT account_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid) ORDER BY b.badge LIMIT 21")
                .bind(target.as_bytes().as_slice()).fetch_all(&mut *tx).await?;
            let mut reply=wire::Reply { earned:Vec::new(),hidden:Vec::new() };
            for row in rows {
                let badge=match decode_badge(row.try_get("badge")?) { Ok(badge)=>badge,Err(error)=>return Ok(Err(error)) };
                let public: bool=row.try_get("public")?;
                if own || public { reply.earned.push(badge); }
                if own && !public { reply.hidden.push(badge); }
            }
            let encoded=reply.encode().map_err(|_|super::super::DurabilityError::InvalidStoredState)?;
            if matches!(request,wire::Request::Set { .. }) {
                sqlx::query("INSERT INTO game_account_badge_visibility_receipts(game_session_id,command_id,character_id,request,result) VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,$4,$5)")
                    .bind(fence.game_session_id.as_bytes().as_slice()).bind(command.to_string()).bind(fence.character_id.as_bytes().as_slice()).bind(bytes).bind(encoded).execute(&mut *tx).await?;
            }
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(reply))
        })).await?
    }
}
