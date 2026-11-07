use super::*;

#[test]
fn boss_cooldown_codec_bounds() -> Result<(), Error> {
    assert_eq!(decode_cooldown_query(&[])?, "");
    let key = format!("oteryn:boss.{}", "a".repeat(500));
    assert_eq!(key.len(), MAX_BOSS_KEY_BYTES);
    let mut query = Vec::new();
    push_message_field(&mut query, 1, key.as_bytes());
    assert_eq!(query.len(), MAX_QUERY_BYTES);
    assert_eq!(decode_cooldown_query(&query)?, key);
    let mut namespaced = Vec::new();
    push_message_field(&mut namespaced, 1, b"pack:Boss/A");
    assert_eq!(decode_cooldown_query(&namespaced)?, "pack:Boss/A");
    let entries = (0..COOLDOWN_PAGE_SIZE)
        .map(|i| BossCooldownEntry {
            boss_key: format!("oteryn:boss.{i:03}{}", "a".repeat(497)),
            available_at_unix_ms: i64::MAX as u64,
        })
        .collect::<Vec<_>>();
    assert!(encode_cooldown_list(i64::MAX as u64, &entries, true)?.len() <= MAX_RESULT_BYTES);
    assert!(encode_cooldown_list(0, &entries[..63], true).is_err());
    let mut bad = entries.clone();
    bad.push(entries[0].clone());
    assert!(encode_cooldown_list(0, &bad, false).is_err());
    let mut bad = entries;
    bad.swap(0, 1);
    assert!(encode_cooldown_list(0, &bad, false).is_err());
    for bad in [
        vec![0],
        vec![10, 1, 0xff],
        vec![10, 0, 10, 0],
        vec![10, 0, 16, 1],
        vec![10, 2, b'a'],
        vec![0; MAX_QUERY_BYTES + 1],
    ] {
        assert!(decode_cooldown_query(&bad).is_err());
    }
    let mut bad = Vec::new();
    push_message_field(&mut bad, 1, format!("{key}a").as_bytes());
    assert!(decode_cooldown_query(&bad).is_err());
    assert!(
        encode_cooldown_list(
            0,
            &[BossCooldownEntry {
                boss_key: "unqualified".into(),
                available_at_unix_ms: 0
            }],
            false
        )
        .is_err()
    );
    assert!(
        encode_cooldown_list(
            0,
            &[BossCooldownEntry {
                boss_key: key,
                available_at_unix_ms: u64::MAX
            }],
            false
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn bosstiary_progress_is_bounded_and_ordered() -> Result<(), Error> {
    let progress = vec![
        BosstiaryProgressEntry {
            boss_key: "oteryn:boss.a".into(),
            kill_count: 5,
        },
        BosstiaryProgressEntry {
            boss_key: "oteryn:boss.b".into(),
            kill_count: 60,
        },
    ];
    let encoded = encode_bosstiary_list(1, &[], false, &progress, 90, false)?;
    assert!(encoded.windows(2).any(|field| field == [40, 90]));
    assert!(
        encoded
            .windows(b"oteryn:boss.a".len())
            .any(|field| field == b"oteryn:boss.a")
    );
    let mut reversed = progress;
    reversed.reverse();
    assert!(encode_bosstiary_list(1, &[], false, &reversed, 90, false).is_err());
    Ok(())
}

#[test]
fn boss_lever_wire_never_accepts_client_policy() -> Result<(), Error> {
    let encoded = encode_boss_key("oteryn:boss.test")?;
    assert_eq!(decode_boss_key(&encoded)?.boss_key, "oteryn:boss.test");
    let mut extra = encoded.clone();
    push_varint_field(&mut extra, 2, 5);
    assert!(decode_boss_key(&extra).is_err());
    assert!(decode_boss_key(&[]).is_err());
    assert!(encode_boss_key("unqualified").is_err());
    let receipt = encode_instance_receipt(&[7; 16], i64::MAX as u64)?;
    assert!(!receipt.is_empty());
    assert!(encode_instance_receipt(&[7; 16], u64::MAX).is_err());
    Ok(())
}

#[test]
fn srx3_taints_query_accepts_explicit_opt_in_without_changing_legacy_cursor() {
    assert_eq!(decode_cooldown_query(&[16, 1]), Ok(""));
    assert_eq!(
        decode_cooldown_query(&[10, 3, b'a', b':', b'b', 16, 1]),
        Ok("a:b")
    );
    for malformed in [&[16, 0][..], &[16, 2], &[16, 1, 16, 1], &[16], &[16, 1, 0]] {
        assert!(decode_cooldown_query(malformed).is_err());
    }
}

#[test]
fn srx3_taints_codec_bounds_and_refusal_preserve_the_original_payload() -> Result<(), Error> {
    let key = format!("oteryn:boss.{}", "a".repeat(500));
    let mut query = Vec::new();
    push_message_field(&mut query, 1, key.as_bytes());
    query.extend_from_slice(&[16, 1]);
    assert_eq!(query.len(), MAX_TAINT_QUERY_BYTES);
    assert_eq!(decode_cooldown_query(&query)?, key);
    let mut bytes = encode_bosstiary_list(0, &[], false, &[], 0, false)?;
    let before = bytes.clone();
    let taints = BossTaints {
        soul_war: 5,
        rotten_blood: 4,
        rotten_blood_flags: 15,
        rotten_blood_final: true,
    };
    append_boss_taints(&mut bytes, taints)?;
    assert_eq!(&bytes[..before.len()], &before);
    assert!(bytes.ends_with(&[58, 8, 8, 5, 16, 4, 24, 15, 32, 1]));
    for invalid in [
        BossTaints {
            soul_war: 6,
            ..taints
        },
        BossTaints {
            rotten_blood: 5,
            ..taints
        },
        BossTaints {
            rotten_blood_flags: 16,
            ..taints
        },
    ] {
        let mut bytes = before.clone();
        assert_eq!(
            append_boss_taints(&mut bytes, invalid),
            Err(Error::Malformed)
        );
        assert_eq!(bytes, before);
    }
    let mut full = vec![0; MAX_RESULT_BYTES - 9];
    assert_eq!(
        append_boss_taints(&mut full, taints),
        Err(Error::LimitExceeded)
    );
    assert_eq!(full.len(), MAX_RESULT_BYTES - 9);
    Ok(())
}
