use super::*;

#[test]
fn selection_is_the_offered_supported_intersection_and_ignores_unknown_ids() {
    let selected = SelectedCapabilities::select(
        INJECTED,
        &[
            1,         // BESTIARY_CHARMS_V1
            3,         // NPC_SERVICE_V1
            8,         // ACHIEVEMENT_NOTICES_V1
            500,       // unknown capability
            4_000_000, // unknown capability
        ],
    );
    let expected = [
        1, // BESTIARY_CHARMS_V1
        8, // ACHIEVEMENT_NOTICES_V1
    ];
    assert_eq!(
        selected.map(|s| s.as_slice().to_vec()),
        Some(expected.to_vec())
    );
    let none = SelectedCapabilities::select(
        INJECTED,
        &[
            2,   // reserved capability
            3,   // NPC_SERVICE_V1
            500, // unknown capability
        ],
    );
    assert_eq!(none, Some(SelectedCapabilities::NONE));
    assert_eq!(
        SelectedCapabilities::select(INJECTED, &[]),
        Some(SelectedCapabilities::NONE)
    );
}

#[test]
fn selection_keeps_a_capability_only_with_all_its_requires() {
    let select = |supported: &[u32]| {
        SelectedCapabilities::select(INJECTED, supported)
            .expect("bounded")
            .as_slice()
            .to_vec()
    };
    // 7 requires 8; 10 requires 1; 6 requires 99, which is never offered.
    assert_eq!(select(&[7]), Vec::<u32>::new());
    let expected = [
        7, // CHAT_V1
        8, // ACHIEVEMENT_NOTICES_V1
    ];
    assert_eq!(
        select(&[
            7, // CHAT_V1
            8, // ACHIEVEMENT_NOTICES_V1
        ]),
        expected
    );
    assert_eq!(select(&[10]), Vec::<u32>::new());
    let expected = [
        1,  // BESTIARY_CHARMS_V1
        10, // ANALYSER_V1
    ];
    assert_eq!(
        select(&[
            1,  // BESTIARY_CHARMS_V1
            10, // ANALYSER_V1
        ]),
        expected
    );
    assert_eq!(
        select(&[
            6,  // WORLD_SPATIAL_ENTITIES
            99, // unoffered test requirement
        ]),
        Vec::<u32>::new()
    );
    let expected = [
        1,  // BESTIARY_CHARMS_V1
        7,  // CHAT_V1
        8,  // ACHIEVEMENT_NOTICES_V1
        10, // ANALYSER_V1
    ];
    assert_eq!(
        select(&[
            1,  // BESTIARY_CHARMS_V1
            6,  // WORLD_SPATIAL_ENTITIES
            7,  // CHAT_V1
            8,  // ACHIEVEMENT_NOTICES_V1
            10, // ANALYSER_V1
            99, // unoffered test requirement
        ]),
        expected
    );
    // The closure is transitive: a requirement that drops out takes its dependants with it.
    const CHAIN: &[OfferedCapability] = &[
        OfferedCapability {
            id: 1,
            requires: &[6],
        },
        OfferedCapability {
            id: 6,
            requires: &[7],
        },
        OfferedCapability {
            id: 7,
            requires: &[8],
        },
        OfferedCapability {
            id: 8,
            requires: &[],
        },
    ];
    let chain = |supported: &[u32]| {
        SelectedCapabilities::select(CHAIN, supported)
            .expect("bounded")
            .as_slice()
            .to_vec()
    };
    assert_eq!(
        chain(&[
            1, // BESTIARY_CHARMS_V1
            6, // WORLD_SPATIAL_ENTITIES
            7, // CHAT_V1
        ]),
        Vec::<u32>::new()
    );
    assert_eq!(
        chain(&[
            1, // BESTIARY_CHARMS_V1
            6, // WORLD_SPATIAL_ENTITIES
            7, // CHAT_V1
            8, // ACHIEVEMENT_NOTICES_V1
        ]),
        [
            1, // BESTIARY_CHARMS_V1
            6, // WORLD_SPATIAL_ENTITIES
            7, // CHAT_V1
            8, // ACHIEVEMENT_NOTICES_V1
        ]
    );
    assert_eq!(
        chain(&[
            1, // BESTIARY_CHARMS_V1
            7, // CHAT_V1
            8, // ACHIEVEMENT_NOTICES_V1
        ]),
        [
            7, // CHAT_V1
            8, // ACHIEVEMENT_NOTICES_V1
        ]
    );
}

