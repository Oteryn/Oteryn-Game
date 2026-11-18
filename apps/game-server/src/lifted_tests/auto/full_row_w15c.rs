#[test]
fn port_hope_walking_legs_and_replay_are_reachable() -> Result<(), Box<dyn std::error::Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            crate::qualification::client_world::full_row_w15c::port_hope_route_smoke()
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Port Hope route smoke thread failed")??;
    Ok(())
}
