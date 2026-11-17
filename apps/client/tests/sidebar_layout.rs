//! The same layout actions consumed by the native miniwindow callbacks survive a restart.
use oteryn_client::miniwindow::{Dock, Layout};
#[test]
fn o033_native_sidebar_actions_preserve_order_resize_and_closed_siblings()
-> Result<(), Box<dyn std::error::Error>> {
    let mut layout = Layout::default();
    layout.open("Battle", [100, 100, 240, 200]);
    layout.open("Inventory", [300, 100, 240, 300]);
    layout.dragged("Inventory", [0, 0, 260, 300], [1280, 900]);
    layout.dragged("Battle", [0, 350, 280, 220], [1280, 900]);
    let visible = vec!["Inventory".into(), "Battle".into()];
    assert_eq!(
        layout.rect("Battle", [1280, 900], &visible),
        Some([0, 304, 280, 220])
    );
    let path = std::env::temp_dir().join(format!("o033-native-{}.json", std::process::id()));
    layout.save(&path)?;
    let mut restored = Layout::load(&path)?;
    assert_eq!(
        restored.rect("Battle", [1280, 900], &["Battle".into()]),
        Some([0, 0, 280, 220])
    );
    restored.dock("Battle", Dock::Right);
    assert_eq!(
        restored.rect("Battle", [1280, 900], &visible),
        Some([1000, 0, 280, 220])
    );
    std::fs::remove_file(path)?;
    Ok(())
}