#[test]
fn resume_keeps_exactly_the_original_selection() {
    let original = SelectedCapabilities::select(
        INJECTED,
        &[
            1, // BESTIARY_CHARMS_V1
            8, // ACHIEVEMENT_NOTICES_V1
        ],
    )
    .expect("bounded");
    assert!(original.resumable_with(&[
        1, // BESTIARY_CHARMS_V1
        8, // ACHIEVEMENT_NOTICES_V1
    ]));
    // A wider supported set is fine but never widens the session's selection.
    assert!(original.resumable_with(&[
        1,  // BESTIARY_CHARMS_V1
        7,  // CHAT_V1
        8,  // ACHIEVEMENT_NOTICES_V1
        10, // ANALYSER_V1
    ]));
    let expected = [
        1, // BESTIARY_CHARMS_V1
        8, // ACHIEVEMENT_NOTICES_V1
    ];
    assert_eq!(original.as_slice(), expected);
    // A supported set that lacks a selected capability cannot keep it.
    assert!(!original.resumable_with(&[1]));
    assert!(!original.resumable_with(&[]));
    assert!(SelectedCapabilities::NONE.resumable_with(&[]));
}

#[test]
fn commands_and_domains_follow_the_selection() {
    let none = SelectedCapabilities::NONE;
    // Core commands and domains never depend on a capability.
    for core in [
        1,  // WORLD_ACTOR_STEP_INTENT
        2,  // USE_INTENT
        3,  // WORLD_ACTOR_SPELL_CAST_INTENT
        10, // ACCOUNT_ACHIEVEMENTS_QUERY
    ] {
        assert!(none.command_selected(core), "command {core}");
    }
    for core in [1, 2, 3] {
        assert!(none.domain_selected(core), "domain {core}");
    }
    // Capability 1: commands 4 and 5, domains 4 and 5. Capability 7: command 13, domain 12.
    // Capability 8: domain 13. Capability 10: domain 15. Capability 16: command 22, domain 16.
    // Capability 17: commands 11 and 12, domain 10. Capability 19: domain 18. Capability 11: command 15.
    // Capability 21: command 23. Capability 23: domains 19 and 20.
    for (capability, commands, domains) in [
        (
            1,
            &[
                4, // CHARM_UNLOCK_STAGE_INTENT
                5, // CHARM_ASSIGN_INTENT
            ][..],
            &[4, 5][..],
        ),
        (37, &[36][..], &[][..]),
        (7, &[13][..], &[12][..]),
        (8, &[][..], &[13][..]),
        (10, &[][..], &[15][..]),
        (16, &[22][..], &[16][..]),
        (
            17,
            &[
                11, // ATTACK_TARGET_INTENT
                12, // FIGHT_MODES_INTENT
            ][..],
            &[10][..],
        ),
        (19, &[][..], &[18][..]),
        (11, &[15][..], &[][..]),
        (21, &[23][..], &[][..]),
        (23, &[][..], &[19, 20][..]),
    ] {
        let selected = selection(&[capability]);
        for &command in commands {
            assert!(!none.command_selected(command), "command {command}");
            assert!(selected.command_selected(command), "command {command}");
        }
        for &domain in domains {
            assert!(!none.domain_selected(domain), "domain {domain}");
            assert!(selected.domain_selected(domain), "domain {domain}");
        }
    }
    assert!(!selection(&[8]).command_selected(13));
    assert!(!selection(&[7]).domain_selected(13));
    assert!(!selection(&[14]).command_selected(22));
    assert!(!selection(&[14]).domain_selected(16));
    assert!(!selection(&[16]).command_selected(11));
    assert!(!selection(&[16]).command_selected(12));
    assert!(!selection(&[16]).domain_selected(10));
    // MAP-WIRE-1 §3: one object domain per World. Capability 18 supersedes core domain 2.
    assert!(
        !selection(&[
            4,  // ITEM_VIEW_MOVE_V1
            6,  // WORLD_SPATIAL_ENTITIES
            18, // WORLD_MAP_VIEW_V1
        ])
        .domain_selected(2)
    );
    assert!(
        selection(&[
            4,  // ITEM_VIEW_MOVE_V1
            6,  // WORLD_SPATIAL_ENTITIES
            18, // WORLD_MAP_VIEW_V1
        ])
        .domain_selected(17)
    );
    assert!(
        selection(&[
            4, // ITEM_VIEW_MOVE_V1
            6, // WORLD_SPATIAL_ENTITIES
        ])
        .domain_selected(2)
    );
}

