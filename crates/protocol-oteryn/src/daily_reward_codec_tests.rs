use super::*;

fn pick(key: &[u8], count: u64) -> Vec<u8> {
    let mut value = Vec::new();
    push_message_field(&mut value, 1, key);
    push_varint_field(&mut value, 2, count);
    let mut out = Vec::new();
    push_message_field(&mut out, 1, &value);
    out
}
#[test]
fn daily_reward_claim_codec_canonicalizes_and_bounds_every_field() -> WireResult<()> {
    let mut bytes = pick(b"oteryn:item.b", 2);
    bytes.extend(pick(b"oteryn:item.a", 3));
    let claim = DailyRewardClaim::decode(&bytes)?;
    assert_eq!(claim.picks[0].definition, "oteryn:item.a");
    assert_eq!(claim.picks.iter().map(|p| p.count).sum::<u32>(), 5);
    assert!(DailyRewardClaim::decode(&pick(b"a", 20)).is_ok());
    assert!(DailyRewardClaim::decode(&pick(&[b'a'; 512], 1)).is_ok());
    for bytes in [
        pick(b"", 1),
        pick(b"a", 0),
        pick(b"a", 21),
        pick(b"a", u64::MAX),
        pick(b"\xff", 1),
        pick(b"a\nb", 1),
        pick(&[b'a'; 513], 1),
        [pick(b"a", 1), pick(b"a", 2)].concat(),
        vec![0x08, 1],
        vec![0x0a, 255],
        vec![0; MAX_DAILY_REWARD_CLAIM_BYTES + 1],
        (0..21)
            .flat_map(|i| pick(format!("key{i}").as_bytes(), 1))
            .collect(),
    ] {
        assert!(DailyRewardClaim::decode(&bytes).is_err(), "{bytes:?}");
    }
    assert_eq!(decode_daily_reward_query(&[]), Ok(()));
    assert!(decode_daily_reward_query(&[8, 1]).is_err());
    Ok(())
}
#[test]
fn daily_reward_wall_and_result_follow_the_reserved_proto_and_bounds() -> WireResult<()> {
    let wall = DailyRewardWall {
        lane_day: 7,
        streak: 100_000,
        jokers: 3,
        claimed: true,
        claim_enabled: false,
    }
    .encode()?;
    assert!(wall.len() <= MAX_DAILY_REWARD_VIEW_BYTES);
    let mut at = 0;
    let mut parts = Vec::new();
    while at < wall.len() {
        let key = read_varint(&wall, &mut at)?;
        if key == 50 {
            parts.push(read_uint32_fields::<2>(read_bytes(&wall, &mut at)?)?);
        } else {
            let _ = read_varint(&wall, &mut at)?;
        }
    }
    assert_eq!(
        parts,
        [[1, 5], [1, 5], [2, 1], [1, 10], [3, 1], [4, 1], [5, 10]]
    );
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    assert_eq!(
        DailyRewardResult {
            disposition: 1,
            transaction_id: Some(id)
        }
        .encode()?
        .len(),
        MAX_DAILY_REWARD_RESULT_BYTES
    );
    for disposition in 2..=7 {
        assert_eq!(
            DailyRewardResult {
                disposition,
                transaction_id: None
            }
            .encode()?,
            [8, disposition]
        );
        assert!(
            DailyRewardResult {
                disposition,
                transaction_id: Some(id)
            }
            .encode()
            .is_err()
        );
    }
    assert!(
        DailyRewardResult {
            disposition: 1,
            transaction_id: None
        }
        .encode()
        .is_err()
    );
    assert!(
        DailyRewardResult {
            disposition: 0,
            transaction_id: None
        }
        .encode()
        .is_err()
    );
    Ok(())
}
