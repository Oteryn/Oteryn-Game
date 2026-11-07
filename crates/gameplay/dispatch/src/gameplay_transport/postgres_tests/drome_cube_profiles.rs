//! Content closure is checked even without a PostgreSQL route. A catalogue key alone
//! is not a portable native Item policy for the Cube's burn/grant transaction.
use crate::content::ReferenceItemField::Known;
use crate::durability::item_loop_move::ItemLoopContent;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::WorldId;
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_gameplay_items::concoctions::ALL;
use oteryn_gameplay_items::drome_cube::DROME_CUBE_KEY;

#[test]
fn drome_cube_native_generation_profiles_cover_the_source_and_every_reward() -> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(check_profiles)
        .map_err(|error| error.to_string())?
        .join()
        .map_err(|_| "Drome native profile check panicked".to_owned())?
}

fn check_profiles() -> Result<(), String> {
    let world = WorldId::decode(&crate::bestiary_postgres_harness::id(42))
        .map_err(|error| format!("{error:?}"))?;
    let (controller, _) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let native = controller
        .active()
        .and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or("missing active native generation")?;
    let index = native.item_index().ok_or("missing native Item index")?;
    let content = NativeItemWeights(native);
    for key in std::iter::once(DROME_CUBE_KEY).chain(ALL.iter().map(|kind| kind.production_key())) {
        let revision = index
            .revision(key)
            .ok_or_else(|| format!("Drome Item index is missing {key}"))?;
        let definition = TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: revision.into(),
        };
        let facts = content
            .loop_facts(&definition)
            .ok_or_else(|| format!("Drome native policy is missing {key}@{revision}"))?;
        assert!(facts.weight.is_some(), "unknown Drome item weight: {key}");
        assert_eq!(facts.container_capacity, None, "non-leaf Drome item: {key}");
        let (maximum, _) = oteryn_gameplay_items::stacking::stack_maximum(facts.stack)
            .map_err(|error| format!("invalid Drome stack {key}: {error:?}"))?;
        assert!(maximum > 0, "zero Drome stack capacity: {key}");
        let record = native
            .item_policy(key, revision)
            .ok_or_else(|| format!("Drome presentation policy is missing {key}"))?
            .record();
        let Known(presentation) = &record.semantics.presentation else {
            return Err(format!("Drome presentation is unknown: {key}"));
        };
        let Known(name) = &presentation.name else {
            return Err(format!("Drome reward name is unknown: {key}"));
        };
        assert!(!name.is_empty(), "empty Drome name: {key}");
        assert!(
            format!("You have received a {name}!").len() <= 255,
            "Drome reward text exceeds the status channel: {key}"
        );
    }
    Ok(())
}
