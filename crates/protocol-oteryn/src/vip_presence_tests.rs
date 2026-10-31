use super::*;

fn entry(status: Status) -> PresenceEntry {
    PresenceEntry {
        entry: 1,
        name: "Alice".into(),
        status,
        announce: status != Status::Unconfirmed,
        confirmed_for_ms: if status == Status::Online {
            CONFIRMED_FOR_MS
        } else {
            0
        },
    }
}

#[test]
fn vip_presence_statuses_round_trip_and_only_online_has_a_lifetime() -> Result<(), Error> {
    for status in [Status::Unconfirmed, Status::Offline, Status::Online] {
        let batch = PresenceBatch {
            entries: vec![entry(status)],
        };
        assert_eq!(decode(&encode(&batch)?)?, batch);
    }
    assert_eq!(decode(&[])?, PresenceBatch::default());
    Ok(())
}

#[test]
fn vip_presence_refuses_unknown_duplicate_and_expired_fields() -> Result<(), Error> {
    let batch = PresenceBatch {
        entries: vec![entry(Status::Online)],
    };
    let valid = encode(&batch)?;
    let mut invalid = valid.clone();
    invalid.extend([16, 0]);
    assert!(decode(&invalid).is_err());
    let mut duplicate = valid.clone();
    duplicate.extend(&valid);
    assert!(decode(&duplicate).is_err());
    for (status, lifetime, announce) in [
        (Status::Online, 0, false),
        (Status::Online, CONFIRMED_FOR_MS + 1, false),
        (Status::Offline, 1, false),
        (Status::Unconfirmed, 0, true),
    ] {
        let mut value = entry(status);
        value.confirmed_for_ms = lifetime;
        value.announce = announce;
        assert!(
            encode(&PresenceBatch {
                entries: vec![value]
            })
            .is_err()
        );
    }
    for truncated in 1..valid.len() {
        assert!(decode(&valid[..truncated]).is_err());
    }
    Ok(())
}

#[test]
fn vip_presence_caps_and_local_numbers_are_enforced() {
    let mut batch = PresenceBatch {
        entries: vec![entry(Status::Offline); MAX_ENTRIES + 1],
    };
    assert!(encode(&batch).is_err());
    batch.entries.truncate(1);
    batch.entries[0].entry = 0;
    assert!(encode(&batch).is_err());
    batch.entries[0].entry = 1;
    batch.entries[0].name = "\n".into();
    assert!(encode(&batch).is_err());
}
