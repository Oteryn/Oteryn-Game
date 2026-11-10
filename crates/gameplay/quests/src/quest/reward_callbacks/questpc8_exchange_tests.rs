//! The Colours opening needs QENGINE2's explicit, selected-instance exchange.
use super::*;

#[test]
fn colours_opening_requires_explicit_carried_exchange() -> Result<(), String> {
    let source = load()?
        .into_iter()
        .find(|callback| callback.carried_item.is_some())
        .ok_or("existing carried tool")?;
    let mut row = serde_json::to_value(source).map_err(|error| error.to_string())?;
    row["carried_exchange"] = true.into();
    row["outfit"] = serde_json::Value::Null;
    row["outfits"] = serde_json::json!([]);
    row["give"] = serde_json::json!({
        "item":"oteryn:item.tibia.i12208", "count":1,
        "contents":[], "backpack_outputs":[]
    });
    super::parse(&serde_json::json!({"schema":1,"callbacks":[row]}).to_string())?;
    Ok(())
}

#[test]
fn colours_blue_opening_preserves_letter_remainder_and_stage_guards() -> Result<(), String> {
    let callbacks = load()?;
    for (name, input, output, before, after) in [
        ("envelope", 12204, 12208, 0, 1),
        ("mark-blue", 12208, 12228, 1, 2),
    ] {
        let key = format!("oteryn:quest-reward/chosen/colours/{name}");
        let callback = callbacks.iter().find(|c| c.key == key).ok_or(key)?;
        assert!(callback.carried_exchange);
        assert_eq!(
            callback.carried_item,
            Some(format!("oteryn:item.tibia.i{input}"))
        );
        assert_eq!(
            callback.take,
            [ItemAmount {
                fluid_subtype: None,
                item: format!("oteryn:item.tibia.i{input}"),
                count: 1
            }]
        );
        let grant = callback.give.as_ref().ok_or("native delivery")?;
        assert_eq!(grant.item, format!("oteryn:item.tibia.i{output}"));
        assert_eq!(grant.count, 1);
        assert_eq!(callback.storage[0].minimum, before);
        assert_eq!(callback.storage[0].maximum, before);
        assert_eq!(callback.storage[0].set, after);
        assert_eq!(
            grant.backpack_outputs.len(),
            usize::from(name == "envelope")
        );
        if name == "envelope" {
            assert_eq!(
                grant.backpack_outputs,
                [ItemAmount {
                    fluid_subtype: None,
                    item: "oteryn:item.tibia.i12205".into(),
                    count: 1
                }]
            );
        } else {
            assert_eq!(
                callback.storage[1].track,
                "oteryn:quest-progress/chosen/colours/colour"
            );
            assert_eq!(callback.storage[1].set, 1);
        }
    }
    Ok(())
}

#[test]
fn colours_exchange_admission_matches_reused_postgresql_bounds() -> Result<(), String> {
    let source = load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-reward/chosen/colours/envelope")
        .ok_or("envelope")?;
    for many_inputs in [true, false] {
        let mut invalid = source.clone();
        if many_inputs {
            invalid.take.extend((5901..5909).map(|item| ItemAmount {
                fluid_subtype: None,
                item: format!("oteryn:item.tibia.i{item}"),
                count: 1,
            }));
        } else {
            invalid.take.push(ItemAmount {
                fluid_subtype: None,
                item: "oteryn:item.tibia.i5901".into(),
                count: 101,
            });
        }
        assert!(
            super::parse(&serde_json::json!({"schema":1,"callbacks":[invalid]}).to_string())
                .is_err(),
            "carried exchange exceeds PostgreSQL bounds; many_inputs={many_inputs}"
        );
    }
    Ok(())
}
