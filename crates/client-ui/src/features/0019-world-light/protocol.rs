use oteryn_client_runtime::world_light::WorldClock;
pub fn apply(state: &mut Option<WorldClock>, clock: WorldClock) {
    *state = Some(clock);
}
