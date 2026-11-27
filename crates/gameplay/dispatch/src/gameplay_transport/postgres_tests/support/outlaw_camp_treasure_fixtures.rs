use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
    encode_use_intent,
};
use std::num::NonZeroU64;

pub(super) const CHEST: Position = Position {
    x: 32620,
    y: 32198,
    floor: -10,
};
pub(super) const START: Position = Position { y: 32200, ..CHEST };
const DOOR: Position = Position { y: 32199, ..CHEST };
const CALLBACK: &str = "oteryn:quest-callback/authored-outlaw-camp-s3-treasure";
const QUEST: &str = "oteryn:quest.the_outlaw_camp_quest";
const STAGE: &str = "oteryn:quest-progress/authored/outlaw-camp/stage";
const TRACK: &str = "oteryn:quest-progress/quest/u6_4/outlaw_camp_quest";
pub(super) const TREASURE_KEY: &str = "oteryn:item.tibia.i2972";

pub(super) fn world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups_fixtures::{ExpectedTile, NativeCrop, native_crop};
    native_crop(
        native,
        NativeCrop {
            bytes: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../content/world/placements/region-z10-x127-y125.b3"
            )),
            region: (10, 127, 125),
            bounds: (32618, 32197, 32622, 32201),
            start: (START.x, START.y),
            expected: vec![
                ExpectedTile {
                    position: (CHEST.x, CHEST.y),
                    ids: vec![408, 2317, 2472],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (DOOR.x, DOOR.y),
                    ids: vec![408, 6251],
                    counted_entry: None,
                },
            ],
        },
    )
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    h: &Harness,
) -> TestResult {
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("object and inventory capabilities")?;
    super::classic_quest_parity_fixtures::set_test_level(h, 45, 1_328_800).await?;
    // This crop starts just after the authored entrance, where the keys
    // have qualified the player for the guarded passage.
    super::qengine_outfit_stories_fixtures::seed_quest_prerequisites(
        h, owner, &admitted, &[(STAGE.into(), 1)], 45,
    ).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let character = id(41);
    let catalogue = owner.quest_catalogue.as_ref().ok_or("Outlaw Camp catalogue")?;
    let pin = catalogue.definition_hash(QUEST).ok_or("Outlaw Camp definition pin")?;
    assert_eq!(sqlx::query(
        "INSERT INTO game_character_quest_states
         (character_id, quest_key, pinned_content_revision, definition_hash,
          started_character_revision, committed_character_revision)
         SELECT character_id,$2,$3,$4,character_revision,character_revision
         FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    ).bind(character.as_slice()).bind(QUEST).bind(catalogue.content_revision()).bind(pin.as_slice())
        .execute(&mut *tx).await?.rows_affected(), 1);
    tx.commit().await?;
    seed_treasure_key(h).await?;
    owner.refresh_quest_session(admitted.game_session_id, false).await;

    let map = owner.bundle_map().ok_or("treasure room")?;
    let target = |at: Position, ordinal| -> TestResult<WorldObjectTarget> {
        let key = oteryn_world_bundle::bundle::placement_key(at.floor, at.x, at.y, ordinal)
            .ok_or("placement key")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        Ok(WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        })
    };
    let tool = owner
        .active_generation
        .as_ref()
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", TREASURE_KEY, "definition-r1"))
        .ok_or("native key definition")?;
    let use_key = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(target(DOOR, 1)?)),
        use_with_position: None,
    })
    .map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let (mut admitted, unlocked, _) =
        super::item_craft_fixtures::exchange(owner, admitted, command, &use_key, Some(tool))
            .await?;
    assert_eq!(unlocked, UseDisposition::Committed);
    super::questpc7_ferumbras_fixtures::walk(owner, &mut admitted, START).await?;
    super::questpc7_ferumbras_fixtures::walk(owner, &mut admitted, DOOR).await?;

    let stage: i64 = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks WHERE track_key=$1"
    ).bind(STAGE).fetch_one(&h.pool).await?;
    assert_eq!(stage, 2, "crossing the keyed door qualifies the chest");
    let use_chest = encode_use_intent(&target(CHEST, 2)?).map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let (admitted, obtained, replay) =
        super::item_craft_fixtures::exchange(owner, admitted, command, &use_chest, None).await?;
    assert_eq!(obtained, UseDisposition::Committed);
    let (admitted, obtained, _) =
        super::item_craft_fixtures::exchange(owner, admitted, command, &replay, None).await?;
    assert_eq!(obtained, UseDisposition::Committed);
    let next = admitted.continuity.next_command_id;
    let (_, duplicate, _) =
        super::item_craft_fixtures::exchange(owner, admitted, next, &use_chest, None).await?;
    assert_eq!(duplicate, UseDisposition::Rejected);

    let children: Vec<(String, i64)> = sqlx::query_as(
        "SELECT child.definition_production_key, child.quantity FROM game_item_nested_entries e
         JOIN game_item_instances child ON child.item_instance_id=e.item_instance_id
         JOIN game_item_instances bag ON bag.item_instance_id=e.parent_item_instance_id
         WHERE bag.definition_production_key='oteryn:item.tibia.i2853' ORDER BY child.definition_production_key",
    ).fetch_all(&h.pool).await?;
    assert_eq!(
        children,
        [
            ("oteryn:item.tibia.i3039".to_owned(), 1),
            ("oteryn:item.tibia.i3295".to_owned(), 1)
        ]
    );
    let atomic: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_reward_claim_mint_receipts m JOIN game_character_quest_receipts q
         ON q.character_id=m.character_id AND q.cause_id=m.game_session_id
         AND q.cause_ordinal=m.command_id AND q.created_xact_id=m.created_xact_id
         JOIN game_character_quest_tracks t ON t.character_id=q.character_id
         WHERE m.claim_production_key=$1 AND q.transition_key=$1 AND q.completes
         AND t.track_key=$2 AND t.value=1
         AND EXISTS (SELECT 1 FROM game_character_quest_tracks stage
                     WHERE stage.character_id=t.character_id AND stage.track_key=$3 AND stage.value=3)",
    ).bind(CALLBACK).bind(TRACK).bind(STAGE).fetch_one(&h.pool).await?;
    assert_eq!(
        atomic, 1,
        "one atomic bag mint, completion and donor storage update"
    );
    Ok(())
}

