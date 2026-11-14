use super::*;
const AUTHORITY: &str = "oteryn:game:primary";
const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../docs/contracts/fixtures/public-projections/"
);
fn fixture(name: &str) -> (String, Snapshot) {
    let wire = std::fs::read_to_string(format!("{FIXTURES}{name}_v2.json")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&wire).unwrap();
    let (subject, world_id, body) = if let Some(id) = value.get("character_id") {
        (
            id.as_str().unwrap().to_owned(),
            String::new(),
            Body::Deaths(serde_json::from_value(value["deaths"].clone()).unwrap()),
        )
    } else if let Some(id) = value.get("guild_id") {
        (
            id.as_str().unwrap().to_owned(),
            String::new(),
            Body::Guild(serde_json::from_value(value["guild"].clone()).unwrap()),
        )
    } else {
        (
            value["house_key"].as_str().unwrap().to_owned(),
            value["world_id"].as_str().unwrap().to_owned(),
            Body::House(serde_json::from_value(value["house"].clone()).unwrap()),
        )
    };
    (
        wire,
        Snapshot {
            subject,
            world_id,
            projection_epoch: 1,
            projection_revision: 3,
            source_observed_at: 1790000000,
            body,
        },
    )
}
#[test]
fn v2_family_wire_matches_generated_goldens() {
    for name in [
        "character_deaths_player",
        "character_deaths_creature",
        "character_deaths_pvp_hidden",
        "guild",
        "house_vacant",
        "house_auction",
        "house_owned",
        "house_move_out_pending",
        "house_unavailable",
    ] {
        let (wire, snapshot) = fixture(name);
        assert_eq!(encode(AUTHORITY, &snapshot).unwrap(), wire, "{name}");
        assert!(
            snapshot
                .family()
                .operation()
                .path()
                .starts_with("/internal/v2/game-public/")
        );
    }
}
#[test]
fn stable_killer_identity_player_creature_and_pvp_hidden() {
    let (_, player) = fixture("character_deaths_player");
    let Body::Deaths(rows) = player.body else {
        panic!("deaths")
    };
    assert!(
        matches!(&rows[0].killers[0],Killer::Player {character_id,name} if character_id.ends_with("4402") && name=="Beren")
    );
    let (_, creature) = fixture("character_deaths_creature");
    let Body::Deaths(rows) = creature.body else {
        panic!("deaths")
    };
    assert!(
        matches!(&rows[0].killers[0],Killer::Creature {creature_key,name} if creature_key=="oteryn:monster.dragon" && name=="Dragon")
    );
    let (_, hidden) = fixture("character_deaths_pvp_hidden");
    let Body::Deaths(rows) = hidden.body else {
        panic!("deaths")
    };
    assert!(rows[0].pvp);
    assert!(rows[0].killers.is_empty());
}
#[test]
fn v2_deaths_refuse_invalid_identity_order_and_over_bound_without_truncation() {
    let (_, mut s) = fixture("character_deaths_player");
    let Body::Deaths(rows) = &mut s.body else {
        panic!("deaths")
    };
    rows[0].killers = vec![rows[0].killers[0].clone(); 17];
    assert!(encode(AUTHORITY, &s).is_err());
    let (_, mut s) = fixture("character_deaths_player");
    let Body::Deaths(rows) = &mut s.body else {
        panic!("deaths")
    };
    rows.push(rows[0].clone());
    assert!(encode(AUTHORITY, &s).is_err());
    let (_, mut s) = fixture("character_deaths_creature");
    let Body::Deaths(rows) = &mut s.body else {
        panic!("deaths")
    };
    rows[0].killers[0] = Killer::Creature {
        creature_key: "Dragon".into(),
        name: "Dragon".into(),
    };
    assert!(encode(AUTHORITY, &s).is_err());
}
#[test]
fn v2_guild_requires_stable_unique_members_and_existing_rank() {
    let (_, mut s) = fixture("guild");
    let Body::Guild(rows) = &mut s.body else {
        panic!("guild")
    };
    rows[0].members[0].rank_level = 4;
    assert!(encode(AUTHORITY, &s).is_err());
    let (_, mut s) = fixture("guild");
    let Body::Guild(rows) = &mut s.body else {
        panic!("guild")
    };
    let duplicate = rows[0].members[0].clone();
    rows[0].members.push(duplicate);
    assert!(encode(AUTHORITY, &s).is_err());
}
#[test]
fn v2_ack_is_version_bound_and_never_accepts_http_status_alone() {
    assert_eq!(
        decode_response(br#"{"contract_version":2,"result":"accepted"}"#).unwrap(),
        Delivery::Accepted
    );
    assert_eq!(
        decode_response(br#"{"contract_version":2,"result":"superseded"}"#).unwrap(),
        Delivery::Superseded
    );
    for raw in [
        br#"{"contract_version":1,"result":"accepted"}"#.as_slice(),
        br#"{}"#,
        br#"{"contract_version":2,"result":"accepted","extra":0}"#,
        br#"{"contract_version":2,"contract_version":2,"result":"accepted"}"#,
    ] {
        assert!(decode_response(raw).is_err());
    }
}

#[test]
fn v2_family_collection_limits_are_inclusive_and_never_truncated() {
    let (_, mut deaths) = fixture("character_deaths_player");
    let Body::Deaths(rows) = &mut deaths.body else {
        panic!("deaths")
    };
    let original = rows[0].clone();
    *rows = (0..21)
        .rev()
        .map(|n| {
            let mut row = original.clone();
            row.death_occurrence_id = format!("01934f10-7c08-7001-805b-{n:012x}");
            row
        })
        .collect();
    assert!(encode(AUTHORITY, &deaths).is_err());
    let Body::Deaths(rows) = &mut deaths.body else {
        panic!("deaths")
    };
    rows.pop();
    assert!(encode(AUTHORITY, &deaths).is_ok());
    let (_, mut guild) = fixture("guild");
    let Body::Guild(rows) = &mut guild.body else {
        panic!("guild")
    };
    let original = rows[0].members[0].clone();
    rows[0].members = (0..2001)
        .map(|n| {
            let mut member = original.clone();
            member.character_id = format!("01934f10-7c04-7001-805b-{n:012x}");
            member
        })
        .collect();
    assert!(encode(AUTHORITY, &guild).is_err());
    let Body::Guild(rows) = &mut guild.body else {
        panic!("guild")
    };
    rows[0].members.pop();
    assert!(encode(AUTHORITY, &guild).is_ok());
    let Body::Guild(rows) = &mut guild.body else {
        panic!("guild")
    };
    rows[0].ranks = (1..=21)
        .map(|level| Rank {
            level,
            name: "Member".into(),
        })
        .collect();
    assert!(encode(AUTHORITY, &guild).is_err());
    let Body::Guild(rows) = &mut guild.body else {
        panic!("guild")
    };
    rows[0].ranks.pop();
    assert!(encode(AUTHORITY, &guild).is_ok());
}

#[test]
fn extension_publication_flags_default_off_independently() {
    for family in Family::ALL {
        let expected = matches!(
            family,
            Family::CharacterDeaths | Family::Guild | Family::House
        );
        assert_eq!(family.enabled(Activation::default()), expected);
    }
    let enabled = Activation {
        character_online_sessions: true,
        ..Activation::default()
    };
    assert!(Family::CharacterOnlineSessions.enabled(enabled));
    assert!(!Family::WorldDailyKills.enabled(enabled));
}

#[test]
fn creature_wire_identity_cannot_use_reserved_analytics_buckets() {
    for reserved in ["player", "unknown"] {
        let (_, mut snapshot) = fixture("character_deaths_creature");
        let Body::Deaths(rows) = &mut snapshot.body else {
            panic!("deaths")
        };
        rows[0].killers[0] = Killer::Creature {
            creature_key: reserved.into(),
            name: "Dragon".into(),
        };
        assert!(encode(AUTHORITY, &snapshot).is_err());
    }
}

#[test]
fn public_projections_v2_new_family_flags_default_off() {
    assert_eq!(
        serde_json::from_str::<Activation>("{}").unwrap(),
        Activation::default()
    );
    let partial: Activation = serde_json::from_str(r#"{"character_equipment":true}"#).unwrap();
    assert!(partial.character_equipment);
    assert!(
        !partial.character_achievements && !partial.character_outfit && !partial.character_skills
    );
    let families = [
        Family::CharacterSkills,
        Family::CharacterAchievements,
        Family::CharacterEquipment,
        Family::CharacterOutfit,
    ];
    for (i, family) in families.iter().enumerate() {
        assert!(!family.enabled(Activation::default()));
        let mut activation = Activation::default();
        match family {
            Family::CharacterSkills => activation.character_skills = true,
            Family::CharacterAchievements => activation.character_achievements = true,
            Family::CharacterEquipment => activation.character_equipment = true,
            Family::CharacterOutfit => activation.character_outfit = true,
            _ => unreachable!(),
        }
        for (j, other) in families.iter().enumerate() {
            assert_eq!(other.enabled(activation), i == j);
        }
    }
}

fn character_view_fixture(name: &str) -> (String, Snapshot) {
    let file = std::fs::read_to_string(format!("{FIXTURES}{name}_v2.json")).unwrap();
    assert!(file.ends_with('\n') && !file.ends_with("\n\n"));
    let wire = file.strip_suffix('\n').unwrap().to_owned();
    let v: serde_json::Value = serde_json::from_str(&wire).unwrap();
    let body = match v["operation"].as_str().unwrap() {
        "PublishCharacterSkillsV2" => {
            Body::Skills(serde_json::from_value(v["skills"].clone()).unwrap())
        }
        "PublishCharacterAchievementsV2" => {
            Body::Achievements(serde_json::from_value(v["achievements"].clone()).unwrap())
        }
        "PublishCharacterEquipmentV2" => {
            Body::Equipment(serde_json::from_value(v["equipment"].clone()).unwrap())
        }
        "PublishCharacterOutfitV2" => {
            Body::Outfit(serde_json::from_value(v["outfit"].clone()).unwrap())
        }
        _ => panic!("unexpected fixture operation"),
    };
    (
        wire,
        Snapshot {
            subject: v["character_id"].as_str().unwrap().into(),
            world_id: String::new(),
            projection_epoch: v["projection_epoch"].as_str().unwrap().parse().unwrap(),
            projection_revision: v["projection_revision"].as_str().unwrap().parse().unwrap(),
            source_observed_at: v["source_observed_at"].as_str().unwrap().parse().unwrap(),
            body,
        },
    )
}
fn character_view_golden(name: &str, operation: Operation, bytes: usize) {
    let (wire, s) = character_view_fixture(name);
    assert_eq!(encode(AUTHORITY, &s).unwrap(), wire);
    assert_eq!(s.family().operation(), operation);
    assert_eq!(operation.request_bytes_max(), bytes);
    assert_eq!(
        operation.path(),
        format!("/internal/v2/game-public/{}", name.replace('_', "-"))
    );
}
#[test]
fn public_projections_v2_skills_golden_bytes() {
    character_view_golden(
        "character_skills",
        Operation::PublishCharacterSkillsV2,
        4096,
    );
}
#[test]
fn public_projections_v2_achievements_golden_bytes() {
    character_view_golden(
        "character_achievements",
        Operation::PublishCharacterAchievementsV2,
        262144,
    );
}
#[test]
fn public_projections_v2_equipment_golden_bytes() {
    character_view_golden(
        "character_equipment",
        Operation::PublishCharacterEquipmentV2,
        8192,
    );
}
#[test]
fn public_projections_v2_outfit_golden_bytes() {
    character_view_golden(
        "character_outfit",
        Operation::PublishCharacterOutfitV2,
        2048,
    );
}
#[test]
fn public_projections_v2_skills_suppression_golden_bytes() {
    let (wire, s) = character_view_fixture("character_skills_suppression");
    assert_eq!(encode(AUTHORITY, &s).unwrap(), wire);
    let Body::Skills(rows) = &s.body else {
        panic!("skills")
    };
    assert!(rows.is_empty());
}
#[test]
fn public_projections_v2_watermark_v2_family_enum_closed() {
    let names = WatermarkFamily::ALL.map(|family| match family {
        WatermarkFamily::CharacterDeaths => "character_deaths",
        WatermarkFamily::Guild => "guild",
        WatermarkFamily::House => "house",
        WatermarkFamily::CharacterSkills => "character_skills",
        WatermarkFamily::CharacterAchievements => "character_achievements",
        WatermarkFamily::CharacterEquipment => "character_equipment",
        WatermarkFamily::CharacterOutfit => "character_outfit",
        WatermarkFamily::CharacterOnlineSessions => "character_online_sessions",
        WatermarkFamily::WorldDailyKills => "world_daily_kills",
        WatermarkFamily::WorldDaily => "world_daily",
    });
    assert_eq!(
        names,
        [
            "character_deaths",
            "guild",
            "house",
            "character_skills",
            "character_achievements",
            "character_equipment",
            "character_outfit",
            "character_online_sessions",
            "world_daily_kills",
            "world_daily"
        ]
    );

    for family in [
        "character_deaths",
        "guild",
        "house",
        "character_skills",
        "character_achievements",
        "character_equipment",
        "character_outfit",
        "character_online_sessions",
        "world_daily_kills",
        "world_daily",
    ] {
        let json = format!("\"{family}\"");
        let parsed: WatermarkFamily = serde_json::from_str(&json).unwrap();
        assert_eq!(serde_json::to_string(&parsed).unwrap(), json);
        assert!(
            encode_watermark(
                AUTHORITY,
                parsed,
                Facts {
                    projection_epoch: 1,
                    oldest_undelivered_ms: None,
                    now_ms: 1790000003000
                },
                Duration::from_secs(3)
            )
            .is_ok()
        );
    }
    for unknown in [
        "character_activity",
        "world_online",
        "unknown",
        "CharacterSkills",
    ] {
        assert!(serde_json::from_str::<WatermarkFamily>(&format!("\"{unknown}\"")).is_err());
    }
    let wire =
        std::fs::read_to_string(format!("{FIXTURES}character_skills_watermark_v2.json")).unwrap();
    assert_eq!(
        encode_watermark(
            AUTHORITY,
            Family::CharacterSkills,
            Facts {
                projection_epoch: 1,
                oldest_undelivered_ms: None,
                now_ms: 1790000003000
            },
            Duration::from_secs(3)
        )
        .unwrap(),
        wire.strip_suffix('\n').unwrap()
    );
}
#[test]
fn public_projections_v2_achievements_over_1024_refused_not_truncated() {
    let (_, mut s) = character_view_fixture("character_achievements");
    let Body::Achievements(rows) = &mut s.body else {
        panic!("achievements")
    };
    rows[0].earned = (0..1024)
        .map(|n| character_views::Earned {
            key: format!("oteryn:achievement/fixture_{n:04}"),
            earned_at: "1789990000".into(),
        })
        .collect();
    assert!(encode(AUTHORITY, &s).is_ok());
    let Body::Achievements(rows) = &mut s.body else {
        panic!("achievements")
    };
    rows[0].earned.push(character_views::Earned {
        key: "oteryn:achievement/fixture_1024".into(),
        earned_at: "1789990000".into(),
    });
    assert!(encode(AUTHORITY, &s).is_err());
    let Body::Achievements(rows) = &s.body else {
        panic!("achievements")
    };
    assert_eq!(rows[0].earned.len(), 1025);
}
#[test]
fn public_projections_v2_equipment_slot_order_and_uniqueness() {
    use character_views::{Equipped, Slot};
    let (_, mut s) = character_view_fixture("character_equipment");
    let Body::Equipment(rows) = &mut s.body else {
        panic!("equipment")
    };
    rows[0].slots = [
        Slot::Head,
        Slot::Necklace,
        Slot::Container,
        Slot::Armor,
        Slot::RightHand,
        Slot::LeftHand,
        Slot::Legs,
        Slot::Feet,
        Slot::Ring,
        Slot::Ammo,
    ]
    .into_iter()
    .map(|slot| Equipped {
        slot,
        item_key: "oteryn:item.fixture-helmet".into(),
        item_revision: "fixture-v1".into(),
    })
    .collect();
    assert!(encode(AUTHORITY, &s).is_ok());
    let Body::Equipment(rows) = &mut s.body else {
        panic!("equipment")
    };
    rows[0].slots.swap(0, 1);
    assert!(encode(AUTHORITY, &s).is_err());
    let Body::Equipment(rows) = &mut s.body else {
        panic!("equipment")
    };
    rows[0].slots.swap(0, 1);
    rows[0].slots[1].slot = Slot::Head;
    assert!(encode(AUTHORITY, &s).is_err());
    assert!(serde_json::from_str::<Slot>("\"head\"").is_err());
}
#[test]
fn public_projections_v2_character_view_bounds_and_exact_members() {
    let (_, mut s) = character_view_fixture("character_skills");
    let Body::Skills(rows) = &mut s.body else {
        panic!("skills")
    };
    rows[0].level = "4294967296".into();
    assert!(encode(AUTHORITY, &s).is_err());
    let Body::Skills(rows) = &mut s.body else {
        panic!("skills")
    };
    rows[0].level = "412".into();
    rows[0].fist_level = 9;
    assert!(encode(AUTHORITY, &s).is_err());
    let (_, mut s) = character_view_fixture("character_outfit");
    let Body::Outfit(rows) = &mut s.body else {
        panic!("outfit")
    };
    let mut raw = serde_json::to_value(&rows[0]).unwrap();
    raw["colours"] = serde_json::json!([1, 2, 3, 65536]);
    assert!(serde_json::from_value::<character_views::Outfit>(raw.clone()).is_err());
    raw["colours"] = serde_json::json!([1, 2, 3, 4]);
    raw["account_id"] = serde_json::json!("private");
    assert!(serde_json::from_value::<character_views::Outfit>(raw).is_err());
    rows[0].mount = vec![
        character_views::Mount {
            mount_key: "mount.fixture".into()
        };
        2
    ];
    assert!(encode(AUTHORITY, &s).is_err());
    for bad in ["", "01", "0", "18446744073709551616"] {
        let (_, mut s) = character_view_fixture("character_skills");
        let Body::Skills(rows) = &mut s.body else {
            panic!("skills")
        };
        rows[0].character_revision = bad.into();
        assert!(encode(AUTHORITY, &s).is_err());
    }
    let (_, mut s) = character_view_fixture("character_achievements");
    let Body::Achievements(rows) = &mut s.body else {
        panic!("achievements")
    };
    rows[0].earned[0].earned_at = "0".into();
    assert!(encode(AUTHORITY, &s).is_err());
    let (_, mut s) = character_view_fixture("character_equipment");
    s.subject = "01934f10-7c04-4001-805b-3b1122334401".into();
    assert!(encode(AUTHORITY, &s).is_err());
}

#[test]
fn public_projections_v2_new_families_never_published_before_source_support() {
    let activation = Activation {
        character_skills: true,
        character_achievements: true,
        character_equipment: true,
        character_outfit: true,
        ..Activation::default()
    };
    let selected: Vec<_> = Family::ALL
        .into_iter()
        .filter(|family| family.publishable(activation))
        .collect();
    for family in [
        Family::CharacterSkills,
        Family::CharacterAchievements,
        Family::CharacterEquipment,
        Family::CharacterOutfit,
    ] {
        assert!(family.enabled(activation));
        assert!(!family.produced());
        assert!(!selected.contains(&family));
    }
    assert_eq!(
        selected,
        [Family::CharacterDeaths, Family::Guild, Family::House]
    );
}

#[test]
fn public_projections_v2_existing_family_rev3_golden_bytes() {
    for (name, revision) in [("guild_rev3_example", 5), ("house_shop_rev3_example", 12)] {
        let (file, mut snapshot) = fixture(name);
        snapshot.projection_revision = revision;
        assert_eq!(
            encode(AUTHORITY, &snapshot).unwrap(),
            file.strip_suffix('\n').unwrap()
        );
    }
}

#[test]
fn public_projections_v2_equipment_conflict_fixture_is_schema_valid() {
    let (original, first) = character_view_fixture("character_equipment");
    let (conflict, changed) = character_view_fixture("character_equipment_invalid_conflict");
    assert_eq!(first.subject, changed.subject);
    assert_eq!(first.family(), changed.family());
    assert_eq!(first.projection_epoch, changed.projection_epoch);
    assert_eq!(first.projection_revision, changed.projection_revision);
    assert_ne!(first.body, changed.body);
    assert_eq!(encode(AUTHORITY, &changed).unwrap(), conflict);
    assert_ne!(original, conflict);
}

#[test]
fn public_projections_v2_invalid_fixture_bytes_are_refused() {
    for name in [
        "character_skills_invalid_types",
        "character_achievements_invalid_account",
    ] {
        let file = std::fs::read_to_string(format!("{FIXTURES}{name}_v2.json")).unwrap();
        assert!(decode_character_snapshot(file.trim_end_matches('\n').as_bytes()).is_err());
    }
    let file = std::fs::read_to_string(format!(
        "{FIXTURES}character_skills_invalid_watermark_v2.json"
    ))
    .unwrap();
    assert!(decode_watermark(file.trim_end_matches('\n').as_bytes()).is_err());
}
#[test]
fn public_projections_v2_transport_boundary_refuses_wire_only_families() {
    for name in [
        "character_skills",
        "character_achievements",
        "character_equipment",
        "character_outfit",
    ] {
        let (wire, snapshot) = character_view_fixture(name);
        assert!(decode_character_snapshot(wire.as_bytes()).is_ok());
        assert!(!publication_supported(snapshot.family().operation(), &wire));
    }
    let wire = encode_watermark(
        AUTHORITY,
        Family::CharacterSkills,
        Facts {
            projection_epoch: 1,
            oldest_undelivered_ms: None,
            now_ms: 1790000003000,
        },
        Duration::from_secs(3),
    )
    .unwrap();
    assert!(!publication_supported(
        Operation::PublishPublicProjectionWatermarkV2,
        &wire
    ));
}
#[test]
fn public_projections_v2_skills_and_equipment_reject_review_boundaries() {
    let (_, s) = character_view_fixture("character_skills");
    let original = encode(AUTHORITY, &s).unwrap();
    assert!(decode_character_snapshot(original.as_bytes()).is_ok());
    for (from, to) in [
        (r#""level":"412""#, r#""level":"0""#),
        (r#""magic_level":12"#, r#""magic_level":1001"#),
        (r#""fist_level":10"#, r#""fist_level":9"#),
        (r#""magic_level":12"#, r#""unknown":0,"magic_level":12"#),
    ] {
        assert!(decode_character_snapshot(original.replace(from, to).as_bytes()).is_err());
    }
    let (wire, _) = character_view_fixture("character_equipment");
    let bad = wire.replace("\"slot\":\"Head\"", "\"private\":0,\"slot\":\"Head\"");
    assert!(decode_character_snapshot(bad.as_bytes()).is_err());
    let row =
        r#"{"slot":"Head","item_key":"oteryn:item.fixture-helmet","item_revision":"fixture-v1"}"#;
    assert!(wire.contains(row));
    assert!(
        decode_character_snapshot(wire.replace(row, &format!("{row},{row}")).as_bytes()).is_err()
    );
    let left = row.replace("Head", "LeftHand");
    let right = row.replace("Head", "RightHand");
    assert!(
        decode_character_snapshot(wire.replace(row, &format!("{left},{right}")).as_bytes())
            .is_err()
    );
}
