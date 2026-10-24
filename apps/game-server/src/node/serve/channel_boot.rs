use super::*;

pub(super) struct ChannelBoot<'a> {
    pub(super) root: &'a DurabilityRoot,
    pub(super) material: &'a Material,
    pub(super) proof: &'a NodeIncarnationProof,
    pub(super) assignment: &'a RuntimeScopeAssignment,
    pub(super) bundle: Option<crate::map::boot::CheckedBundle>,
    pub(super) npc_boot: Option<&'a bundle_npcs::NpcBoot>,
    pub(super) reset_epoch: u64,
    pub(super) signalled: &'a CancellationToken,
}

pub(super) struct PreparedChannel {
    pub(super) active_content: ContentActivationController,
    pub(super) qualified_room: crate::content::QualifiedNativeEntryRoom,
    pub(super) movement_cells: crate::content::NativeEntryMovementCells,
    pub(super) bundle: Option<crate::map::boot::BundleWorld>,
    pub(super) combat_room: Option<crate::content::QualifiedNativeEntryRoom>,
    pub(super) spells: crate::spell::SpellBook,
    pub(super) imported_charms: crate::content::charm_source::CanonicalCharmCatalogue,
    pub(super) _wheel_gem_data: oteryn_gameplay_progression::wheel_gem_data::WheelGemData,
    pub(super) achievements: crate::achievement_catalogue::AchievementCatalogue,
    pub(super) quest_catalogue: std::sync::Arc<oteryn_gameplay_quests::quest::QuestStateCatalogue>,
    pub(super) quest_log_content: crate::gameplay_transport::quest_log::QuestLogLines,
    pub(super) door: Mutex<crate::world_runtime::LocalObjectRuntime>,
    pub(super) chest: crate::content::CanonicalReferencePlayableContent,
    pub(super) runtime: Mutex<ChannelRuntimeV1>,
    pub(super) required_capabilities: std::sync::Arc<
        crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities,
    >,
}

