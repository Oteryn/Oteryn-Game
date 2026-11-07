macro_rules! writer_data { ($field_visibility:vis) => {
        pub struct ResignWrite {
            $field_visibility node: NodeIncarnationProof,
            $field_visibility fence: CurrentCharacterItemFence,
            pub resign: GuildResignRequest,
            pub binding: Vec<u8>,
            pub accounts: Option<[[u8; 16]; 2]>,
        }
    }; }
#[cfg(any(test, feature = "test-support"))]
writer_data!(pub);
#[cfg(not(any(test, feature = "test-support")))]
writer_data!(pub(super));
