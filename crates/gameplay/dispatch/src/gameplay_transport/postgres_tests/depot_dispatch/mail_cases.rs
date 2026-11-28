//! MAIL E09: a carried letter, and a parcel whose label names the recipient, moved onto a
//! mailbox post into the recipient's Inbox once, through the production item-move owner and
//! the command 124 composition. Depot search (E08) then retrieves them from the Inbox.
use super::*;
use crate::gameplay_transport::item_move::{LoopMove, LoopMoveRefusal, LoopMoveResult, LoopTarget};
use oteryn_protocol_oteryn::item_view::WorldTilePosition;
use oteryn_protocol_oteryn::mail::{MailIntent, MailOutcome, encode_intent};

/// The disclosed fixture text of a letter or label; production reads it as the posting owner
/// does. Writing text is the write-text owner's mechanic, not under test here.
async fn set_text(pool: &sqlx::PgPool, item: [u8; 16], text: &str) -> TestResult {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_text_revisions VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING")
        .bind(item.as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_texts(item_instance_id,text,written_at,text_revision) VALUES(encode($1,'hex')::uuid,$2,now(),1) ON CONFLICT(item_instance_id) DO UPDATE SET text=EXCLUDED.text")
        .bind(item.as_slice()).bind(text).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

/// The letter's definition, its Inbox rows and every MAIL receipt.
async fn state(pool: &sqlx::PgPool, item: [u8; 16]) -> TestResult<(String, i64, i64)> {
    Ok(sqlx::query_as("SELECT i.definition_production_key,(SELECT count(*) FROM game_item_character_inbox_locations WHERE item_instance_id=i.item_instance_id),(SELECT count(*) FROM game_mail_operations) FROM game_item_instances i WHERE i.item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).fetch_one(pool).await?)
}

// Tibia Fandom /wiki/Mailbox, Royal Mailboxes (locked), Orc Fortress.
pub(super) const ROYAL_MAILBOX: crate::map::overlay::TilePos = crate::map::overlay::TilePos {
    x: 32970,
    y: 31778,
    floor: -7,
};

pub(super) async fn exercise_royal(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
    pool: &sqlx::PgPool,
    letter: [u8; 16],
    mailbox: ItemKey,
) -> TestResult {
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 18, 37, 62, 115, 153])
            .ok_or("mail capability")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let fence = admitted.item_fence.ok_or("fence")?;
    let name: String = sqlx::query_scalar(
        "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(pool)
    .await?;
    set_text(pool, letter, &name).await?;
    let item = ItemKey::Instance(letter);
    let post = encode_intent(&MailIntent::Post {
        item_handle: 1,
        mailbox_handle: 2,
    })
    .map_err(debug)?;
    let send = |command_id| {
        owner.mail_command(
            actor,
            admitted,
            command_id,
            &post,
            Some(item),
            Some(mailbox),
        )
    };
    let move_letter = |command_id| {
        owner.move_loop_item(
            actor,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id,
                item_fence: Some(fence),
            },
            LoopMove {
                item,
                count: None,
                slot: None,
                target: LoopTarget::Ground(WorldTilePosition {
                    x: ROYAL_MAILBOX.x.into(),
                    y: ROYAL_MAILBOX.y.into(),
                    floor: ROYAL_MAILBOX.floor.into(),
                }),
            },
        )
    };

    // Mission progress is insufficient. Only Kevin's final promotion completes the quest.
    const POSTMAN: &str = "oteryn:quest.the_postman_missions_quest";
    let catalogue = super::super::quest_container_chests_fixtures::embedded_catalogue()?;
    let hash = catalogue
        .definition_hash(POSTMAN)
        .ok_or("Postman definition")?;
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_quest_states(character_id,quest_key,pinned_content_revision,definition_hash,started_character_revision,committed_character_revision) VALUES(encode($1,'hex')::uuid,$2,$3,$4,2,2)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(POSTMAN)
        .bind(catalogue.content_revision()).bind(hash.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    assert_eq!(
        send(500).await,
        Some(MailOutcome::Rejected),
        "unfinished Postman cannot post to the royal mailbox"
    );
    assert_eq!(
        move_letter(501).await,
        LoopMoveResult::Refused(LoopMoveRefusal::Rejected)
    );
    assert_eq!(
        state(pool, letter).await?,
        ("oteryn:item.tibia.i3505".into(), 0, 0)
    );
    let carried = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("sender backpack")?;
    assert!(
        carried
            .entries
            .iter()
            .any(|entry| entry.item.item_instance_id == letter)
    );

    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_quest_states SET completed_character_revision=3,committed_character_revision=3 WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2")
        .bind(fence.character_id.as_bytes().as_slice()).bind(POSTMAN).execute(&mut *tx).await?;
    tx.commit().await?;
    assert_eq!(move_letter(502).await, LoopMoveResult::Committed);
    assert_eq!(send(502).await, Some(MailOutcome::Posted));
    assert_eq!(
        state(pool, letter).await?,
        ("oteryn:item.tibia.i3506".into(), 1, 1)
    );
    let carried = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("sender backpack")?;
    assert!(
        carried
            .entries
            .iter()
            .all(|entry| entry.item.item_instance_id != letter)
    );
    let inbox = owner
        .root
        .read_depot_search(owner.character, fence.character_id, 3506)
        .await
        .map_err(debug)?
        .ok_or("stamped letter in Inbox search")?;
    assert_eq!(inbox.inbox, 1);
    use oteryn_protocol_oteryn::depot_search::{Outcome, Request};
    let retrieved = owner
        .depot_search_command(
            actor,
            admitted,
            503,
            Request::Retrieve {
                client_id: 3506,
                all: true,
            },
        )
        .await;
    assert_eq!(
        (retrieved.outcome, retrieved.retrieved),
        (Outcome::Retrieved, 1)
    );
    assert_eq!(
        state(pool, letter).await?,
        ("oteryn:item.tibia.i3506".into(), 0, 1)
    );
    eprintln!(
        "POSTGRES_CASE_COMPLETED royal_mailbox_requires_postman_completion_even_when_premium_expires"
    );
    Ok(())
}

pub(super) async fn exercise(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
    pool: &sqlx::PgPool,
    letter: [u8; 16],
    parcel: [u8; 16],
    label: [u8; 16],
    mode: Mode,
) -> TestResult {
    let list = mode == Mode::List;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 18, 37, 62, 115, 153])
            .ok_or("mail capability")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let fence = admitted.item_fence.ok_or("fence")?;
    // Historical initial inventory: a partially used keg. Actual parcel insertion,
    // mail posting, Inbox retrieval and replay below must preserve its whole row.
    let charge_item=(!list).then_some(crate::bestiary_postgres_harness::id(240));
    if let Some(item)=charge_item {
        let mut tx=pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i25904','definition-r1',1,1,encode($3,'hex')::uuid)")
            .bind(item.as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(crate::bestiary_postgres_harness::id(241).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT encode($1,'hex')::uuid,world_id,character_id,item_instance_id,99,encode($2,'hex')::uuid FROM game_item_container_slots WHERE character_id=encode($3,'hex')::uuid")
            .bind(item.as_slice()).bind(crate::bestiary_postgres_harness::id(242).as_slice()).bind(fence.character_id.as_bytes().as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_timed_definitions VALUES('Item','oteryn:item.tibia.i25904','definition-r1',500,NULL,false) ON CONFLICT DO NOTHING").execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_item_timed_states(item_instance_id,charges,state_revision) VALUES(encode($1,'hex')::uuid,441,1)").bind(item.as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
    }
    let frozen=if let Some(item)=charge_item {Some(sqlx::query_scalar::<_,serde_json::Value>("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).fetch_one(pool).await?)} else {None};

    let name: String = sqlx::query_scalar(
        "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_one(pool)
    .await?;
    let command = |command_id| UseCommand {
        game_session_id: admitted.game_session_id,
        command_id,
        item_fence: Some(fence),
    };
    let move_to = |item, x| LoopMove {
        item: ItemKey::Instance(item),
        count: None,
        target: LoopTarget::Ground(WorldTilePosition { x, y: 0, floor: -7 }),
        slot: None,
    };
    let to = |x| move_to(letter, x);
    let carried = (String::from("oteryn:item.tibia.i3505"), 0, 0);
    let base = |x, ordinal| -> TestResult<ItemKey> {
        Ok(ItemKey::MapBase {
            bundle_digest: [0; 32],
            placement_key: crate::map::view::placement_key(
                crate::map::overlay::TilePos { x, y: 0, floor: -7 },
                ordinal,
            )
            .ok_or("placement key")?,
            reset_epoch: 0,
        })
    };
    let post = encode_intent(&MailIntent::Post {
        item_handle: 1,
        mailbox_handle: 2,
    })
    .map_err(debug)?;
    let write_started = std::time::Instant::now();
    let write = encode_intent(&MailIntent::WriteText { item_handle: 1, text: name.clone() }).map_err(debug)?;
    for _ in 0..2 {
        assert_eq!(owner.mail_command(actor, admitted, 500, &write, Some(ItemKey::Instance(letter)), None).await,
            Some(MailOutcome::TextWritten));
    }
    eprintln!("GAP-SV2 I04 PostgreSQL production text ingress/replay PASS in {:?}", write_started.elapsed());
    // Command 124 names a mailbox: the adjacent depot locker is none, the mailbox two tiles
    // away is out of reach. Neither writes.
    for (command_id, mailbox) in [(501, base(6, 1)?), (502, base(10, 1)?)] {
        assert_eq!(
            owner
                .mail_command(actor, admitted, command_id, &post, Some(ItemKey::Instance(letter)), Some(mailbox))
                .await,
            Some(MailOutcome::Rejected)
        );
    }
    assert_eq!(state(pool, letter).await?, carried);
    // A move onto the far mailbox is out of reach; an unknown addressee keeps the letter.
    assert_eq!(
        owner.move_loop_item(actor, command(503), to(10)).await,
        LoopMoveResult::TooFar
    );
    set_text(pool, letter, "Nobody Of That Name").await?;
    assert_eq!(
        owner.move_loop_item(actor, command(504), to(8)).await,
        LoopMoveResult::Refused(LoopMoveRefusal::Rejected)
    );
    assert_eq!(state(pool, letter).await?, carried);
    // The addressed letter is stamped into the recipient's Inbox once.
    set_text(pool, letter, &format!("{name}\nThais")).await?;
    assert_eq!(
        owner.move_loop_item(actor, command(505), to(8)).await,
        LoopMoveResult::Committed
    );
    let posted = (String::from("oteryn:item.tibia.i3506"), 1, 1);
    assert_eq!(state(pool, letter).await?, posted);
    let backpack = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("backpack")?;
    assert!(backpack.entries.iter().all(|e| e.item.item_instance_id != letter));
    // A replayed move and the same command id on command 124 answer the receipt; nothing more.
    assert_eq!(
        owner.move_loop_item(actor, command(505), to(8)).await,
        LoopMoveResult::Committed
    );
    assert_eq!(
        owner
            .mail_command(actor, admitted, 505, &post, Some(ItemKey::Instance(letter)), Some(base(8, 1)?))
            .await,
        Some(MailOutcome::Posted)
    );
    assert_eq!(state(pool, letter).await?, posted);
    // A parcel carries its label inside; the label names the recipient.
    assert_eq!(
        owner
            .move_loop_item(
                actor,
                command(506),
                LoopMove {
                    item: ItemKey::Instance(label),
                    count: None,
                    target: LoopTarget::Container(ItemKey::Instance(parcel)),
                    slot: None,
                },
            )
            .await,
        LoopMoveResult::Committed
    );
    set_text(pool, label, &format!("{name}\nThais")).await?;
    assert_eq!(
        state(pool, parcel).await?,
        (String::from("oteryn:item.tibia.i3503"), 0, 1)
    );
    if let Some(item)=charge_item {
        assert_eq!(owner.move_loop_item(actor,command(508),LoopMove{item:ItemKey::Instance(item),count:None,
            target:LoopTarget::Container(ItemKey::Instance(parcel)),slot:None}).await,LoopMoveResult::Committed);
    }
    // The mail owner admits one post per sender each second.
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let parcel_command = if mode == Mode::MailPostCapacity {
        post_parcel_and_observe_capacity(owner, admitted).await?
    } else {
        assert_eq!(
            owner
                .move_loop_item(actor, command(507), move_to(parcel, 8))
                .await,
            LoopMoveResult::Committed
        );
        507
    };
    let stamped = (String::from("oteryn:item.tibia.i3504"), 1, 2);
    assert_eq!(state(pool, parcel).await?, stamped);
    assert_eq!(
        owner
            .move_loop_item(actor, command(parcel_command), move_to(parcel, 8))
            .await,
        LoopMoveResult::Committed
    );
    assert_eq!(state(pool, parcel).await?, stamped);
    let parent: String = sqlx::query_scalar("SELECT parent_item_instance_id::text FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(label.as_slice()).fetch_one(pool).await?;
    assert_eq!(parent.replace('-', ""), hex(parcel));
    let backpack = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("backpack")?;
    assert!(
        backpack
            .entries
            .iter()
            .all(|e| e.item.item_instance_id != parcel)
    );
    if list {
        super::depot_list::exercise(owner, admitted).await?;
    }
    if mode == Mode::MailCapacity {
        super::mail_capacity::exercise(
            owner,
            admitted,
            pool,
            parcel,
            label,
            charge_item.ok_or("capacity fixture keg")?,
        )
        .await?;
    } else {
        retrieve_from_inbox(owner, actor, admitted, pool, [letter,parcel,label],charge_item).await?;
    }
    if let Some(item)=charge_item {
        assert_eq!(Some(sqlx::query_scalar::<_,serde_json::Value>("SELECT to_jsonb(t) FROM game_item_timed_states t WHERE item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).fetch_one(pool).await?),frozen);
        assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_item_timed_state_writes WHERE item_instance_id=encode($1,'hex')::uuid").bind(item.as_slice()).fetch_one(pool).await?,0);
        eprintln!("SRX1_PG partially used keg: parcel/Inbox/retrieval/replay preserves 441 charges");
    }
    Ok(())
}

async fn post_parcel_and_observe_capacity(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult<u64> {
    use crate::gameplay_transport::{
        connection::serve_admitted,
        tcp_tls::{read_frame, write_frame},
    };
    use oteryn_protocol_oteryn::{
        ClientCommandValue, CommandStatus, MessageType, decode_command_result,
        decode_snapshot_chunk, decode_wire_envelope, encode_client_command, item_view,
        world_map::{self, MapDefinition, MapOrigin},
    };
    use tokio::io::AsyncWriteExt;
    let selected = &[4, 6, 18, 115, 508];
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), selected)
            .ok_or("mail capacity capabilities")?;
    let command_id = admitted.continuity.next_command_id;
    let index = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .and_then(|native| native.item_index())
        .ok_or("mail item index")?;
    let parcel = index
        .definition_ref("Item", "oteryn:item.tibia.i3503", "definition-r1")
        .ok_or("parcel definition")?;
    let mailbox = MapDefinition::Item(
        index
            .definition_ref("Item", "oteryn:item.tibia.i3501", "definition-r1")
            .ok_or("mailbox definition")?,
    );
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        serve_admitted(&mut server, admitted, owner, SCENARIO_LIVENESS).await;
        drop(server);
    };
    let driving = async {
        let mut parcel_handle = None;
        let mut mailbox_handle = None;
        let before = loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::SnapshotChunk {
                for domain in decode_snapshot_chunk(envelope.payload()).map_err(debug)?.1 {
                    if domain.domain_id == item_view::STATE_DOMAIN_CHARACTER_INVENTORY {
                        let inventory =
                            item_view::decode_character_inventory(domain.payload).map_err(debug)?;
                        parcel_handle = inventory
                            .entries
                            .iter()
                            .find(|entry| entry.item_definition_ref == parcel)
                            .map(|entry| entry.handle.get());
                    }
                    if domain.domain_id == world_map::STATE_DOMAIN_WORLD_MAP_VIEW {
                        let map =
                            world_map::decode_world_map_snapshot(domain.payload).map_err(debug)?;
                        mailbox_handle = map
                            .tiles
                            .iter()
                            .filter(|tile| tile.position.x == 8)
                            .flat_map(|tile| &tile.items)
                            .find_map(|entry| match entry.origin {
                                MapOrigin::Handle(handle) if entry.definition == mailbox => {
                                    Some(handle.get())
                                }
                                _ => None,
                            });
                    }
                }
            }
            if let Some(capacity) = super::super::player_trade_fixtures::capacity_frame(&frame, selected)? {
                break capacity;
            }
        };
        let post = encode_intent(&MailIntent::Post {
            item_handle: parcel_handle.ok_or("wire parcel handle")?,
            mailbox_handle: mailbox_handle.ok_or("wire mailbox handle")?,
        })
        .map_err(debug)?;
        let frame = encode_client_command(
            admitted.continuity.connection_generation,
            &ClientCommandValue {
                command_id,
                command_type: u32::from(oteryn_protocol_oteryn::mail::COMMAND_TYPE_MAIL),
                payload: &post,
            },
        )
        .map_err(debug)?;
        write_frame(&mut client, &frame).await?;
        let mut posted = false;
        tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                let frame = read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.command_id, command_id);
                    assert_eq!(result.status, CommandStatus::Accepted);
                    assert_eq!(result.payload, [MailOutcome::Posted as u8]);
                    posted = true;
                }
                if let Some(after) = super::super::player_trade_fixtures::capacity_frame(&frame, selected)? {
                    assert!(posted);
                    assert_eq!(after.capacity, before.capacity);
                    assert!(
                        after.free > before.free,
                        "posting the parcel frees carrying capacity"
                    );
                    return Ok::<_, Box<dyn std::error::Error>>(());
                }
            }
        })
        .await??;
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (_, result) = tokio::time::timeout(std::time::Duration::from_secs(12), async {
        tokio::join!(serving, driving)
    })
    .await?;
    result?;
    Ok(command_id)
}

fn hex(item: [u8; 16]) -> String {
    item.iter().map(|b| format!("{b:02x}")).collect()
}

/// E08: depot search retrieves the label nested in the Inbox parcel, then the parcel and the
/// letter, each Inbox root once; a replayed command changes nothing.
async fn retrieve_from_inbox(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    pool: &sqlx::PgPool,
    items: [[u8;16];3],
    charge_item:Option<[u8;16]>,
) -> TestResult {
    let [letter,parcel,label]=items;
    use oteryn_protocol_oteryn::depot_search::{Outcome, Request};
    let inbox = async || -> TestResult<i64> {
        Ok(
            sqlx::query_scalar("SELECT count(*) FROM game_item_character_inbox_locations")
                .fetch_one(pool)
                .await?,
        )
    };
    let carried = async |item: [u8; 16]| -> TestResult<bool> {
        let backpack = owner
            .root
            .read_character_backpack(
                owner.character,
                admitted.item_fence.ok_or("fence")?.character_id,
            )
            .await
            .map_err(debug)?
            .ok_or("backpack")?;
        Ok(backpack.entries.iter().any(|e| e.item.item_instance_id == item))
    };
    assert_eq!(inbox().await?, 2);
    let mut retrievals=vec![(601,3507,label,2),(602,3504,parcel,1),(603,3506,letter,0)];
    if let Some(item)=charge_item {retrievals.insert(0,(600,25904,item,2));}
    for (command_id, client_id, item, left) in retrievals {
        let request = Request::Retrieve {
            client_id,
            all: true,
        };
        let reply = owner
            .depot_search_command(actor, admitted, command_id, request)
            .await;
        assert_eq!((reply.outcome, reply.retrieved), (Outcome::Retrieved, 1));
        assert!(carried(item).await?);
        assert_eq!(inbox().await?, left);
        let replay = owner.depot_search_command(actor, admitted, command_id, request).await;
        assert_eq!(replay, reply);
        assert_eq!(inbox().await?, left);
    }
    let counter: String = sqlx::query_scalar("SELECT committed::text FROM game_character_inbox_counters WHERE character_id=encode($1,'hex')::uuid")
        .bind(admitted.item_fence.ok_or("fence")?.character_id.as_bytes().as_slice()).fetch_one(pool).await?;
    assert_eq!(counter, "0");
    let kinds: Vec<i16> = sqlx::query_scalar("SELECT l.from_kind FROM game_item_loop_move_lines l JOIN game_item_loop_move_receipts r USING(transaction_id) WHERE r.quick_loot_ordinal>0 AND l.from_kind IN(6,7) ORDER BY r.command_id")
        .fetch_all(pool).await?;
    assert_eq!(kinds, if charge_item.is_some() {vec![7,7,6,6]} else {vec![7,6,6]});
    let reply = owner
        .depot_search_command(
            actor,
            admitted,
            604,
            Request::Retrieve {
                client_id: 3504,
                all: true,
            },
        )
        .await;
    assert_eq!(reply.outcome, Outcome::NotFound);
    Ok(())
}
