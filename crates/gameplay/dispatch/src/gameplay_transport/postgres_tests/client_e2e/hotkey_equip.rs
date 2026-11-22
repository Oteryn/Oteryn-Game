use super::super::fixture::{Harness, debug, id};
use super::TestResult;
use super::support::{self, Case, count};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::{self as protocol, hotkey_equip as wire};

#[test]
fn client_e2e_hotkey_equip() -> TestResult {
    support::scenario(Case::HotkeyEquip)
}

pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let key = "oteryn:item.tibia.i3264";
    super::super::item_forge_fixtures::support::seed_items(h, &[(90, key, 1, 0)]).await?;
    let definition = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|index| index.definition_ref("Item", key, "definition-r1"))
        .ok_or("sword definition")?;
    // PlayLink::hotkey_equip_request -> hotkey_equip::command forwards these four
    // bytes. The session result codec is hotkey_equip::decode_reply.
    let payload = wire::Request { definition }.encode().to_vec();
    let commands = [
        (1, wire::COMMAND_TYPE_HOTKEY_EQUIP, payload.clone()),
        (1, wire::COMMAND_TYPE_HOTKEY_EQUIP, payload),
    ];
    let frames = support::events(owner, *admitted, &commands).await?;
    let mut replies = 0;
    let mut equipped = false;
    for frame in frames {
        let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
        match envelope.message_type() {
            protocol::MessageType::CommandResult => {
                let result = protocol::decode_command_result(envelope.payload()).map_err(debug)?;
                assert!(matches!(
                    result.status,
                    protocol::CommandStatus::Accepted | protocol::CommandStatus::DuplicateReplay
                ));
                assert_eq!(
                    wire::decode_reply(result.payload).map_err(debug)?,
                    protocol::item_view::ItemMoveOutcome::Moved
                );
                replies += 1;
            }
            protocol::MessageType::StateDelta => {
                let delta = protocol::decode_state_delta(envelope.payload()).map_err(debug)?;
                if delta.domain_id == protocol::item_view::STATE_DOMAIN_CHARACTER_INVENTORY {
                    let inventory =
                        protocol::item_view::decode_character_inventory_with_equip_drop(
                            delta.payload,
                            true,
                        )
                        .map_err(debug)?;
                    equipped |= inventory
                        .equipment
                        .iter()
                        .any(|entry| entry.item.item_definition_ref == definition);
                }
            }
            _ => {}
        }
    }
    assert_eq!(replies, 2);
    assert!(equipped, "client inventory event equips the sword");
    assert_eq!(
        count(h, "SELECT count(*) FROM game_character_equipment_slots").await?,
        1
    );
    assert_eq!(count(h,"SELECT count(*) FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i3264'").await?,0);
    assert_eq!(
        count(h, "SELECT count(*) FROM game_character_equipment_receipts").await?,
        1
    );
    Ok(())
}

#[test]
fn open_pvp_knight_hotkey_equips_and_publishes_tier_zero_axe_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    support::scenario_with_admin(Case::HotkeyEquipOpenPvp, admin)
}

pub(super) async fn exercise_open_pvp(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult {
    use crate::durability::character_build::{
        BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, convert_vocation,
    };
    use protocol::item_view::{self, ItemMoveOutcome};

    let axe = "oteryn:item.tibia.i3276";
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native gameplay")?;
    let definition = native
        .item_index()
        .and_then(|index| index.definition_ref("Item", axe, "definition-r1"))
        .ok_or("axe definition")?;
    let formula = native.training_formula().ok_or("training formula")?;
    let before = DurableBuildState::default();
    let knight = convert_vocation(formula, &before, "knight").map_err(debug)?;
    owner
        .root
        .commit_character_build(
            owner.character,
            owner.holder,
            super::super::death_ml_loss_composed_fixtures::current_fence(
                owner,
                admitted.game_session_id,
            )
            .await?,
            BuildChangeRequest {
                occurrence: BuildOccurrence::from_bytes(id(89)).map_err(debug)?,
                cause: BuildCause::VocationChoice,
                before,
                after: knight,
                pruned_stance: None,
            },
            formula,
        )
        .await
        .map_err(debug)?;
    sqlx::query("INSERT INTO game_control_field_policy_grants VALUES(session_user,encode($1,'hex')::uuid,'policy-1')")
        .bind(id(42).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_spell_field_world_policies(world_id,world_policy_revision,world_type,protection_level,in_fight_ms,source_pin,control_role,decision_identity) VALUES(encode($1,'hex')::uuid,'policy-1',2,8,60000,'99902524e052f37574194466c2949c576e4ab269',session_user,'pvp-live-fixture')")
        .bind(id(42).as_slice()).execute(&h.pool).await?;
    super::super::item_forge_fixtures::support::seed_items(h, &[(90, axe, 1, 0)]).await?;

    let payload = wire::TieredRequest {
        request: wire::Request { definition },
        upgrade_tier: Some(0),
    }
    .encode()
    .ok_or("tier-zero axe payload")?;
    let frames = support::events(
        owner,
        admitted,
        &[(1, wire::COMMAND_TYPE_HOTKEY_EQUIP, payload)],
    )
    .await?;
    let mut moved = false;
    let mut published = false;
    for frame in frames {
        let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
        match envelope.message_type() {
            protocol::MessageType::CommandResult => {
                let result = protocol::decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(result.command_id, 1);
                assert_eq!(result.status, protocol::CommandStatus::Accepted);
                assert_eq!(
                    wire::decode_reply(result.payload).map_err(debug)?,
                    ItemMoveOutcome::Moved
                );
                moved = true;
            }
            protocol::MessageType::StateDelta => {
                let delta = protocol::decode_state_delta(envelope.payload()).map_err(debug)?;
                if delta.domain_id == item_view::STATE_DOMAIN_CHARACTER_INVENTORY {
                    let inventory =
                        item_view::decode_character_inventory_with_equip_drop(delta.payload, true)
                            .map_err(debug)?;
                    published |= inventory
                        .equipment
                        .iter()
                        .any(|entry| entry.item.item_definition_ref == definition)
                        && inventory
                            .entries
                            .iter()
                            .all(|entry| entry.item_definition_ref != definition);
                }
            }
            _ => {}
        }
    }
    assert!(moved, "command 348 returned Moved");
    assert!(
        published,
        "the inventory delta transferred the axe to equipment"
    );
    Ok(())
}
