//! MARKET-1: native migration-backed storage cases, auto-mounted exactly once.
#![allow(clippy::expect_used)]
use crate::bestiary_postgres_harness as harness;
use crate::domain::{CharacterId, WorldId};
use crate::durability::DurabilityError;
use crate::durability::market::*;
use crate::foundation::{CommandId, CommandRef, GameSessionId};
use harness::{CHARACTER, TestResult, WORLD, id};
use sqlx::{Connection, Executor, PgConnection};

#[path = "market_postgres_cases/fixture.rs"]
mod fixture;
use fixture::*;
#[path = "market_postgres_cases/operation_tests.rs"]
mod operation_tests;
#[path = "market_postgres_cases/reads_tests.rs"]
mod reads_tests;
#[path = "market_postgres_cases/selection_tests.rs"]
mod selection_tests;
#[path = "market_postgres_cases/storage_tests.rs"]
mod storage_tests;
