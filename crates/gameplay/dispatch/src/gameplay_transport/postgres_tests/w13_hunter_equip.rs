use super::full_world_support::*;
use crate::gameplay_transport::connection::{AdmittedSession, IDLE_LIVENESS, serve_admitted};
use oteryn_protocol_oteryn::{self as wire, item_view};
use tokio::io::AsyncWriteExt as _;

// Global Relic Sword (Tibia Fandom archive): one hand, level 50, attack 42.
const RELIC_SWORD: &str = "oteryn:item.tibia.i7383";
const DEMONRAGE_SWORD: &str = "oteryn:item.tibia.i7382";
const SHIELD: &str = "oteryn:item.tibia.i3425";

#[test]
fn w13_knight_equips_one_handed_fixture_sword_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("w13-hunter-equip", scenario)
}

fn scenario() -> TestResult {
    world_scenario!("w13_hunter_equip", 4, crate::spell::Vocation::Knight, 200, |h, request, seal, authority, world_id, native, world, cells, room, pin, deps, actor, session, owner, fence, admitted| {
        let _ = (request, world_id, world, room, pin, actor, session, fence);
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        for (ordinal, (seed, key)) in [(90, RELIC_SWORD), (91, SHIELD), (92, DEMONRAGE_SWORD)].into_iter().enumerate() {
            let placed = sqlx::query("WITH bag AS (SELECT * FROM game_item_container_slots WHERE character_id=encode($1,'hex')::uuid), minted AS (INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT encode($2,'hex')::uuid,world_id,'Item',$3,'definition-r1',1,1,encode($2,'hex')::uuid FROM bag RETURNING item_instance_id) INSERT INTO game_item_container_entries(character_id,world_id,parent_item_instance_id,item_instance_id,placement_ordinal,placed_transaction_id) SELECT bag.character_id,bag.world_id,bag.item_instance_id,minted.item_instance_id,$4,minted.item_instance_id FROM bag CROSS JOIN minted")
                .bind(id(41).as_slice()).bind(id(seed).as_slice()).bind(key).bind(i64::try_from(ordinal)? + 20).execute(&mut *tx).await?;
            assert_eq!(placed.rows_affected(), 1);
        }
        let build = sqlx::query("INSERT INTO game_character_build_state(character_id,vocation,magic_level,mana_spent,fist_level,fist_tries,club_level,club_tries,sword_level,sword_tries,axe_level,axe_tries,distance_level,distance_tries,shielding_level,shielding_tries,fishing_level,fishing_tries,committed_character_revision,last_build_occurrence_id) SELECT character_id,'knight',0,0,10,0,10,0,110,0,10,0,10,0,10,0,10,0,character_revision,encode($2,'hex')::uuid FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id) DO UPDATE SET vocation='knight',sword_level=110")
            .bind(id(41).as_slice()).bind(id(30).as_slice()).execute(&mut *tx).await?;
        assert_eq!(build.rows_affected(), 1);
        let progression = sqlx::query("UPDATE game_character_progression_state SET level=200,total_experience=129389800 WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(41).as_slice()).execute(&mut *tx).await?;
        assert_eq!(progression.rows_affected(), 1);
        let shield = sqlx::query("WITH carried AS (DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid RETURNING character_id,world_id,item_instance_id,placed_transaction_id) INSERT INTO game_character_equipment_slots(character_id,world_id,item_instance_id,placed_transaction_id,slot) SELECT character_id,world_id,item_instance_id,placed_transaction_id,6 FROM carried")
            .bind(id(91).as_slice()).execute(&mut *tx).await?;
        assert_eq!(shield.rows_affected(), 1);
        tx.commit().await?;

        let mut caps = [
            wire::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES,
            item_view::CAPABILITY_ITEM_VIEW_MOVE_V1,
            item_view::CAPABILITY_ITEM_EQUIP_DROP_V1,
        ];
        caps.sort_unstable();
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &caps)
                .ok_or("W13 equipment selection")?;
        assert!(admitted.continuity.selected_capabilities.contains(item_view::CAPABILITY_ITEM_EQUIP_DROP_V1));
        let index = native.item_index().ok_or("active item index")?;
        let relic = index.definition_ref("Item", RELIC_SWORD, "definition-r1").ok_or("Relic Sword definition")?;
        let demonrage = index.definition_ref("Item", DEMONRAGE_SWORD, "definition-r1").ok_or("Demonrage Sword definition")?;
        exercise(&owner, admitted, [demonrage, relic]).await?;

        let worn: Vec<(i16, String)> = sqlx::query_as("SELECT s.slot::smallint,i.definition_production_key FROM game_character_equipment_slots s JOIN game_item_instances i USING (item_instance_id) WHERE s.character_id=encode($1,'hex')::uuid ORDER BY s.slot")
            .bind(id(41).as_slice()).fetch_all(&h.pool).await?;
        assert_eq!(worn, [(5, RELIC_SWORD.into()), (6, SHIELD.into())]);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    definitions: [std::num::NonZeroU32; 2],
) -> TestResult {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serve = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let drive = async {
        let mut snapshot = Vec::new();
        loop {
            let frame = super::item_look_fixtures::read_frame(&mut client).await?;
            let envelope = wire::decode_wire_envelope(&frame).map_err(debug)?;
            match envelope.message_type() {
                wire::MessageType::SnapshotChunk => {
                    let (_, bytes) = wire::decode_snapshot_chunk_framing(envelope.payload()).map_err(debug)?;
                    snapshot.extend_from_slice(bytes);
                }
                wire::MessageType::SnapshotCommit => break,
                _ => {}
            }
        }
        let domains = wire::decode_snapshot_body(&snapshot).map_err(debug)?;
        let domain = domains.iter().find(|domain| domain.domain_id == item_view::STATE_DOMAIN_CHARACTER_INVENTORY).ok_or("initial inventory domain")?;
        let inventory = item_view::decode_character_inventory_with_equip_drop(domain.payload, true).map_err(debug)?;
        for (offset, (definition, expected)) in definitions.into_iter().zip([
            item_view::ItemMoveOutcome::SlotMismatch,
            item_view::ItemMoveOutcome::Moved,
        ]).enumerate() {
            let source = inventory.entries.iter().find(|item| item.item_definition_ref == definition).ok_or("sword in backpack")?.handle;
            let payload = item_view::encode_item_move_intent_for(
                &item_view::ItemMoveIntent { source, destination: item_view::ItemMoveDestination::Equipment(item_view::EquipmentSlot::RightHand) },
                item_view::ItemMoveSelection { equip_drop: true, ..Default::default() },
            ).map_err(debug)?;
            let id = u64::try_from(offset)? + 1;
            let command = wire::encode_client_command(admitted.continuity.connection_generation, &wire::ClientCommandValue {
                command_id: id, command_type: item_view::COMMAND_TYPE_ITEM_MOVE_INTENT, payload: &payload,
            }).map_err(debug)?;
            crate::gameplay_transport::tcp_tls::write_frame(&mut client, &command).await?;
            loop {
                let frame = super::item_look_fixtures::read_frame(&mut client).await?;
                let envelope = wire::decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == wire::MessageType::CommandResult {
                    let reply = wire::decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(reply.command_id, id);
                    assert_eq!(reply.status, wire::CommandStatus::Accepted);
                    assert_eq!(item_view::decode_item_move_result_with_equip_drop(reply.payload, true).map_err(debug)?, expected);
                    break;
                }
            }
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (_, result) = tokio::time::timeout(
        Duration::from_secs(30),
        super::fixture::join_two(serve, drive),
    ).await?;
    result
}
