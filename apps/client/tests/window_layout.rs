use oteryn_client::window_layout::WindowLayout;

#[test]
fn o033_window_geometry_survives_restart_and_rejects_bad_bounds() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!("cg-gamewin-layout-{}.json", std::process::id()));
    let mut layout = WindowLayout::load(&path)?;
    layout.record("Spell catalogue", [40, 80, 680, 760])?;
    assert!(layout.record("bad", [0, 0, 0, 760]).is_err());
    layout.save(&path)?;
    let restored = WindowLayout::load(&path)?;
    assert_eq!(restored.bounds("Spell catalogue"), Some([40, 80, 680, 760]));
    assert_eq!(restored.bounds("bad"), None);
    std::fs::remove_file(path)?;
    Ok(())
}
