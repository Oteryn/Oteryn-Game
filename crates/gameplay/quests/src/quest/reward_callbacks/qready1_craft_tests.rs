use super::*;

#[test]
fn cursed_first_mixture_binds_both_directions_as_paid_recipes() -> Result<(), String> {
    let callbacks = load()?;
    let recipes: Vec<_> = callbacks
        .iter()
        .filter(|callback| {
            callback
                .key
                .starts_with("oteryn:quest-reward/qready1/cursed-crystal-first-")
        })
        .collect();
    assert_eq!(recipes.len(), 2);
    for callback in recipes {
        assert!(callback.is_paid_exchange());
        assert!(callback.storage.is_empty());
        assert_eq!(callback.take.len(), 2);
        let source = callback.carried_item.as_deref().ok_or("source")?;
        let target = callback.carried_target.as_deref().ok_or("target")?;
        assert_ne!(source, target);
        assert_eq!(callback.take[0].item, source);
        assert_eq!(callback.take[1].item, target);
        assert!(callback.take.iter().all(|input| input.count == 1));
        let output = callback.give.as_ref().ok_or("output")?;
        assert_eq!(output.item, "oteryn:item.tibia.i21505");
        assert_eq!(output.count, 1);
        let plan = durable_plan(callback)?;
        assert_eq!(plan["recipe_source"], source);
        assert_eq!(plan["carried_target"], target);
        assert!(plan.get("carried_item").is_none());
    }
    Ok(())
}
