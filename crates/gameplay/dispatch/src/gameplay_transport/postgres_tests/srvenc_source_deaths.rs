//! Actual native lethal capture → existing queue → fenced source quest progress.
//! Coordinates and the pre-existing source mission stage are explicit fixture data.
use super::{TestResult, srvenc_source_deaths_fixtures::scenario};

macro_rules! native_case {
    ($name:ident,$creature:literal) => {
        #[test]
        fn $name() -> TestResult {
            scenario(stringify!($name), $creature)
        }
    };
}
native_case!(srvenc_m24_postgres_anomaly_room, "anomaly");
native_case!(srvenc_m24_postgres_world_devourer_room, "world_devourer");
native_case!(srvenc_m24_postgres_court_warlock_room, "court_warlock");
native_case!(srvenc_m24_postgres_goblin_leader_damage, "goblin_leader");
native_case!(srvenc_m24_postgres_magma_bubble_damage, "magma_bubble");
native_case!(srvenc_m24_postgres_cake_golem_twenty_and_cap, "cake_golem");
native_case!(srvenc_m24_postgres_omruc_stage, "omruc");
native_case!(srvenc_m24_postgres_thalas_stage, "thalas");
native_case!(srvenc_m24_postgres_dipthrah_stage, "dipthrah");
native_case!(srvenc_m24_postgres_mahrdis_stage, "mahrdis");
native_case!(srvenc_m24_postgres_vashresamun_stage, "vashresamun");
native_case!(srvenc_m24_postgres_morguthis_stage, "morguthis");
native_case!(srvenc_m24_postgres_rahemos_stage, "rahemos");
native_case!(srvenc_m24_postgres_herald_top_damage, "herald_of_fire");
