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

pub(super) async fn exercise(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
    pool: &sqlx::PgPool,
    letter: [u8; 16],
    parcel: [u8; 16],
    label: [u8; 16],
    list: bool,
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
    let command = |command_id| UseCommand {
        game_session_id: admitted.game_session_id,
        command_id,
        item_fence: Some(fence),
    };
    let move_to = |item, x| LoopMove {
        item: ItemKey::Instance(item),
        count: None,
        target: LoopTarget::Ground(WorldTilePosition { x, y: 0, floor: -7 }),
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
    set_text(pool, letter, &name).await?;
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
    // The mail owner admits one post per sender each second.
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    assert_eq!(
        owner.move_loop_item(actor, command(507), move_to(parcel, 8)).await,
        LoopMoveResult::Committed
    );
    let stamped = (String::from("oteryn:item.tibia.i3504"), 1, 2);
    assert_eq!(state(pool, parcel).await?, stamped);
    assert_eq!(
        owner.move_loop_item(actor, command(507), move_to(parcel, 8)).await,
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
    retrieve_from_inbox(owner, actor, admitted, pool, letter, parcel, label).await
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
    letter: [u8; 16],
    parcel: [u8; 16],
    label: [u8; 16],
) -> TestResult {
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
    for (command_id, client_id, item, left) in [
        (601, 3507, label, 2),
        (602, 3504, parcel, 1),
        (603, 3506, letter, 0),
    ] {
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
    assert_eq!(kinds, [7, 6, 6]);
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
