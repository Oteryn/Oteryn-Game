{
    #[path = "full_e2ef6.rs"]
    mod full_e2ef6;
    // Observe pristine starter custody before packet journeys can earn loot or move items.
    if groups.runs("c19a") {
        full_e2ef6::run_starter(clients).await;
    }
    full_e2ef7::run_packet(clients, &groups).await;
    // Keep every remaining ordinary journey before profile fixture provisioning.
    if groups.runs("f6") {
        full_e2ef6::run(clients, world, &metadata.spawns, &metadata.npcs).await;
    } else if groups.runs("p16") {
        full_e2ef6::run_blessing_services(clients, world, &metadata.spawns, &metadata.npcs).await;
    }
}
