use super::*;

pub(super) async fn publication(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    channel: u16,
) -> Result<Option<Publication>> {
    let row = sqlx::query("SELECT c.epoch,r.name FROM game_private_chat_channels c JOIN game_private_chat_members m ON m.world_id=c.world_id AND m.channel_id=c.channel_id JOIN game_private_chat_participants p ON p.game_session_id=c.owner_session_id AND p.enabled JOIN game_durability_reconnect_sessions s ON s.game_session_id=c.owner_session_id AND s.session_state IN(1,2) JOIN game_character_roots r ON r.character_id=encode($3,'hex')::uuid WHERE c.world_id=encode($1,'hex')::uuid AND c.channel_id=$2 AND m.game_session_id=encode($4,'hex')::uuid AND m.joined")
        .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(i32::from(channel))
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
        .fetch_optional(&mut **tx).await?;
    row.map(|row| {
        Ok(Publication {
            channel_id: channel,
            epoch: u64::try_from(row.try_get::<i64, _>("epoch")?)
                .map_err(|_| DurabilityError::Unavailable)?,
            sender_name: CharacterName::parse(&row.try_get::<String, _>("name")?)
                .map_err(|_| DurabilityError::Unavailable)?,
        })
    })
    .transpose()
}

impl DurabilityRoot {
    /// Register capability 80 for this exact connection. Disabling it closes the owner's
    /// channel and withdraws every invitation/membership in the same transaction.
    pub async fn private_channel_admit(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        enabled: bool,
    ) -> Result<bool> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline| Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            if current(&mut tx,&recovery,&node,&fence,false).await?.is_none() { return Ok(false); }
            lock_world(&mut tx,&fence).await?;
            sqlx::query("INSERT INTO game_private_chat_participants(game_session_id,world_id,character_id,connection_generation,enabled) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,$5) ON CONFLICT(game_session_id) DO UPDATE SET connection_generation=EXCLUDED.connection_generation,enabled=EXCLUDED.enabled WHERE game_private_chat_participants.world_id=EXCLUDED.world_id AND game_private_chat_participants.character_id=EXCLUDED.character_id")
                .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.connection_generation.get().to_string()).bind(enabled).execute(&mut *tx).await?;
            if !enabled {
                sqlx::query("DELETE FROM game_private_chat_channels WHERE world_id=encode($1,'hex')::uuid AND owner_session_id=encode($2,'hex')::uuid")
                    .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).execute(&mut *tx).await?;
                sqlx::query("DELETE FROM game_private_chat_members WHERE game_session_id=encode($1,'hex')::uuid")
                    .bind(fence.game_session_id.as_bytes().as_slice()).execute(&mut *tx).await?;
            }
            commit_semantic_transaction(tx,deadline).await?;
            Ok(true)
        })).await
    }

    /// At most 16 invitations plus one owned channel. Public room eligibility belongs to
    /// the public-channel owner, which fills `public_rooms` after this metadata read.
    pub async fn read_private_channel_list(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
    ) -> Result<Option<ChannelView>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline| Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            if current(&mut tx,&recovery,&node,&fence,true).await?.is_none() { return Ok(None); }
            lock_world(&mut tx,&fence).await?;
            let rows=sqlx::query("SELECT c.channel_id,c.epoch,m.joined,m.joined_at_micros,c.owner_session_id=encode($2,'hex')::uuid AS own,r.name FROM game_private_chat_members m JOIN game_private_chat_channels c ON c.world_id=m.world_id AND c.channel_id=m.channel_id JOIN game_private_chat_participants p ON p.game_session_id=c.owner_session_id AND p.enabled JOIN game_durability_reconnect_sessions s ON s.game_session_id=c.owner_session_id AND s.session_state IN(1,2) JOIN game_character_roots r ON r.character_id=p.character_id WHERE m.world_id=encode($1,'hex')::uuid AND m.game_session_id=encode($2,'hex')::uuid ORDER BY c.channel_id LIMIT 17")
                .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).fetch_all(&mut *tx).await?;
            let mut list=ChatChannelList { public_rooms:ChatRoomSet::default(),own_channel:None,invited:Vec::new() };
            let mut joined=Vec::new();
            for row in rows {
                let id=u16::try_from(row.try_get::<i32,_>("channel_id")?).map_err(|_|DurabilityError::Unavailable)?;
                if row.try_get::<bool,_>("joined")? {
                    joined.push(JoinedChannel {channel_id:id,
                        epoch:u64::try_from(row.try_get::<i64,_>("epoch")?).map_err(|_|DurabilityError::Unavailable)?,
                        since_micros:u64::try_from(row.try_get::<i64,_>("joined_at_micros")?).map_err(|_|DurabilityError::Unavailable)?,
                    });
                }
                if row.try_get::<bool,_>("own")? { list.own_channel=Some(id); }
                else { list.invited.push(InvitedChannel { channel_id:id,owner_name:row.try_get("name")? }); }
            }
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Some(ChannelView {list,joined}))
        })).await
    }

    /// Retain membership, exclusion and session locks through synchronous queue custody.
    /// Joining/rejoining later than a line's timestamp never grants historical delivery.
    pub async fn with_private_channel_custody<T: Send, F>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        destination: (u16, u64, u64),
        accept: F,
    ) -> Result<T>
    where
        F: FnOnce(bool) -> T + Send,
    {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let mut callback = Some(accept);
        self.try_issue_semantic_pass()?.run_with_context(&mut callback,move |holder,deadline,callback| Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            let mut allowed=current(&mut tx,&recovery,&node,&fence,true).await?.is_some();
            if allowed {
                lock_world(&mut tx,&fence).await?;
                allowed=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_private_chat_members m JOIN game_private_chat_channels c ON c.world_id=m.world_id AND c.channel_id=m.channel_id JOIN game_private_chat_participants p ON p.game_session_id=c.owner_session_id AND p.enabled JOIN game_durability_reconnect_sessions s ON s.game_session_id=c.owner_session_id AND s.session_state IN(1,2) WHERE m.world_id=encode($1,'hex')::uuid AND m.channel_id=$2 AND c.epoch=$3 AND m.game_session_id=encode($4,'hex')::uuid AND m.joined AND m.joined_at_micros<=$5)")
                    .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(i32::from(destination.0))
                    .bind(i64::try_from(destination.1).map_err(|_|DurabilityError::Unavailable)?)
                    .bind(fence.game_session_id.as_bytes().as_slice()).bind(i64::try_from(destination.2).map_err(|_|DurabilityError::Unavailable)?)
                    .fetch_one(&mut *tx).await?;
            }
            let callback=callback.take().ok_or(DurabilityError::Unavailable)?;
            let result=callback(allowed);
            commit_semantic_transaction(tx,deadline).await?;
            Ok(result)
        })).await
    }
}
