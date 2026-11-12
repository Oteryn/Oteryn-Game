//! Gem Atelier v2 (WU26-09): the live query's ordered Gem Vessel slots and Replace Vessel
//! through the composed owner and the fenced durable Wheel writer.
use super::*;
use crate::gameplay_transport::capabilities::OfferedCapability;
use oteryn_protocol_oteryn::atelier::v2 as atelier;

async fn send_v2(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    command_type: u32,
    body: &[u8],
) -> TestResult<atelier::AtelierReply> {
    let (status, payload) = send_kind(owner, admitted, command_type, body).await?;
    assert_eq!(status, CommandStatus::Accepted);
    atelier::decode_reply(&payload).map_err(debug)
}

fn replace(
    occurrence: u8,
    expected_wheel_revision: u64,
    slot: u8,
    gem: Option<[u8; 16]>,
) -> TestResult<Vec<u8>> {
    atelier::encode_replace_vessel(&atelier::ReplaceVesselRequest {
        occurrence: id(occurrence),
        expected_wheel_revision,
        slot,
        gem,
    })
    .map_err(debug)
}

async fn character_revision(h: &Harness, character: &[u8]) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character)
    .fetch_one(&h.pool)
    .await?)
}

/// Starts with the Regular initial gem socketed by the legacy route at Wheel revision 1.
pub(super) async fn replace_vessel(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    socketed: [u8; 16],
) -> TestResult {
    let legacy = admitted.continuity.selected_capabilities;
    assert_eq!(
        send_kind(owner, admitted, atelier::COMMAND_TYPE_ATELIER_V2_QUERY, &[])
            .await?
            .0,
        CommandStatus::Rejected,
        "a capability 148 client cannot use the v2 commands"
    );
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        &[
            OfferedCapability {
                id: wire::CAPABILITY_WHEEL_ALLOCATION_V1,
                requires: &[],
            },
            OfferedCapability {
                id: atelier::CAPABILITY_GEM_ATELIER_V2,
                requires: &[wire::CAPABILITY_WHEEL_ALLOCATION_V1],
            },
        ],
        &[
            wire::CAPABILITY_WHEEL_ALLOCATION_V1,
            atelier::CAPABILITY_GEM_ATELIER_V2,
        ],
    )
    .ok_or("Atelier v2 selection")?;
    let character = admitted
        .item_fence
        .ok_or("item fence")?
        .character_id
        .as_bytes()
        .to_vec();

    let view = send_v2(owner, admitted, atelier::COMMAND_TYPE_ATELIER_V2_QUERY, &[]).await?;
    assert!(!view.refused);
    assert_eq!(view.wheel_revision, 1);
    assert!(view.premium);
    assert_eq!(view.vessels.len(), 12);
    assert_eq!(view.vessel_nodes.len(), 12);
    assert_eq!(view.vessel_node_bonus, 1);
    assert_eq!(view.vessels[1].gem, Some(socketed), "first Regular slot");
    assert!(view.vessels[1].open);
    assert_eq!(view.vessels[1].quality, 1);
    assert!(!view.vessels[10].open, "a partial Wheel cannot open the eleventh slot");
    let socketed_view = view.gems.iter().find(|gem| gem.id == socketed);
    assert_eq!(socketed_view.and_then(|gem| gem.slot), Some(1));
    assert_eq!(view.gems.iter().filter(|gem| gem.slot.is_some()).count(), 1);
    assert!(!view.quality_quotes.is_empty(), "a query carries current prices");

    // Rule refusals: a slot of another quality, a closed slot and a stale Wheel revision.
    let before = character_revision(h, &character).await?;
    for (occurrence, expected, slot) in [(180, 1, 0), (181, 1, 10), (182, 0, 4)] {
        super::next_fresh_wheel_action(owner)?;
        let refused = send_v2(
            owner,
            admitted,
            atelier::COMMAND_TYPE_ATELIER_V2_REPLACE_VESSEL,
            &replace(occurrence, expected, slot, Some(socketed))?,
        )
        .await?;
        assert!(refused.refused, "slot {slot} expected revision {expected}");
        assert_eq!(refused.wheel_revision, 1);
        assert_eq!(refused.vessels[1].gem, Some(socketed));
    }
    assert_eq!(character_revision(h, &character).await?, before);

    // Emptying the slot returns the gem; the same occurrence replays without another change.
    super::next_fresh_wheel_action(owner)?;
    let empty = replace(183, 1, 1, None)?;
    let emptied = send_v2(
        owner,
        admitted,
        atelier::COMMAND_TYPE_ATELIER_V2_REPLACE_VESSEL,
        &empty,
    )
    .await?;
    assert!(!emptied.refused);
    assert_eq!(emptied.wheel_revision, 2);
    assert!(emptied.vessels.iter().all(|slot| slot.gem.is_none()));
    assert!(emptied.gems.iter().all(|gem| gem.slot.is_none()));
    let after = character_revision(h, &character).await?;
    super::next_fresh_wheel_action(owner)?;
    assert_eq!(
        send_v2(
            owner,
            admitted,
            atelier::COMMAND_TYPE_ATELIER_V2_REPLACE_VESSEL,
            &empty
        )
        .await?,
        emptied
    );
    assert_eq!(character_revision(h, &character).await?, after);
    super::next_fresh_wheel_action(owner)?;
    assert_eq!(
        send_kind(
            owner,
            admitted,
            atelier::COMMAND_TYPE_ATELIER_V2_REPLACE_VESSEL,
            &replace(183, 1, 1, Some(socketed))?,
        )
        .await?
        .0,
        CommandStatus::Rejected,
        "one receipt identity cannot be reused for another slot change"
    );

    // Putting it back is one more durable Wheel change with the twelve-slot receipt vector.
    super::next_fresh_wheel_action(owner)?;
    let back = send_v2(
        owner,
        admitted,
        atelier::COMMAND_TYPE_ATELIER_V2_REPLACE_VESSEL,
        &replace(184, 2, 1, Some(socketed))?,
    )
    .await?;
    assert!(!back.refused);
    assert_eq!(back.wheel_revision, 3);
    assert_eq!(back.vessels[1].gem, Some(socketed));
    let mut expected = vec![serde_json::Value::Null; 12];
    expected[1] = serde_json::to_value(socketed)?;
    let (recorded, legacy_cells): (serde_json::Value, serde_json::Value) = sqlx::query_as(
        "SELECT gem_vessels_after, vessels_after FROM game_character_wheel_receipts
          WHERE character_id=encode($1,'hex')::uuid ORDER BY after_wheel_revision DESC LIMIT 1",
    )
    .bind(character.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(recorded, serde_json::Value::Array(expected));
    assert_eq!(legacy_cells, serde_json::json!([null, null, null, null]));
    let stored: serde_json::Value = sqlx::query_scalar(
        "SELECT vessels FROM game_character_wheel_vessels WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(stored, recorded);

    // Switch Domain does not exist on the v2 wire.
    let switch = oteryn_protocol_oteryn::atelier::encode_mutate(
        &oteryn_protocol_oteryn::atelier::MutateRequest {
            occurrence: id(185),
            expected_revision: view.revision,
            mutation: oteryn_protocol_oteryn::atelier::Mutation::SwitchDomain { gem: socketed },
        },
    )
    .map_err(debug)?;
    assert_eq!(
        send_kind(
            owner,
            admitted,
            atelier::COMMAND_TYPE_ATELIER_V2_MUTATE,
            &switch
        )
        .await?
        .0,
        CommandStatus::Rejected
    );
    admitted.continuity.selected_capabilities = legacy;
    Ok(())
}
