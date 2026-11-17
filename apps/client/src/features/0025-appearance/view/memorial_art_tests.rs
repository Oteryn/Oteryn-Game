use super::*;
#[test]
fn memorial_golden_and_token_previews_draw_real_pinned_artwork()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_ui::appearance::{MemorialAction as Action, MemorialPanel};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut assets = PreviewAssets::load(&root)?;
    let metadata: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/cosmetics/outfits/authoring.json"),
    )?)?;
    let outfits = metadata["document"]["outfits"]
        .as_array()
        .ok_or("outfits")?;
    let mut panel = MemorialPanel::default();
    for tokens in [false, true] {
        panel.navigate(if tokens {
            Action::Tokens
        } else {
            Action::Golden
        });
        for _ in 0..if tokens { 4 } else { 3 } {
            let offer = panel.offer();
            let source = outfits
                .iter()
                .find(|outfit| outfit["key"].as_str() == Some(offer.key))
                .ok_or("source outfit")?;
            assert_eq!(source["male"]["id"].as_u64(), Some(u64::from(offer.male)));
            assert_eq!(
                source["female"]["id"].as_u64(),
                Some(u64::from(offer.female))
            );
            for _ in 0..2 {
                let image = assets.render_memorial(&panel)?;
                assert!(
                    image
                        .rgba
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .any(|pixel| *pixel != [32, 32, 32, 255])
                );
                panel.navigate(Action::Gender);
            }
            panel.navigate(Action::Next);
        }
    }
    Ok(())
}
