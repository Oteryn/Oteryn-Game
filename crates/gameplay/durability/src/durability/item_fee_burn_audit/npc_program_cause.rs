//! Distinct source-bound NPC fee proof. The immutable fee-source snapshots below
//! keep historical validation independent of later edits to the embedded catalogue.
use super::*;
use super::super::item_mint_audit::check_content_key;

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemQuestNpcProgramFeeV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub occurrence: Vec<u8>,
    #[prost(string, tag = "2")]
    pub quest_key: String,
    #[prost(bytes = "vec", tag = "3")]
    pub program_sha256: Vec<u8>,
    #[prost(string, tag = "4")]
    pub npc_key: String,
    #[prost(string, repeated, tag = "5")]
    pub selected_rule_keys: Vec<String>,
    #[prost(bytes = "vec", tag = "6")]
    pub game_session_id: Vec<u8>,
    #[prost(uint64, tag = "7")]
    pub command_id: u64,
    #[prost(bytes = "vec", tag = "8")]
    pub character_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "9")]
    pub world_id: Vec<u8>,
    #[prost(uint64, tag = "10")]
    pub amount: u64,
    #[prost(bool, tag = "11")]
    pub include_bank: bool,
    #[prost(uint32, tag = "12")]
    pub effect_ordinal: u32,
    #[prost(string, tag = "13")]
    pub source_rule_key: String,
    #[prost(string, repeated, tag = "14")]
    pub source_references: Vec<String>,
}

struct Source {
    quest: &'static str,
    digest: &'static str,
    npc: &'static str,
    rule: &'static str,
    amount: u64,
    include_bank: bool,
    references: &'static [&'static str],
}

pub(crate) fn digest_hex(digest: &[u8]) -> String {
    use std::fmt::Write;
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest { let _ = write!(&mut hex, "{byte:02x}"); }
    hex
}

pub(crate) fn check_proof(proof: &OneItemQuestNpcProgramFeeV1) -> Result<(), AuditError> {
    if proof.encoded_len() > 8_192 || proof.program_sha256.len() != 32
        || proof.command_id == 0 || proof.effect_ordinal >= 128
        || proof.selected_rule_keys.is_empty() || proof.selected_rule_keys.len() > 128
    { return Err(AuditError::InvalidInput); }
    for id in [&proof.occurrence, &proof.game_session_id, &proof.character_id, &proof.world_id] {
        check_uuid_v7(id)?;
    }
    for key in [&proof.quest_key, &proof.npc_key, &proof.source_rule_key] {
        check_content_key(key)?;
    }
    let mut unique = std::collections::BTreeSet::new();
    for key in &proof.selected_rule_keys {
        check_content_key(key)?;
        if !unique.insert(key) { return Err(AuditError::InvalidInput); }
    }
    if !unique.contains(&proof.source_rule_key) { return Err(AuditError::InvalidInput); }
    let digest = digest_hex(&proof.program_sha256);
    if !SOURCES.iter().any(|source| source.quest == proof.quest_key
        && source.digest == digest && source.npc == proof.npc_key
        && source.rule == proof.source_rule_key && source.amount == proof.amount
        && source.include_bank == proof.include_bank
        && source.references.iter().copied().eq(proof.source_references.iter().map(String::as_str)))
    { return Err(AuditError::InvalidInput); }
    Ok(())
}

