use super::{BossRewardChestEntry, encode_reward_chest, take, var};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewardChest {
    pub now: u64,
    pub entries: Vec<BossRewardChestEntry>,
}
/// Reuses the server encoder to validate the bounded personal chest projection.
pub fn decode_rewards(bytes: &[u8]) -> Option<RewardChest> {
    if bytes.len() > super::MAX_RESULT_BYTES {
        return None;
    }
    let mut at = 0;
    if var(bytes, &mut at)? != 8 {
        return None;
    }
    let now = var(bytes, &mut at)?;
    let mut entries = Vec::new();
    while at < bytes.len() {
        if entries.len() == super::REWARD_CHEST_PAGE_SIZE || var(bytes, &mut at)? != 18 {
            return None;
        }
        let body = take(bytes, &mut at)?;
        let mut pos = 0;
        if var(body, &mut pos)? != 10 {
            return None;
        }
        let death = take(body, &mut pos)?.try_into().ok()?;
        if var(body, &mut pos)? != 18 {
            return None;
        }
        let boss_key = std::str::from_utf8(take(body, &mut pos)?).ok()?.to_owned();
        if var(body, &mut pos)? != 24 {
            return None;
        }
        let minted_at_unix_ms = var(body, &mut pos)?;
        if var(body, &mut pos)? != 32 {
            return None;
        }
        let expires_at_unix_ms = var(body, &mut pos)?;
        if var(body, &mut pos)? != 40 {
            return None;
        }
        let item_count = u32::try_from(var(body, &mut pos)?).ok()?;
        if pos != body.len() {
            return None;
        }
        entries.push(BossRewardChestEntry {
            death,
            boss_key,
            minted_at_unix_ms,
            expires_at_unix_ms,
            item_count,
        });
    }
    (encode_reward_chest(now, &entries).ok()?.as_slice() == bytes)
        .then_some(RewardChest { now, entries })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chest_decoder_reuses_server_bounds_and_rejects_malformed_pages() -> Result<(), String> {
        let entry = BossRewardChestEntry {
            death: [7; 16],
            boss_key: "boss:test".into(),
            minted_at_unix_ms: 1,
            expires_at_unix_ms: 1 + crate::boss_encounter::BOSS_REWARD_EXPIRY_MS,
            item_count: 2,
        };
        let payload =
            encode_reward_chest(10, std::slice::from_ref(&entry)).map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            decode_rewards(&payload),
            Some(RewardChest {
                now: 10,
                entries: vec![entry]
            })
        );
        for end in 0..payload.len() {
            // The complete header is itself a valid empty page.
            if end != 2 {
                assert!(decode_rewards(&payload[..end]).is_none());
            }
        }
        let mut trailing = payload.clone();
        trailing.push(0);
        assert!(decode_rewards(&trailing).is_none());
        let mut duplicate = payload.clone();
        duplicate.extend([8, 10]);
        assert!(decode_rewards(&duplicate).is_none());
        assert!(decode_rewards(&vec![0; crate::boss_encounter::MAX_RESULT_BYTES + 1]).is_none());
        Ok(())
    }
}