pub(super) async fn prepare_channel(
    boot: ChannelBoot<'_>,
) -> Result<Option<PreparedChannel>, BootError> {
    let ChannelBoot {
        root,
        material,
        proof,
        assignment,
        bundle,
        npc_boot,
        reset_epoch,
        signalled,
    } = boot;
    let config = &material.config;
    let scope = RuntimeScopeRefV1::channel(material.world, material.channel);
    let generation = assignment.ownership_generation;
    let fact = proof.fact();
    let (active_content, content) =
        activate_content(root, material.world, material.channel, reset_epoch).await?;
    let qualified_room = content.qualified_room().clone();
    let (channel_pin, movement_cells, door_content, spawn_source) = content.into_channel_parts();
    // Spell cast §3 (SPELL-D1): the V1 spell book is loaded with the Content activation, before
    // the Channel runtime; a book that does not load refuses readiness.
    let gameplay = active_content
        .active()
        .ok_or(BootError::ContentActivation("active generation"))?
        .native_gameplay();
    // MAP-CUTOVER-1b: a bundle World's Channel moves over the bundle from its configured start,
    // carries its map view in the movement cells and has no entry room: no door, chest, spell
    // or field qualification of the fixture room applies to it (§1.2).
    let bundle = bundle
        .map(|checked| boot_bundle_world(checked, material.world, material.channel, gameplay))
        .transpose()?;
    if let Some(world) = &bundle {
        activate_floor_changes(world)?;
    }
    // ADMIT0/D1081: validate the real scope offer before any readiness publication.
    // The generation is the operator assignment just resolved above, not config.
    // D1442 registers WORLD_SPATIAL_FIELDS=30 behind a closed offer gate; D1083 keeps
    // WORLD_INTERACTION's gate closed, so only offered floor members bind (fail closed).
    let required_capabilities =
        crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
            &config.readiness.required_gameplay_capabilities,
            scope,
            generation,
            &config.readiness.world_policy_revision,
            crate::gameplay_transport::world_required_capabilities::offered_for_scope(
                bundle.is_some(), active_content.active(),
            ),
        ).map_err(|_| BootError::Readiness("required gameplay capabilities"))?;
    let (channel_pin, movement_cells) = match &bundle {
        Some(world) => (
            bundle_channel_pin(&channel_pin, world)?,
            world
                .movement_cells(&movement_cells)
                .map_err(|_| BootError::ContentActivation("world bundle movement cells"))?,
        ),
        None => (channel_pin, movement_cells),
    };
    // BUNDLE-COMBAT-ENABLE-1: player auto-attack and native combat casts read the qualified
    // room; a bundle World serves them the room's bundle view, whose identities are the bundle
    // pin's. Every other `qualified_room` consumer stays closed for a bundle World (§1.2).
    let combat_room = match &bundle {
        Some(world) => bundle_combat_room(&qualified_room, world, material.world, &movement_cells),
        None => Some(qualified_room.clone()),
    };
    let spells = match gameplay {
        Some(native) => native.spell_book().clone(),
        None => crate::spell::cast::v1_spell_book()
            .map_err(|_| BootError::ContentActivation("spell book"))?,
    };
    // Data-only Charm import: retain the complete typed catalogue for this serve lifetime.
    // A malformed source refuses readiness; importing it does not qualify a generation,
    // advertise Charm commands or activate effects whose gameplay consumers are unfinished.
    let imported_charms = crate::content::charm_source::CanonicalCharmCatalogue::embedded()
        .map_err(|_| BootError::ContentActivation("charm source catalogue"))?;
    let (charm_index, charm_shard) = imported_charms.source_bytes();
    event(&format!(
        "event=charm_catalogue state=imported definitions={} source_bytes={} source_sha256={}",
        imported_charms.catalogue().definitions().count(),
        charm_index.len() + charm_shard.len(),
        hex(&imported_charms.source_digest()),
    ));
    // Data import only: retain the qualified reference catalogue for this boot. Loading
    // it does not admit Wheel stages, effects, paid Atelier operations or Item definitions.
    let _wheel_gem_data = oteryn_gameplay_progression::wheel_gem_data::WheelGemData::embedded()
        .map_err(|_| BootError::ContentActivation("Wheel/Gem reference data"))?;
    event("event=wheel_gem_data state=loaded runtime_admitted=false");
    // ACHIEVEMENT: the Achievement catalogue loads with the Content activation as well; a
    // malformed catalogue, or activated Content whose RewardClaim names an achievement the
    // catalogue lacks (contract §3.3), refuses readiness. The gameplay seam takes it for the
    // `ACCOUNT_ACHIEVEMENTS_QUERY` display read and the chest USE dispatch (C2).
    let achievements = crate::achievement_catalogue::AchievementCatalogue::embedded()
        .map_err(|_| BootError::ContentActivation("achievement catalogue"))?;
    if !achievements
        .unbound_reward_claim_achievements(&door_content)
        .is_empty()
    {
        return Err(BootError::ContentActivation(
            "reward claim achievement not in the catalogue",
        ));
    }
    let (quest_catalogue, quest_log_content, quest_event) = load_quest_catalogue(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state,
        &config.readiness.content_revision,
    )?;
    event(&quest_event);
    // #162 5868482467 (M2b): bind the entry room's one door `LocalObjectRuntime` once, at
    // Channel activation, from this exact activated content — never from a value a later
    // `USE_INTENT` is validating against it. `scope`/`generation` are this same activation's
    // own committed scope and ownership generation (bound above, before the Channel runtime).
    let door_scope_generation = ScopeOwnershipGeneration::new(assignment.ownership_generation)
        .map_err(|_| BootError::Readiness("native entry door scope generation"))?;
    let door_content_generation =
        crate::world_runtime::ReferenceContentGeneration::from_content(&door_content)
            .map_err(|_| BootError::Readiness("native entry door content generation"))?;
    let door_fence = crate::world_runtime::ScopeContentGenerationFence::for_activation(
        scope,
        door_scope_generation,
        door_content_generation,
    );
    let door = Mutex::new(
        crate::world_runtime::bind_native_entry_door(
            &door_content,
            &door_fence,
            scope,
            door_scope_generation,
        )
        .map_err(|_| BootError::Readiness("native entry door runtime binding"))?,
    );
    // C2 (#162 5914960502 Q2a): inject the entry room's one reward chest, its RewardClaim and
    // the reward and backpack Item definitions into a clone of this same activated content,
    // like the door above. The compiled Content and its digests stay unchanged. Its claim's
    // achievement (none today) must be in the catalogue, as for the activated content.
    let chest = match bundle {
        Some(_) => door_content.clone(),
        None => crate::interaction_chest_use::with_entry_chest(&door_content)
            .map_err(|_| BootError::ContentActivation("native entry chest content"))?,
    };
    if !achievements
        .unbound_reward_claim_achievements(&chest)
        .is_empty()
    {
        return Err(BootError::ContentActivation(
            "reward claim achievement not in the catalogue",
        ));
    }
    check_chest_quest_transitions(&chest, &quest_catalogue)?;
    let mut channel_runtime = ChannelRuntimeV1::from_committed_assignment(
        material.world,
        material.channel,
        fact.node_id(),
        fact.registration_revision(),
        assignment.ownership_generation,
        assignment.source_revision,
        &assignment.decision_identity,
        usize::try_from(config.scope.preproduction_actor_capacity)
            .map_err(|_| BootError::Readiness("channel runtime capacity"))?,
        channel_pin,
    )
    .map_err(|_| BootError::Readiness("channel runtime composition"))?;
    if let Some(native) = gameplay {
        native
            .install_companion_policies(&mut channel_runtime)
            .map_err(|_| BootError::ContentActivation("active creature policies"))?;
    }
    // NPC-LOAD-1: the bundle's NPC actors take their cells at Channel start, before any creature.
    bundle_npcs::place(&mut channel_runtime, npc_boot)?;
    let (spawned_monsters, spawn_windows) = match &bundle {
        Some(world) => match realize_bundle_spawns(&mut channel_runtime, world, signalled).await? {
            Some(realized) => realized,
            None => {
                stopped_before_ready();
                return Ok(None);
            }
        },
        None => {
            let realized = realize_fixture_spawns(&mut channel_runtime, spawn_source, false)?;
            (realized, u64::from(realized > 0))
        }
    };
    event(&format!(
        "event=creature_spawns state=realized spawned_monsters={spawned_monsters} windows={spawn_windows}"
    ));
    let runtime = Mutex::new(channel_runtime);
    event(&format!(
        "event=channel_runtime state=bootstrapped ownership_generation={} source_revision={} capacity={}",
        assignment.ownership_generation,
        assignment.source_revision,
        config.scope.preproduction_actor_capacity
    ));
    Ok(Some(PreparedChannel {
        active_content,
        qualified_room,
        movement_cells,
        bundle,
        combat_room,
        spells,
        imported_charms,
        _wheel_gem_data,
        achievements,
        quest_catalogue,
        quest_log_content,
        door,
        chest,
        runtime,
        required_capabilities,
    }))
}
