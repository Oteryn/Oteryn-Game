//! The Crystal term must reach the server unchanged; do not silently extend it.
//! Crystal 201189f871d23597b82caf88fd611454a2ddefb8,
//! data/scripts/talkactions/player/war.lua:183,188-190: 1000 frags, 24 hours.
#[test]
fn mp06_crystal_one_day_term_exposes_native_rules_mismatch()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_protocol_oteryn::guild_war::{
        GuildWarIntent, decode_guild_war_intent, encode_guild_war_intent,
    };
    let opponent = [
        0x01, 0x90, 0x00, 0x00, 0x00, 0x00, 0x70, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x01,
    ];
    let intent = GuildWarIntent::Declare {
        opponent,
        frag_limit: 1000,
        duration_days: 1,
    };
    let bytes = encode_guild_war_intent(&intent).map_err(|e| format!("{e:?}"))?;
    let decoded = decode_guild_war_intent(&bytes).map_err(|e| format!("{e:?}"))?;
    assert_eq!(decoded, intent);
    // This is the named server gap, not evidence of successful war qualification.
    assert!(oteryn_gameplay_social_rules::guild::validate_guild_war_terms(1000, 1).is_err());
    Ok(())
}