#[test]
fn the_bundle_world_offered_set_is_3_the_item_view_set_12_14_and_18_requiring_4_and_6()
-> Result<(), Box<dyn Error>> {
    // MAP-CUTOVER-1b: the bundle set is the Item view set (which offers 4) with 3 and 18;
    // ITEM-LOOP-1 (CP D1438) adds 12 (requires 4) and 14 (requires 4 and 12). 18's crate and
    // registry `requires` stay [4, 6] and the registry keeps 3, 12, 14 and 18 `offered: false`.
    use oteryn_protocol_oteryn::container_tree::{
        CAPABILITY_CONTAINER_TREE_V1, CAPABILITY_CONTAINER_TREE_V1_REQUIRES,
    };
    use oteryn_protocol_oteryn::item_view::{
        CAPABILITY_ITEM_EQUIP_DROP_V1, CAPABILITY_ITEM_EQUIP_DROP_V1_REQUIRES,
    };
    let bundle_only: Vec<u32> = runtime_fragments()?
        .iter()
        .filter(|entry| entry["offers"]["bundle_world"] == true && entry["offers"]["item_view"] == false)
        .map(|entry| Ok(u32::try_from(entry["id"].as_u64().ok_or("id")?)?))
        .collect::<Result<_, Box<dyn Error>>>()?;
    let shared: Vec<OfferedCapability> = BUNDLE_WORLD_OFFERED_CAPABILITIES
        .iter()
        .filter(|offer| !bundle_only.contains(&offer.id))
        .copied()
        .collect();
    assert_eq!(shared, ITEM_VIEW_OFFERED_CAPABILITIES);
    let requires = |id: u32| {
        BUNDLE_WORLD_OFFERED_CAPABILITIES
            .iter()
            .find(|offer| offer.id == id)
            .map(|offer| offer.requires)
    };
    assert_eq!(requires(bundle_only[0]), Some(&[][..]));
    assert_eq!(
        requires(CAPABILITY_ITEM_EQUIP_DROP_V1),
        Some(CAPABILITY_ITEM_EQUIP_DROP_V1_REQUIRES)
    );
    assert_eq!(
        requires(CAPABILITY_CONTAINER_TREE_V1),
        Some(CAPABILITY_CONTAINER_TREE_V1_REQUIRES)
    );
    assert_eq!(requires(CAPABILITY_WORLD_MAP_VIEW_V1), Some(&[4, 6][..]));
    // WEATHER-1: the bundle set offers 26 (shared with the Item view set) with its own `requires`.
    assert_eq!(
        requires(oteryn_protocol_oteryn::world_weather::CAPABILITY_WORLD_WEATHER_V1),
        Some(WORLD_WEATHER.requires)
    );
    assert_eq!(requires(CAPABILITY_ITEM_LOOK_V1), Some(&[][..]));
    assert_eq!(requires(11), Some(&[15, 18][..]));
    assert_eq!(requires(15), Some(&[4][..]));
    assert_eq!(
        requires(oteryn_protocol_oteryn::vip::CAPABILITY_VIP_V1),
        Some(&[][..])
    );
    assert_eq!(
        requires(oteryn_protocol_oteryn::depot::CAPABILITY_DEPOT_V1),
        Some(&[4][..])
    );
    assert_eq!(
        BUNDLE_WORLD_OFFERED_CAPABILITIES.len(),
        shared.len() + bundle_only.len()
    );
    assert!(
        BUNDLE_WORLD_OFFERED_CAPABILITIES
            .windows(2)
            .all(|pair| pair[0].id < pair[1].id)
    );
    // Every requirement of an offered capability is offered.
    assert!(BUNDLE_WORLD_OFFERED_CAPABILITIES.iter().all(|offer| {
        offer
            .requires
            .iter()
            .all(|required| requires(*required).is_some())
    }));
    assert!(BUNDLE_WORLD_OFFERED_CAPABILITIES.len() <= SELECTED_CAPACITY);
    let registry = registry_capabilities()?;
    for id in bundle_only.into_iter().filter(|id| ![11, 15].contains(id)) {
        let entry = registry
            .iter()
            .find(|capability| capability["id"].as_u64() == Some(u64::from(id)))
            .ok_or("bundle-only capability")?;
        assert_eq!(entry["offered"].as_bool(), Some(false));
        let gate = entry["offer_gate"].as_str().ok_or("offer_gate")?;
        assert!(gate.contains("BUNDLE_WORLD_OFFERED_CAPABILITIES"), "{id}");
    }
    let entry = registry
        .iter()
        .find(|capability| capability["id"].as_u64() == Some(18))
        .ok_or("capability 18")?;
    assert_eq!(ids(&entry["requires"])?, [4, 6]);
    Ok(())
}

