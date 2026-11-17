// Conditional helper/import cfgs for dispatch's generated shared fixture mount.
pub const PATCHES: &[(&str, &str)] = &[
    (
        r####"async fn bypass(
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
async fn bypass(
"####,
    ),
    (
        r####"fn monk(tag: u8) -> TestResult<MonkStateSaveRequest> {
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
fn monk(tag: u8) -> TestResult<MonkStateSaveRequest> {
"####,
    ),
    (
        r####"fn death(tag: u8) -> TestResult<CharacterDeathRequest<2>> {
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
fn death(tag: u8) -> TestResult<CharacterDeathRequest<2>> {
"####,
    ),
    (
        r####"fn kill(tag: u8) -> TestResult<BestiaryKillRequest> {
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
fn kill(tag: u8) -> TestResult<BestiaryKillRequest> {
"####,
    ),
    (
        r####"async fn revision(pool: &sqlx::PgPool) -> TestResult<u64> {
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
async fn revision(pool: &sqlx::PgPool) -> TestResult<u64> {
"####,
    ),
    (
        r####"pub(super) fn xp_request(tag: u8) -> TestResult<ExperienceAwardRequest<2>> {
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
pub(super) fn xp_request(tag: u8) -> TestResult<ExperienceAwardRequest<2>> {
"####,
    ),
    (
        r####"    fn new(points: &[u32], stages: &[(&str, u8)]) -> Self {
"####,
        r####"    #[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
    fn new(points: &[u32], stages: &[(&str, u8)]) -> Self {
"####,
    ),
    (
        r####"struct Facts {
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
struct Facts {
"####,
    ),
    (
        r####"fn unlock(tag: u8, name: &str) -> CharmCommandRequest {
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
fn unlock(tag: u8, name: &str) -> CharmCommandRequest {
"####,
    ),
    (
        r####"fn occurrence(tag: u8) -> CharmCommandOccurrence {
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
fn occurrence(tag: u8) -> CharmCommandOccurrence {
"####,
    ),
    (
        r####"fn catalogue() -> CharmCatalogue {
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
fn catalogue() -> CharmCatalogue {
"####,
    ),
    (
        r####"fn race(name: &str) -> BestiaryRaceKey {
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
fn race(name: &str) -> BestiaryRaceKey {
"####,
    ),
    (
        r####"fn charm(name: &str) -> CharmKey {
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
fn charm(name: &str) -> CharmKey {
"####,
    ),
    (
        r####"    pub(super) async fn create(admin: String, tag: &str, initialized: bool) -> TestResult<Self> {
"####,
        r####"    #[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
    pub(super) async fn create(admin: String, tag: &str, initialized: bool) -> TestResult<Self> {
"####,
    ),
    (
        r####"pub(super) async fn join_two<A, B>(first: A, second: B) -> (A::Output, B::Output)
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
pub(super) async fn join_two<A, B>(first: A, second: B) -> (A::Output, B::Output)
"####,
    ),
    (
        r####"use crate::durability::monk_state::{
    DurableMonkState, MonkStateSaveOccurrence, MonkStateSaveOutcome, MonkStateSaveRequest,
};
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::monk_state::{
    DurableMonkState, MonkStateSaveOccurrence, MonkStateSaveOutcome, MonkStateSaveRequest,
};

#[cfg(not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4")))]
use crate::durability::monk_state::{
    DurableMonkState, MonkStateSaveOccurrence, MonkStateSaveRequest,
};
"####,
    ),
    (
        r####"use crate::durability::charm_state::{
    CharmCommand, CharmCommandOccurrence, CharmCommandOutcome, CharmCommandRequest, CharmFacts,
    CharmStateError,
};
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::charm_state::{
    CharmCommand, CharmCommandOccurrence, CharmCommandOutcome, CharmCommandRequest, CharmFacts,
    CharmStateError,
};

#[cfg(not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4")))]
use crate::durability::charm_state::{
    CharmCommand, CharmCommandOccurrence, CharmCommandRequest, CharmFacts,
};
"####,
    ),
    (
        r####"use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
"####,
    ),
    (
        r####"use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, ExperienceRewardOccurrence,
};
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, ExperienceRewardOccurrence,
};

#[cfg(not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4")))]
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, ExperienceRewardOccurrence,
};
"####,
    ),
    (
        r####"use crate::durability::bestiary_progress::{
    BestiaryKillOccurrence, BestiaryKillOutcome, BestiaryKillRequest,
};
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::bestiary_progress::{
    BestiaryKillOccurrence, BestiaryKillOutcome, BestiaryKillRequest,
};

#[cfg(not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4")))]
use crate::durability::bestiary_progress::{
    BestiaryKillOccurrence, BestiaryKillRequest,
};
"####,
    ),
    (
        r####"use std::task::Poll;
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use std::task::Poll;
"####,
    ),
    (
        r####"use std::collections::BTreeMap;
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use std::collections::BTreeMap;
"####,
    ),
    (
        r####"use oteryn_simulation_determinism::{ExactI64, RoundingMode};
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use oteryn_simulation_determinism::{ExactI64, RoundingMode};
"####,
    ),
    (
        r####"use crate::durability::monk_state::{
    DurableMonkState, MonkStateSaveOccurrence, MonkStateSaveRequest,
};
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::monk_state::{
    DurableMonkState, MonkStateSaveOccurrence, MonkStateSaveRequest,
};
"####,
    ),
    (
        r####"use crate::durability::charm_state::{
    CharmCommand, CharmCommandOccurrence, CharmCommandRequest, CharmFacts,
};
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::charm_state::{
    CharmCommand, CharmCommandOccurrence, CharmCommandRequest, CharmFacts,
};

#[cfg(not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4")))]
#[cfg(not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4")))]
use crate::durability::charm_state::CharmFacts;
"####,
    ),
    (
        r####"use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, ExperienceRewardOccurrence,
};
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, ExperienceRewardOccurrence,
};

#[cfg(not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4")))]
#[cfg(not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4")))]
use crate::durability::character_progression::CurrentCharacterGameplayFence;
"####,
    ),
    (
        r####"use crate::durability::character_death::{CharacterDeathRequest, DeathCell, PlayerDeathOccurrence};
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::character_death::{CharacterDeathRequest, DeathCell, PlayerDeathOccurrence};
"####,
    ),
    (
        r####"use crate::durability::character_authority::ReconciledCharacterAuthority;
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::character_authority::ReconciledCharacterAuthority;
"####,
    ),
    (
        r####"use crate::durability::bestiary_progress::{
    BestiaryKillOccurrence, BestiaryKillRequest,
};
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::bestiary_progress::{
    BestiaryKillOccurrence, BestiaryKillRequest,
};
"####,
    ),
    (
        r####"use crate::domain::progression::{
    FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
};
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::domain::progression::{
    FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
};
"####,
    ),
    (
        r####"use crate::domain::charm::{
    BestiaryRaceKey, BestiaryStage, CharmCatalogue, CharmCategory, CharmDefinition, CharmKey,
    CharmSlotEntitlement,
};
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::domain::charm::{
    BestiaryRaceKey, BestiaryStage, CharmCatalogue, CharmCategory, CharmDefinition, CharmKey,
    CharmSlotEntitlement,
};

#[cfg(not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4")))]
use crate::domain::charm::{
    BestiaryRaceKey, BestiaryStage,
    CharmSlotEntitlement,
};
"####,
    ),
    (
        r####"impl Facts {
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
impl Facts {
"####,
    ),
    (
        r####"impl CharmFacts for Facts {
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
impl CharmFacts for Facts {
"####,
    ),
    (
        r####"use crate::durability::{DurabilityError, DurabilityRoot};
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::{DurabilityError, DurabilityRoot};

#[cfg(not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4")))]
use crate::durability::DurabilityRoot;
"####,
    ),
    (
        r####"use crate::durability::charm_state::CharmFacts;
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::charm_state::CharmFacts;
"####,
    ),
    (
        r####"use crate::domain::charm::{
    BestiaryRaceKey, BestiaryStage,
    CharmSlotEntitlement,
};
"####,
        r####"#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::domain::charm::{
    BestiaryRaceKey, BestiaryStage,
    CharmSlotEntitlement,
};
"####,
    ),
    (
        r"async fn bypass(
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
async fn bypass(
"#,
    ),
    (
        r"fn monk(tag: u8) -> TestResult<MonkStateSaveRequest> {
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
fn monk(tag: u8) -> TestResult<MonkStateSaveRequest> {
"#,
    ),
    (
        r"fn death(tag: u8) -> TestResult<CharacterDeathRequest<2>> {
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
fn death(tag: u8) -> TestResult<CharacterDeathRequest<2>> {
"#,
    ),
    (
        r"fn kill(tag: u8) -> TestResult<BestiaryKillRequest> {
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
fn kill(tag: u8) -> TestResult<BestiaryKillRequest> {
"#,
    ),
    (
        r"async fn revision(pool: &sqlx::PgPool) -> TestResult<u64> {
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
async fn revision(pool: &sqlx::PgPool) -> TestResult<u64> {
"#,
    ),
    (
        r"pub(super) fn xp_request(tag: u8) -> TestResult<ExperienceAwardRequest<2>> {
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
pub(super) fn xp_request(tag: u8) -> TestResult<ExperienceAwardRequest<2>> {
"#,
    ),
    (
        r"impl CharmFacts for Facts {
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
impl CharmFacts for Facts {
"#,
    ),
    (
        r"    fn new(points: &[u32], stages: &[(&str, u8)]) -> Self {
",
        r#"    #[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
    #[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
    #[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
    fn new(points: &[u32], stages: &[(&str, u8)]) -> Self {
"#,
    ),
    (
        r"impl Facts {
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
impl Facts {
"#,
    ),
    (
        r"struct Facts {
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
struct Facts {
"#,
    ),
    (
        r"fn unlock(tag: u8, name: &str) -> CharmCommandRequest {
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
fn unlock(tag: u8, name: &str) -> CharmCommandRequest {
"#,
    ),
    (
        r"fn occurrence(tag: u8) -> CharmCommandOccurrence {
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
fn occurrence(tag: u8) -> CharmCommandOccurrence {
"#,
    ),
    (
        r"fn catalogue() -> CharmCatalogue {
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
fn catalogue() -> CharmCatalogue {
"#,
    ),
    (
        r"fn race(name: &str) -> BestiaryRaceKey {
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
fn race(name: &str) -> BestiaryRaceKey {
"#,
    ),
    (
        r"fn charm(name: &str) -> CharmKey {
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
fn charm(name: &str) -> CharmKey {
"#,
    ),
    (
        r"use crate::durability::monk_state::{
    DurableMonkState, MonkStateSaveOccurrence, MonkStateSaveOutcome, MonkStateSaveRequest,
};
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
use crate::durability::monk_state::{
    DurableMonkState, MonkStateSaveOccurrence, MonkStateSaveOutcome, MonkStateSaveRequest,
};

#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(all(feature = "pg-tests-g2", not(any(feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))), all(feature = "pg-tests-g3", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))), all(feature = "pg-tests-g4", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3")))))]
use crate::durability::monk_state::{
    DurableMonkState, MonkStateSaveOccurrence, MonkStateSaveRequest,
};
"#,
    ),
    (
        r"use crate::durability::charm_state::{
    CharmCommand, CharmCommandOccurrence, CharmCommandOutcome, CharmCommandRequest, CharmFacts,
    CharmStateError,
};
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
use crate::durability::charm_state::{
    CharmCommand, CharmCommandOccurrence, CharmCommandOutcome, CharmCommandRequest, CharmFacts,
    CharmStateError,
};

#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(all(feature = "pg-tests-g2", not(any(feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))), all(feature = "pg-tests-g3", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))), all(feature = "pg-tests-g4", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3")))))]
use crate::durability::charm_state::{
    CharmCommand, CharmCommandOccurrence, CharmCommandRequest, CharmFacts,
};
"#,
    ),
    (
        r"use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
use crate::durability::character_revision_sequencer::CharacterRevisionSequencer;
"#,
    ),
    (
        r"use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, ExperienceRewardOccurrence,
};
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, ExperienceRewardOccurrence,
};

#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(all(feature = "pg-tests-g2", not(any(feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))), all(feature = "pg-tests-g3", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))), all(feature = "pg-tests-g4", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3")))))]
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, ExperienceRewardOccurrence,
};
"#,
    ),
    (
        r"use crate::durability::bestiary_progress::{
    BestiaryKillOccurrence, BestiaryKillOutcome, BestiaryKillRequest,
};
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
use crate::durability::bestiary_progress::{
    BestiaryKillOccurrence, BestiaryKillOutcome, BestiaryKillRequest,
};

#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(all(feature = "pg-tests-g2", not(any(feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))), all(feature = "pg-tests-g3", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))), all(feature = "pg-tests-g4", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3")))))]
use crate::durability::bestiary_progress::{
    BestiaryKillOccurrence, BestiaryKillRequest,
};
"#,
    ),
    (
        r"use std::collections::BTreeMap;
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use std::collections::BTreeMap;
"#,
    ),
    (
        r"use oteryn_simulation_determinism::{ExactI64, RoundingMode};
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use oteryn_simulation_determinism::{ExactI64, RoundingMode};
"#,
    ),
    (
        r"use crate::durability::{DurabilityError, DurabilityRoot};
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::{DurabilityError, DurabilityRoot};

#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(all(feature = "pg-tests-g2", not(any(feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))))]
use crate::durability::DurabilityRoot;
"#,
    ),
    (
        r#"#[cfg(any(all(feature = "pg-tests-g2", not(any(feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))), all(feature = "pg-tests-g3", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))), all(feature = "pg-tests-g4", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3")))))]
use crate::durability::monk_state::{
    DurableMonkState, MonkStateSaveOccurrence, MonkStateSaveRequest,
};
"#,
        r#"#[cfg(any(all(feature = "pg-tests-g2", not(any(feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))), all(feature = "pg-tests-g3", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))), all(feature = "pg-tests-g4", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3")))))]
#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::monk_state::{
    DurableMonkState, MonkStateSaveOccurrence, MonkStateSaveRequest,
};
"#,
    ),
    (
        r#"#[cfg(any(all(feature = "pg-tests-g2", not(any(feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))), all(feature = "pg-tests-g3", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))), all(feature = "pg-tests-g4", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3")))))]
use crate::durability::charm_state::{
    CharmCommand, CharmCommandOccurrence, CharmCommandRequest, CharmFacts,
};
"#,
        r#"#[cfg(any(all(feature = "pg-tests-g2", not(any(feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))), all(feature = "pg-tests-g3", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))), all(feature = "pg-tests-g4", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3")))))]
#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::charm_state::{
    CharmCommand, CharmCommandOccurrence, CharmCommandRequest, CharmFacts,
};
"#,
    ),
    (
        r#"#[cfg(any(all(feature = "pg-tests-g2", not(any(feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))), all(feature = "pg-tests-g3", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))), all(feature = "pg-tests-g4", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3")))))]
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, ExperienceRewardOccurrence,
};
"#,
        r#"#[cfg(any(all(feature = "pg-tests-g2", not(any(feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))), all(feature = "pg-tests-g3", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))), all(feature = "pg-tests-g4", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3")))))]
#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, ExperienceRewardOccurrence,
};

#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(all(feature = "pg-tests-g2", not(any(feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))), all(feature = "pg-tests-g3", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))), all(feature = "pg-tests-g4", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3")))))]
#[cfg(all(feature = "pg-tests-g2", not(any(feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))))]
use crate::durability::character_progression::CurrentCharacterGameplayFence;
"#,
    ),
    (
        r"use crate::durability::character_death::{CharacterDeathRequest, DeathCell, PlayerDeathOccurrence};
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::character_death::{CharacterDeathRequest, DeathCell, PlayerDeathOccurrence};
"#,
    ),
    (
        r"use crate::durability::character_authority::ReconciledCharacterAuthority;
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::character_authority::ReconciledCharacterAuthority;
"#,
    ),
    (
        r#"#[cfg(any(all(feature = "pg-tests-g2", not(any(feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))), all(feature = "pg-tests-g3", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))), all(feature = "pg-tests-g4", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3")))))]
use crate::durability::bestiary_progress::{
    BestiaryKillOccurrence, BestiaryKillRequest,
};
"#,
        r#"#[cfg(any(all(feature = "pg-tests-g2", not(any(feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))), all(feature = "pg-tests-g3", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))), all(feature = "pg-tests-g4", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3")))))]
#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::bestiary_progress::{
    BestiaryKillOccurrence, BestiaryKillRequest,
};
"#,
    ),
    (
        r"use crate::domain::progression::{
    FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
};
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::domain::progression::{
    FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
};
"#,
    ),
    (
        r"use crate::domain::charm::{
    BestiaryRaceKey, BestiaryStage, CharmCatalogue, CharmCategory, CharmDefinition, CharmKey,
    CharmSlotEntitlement,
};
",
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::domain::charm::{
    BestiaryRaceKey, BestiaryStage, CharmCatalogue, CharmCategory, CharmDefinition, CharmKey,
    CharmSlotEntitlement,
};
"#,
    ),
    (
        r"use std::collections::BTreeMap;
",
        r#"#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
use std::collections::BTreeMap;
"#,
    ),
    (
        r"use crate::durability::{DurabilityError, DurabilityRoot};
",
        r#"#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
use crate::durability::{DurabilityError, DurabilityRoot};

#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(all(feature = "pg-tests-g3", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))))]
use crate::durability::DurabilityRoot;
"#,
    ),
    (
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::monk_state::{
    DurableMonkState, MonkStateSaveOccurrence, MonkStateSaveRequest,
};
"#,
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
use crate::durability::monk_state::{
    DurableMonkState, MonkStateSaveOccurrence, MonkStateSaveRequest,
};
"#,
    ),
    (
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::charm_state::{
    CharmCommand, CharmCommandOccurrence, CharmCommandRequest, CharmFacts,
};
"#,
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
use crate::durability::charm_state::{
    CharmCommand, CharmCommandOccurrence, CharmCommandRequest, CharmFacts,
};
"#,
    ),
    (
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, ExperienceRewardOccurrence,
};
"#,
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, ExperienceRewardOccurrence,
};

#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(all(feature = "pg-tests-g2", not(any(feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))), all(feature = "pg-tests-g3", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))), all(feature = "pg-tests-g4", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3")))))]
#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(all(feature = "pg-tests-g3", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))))]
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, ExperienceAwardRequest, ExperienceRewardOccurrence,
};
"#,
    ),
    (
        r"use crate::durability::character_death::{CharacterDeathRequest, DeathCell, PlayerDeathOccurrence};
",
        r#"#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
use crate::durability::character_death::{CharacterDeathRequest, DeathCell, PlayerDeathOccurrence};
"#,
    ),
    (
        r"use crate::durability::character_authority::ReconciledCharacterAuthority;
",
        r#"#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
use crate::durability::character_authority::ReconciledCharacterAuthority;
"#,
    ),
    (
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
use crate::durability::bestiary_progress::{
    BestiaryKillOccurrence, BestiaryKillRequest,
};
"#,
        r#"#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
use crate::durability::bestiary_progress::{
    BestiaryKillOccurrence, BestiaryKillRequest,
};
"#,
    ),
    (
        r"use crate::domain::charm::{
    BestiaryRaceKey, BestiaryStage, CharmCatalogue, CharmCategory, CharmDefinition, CharmKey,
    CharmSlotEntitlement,
};
",
        r#"#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
use crate::domain::charm::{
    BestiaryRaceKey, BestiaryStage, CharmCatalogue, CharmCategory, CharmDefinition, CharmKey,
    CharmSlotEntitlement,
};
"#,
    ),
    (
        r"use std::collections::BTreeMap;
",
        r#"#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
use std::collections::BTreeMap;
"#,
    ),
    (
        r"use oteryn_simulation_determinism::{ExactI64, RoundingMode};
",
        r#"#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
use oteryn_simulation_determinism::{ExactI64, RoundingMode};
"#,
    ),
    (
        r"use crate::durability::{DurabilityError, DurabilityRoot};
",
        r#"#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
use crate::durability::{DurabilityError, DurabilityRoot};

#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(all(feature = "pg-tests-g4", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))))]
use crate::durability::DurabilityRoot;
"#,
    ),
    (
        r#"#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
use crate::durability::monk_state::{
    DurableMonkState, MonkStateSaveOccurrence, MonkStateSaveRequest,
};
"#,
        r#"#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
use crate::durability::monk_state::{
    DurableMonkState, MonkStateSaveOccurrence, MonkStateSaveRequest,
};
"#,
    ),
    (
        r#"#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
use crate::durability::charm_state::{
    CharmCommand, CharmCommandOccurrence, CharmCommandRequest, CharmFacts,
};
"#,
        r#"#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
use crate::durability::charm_state::{
    CharmCommand, CharmCommandOccurrence, CharmCommandRequest, CharmFacts,
};
"#,
    ),
    (
        r#"#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, ExperienceRewardOccurrence,
};
"#,
        r#"#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
