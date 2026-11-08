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