#[test]
fn item_use_wire_capability_15_is_registered_not_offered_and_requires_4()
-> Result<(), Box<dyn Error>> {
    let entry = registry_capabilities()?
        .into_iter()
        .find(|capability| capability["id"] == CAPABILITY_ITEM_USE_V1)
        .ok_or("capability 15")?;
    assert_eq!(entry["offered"], false);
    assert_eq!(ids(&entry["requires"])?, [CAPABILITY_ITEM_VIEW_MOVE_V1]);
    assert!(
        PRODUCTION_OFFERED_CAPABILITIES
            .iter()
            .all(|offered| offered.id != CAPABILITY_ITEM_USE_V1)
    );
    // CAP-NEG-1: 15 without 4 is never selected.
    let select = |supported: &[u32]| {
        SelectedCapabilities::select(ITEM_USE_OFFERED, supported)
            .expect("bounded")
            .as_slice()
            .to_vec()
    };
    assert_eq!(select(&[15]), Vec::<u32>::new());
    let expected = [
        4,  // ITEM_VIEW_MOVE_V1
        15, // ITEM_USE_V1
    ];
    assert_eq!(
        select(&[
            4,  // ITEM_VIEW_MOVE_V1
            15, // ITEM_USE_V1
        ]),
        expected
    );
    // It owns no command type or domain: it selects none of capability 4's.
    let selected = selection(&[15]);
    assert!(!selected.command_selected(COMMAND_TYPE_ITEM_MOVE_INTENT));
    assert!(!selected.domain_selected(STATE_DOMAIN_CHARACTER_INVENTORY));
    assert!(!selected.domain_selected(STATE_DOMAIN_OPEN_CONTAINER));
    Ok(())
}

#[test]
fn item_use_wire_a_resume_that_drops_15_falls_back_to_fresh_admission() {
    let original = SelectedCapabilities::select(
        ITEM_USE_OFFERED,
        &[
            4,  // ITEM_VIEW_MOVE_V1
            15, // ITEM_USE_V1
        ],
    )
    .expect("bounded");
    let expected = [
        4,  // ITEM_VIEW_MOVE_V1
        15, // ITEM_USE_V1
    ];
    assert_eq!(original.as_slice(), expected);
    assert!(original.resumable_with(&[
        4,  // ITEM_VIEW_MOVE_V1
        15, // ITEM_USE_V1
    ]));
    // A resume whose supported set lacks 15 (or 4) would change the set: it is refused as
    // resume-unavailable and the client falls back to fresh admission.
    assert!(!original.resumable_with(&[4]));
    assert!(!original.resumable_with(&[15]));
    // A session admitted without 15 never gains it on resume.
    let without = SelectedCapabilities::select(ITEM_USE_OFFERED, &[4]).expect("bounded");
    assert!(without.resumable_with(&[
        4,  // ITEM_VIEW_MOVE_V1
        15, // ITEM_USE_V1
    ]));
    assert_eq!(without.as_slice(), [4]);
}

#[test]
fn map_wire_capability_18_is_registered_not_offered_and_requires_4_and_6()
-> Result<(), Box<dyn Error>> {
    use oteryn_protocol_oteryn::world_map::{
        CAPABILITY_WORLD_MAP_VIEW_REQUIRES, STATE_DOMAIN_WORLD_MAP_VIEW,
    };
    let entry = registry_capabilities()?
        .into_iter()
        .find(|capability| capability["id"] == CAPABILITY_WORLD_MAP_VIEW_V1)
        .ok_or("capability 18")?;
    assert_eq!(entry["offered"], false);
    assert_eq!(ids(&entry["requires"])?, CAPABILITY_WORLD_MAP_VIEW_REQUIRES);
    assert!(
        PRODUCTION_OFFERED_CAPABILITIES
            .iter()
            .all(|offered| offered.id != CAPABILITY_WORLD_MAP_VIEW_V1)
    );
    let select = |supported: &[u32]| {
        SelectedCapabilities::select(WORLD_MAP_OFFERED, supported)
            .expect("bounded")
            .as_slice()
            .to_vec()
    };
    // CAP-NEG-1: 18 without 4 or without 6 is never selected.
    assert_eq!(
        select(&[
            6,  // WORLD_SPATIAL_ENTITIES
            18, // WORLD_MAP_VIEW_V1
        ]),
        [6]
    );
    assert_eq!(
        select(&[
            4,  // ITEM_VIEW_MOVE_V1
            18, // WORLD_MAP_VIEW_V1
        ]),
        [4]
    );
    assert_eq!(select(&[18]), Vec::<u32>::new());
    let expected = [
        4,  // ITEM_VIEW_MOVE_V1
        6,  // WORLD_SPATIAL_ENTITIES
        18, // WORLD_MAP_VIEW_V1
    ];
    assert_eq!(
        select(&[
            4,  // ITEM_VIEW_MOVE_V1
            6,  // WORLD_SPATIAL_ENTITIES
            18, // WORLD_MAP_VIEW_V1
        ]),
        expected
    );
    // Domain 17 is sent only with 18 selected.
    assert!(
        !selection(&[
            4, // ITEM_VIEW_MOVE_V1
            6, // WORLD_SPATIAL_ENTITIES
        ])
        .domain_selected(STATE_DOMAIN_WORLD_MAP_VIEW)
    );
    assert!(
        selection(&[
            4,  // ITEM_VIEW_MOVE_V1
            6,  // WORLD_SPATIAL_ENTITIES
            18, // WORLD_MAP_VIEW_V1
        ])
        .domain_selected(STATE_DOMAIN_WORLD_MAP_VIEW)
    );
    // A resume that drops 18 falls back to fresh admission.
    let original = SelectedCapabilities::select(
        WORLD_MAP_OFFERED,
        &[
            4,  // ITEM_VIEW_MOVE_V1
            6,  // WORLD_SPATIAL_ENTITIES
            18, // WORLD_MAP_VIEW_V1
        ],
    )
    .expect("bounded");
    assert!(!original.resumable_with(&[
        4, // ITEM_VIEW_MOVE_V1
        6, // WORLD_SPATIAL_ENTITIES
    ]));
    Ok(())
}

