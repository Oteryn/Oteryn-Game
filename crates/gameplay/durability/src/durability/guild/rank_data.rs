macro_rules! writer_data { ($field_visibility:vis) => {
        pub struct RankWrite {
            $field_visibility node: NodeIncarnationProof,
            $field_visibility fence: CurrentCharacterItemFence,
            pub rank: GuildRankRequest,
            pub binding: Vec<u8>,
            pub account: Option<[u8; 16]>,
        }
    }; }
#[cfg(any(test, feature = "test-support"))]
writer_data!(pub);
#[cfg(not(any(test, feature = "test-support")))]
writer_data!(pub(super));
