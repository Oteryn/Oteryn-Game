//! MAIL E09: a carried letter moved onto a mailbox posts it into the recipient's Inbox once,
//! through the production item-move owner and the command 124 composition.
use super::*;
use crate::gameplay_transport::item_move::{
    LoopMove, LoopMoveRefusal, LoopMoveResult, LoopTarget,
};
use oteryn_protocol_oteryn::item_view::WorldTilePosition;
use oteryn_protocol_oteryn::mail::{MailIntent, MailOutcome, encode_intent};

/// The disclosed fixture text of the letter; production reads it as the posting owner does.
/// Content does not yet admit a letter into inventory (i3505 `materializable: false`), so the
/// fixture re-keys the carried single coin as the mail authority cases do.
async fn set_text(pool: &sqlx::PgPool, item: [u8; 16], text: &str) -> TestResult {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_item_instances SET definition_production_key='oteryn:item.tibia.i3505' WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).execute(&mut *tx).await?;
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
) -> TestResult {
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        owner.offered_capabilities(),
        &[4, 6, 18, 37, 62, 115, 153],
    )
    .ok_or("mail capability")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let fence = admitted.item_fence.ok_or("fence")?;
    let name: String =
        sqlx::query_scalar("SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice())
            .fetch_one(pool)
            .await?;
    let command = |command_id| UseCommand {
        game_session_id: admitted.game_session_id,
        command_id,
        item_fence: Some(fence),
    };
    let to = |x| LoopMove {
        item: ItemKey::Instance(letter),
        count: None,
        target: LoopTarget::Ground(WorldTilePosition { x, y: 0, floor: -7 }),
    };
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
    Ok(())
}
