use super::*;

#[test]
fn cursed_final_mixture_binds_both_directions_and_preserves_progress() -> Result<(), String> {
    let callbacks = load()?;
    assert!(
        !callbacks.iter().any(|callback| {
            callback.carried_item.as_deref() == Some("oteryn:item.tibia.i11466")
                && callback.carried_target.as_deref() == Some("oteryn:item.tibia.i21504")
        }),
        "the native QN4 first-mixture owner retains its recipe"
    );
    let recipes: Vec<_> = callbacks
        .iter()
        .filter(|callback| {
            callback
                .key
                .starts_with("oteryn:quest-reward/m20/cursed-crystal-final-")
        })
        .collect();
    assert_eq!(recipes.len(), 2);
    for callback in recipes {
        // Crystal 201189f actions_MedusaOil.lua:50-61 consumes both inputs on
        // every mixture; higher mission progress remains intact below.
        assert!(callback.is_paid_exchange());
        assert_eq!(callback.storage.len(), 1);
        assert!(callback.storage[0].at_least);
        assert_eq!(callback.storage[0].set, 2);
        assert_eq!(callback.take.len(), 2);
        let source = callback.carried_item.as_deref().ok_or("source")?;
        let target = callback.carried_target.as_deref().ok_or("target")?;
        assert_ne!(source, target);
        assert_eq!(callback.take[0].item, source);
        assert_eq!(callback.take[1].item, target);
        assert!(callback.take.iter().all(|input| input.count == 1));
        let output = callback.give.as_ref().ok_or("output")?;
        assert_eq!(output.item, "oteryn:item.tibia.i21506");
        assert_eq!(output.count, 1);
        let plan = durable_plan(callback)?;
        assert_eq!(plan["recipe_source"], source);
        assert_eq!(plan["carried_target"], target);
        assert!(plan.get("carried_item").is_none());
    }
    Ok(())
}
