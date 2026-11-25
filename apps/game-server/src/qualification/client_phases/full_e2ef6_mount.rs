{
    #[path = "full_e2ef6.rs"]
    mod full_e2ef6;
    // Observe pristine starter custody before packet journeys can earn loot or move items.
    if groups.runs("f6") {
        full_e2ef6::run_starter(clients).await;
    }
    full_e2ef7::run_packet(clients, &groups).await;
    // Keep every remaining ordinary journey before profile fixture provisioning.
    if groups.runs("f6") {
        full_e2ef6::run(clients, world, &metadata.spawns, &metadata.npcs).await;
    } else if groups.runs("p16") {
        full_e2ef6::run_blessing_services(clients, world, &metadata.spawns, &metadata.npcs).await;
    }
    if groups.explicit("thaisroutes") {
        verdict::dawnport_excluded("C11.full");
    }
    if groups.explicit("thaisroutes") || groups.explicit("p20") {
        verdict::indexed("P20.full", async {
            full_row_p20::run_thais_packet(core_clients).await;
            release_row_characters(core_clients).await
        }).await;
    }
    if groups.explicit("thaisroutes") {
        let [monk, _, _] = extra_world_fixtures(core_clients, clients).await?;
        let monk_clients = monk.clients(core_clients);
        verdict::indexed("C12.full", async {
            full_row_c12::run_packet(&monk_clients).await;
            release_row_characters(&monk_clients).await
        }).await;
    }
}
