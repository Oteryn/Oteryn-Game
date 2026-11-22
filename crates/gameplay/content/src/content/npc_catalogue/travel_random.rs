//! Closed destination alternatives for authored travel routes.
use super::TravelRoute;

#[derive(Clone, Copy)]
pub struct RandomDestination {
    pub faces: u8,
    pub alternate: (i32, i32, i16),
}

impl TravelRoute {
    pub fn random_destination(&self) -> Option<RandomDestination> {
        // Crystal 201189f8 data-global/npc/{captain_fearless,captain_pelagia,jack_fate}.lua.
        let (faces, alternate) = match (self.service.as_str(), self.route.key.as_str()) {
            (
                "oteryn:service.travel.captain_fearless" | "oteryn:service.travel.captain_pelagia",
                "darashia",
            ) => (10, (33324, 32173, 6)),
            ("oteryn:service.travel.jack_fate", "thais") => (8, (32161, 32558, 6)),
            _ => return None,
        };
        Some(RandomDestination { faces, alternate })
    }
}
