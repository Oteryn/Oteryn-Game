//! Framed House USE followed by replay through a new command directory.
use super::*;
use oteryn_protocol_oteryn::world_object::{MapUseTarget, WorldObjectTarget};
use serde::Deserialize;

#[derive(Deserialize)]
struct Pair {
    item_id: u32,
    target: u32,
    kind: Option<String>,
}

#[derive(Deserialize)]
struct BedKit {
    item_id: u32,
    south: [u32; 2],
}

#[derive(Deserialize)]
struct Rules {
    bed_kits: Vec<BedKit>,
    carpets: Vec<Pair>,
    native_transforms: Vec<Pair>,
}

fn rules() -> TestResult<Rules> {
    Ok(serde_json::from_str(include_str!(
        "../../../../../../../tools/content-schema/native-gameplay/house-decoration-actions.json"
    ))?)
}

pub(super) async fn use_and_replay(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    replay_owner: ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    case: Case,
) -> TestResult {
    let rules = rules()?;
    let before = item(h).await?;
    let before_custody = custody(h).await?;
    let item_key = |item_id| format!("oteryn:item.tibia.i{item_id}");
    let index = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .ok_or("decoration Item index")?;
    let definition = |item_id| {
        index
            .definition_ref("Item", &item_key(item_id), "definition-r1")
            .ok_or("decoration Item definition")
    };
    let (source, after, family, target, bed) = match case {
        Case::Decoration(source, expected) => {
            let (rule, family) = if let Some(rule) =
                rules.carpets.iter().find(|rule| rule.item_id == source)
            {
                (rule, "carpet")
            } else {
                (
                    rules
                        .native_transforms
                        .iter()
                        .find(|rule| rule.item_id == source && rule.kind.as_deref() == Some("USE"))
                        .ok_or("native furniture USE rule")?,
                    "native_transform",
                )
            };
            assert_eq!(rule.target, expected);
            (source, Some(rule.target), family, None, None)
        }
        Case::BedKit(source) => {
            let kit = rules
                .bed_kits
                .iter()
                .find(|kit| kit.item_id == source)
                .ok_or("bed modification kit")?;
            let [x, y, floor] = POSITION;
            let bundle = owner
                .bundle_map()
                .ok_or("House bundle")?
                .overlay
                .base()
                .digest();
            let placement = crate::map::view::placement_key(
                crate::map::overlay::TilePos {
                    x: u16::try_from(x)?,
                    y: u16::try_from(y)?,
                    floor: i8::try_from(-floor)?,
                },
                1,
            )
            .ok_or("bed placement")?;
            // The south-facing source bed fixture has its foot one tile south.
            let partner: Vec<u8> = [x, y + 1, floor]
                .into_iter()
                .flat_map(i32::to_be_bytes)
                .collect();
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO game_house_tiles VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT DO NOTHING")
                .bind(admitted.world_id.as_bytes().as_slice())
                .bind(HOUSE)
                .bind(partner)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
            let target = MapUseTarget::Object(WorldObjectTarget {
                placement: bundle.into_iter().chain(placement.to_be_bytes()).collect(),
                expected_revision: 0,
            });
            (source, None, "bed_kit", Some(target), Some(kit.south))
        }
        _ => return Err("expected framed decoration or bed kit case".into()),
    };
    // The wire fixture resolves this target from the actual snapshot before sending.
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: target,
        use_with_position: None,
    })
    .map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let (mut admitted, outcome, original, deltas) = exchange(
        owner,
        admitted,
        command,
        &payload,
        HouseExchange {
            current: Some(definition(source)?),
            after: after.map(definition).transpose()?,
            nested: None,
            foreign_after_snapshot: None,
        },
    )
    .await?;
    assert_eq!(outcome, UseDisposition::Committed, "{family} {source}");
    if after.is_some() {
        assert_eq!(deltas, 1, "clients see the transformed furniture");
    }
    let stored = item(h).await?;
    let stored_custody = custody(h).await?;
    assert_eq!(
        stored["minted_transaction_id"],
        before["minted_transaction_id"]
    );
    assert_eq!(stored["state_revision"], 2);
    if let Some(after) = after {
        assert_eq!(stored["definition_production_key"], item_key(after));
        assert_eq!(stored["quantity"], before["quantity"]);
        assert_eq!(stored_custody, before_custody);
    }
    if bed.is_some() {
        assert_eq!(stored["quantity"], 0);
        assert_eq!(stored["lifecycle"], 2);
        assert!(
            stored_custody
                .as_object()
                .ok_or("kit custody")?
                .values()
                .all(Value::is_null)
        );
    }
    let bed_rows = || {
        sqlx::query_as::<_, (String, String, String)>(
            "SELECT d.definition_key, d.revision::text, r.parent_item_transaction::text
             FROM game_house_base_decorations d
             JOIN game_house_base_decoration_receipts r ON r.transaction_id=d.last_transaction_id
             WHERE d.world_id=encode($1,'hex')::uuid AND d.house_key=$2
             ORDER BY d.placement_key LIMIT 3",
        )
        .bind(admitted.world_id.as_bytes().to_vec())
        .bind(HOUSE)
        .fetch_all(&h.pool)
    };
    let receipt = || {
        sqlx::query_scalar::<_, Value>(
            "SELECT to_jsonb(r) FROM game_item_consume_transform_records r
             WHERE source_item_instance_id=encode($1,'hex')::uuid
               AND game_session_id=encode($2,'hex')::uuid AND command_id=$3::text::numeric",
        )
        .bind(id(71).to_vec())
        .bind(admitted.game_session_id.as_bytes().to_vec())
        .bind(command.to_string())
        .fetch_one(&h.pool)
    };
    let stored_receipt = receipt().await?;
    assert_eq!(stored_receipt["effects"]["decoration_family"], family);
    assert_eq!(stored_receipt["effects"]["source_item_id"], source);
    let transaction = stored_receipt["transaction_id"]
        .as_str()
        .ok_or("decoration receipt transaction")?;
    assert_eq!(stored["last_transaction_id"], transaction);
    let expected_bed = bed.map_or_else(Vec::new, |pair| {
        pair.map(|part| (item_key(part), "1".to_owned(), transaction.to_owned()))
            .to_vec()
    });
    assert_eq!(bed_rows().await?, expected_bed);
    assert_eq!(receipts(h).await?, 1);

    // Replay the exact old payload through a fresh per-owner command directory.
    // A bed kit's handle is now gone and the base target's revision changed.
    admitted.continuity.next_command_id = command;
    let (_, outcome, replay, deltas) = exchange(
        &replay_owner,
        admitted,
        command,
        &original,
        HouseExchange {
            current: None,
            after: None,
            nested: None,
            foreign_after_snapshot: None,
        },
    )
    .await?;
    assert_eq!(outcome, UseDisposition::Committed);
    assert_eq!(replay, original);
    assert_eq!(deltas, 0, "replay publishes no second mutation");
    assert_eq!(item(h).await?, stored);
    assert_eq!(custody(h).await?, stored_custody);
    assert_eq!(bed_rows().await?, expected_bed);
    assert_eq!(receipts(h).await?, 1);
    assert_eq!(receipt().await?, stored_receipt);
    eprintln!("W18 POSTGRES_CASE_COMPLETED framed_{family}_durable_replay source={source}");
    Ok(())
}
