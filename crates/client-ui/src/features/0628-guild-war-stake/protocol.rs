use oteryn_client_runtime::guild_war::{
    GuildWarIntent, GuildWarResult, MAX_GUILD_WAR_STAKE, decode_guild_war_result,
};

/// Add the agreed amount only after the server selected the stake extension.
pub fn with_stake(
    intent: GuildWarIntent,
    selected: bool,
    draft: &str,
) -> Result<GuildWarIntent, &'static str> {
    let draft = draft.trim();
    if draft.is_empty() {
        return Ok(intent);
    }
    if !selected {
        return Err(tr!("Unavailable in this session"));
    }
    let stake = draft
        .parse::<u32>()
        .ok()
        .filter(|&value| value <= MAX_GUILD_WAR_STAKE)
        .ok_or("Enter a stake of 0 to 2,000,000,000 gold")?;
    match intent {
        GuildWarIntent::Declare {
            opponent,
            frag_limit,
            duration_days,
        } => Ok(GuildWarIntent::DeclareWithStake {
            opponent,
            frag_limit,
            duration_days,
            stake,
        }),
        GuildWarIntent::Accept { war } => Ok(GuildWarIntent::AcceptWithStake { war, stake }),
        _ => Err(tr!("The server refused the guild war request")),
    }
}

/// Decode the confirmed response through the unchanged command 119 result codec.
pub fn result(payload: &[u8]) -> Option<GuildWarResult> {
    decode_guild_war_result(payload).ok()
}

#[cfg(test)]
#[test]
fn acceptance_sends_exact_terms_only_with_selection() -> Result<(), &'static str> {
    let legacy = GuildWarIntent::Accept { war: [1; 16] };
    assert_eq!(with_stake(legacy, false, "")?, legacy);
    assert!(with_stake(legacy, false, "2000").is_err());
    assert_eq!(with_stake(legacy, true, " 2000 ")?.stake(), Some(2000));
    assert_eq!(with_stake(legacy, true, "0")?.stake(), Some(0));
    assert!(with_stake(legacy, true, "2000000001").is_err());
    let reply = GuildWarResult {
        code: 1,
        war: [1, 1, 1, 1, 1, 1, 0x71, 1, 0x81, 1, 1, 1, 1, 1, 1, 1],
    };
    let response = oteryn_client_runtime::guild_war::encode_guild_war_result(&reply)
        .map_err(|_| "guild war response could not be encoded")?;
    assert_eq!(result(&response), Some(reply));
    Ok(())
}