pub(super) async fn seed_treasure_key(h: &Harness) -> TestResult {
    let key_id = id(71);
    let character = id(41);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let inserted = sqlx::query(
        "INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,
         definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
         SELECT encode($1,'hex')::uuid,world_id,'Item',$2,'definition-r1',1,1,game_character_uuid_v7()
         FROM game_item_container_slots WHERE character_id=encode($3,'hex')::uuid",
    )
    .bind(key_id.as_slice())
    .bind(TREASURE_KEY)
    .bind(character.as_slice())
    .execute(&mut *tx)
    .await?;
    assert_eq!(inserted.rows_affected(), 1);
    sqlx::query(
        "INSERT INTO game_item_container_entries(character_id,world_id,parent_item_instance_id,
         item_instance_id,placement_ordinal,placed_transaction_id)
         SELECT s.character_id,s.world_id,s.item_instance_id,encode($1,'hex')::uuid,
                coalesce((SELECT max(e.placement_ordinal) FROM game_item_container_entries e
                          WHERE e.parent_item_instance_id=s.item_instance_id),0)+1,
                game_character_uuid_v7()
         FROM game_item_container_slots s WHERE s.character_id=encode($2,'hex')::uuid",
    )
    .bind(key_id.as_slice())
    .bind(character.as_slice())
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_item_keys(item_instance_id,key_number,minted_transaction_id)
        SELECT item_instance_id,3304,game_character_uuid_v7() FROM game_item_instances
        WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(key_id.as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}
