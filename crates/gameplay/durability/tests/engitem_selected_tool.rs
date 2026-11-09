use oteryn_gameplay_durability::durability::quest_world_callback::{
    QuestWorldContext, QuestWorldToolContext,
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};

#[test]
fn selected_tool_envelope_preserves_legacy_context_and_rejects_wrong_ingress()
-> Result<(), Box<dyn std::error::Error>> {
    let context = QuestWorldContext {
        map_revision: "fixture-map".into(),
        content_revision: "fixture-content".into(),
        placement_context: vec![1; 16],
        reset_epoch: 0,
        actor_placement: [2; 16],
        movement_revision: 1,
    };
    assert_eq!(
        serde_json::to_value(&context)?,
        serde_json::to_value(QuestWorldToolContext::from(context.clone()))?
    );
    let mut use_ = UseIntent {
        target: UseTarget::ItemByDefinition(std::num::NonZeroU32::new(1).ok_or("definition")?),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement: vec![3; 40],
            expected_revision: 1,
        })),
        use_with_position: None,
    };
    let bytes = encode_item_use_intent(&use_).map_err(|e| format!("{e:?}"))?;
    let selected = context
        .clone()
        .with_selected_tool([4; 16], bytes.clone())
        .ok_or("checked selection")?;
    assert_eq!(
        serde_json::to_value(selected)?["selected_tool"]["source"],
        serde_json::to_value([4; 16])?
    );
    assert!(context.clone().with_selected_tool([0; 16], bytes).is_none());
    assert!(
        context
            .clone()
            .with_selected_tool([4; 16], vec![])
            .is_none()
    );
    use_.use_with_map_item = Some(MapUseTarget::Item(
        std::num::NonZeroU64::new(1).ok_or("handle")?,
    ));
    assert!(
        context
            .with_selected_tool(
                [4; 16],
                encode_item_use_intent(&use_).map_err(|e| format!("{e:?}"))?
            )
            .is_none(),
        "corpse sampling retains its separate owner"
    );
    Ok(())
}
