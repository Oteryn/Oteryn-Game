use super::*;
fn cache() -> Result<PresentationCache, BatchError> {
    let mut cache = PresentationCache {
        palette: vec![[255, 0, 0], [0, 255, 0]],
        ..Default::default()
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    cache.begin(World::builtin()?.atlas(), Some(root));
    Ok(cache)
}
fn look(colour: u16, addons: u8) -> oteryn_session::Look {
    oteryn_session::Look::Outfit {
        look_type: 128,
        colours: [colour; 4],
        addons,
    }
}
fn pixels(cache: &PresentationCache, draws: &[Draw]) -> Vec<u8> {
    draws
        .iter()
        .flat_map(|draw| {
            let start = usize::from(draw.cell) * 32 * 32 * 4;
            &cache.pixels[start..start + 32 * 32 * 4]
        })
        .copied()
        .collect()
}
#[test]
fn outfit_colours_change_pixels_and_cache_identity() -> Result<(), BatchError> {
    let mut cache = cache()?;
    let red = cache.outfit(Some(&look(0, 0)), 2);
    let green = cache.outfit(Some(&look(1, 0)), 2);
    assert_ne!(red, green);
    assert_ne!(pixels(&cache, &red), pixels(&cache, &green));
    assert_eq!(red, cache.outfit(Some(&look(0, 0)), 2));
    for channel in 0..4 {
        let mut colours = [0; 4];
        colours[channel] = 1;
        let changed = cache.outfit(
            Some(&oteryn_session::Look::Outfit {
                look_type: 128,
                colours,
                addons: 0,
            }),
            2,
        );
        assert_ne!(
            pixels(&cache, &red),
            pixels(&cache, &changed),
            "colour channel {channel}"
        );
    }
    Ok(())
}
#[test]
fn outfit_addon_bits_render_distinct_additional_layers() -> Result<(), BatchError> {
    let mut cache = cache()?;
    let base = cache.outfit(Some(&look(0, 0)), 2);
    let first = cache.outfit(Some(&look(0, 1)), 2);
    let second = cache.outfit(Some(&look(0, 2)), 2);
    let both = cache.outfit(Some(&look(0, 3)), 2);
    assert!(first.len() > base.len());
    assert!(second.len() > base.len());
    assert_eq!(both.len(), first.len() + second.len() - base.len());
    assert_ne!(pixels(&cache, &first), pixels(&cache, &second));
    Ok(())
}

#[test]
fn outfit_animation_caches_each_phase_without_repacking_repeated_frames() -> Result<(), BatchError>
{
    let mut cache = cache()?;
    let look = look(0, 0);
    let first = cache.outfit_at(Some(&look), 2, true, 0);
    let mut changed = false;
    for elapsed in (50..2000).step_by(50) {
        let frame = cache.outfit_at(Some(&look), 2, true, elapsed);
        if pixels(&cache, &frame) != pixels(&cache, &first) {
            changed = true;
        }
        let count = cache.count();
        assert_eq!(frame, cache.outfit_at(Some(&look), 2, true, elapsed));
        assert_eq!(count, cache.count());
    }
    assert!(changed);
    Ok(())
}
