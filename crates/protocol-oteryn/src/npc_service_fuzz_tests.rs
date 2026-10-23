#![allow(clippy::expect_used)]

use super::*;

// TEST-FUZZ-1: bounded randomized decoder and generated-value round trips.

#[test]
fn fuzz_decode_npc_talk_intent() {
    crate::decoder_fuzz_support::codec(
        "decode_npc_talk_intent",
        |rng| talk(&"é".repeat((rng.below(127) + 1) as usize)),
        decode_npc_talk_intent,
        |v| encode_npc_talk_intent(v).expect("talk"),
    );
}

#[test]
fn fuzz_decode_npc_trade_intent() {
    crate::decoder_fuzz_support::codec(
        "decode_npc_trade_intent",
        |rng| NpcTradeIntent {
            quantity: (rng.u64() as u32).max(1),
            side: *rng.pick(&[NpcTradeSide::Buy, NpcTradeSide::Sell]),
            ..trade()
        },
        decode_npc_trade_intent,
        |v| encode_npc_trade_intent(v).expect("trade"),
    );
}

#[test]
fn fuzz_decode_npc_intent_result() {
    crate::decoder_fuzz_support::codec(
        "decode_npc_intent_result",
        |rng| *rng.pick(&NpcIntentDisposition::ALL),
        decode_npc_intent_result,
        |v| encode_npc_intent_result(*v),
    );
}

#[test]
fn fuzz_decode_npc_conversation() {
    crate::decoder_fuzz_support::codec(
        "decode_npc_conversation",
        |rng| conversation(rng.below(4) as usize, (rng.below(80) + 1) as usize),
        decode_npc_conversation,
        |v| encode_npc_conversation(v).expect("conversation"),
    );
}

#[test]
fn fuzz_decode_npc_trade_window() {
    crate::decoder_fuzz_support::codec(
        "decode_npc_trade_window",
        |rng| window(rng.below(8) as usize),
        decode_npc_trade_window,
        |v| encode_npc_trade_window(v).expect("window"),
    );
}
