{
    #[path = "full_e2ef6.rs"]
    mod full_e2ef6;
    // Observe pristine starter custody before packet journeys can earn loot or move items.
    full_e2ef6::run_starter(clients).await;
    full_e2ef7::run_packet(clients).await;
    // Keep every remaining ordinary journey before profile fixture provisioning.
    full_e2ef6::run(clients, &world, &metadata.spawns, &metadata.npcs).await;
}