#[test]
fn npc_wire_capability_3_is_registered_not_offered_and_gates_commands_7_and_8()
-> Result<(), Box<dyn Error>> {
    use oteryn_protocol_oteryn::npc_service::{
        CAPABILITY_NPC_SERVICE_V1, COMMAND_TYPE_NPC_TALK_INTENT, COMMAND_TYPE_NPC_TRADE_INTENT,
        STATE_DOMAIN_NPC_CONVERSATION, STATE_DOMAIN_NPC_TRADE_WINDOW,
    };
    let entry = registry_capabilities()?
        .into_iter()
        .find(|capability| capability["id"] == CAPABILITY_NPC_SERVICE_V1)
        .ok_or("capability 3")?;
    assert_eq!(entry["name"], "NPC_SERVICE_V1");
    assert_eq!(entry["offered"], false);
    assert!(entry.get("requires").is_none());
    assert_eq!(
        ids(&entry["command_types"])?,
        [
            COMMAND_TYPE_NPC_TALK_INTENT,  // NPC_TALK_INTENT
            COMMAND_TYPE_NPC_TRADE_INTENT, // NPC_TRADE_INTENT
        ]
    );
    assert_eq!(
        ids(&entry["state_domains"])?,
        [STATE_DOMAIN_NPC_CONVERSATION, STATE_DOMAIN_NPC_TRADE_WINDOW]
    );
    assert!(
        PRODUCTION_OFFERED_CAPABILITIES
            .iter()
            .all(|offered| offered.id != CAPABILITY_NPC_SERVICE_V1)
    );
    // Without 3 selected no domain 7 or 8 is sent and no command 7 or 8 is dispatched.
    let none = SelectedCapabilities::NONE;
    assert!(!none.command_selected(COMMAND_TYPE_NPC_TALK_INTENT));
    assert!(!none.command_selected(COMMAND_TYPE_NPC_TRADE_INTENT));
    assert!(!none.domain_selected(STATE_DOMAIN_NPC_CONVERSATION));
    assert!(!none.domain_selected(STATE_DOMAIN_NPC_TRADE_WINDOW));
    run(async {
        // A client asking for capability 3 is not granted it.
        let authority = NegotiatingAuthority::new(None);
        let (admitted, frames) = admit(
            &authority,
            &bootstrap(&[
                1, // BESTIARY_CHARMS_V1
                3, // NPC_SERVICE_V1
                6, // WORLD_SPATIAL_ENTITIES
            ])?,
        )
        .await?;
        assert_eq!(accepted_selection(&frames)?, [6]);
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        assert!(!admitted.continuity.selected_capabilities.contains(3));
        // Commands 7 and 8 are refused as unsupported for everyone.
        let frames = [command(1, 7, b"hi"), command(2, 8, b"buy")];
        let (_, written) = serve(&authority, session(SessionContinuity::FRESH)?, &frames).await?;
        let rejected = |sequence, id| {
            crate::foundation::encode_command_result(
                1,
                sequence,
                id,
                crate::foundation::CommandStatus::Rejected,
                &[],
            )
        };
        assert!(written.contains(&rejected(1, 1)?));
        assert!(written.contains(&rejected(2, 2)?));
        Ok(())
    })
}