pub(super) fn check(proof: &OneItemQuestNpcProgramFeeV1, fee: &OneItemFeeBurnV1) -> Result<(), AuditError> {
    check_proof(proof)?;
    if proof.character_id != fee.character_id || proof.world_id != fee.world_id
        || proof.amount != fee.fee_gold_units || (!proof.include_bank && fee.bank_debit.is_some())
    { return Err(AuditError::InvalidInput); }
    Ok(())
}
// Immutable Crystal 201189f fee snapshots. Retain historical entries when adding sources.
const SOURCES: &[Source] = &[
    Source { quest: "oteryn:quest.bigfoot_s_burden_quest", digest: "cf56427d50f832f380fd39718d94c31e3f92b152b0ff44f6fd61dc6306dff77b", npc: "oteryn:npc.gnominus", rule: "oteryn:quest-npc/bigfoot_s_burden_quest/gnominus/beer-payment-blocked", amount: 10, include_bank: true, references: &["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/gnominus.lua:81"] },
    Source { quest: "oteryn:quest.in_service_of_yalahar_quest", digest: "ea3db1bb7244ebea68e4d3e1843cf7bd7c69a9992fa6343f31161fb653525bc6", npc: "oteryn:npc.karith", rule: "oteryn:quest-npc/in_service_of_yalahar_quest/karith/branch-248-36-1-1", amount: 160, include_bank: true, references: &["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/karith.lua:248"] },
    Source { quest: "oteryn:quest.in_service_of_yalahar_quest", digest: "ea3db1bb7244ebea68e4d3e1843cf7bd7c69a9992fa6343f31161fb653525bc6", npc: "oteryn:npc.karith", rule: "oteryn:quest-npc/in_service_of_yalahar_quest/karith/branch-258-38-1-1", amount: 210, include_bank: true, references: &["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/karith.lua:258"] },
    Source { quest: "oteryn:quest.in_service_of_yalahar_quest", digest: "ea3db1bb7244ebea68e4d3e1843cf7bd7c69a9992fa6343f31161fb653525bc6", npc: "oteryn:npc.karith", rule: "oteryn:quest-npc/in_service_of_yalahar_quest/karith/branch-268-40-1-1", amount: 185, include_bank: true, references: &["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/karith.lua:268"] },
    Source { quest: "oteryn:quest.in_service_of_yalahar_quest", digest: "ea3db1bb7244ebea68e4d3e1843cf7bd7c69a9992fa6343f31161fb653525bc6", npc: "oteryn:npc.karith", rule: "oteryn:quest-npc/in_service_of_yalahar_quest/karith/branch-278-42-1-1", amount: 230, include_bank: true, references: &["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/karith.lua:278"] },
    Source { quest: "oteryn:quest.in_service_of_yalahar_quest", digest: "ea3db1bb7244ebea68e4d3e1843cf7bd7c69a9992fa6343f31161fb653525bc6", npc: "oteryn:npc.karith", rule: "oteryn:quest-npc/in_service_of_yalahar_quest/karith/branch-288-44-1-1", amount: 260, include_bank: true, references: &["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/karith.lua:288"] },
    Source { quest: "oteryn:quest.in_service_of_yalahar_quest", digest: "ea3db1bb7244ebea68e4d3e1843cf7bd7c69a9992fa6343f31161fb653525bc6", npc: "oteryn:npc.karith", rule: "oteryn:quest-npc/in_service_of_yalahar_quest/karith/branch-298-46-1-1", amount: 200, include_bank: true, references: &["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/karith.lua:298"] },
    Source { quest: "oteryn:quest.in_service_of_yalahar_quest", digest: "ea3db1bb7244ebea68e4d3e1843cf7bd7c69a9992fa6343f31161fb653525bc6", npc: "oteryn:npc.karith", rule: "oteryn:quest-npc/in_service_of_yalahar_quest/karith/branch-308-48-1-1", amount: 275, include_bank: true, references: &["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/karith.lua:308"] },
    Source { quest: "oteryn:quest.in_service_of_yalahar_quest", digest: "ea3db1bb7244ebea68e4d3e1843cf7bd7c69a9992fa6343f31161fb653525bc6", npc: "oteryn:npc.karith", rule: "oteryn:quest-npc/in_service_of_yalahar_quest/karith/branch-318-50-1-1", amount: 185, include_bank: true, references: &["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/karith.lua:318"] },
    Source { quest: "oteryn:quest.in_service_of_yalahar_quest", digest: "ea3db1bb7244ebea68e4d3e1843cf7bd7c69a9992fa6343f31161fb653525bc6", npc: "oteryn:npc.siflind", rule: "oteryn:quest-npc/in_service_of_yalahar_quest/siflind/branch-202-3-1-1", amount: 400, include_bank: true, references: &["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/siflind.lua:202","crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/siflind.lua:203"] },
    Source { quest: "oteryn:quest.the_shattered_isles_quest", digest: "e08a9c9e22621e85ba4517432db9226dbf549491cfc8162a5d42f9da7e20e39d", npc: "oteryn:npc.charlotta", rule: "oteryn:quest-npc/the_shattered_isles_quest/charlotta/errand-paid", amount: 200, include_bank: true, references: &["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/charlotta.lua:68"] },
    Source { quest: "oteryn:quest.the_shattered_isles_quest", digest: "e08a9c9e22621e85ba4517432db9226dbf549491cfc8162a5d42f9da7e20e39d", npc: "oteryn:npc.captain_waverider", rule: "oteryn:quest-npc/the_shattered_isles_quest/captain_waverider/meriana-sail", amount: 50, include_bank: true, references: &["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/captain_waverider.lua:73"] },
    Source { quest: "oteryn:quest.the_shattered_isles_quest", digest: "e08a9c9e22621e85ba4517432db9226dbf549491cfc8162a5d42f9da7e20e39d", npc: "oteryn:npc.elvith", rule: "oteryn:quest-npc/the_shattered_isles_quest/elvith/poem-buy", amount: 200, include_bank: true, references: &["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/elvith.lua:88"] },
    Source { quest: "oteryn:quest.the_shattered_isles_quest", digest: "e08a9c9e22621e85ba4517432db9226dbf549491cfc8162a5d42f9da7e20e39d", npc: "oteryn:npc.sebastian", rule: "oteryn:quest-npc/the_shattered_isles_quest/sebastian/nargor-sail", amount: 50, include_bank: true, references: &["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/npc/sebastian.lua:64"] },
];
