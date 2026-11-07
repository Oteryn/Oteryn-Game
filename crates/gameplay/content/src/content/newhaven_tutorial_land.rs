//! Exact two Newhaven land components in runtime coordinates (Tibia z, 0..=15).
//! Generated from committed B3 ground classes.
//! Regenerate with tools/content-schema/world-authoring/newhaven_tutorial_land.py.
const LAND: &[u8; 1903] = include_bytes!("newhaven_tutorial_land.bin");

pub fn contains(x: i32, y: i32, floor: i16) -> bool {
    if floor != 7 || !(32506..=32634).contains(&x) || !(32405..=32522).contains(&y) {
        return false;
    }
    let index = ((y - 32405) * 129 + x - 32506) as usize;
    LAND[index / 8] & (1 << (index % 8)) != 0
}

#[cfg(test)]
#[path = "newhaven_tutorial_land_tests.rs"]
mod tests;
