// Same request DATA; tests may mutate one invariant without widening the shipping surface.
macro_rules! request_data {
    ($field_visibility:vis) => {
        #[derive(Debug, Clone)]
        pub struct GuildDisbandRequest {
            $field_visibility common: GuildRequest,
            $field_visibility confirmation: String,
        }
        #[derive(Debug, Clone)]
        pub struct GuildTextRequest {
            $field_visibility common: GuildRequest,
            $field_visibility expected_revision: u64,
            $field_visibility text: String,
        }
        #[derive(Debug, Clone)]
        pub struct GuildRankRequest {
            $field_visibility common: GuildRequest,
            $field_visibility expected_revision: u64,
            $field_visibility level: GuildRank,
        }
        #[derive(Debug, Clone)]
        pub struct GuildResignRequest {
            $field_visibility common: GuildRequest,
            $field_visibility expected_revision: u64,
        }
        #[derive(Debug, Clone)]
        pub struct GuildEditRanksRequest {
            $field_visibility common: GuildRequest,
            $field_visibility expected_revision: u64,
            $field_visibility names: Vec<String>,
        }
    };
}
#[cfg(any(test, feature = "test-support"))]
request_data!(pub);
#[cfg(not(any(test, feature = "test-support")))]
request_data!(pub(super));
