//! Dronk's three pump placements in Crystal 201189f8 data-global/world/world.otbm.

pub const COAL: &str = "oteryn:item.tibia.i12600";
pub const PUMP_RECIPE: &str = "oteryn:world-change.awash.coal";

const PUMPS: [(&str, u16, u16, i8); 3] = [
    ("oteryn:item.tibia.i12666", 32558, 32007, -9),
    ("oteryn:item.tibia.i12667", 32560, 32014, -9),
    ("oteryn:item.tibia.i12666", 32567, 32024, -9),
];

pub fn is_pump_item(key: &str) -> bool {
    PUMPS.iter().any(|pump| pump.0 == key)
}

pub fn is_dronk_pump(key: &str, x: u16, y: u16, floor: i8) -> bool {
    PUMPS.contains(&(key, x, y, floor))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn awash_admits_only_dronks_three_exact_pumps() {
        let common = "oteryn:item.tibia.i12666";
        let second = "oteryn:item.tibia.i12667";
        assert!(is_dronk_pump(common, 32558, 32007, -9));
        assert!(is_dronk_pump(second, 32560, 32014, -9));
        assert!(is_dronk_pump(common, 32567, 32024, -9));
        assert!(!is_dronk_pump(common, 32560, 32014, -9));
        assert!(!is_dronk_pump(common, 32558, 32007, -8));
        assert!(!is_dronk_pump(common, 32558, 32008, -9));
        assert!(!is_dronk_pump(COAL, 32558, 32007, -9));
    }
}
