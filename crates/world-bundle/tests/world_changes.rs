use oteryn_world_bundle::world_changes::Package;

#[test]
fn world_change_package_binds_exact_recipes_and_layers() {
    let package = Package::new("[]".into(), "{\"layers\":[]}".into(), "{}".into());
    assert!(package.validate().is_ok());
    let mut substituted = package.clone();
    substituted.recipes.push(' ');
    assert!(substituted.validate().is_err());
    let mut substituted = package;
    substituted.layers.push(' ');
    assert!(substituted.validate().is_err());
}
