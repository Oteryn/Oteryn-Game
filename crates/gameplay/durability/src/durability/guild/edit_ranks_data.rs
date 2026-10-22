macro_rules! writer_data { ($field_visibility:vis) => {
        pub struct EditRanksWrite {
            $field_visibility node: NodeIncarnationProof,
            $field_visibility fence: CurrentCharacterItemFence,
            pub edit_ranks: GuildEditRanksRequest,
            pub binding: Vec<u8>,
        }
    }; }
#[cfg(any(test, feature = "test-support"))]
writer_data!(pub);
#[cfg(not(any(test, feature = "test-support")))]
writer_data!(pub(super));
