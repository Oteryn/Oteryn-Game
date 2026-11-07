#![allow(clippy::expect_used)]
use super::*;

#[test]
fn mining_reference_table_is_exact_and_consumes_only_a_shiny_stone() {
    let mut counts = [0; 4];
    for draw in 1..=100 {
        let item = refined_item(PICK, SHINY_STONE, draw);
        let index = [
            "oteryn:item.tibia.i3043",
            "oteryn:item.tibia.i3031",
            "oteryn:item.tibia.i3035",
            "oteryn:item.tibia.i3028",
        ]
        .iter()
        .position(|key| Some(*key) == item)
        .expect("Reference outcome");
        counts[index] += 1;
        assert_eq!(refined_item("sword", SHINY_STONE, draw), None);
        assert_eq!(refined_item(PICK, "ore", draw), None);
    }
    assert_eq!(counts, [1, 5, 45, 49]);
    assert_eq!(refined_item(PICK, SHINY_STONE, 0), None);
    assert_eq!(refined_item(PICK, SHINY_STONE, 101), None);
}