use crate::durability::character_progression::{
    CurrentCharacterGameplayFence, ExperienceAwardRequest,
    ExperienceCommitOutcome, ExperienceRewardOccurrence,
};

#[cfg(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(all(feature = "pg-tests-g2", not(any(feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))), all(feature = "pg-tests-g3", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))), all(feature = "pg-tests-g4", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3")))))]
#[cfg(any(not(feature = "pg-tests-g2"), feature = "pg-tests-g1", feature = "pg-tests-g3", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(all(feature = "pg-tests-g4", not(any(feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))))]
use crate::durability::character_progression::CurrentCharacterGameplayFence;
"#,
    ),
    (
        r"use crate::durability::character_death::{CharacterDeathRequest, DeathCell, PlayerDeathOccurrence};
",
        r#"#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
use crate::durability::character_death::{CharacterDeathRequest, DeathCell, PlayerDeathOccurrence};
"#,
    ),
    (
        r"use crate::durability::character_authority::ReconciledCharacterAuthority;
",
        r#"#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
use crate::durability::character_authority::ReconciledCharacterAuthority;
"#,
    ),
    (
        r#"#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
use crate::durability::bestiary_progress::{
    BestiaryKillOccurrence, BestiaryKillRequest,
};
"#,
        r#"#[cfg(any(not(feature = "pg-tests-g3"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g4"))]
#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
use crate::durability::bestiary_progress::{
    BestiaryKillOccurrence, BestiaryKillRequest,
};
"#,
    ),
    (
        r"use crate::domain::progression::{
    FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
};
",
        r#"#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
use crate::domain::progression::{
    FiniteProgressionPolicy, LevelThreshold, ProgressionRevisionContext,
};
"#,
    ),
    (
        r"use crate::domain::charm::{
    BestiaryRaceKey, BestiaryStage, CharmCatalogue, CharmCategory, CharmDefinition, CharmKey,
    CharmSlotEntitlement,
};
",
        r#"#[cfg(any(not(feature = "pg-tests-g4"), feature = "pg-tests-g1", feature = "pg-tests-g2", feature = "pg-tests-g3"))]
use crate::domain::charm::{
    BestiaryRaceKey, BestiaryStage, CharmCatalogue, CharmCategory, CharmDefinition, CharmKey,
    CharmSlotEntitlement,
};
"#,
    ),
];
