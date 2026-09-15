use super::artifact::*;
use super::model::*;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledContent {
    pub graph: CanonicalGraph,
    pub server_artifact: Vec<u8>,
    pub client_artifact: Vec<u8>,
    server_digest: [u8; 32],
    client_digest: [u8; 32],
}

impl CompiledContent {
    pub const fn server_digest(&self) -> [u8; 32] {
        self.server_digest
    }
    pub const fn client_digest(&self) -> [u8; 32] {
        self.client_digest
    }
    pub fn expectation(&self) -> ArtifactExpectation {
        ArtifactExpectation::from_graph(&self.graph)
    }
}

pub fn compile(
    source: &FixtureSource,
    limits: &EvidenceLimits,
    target: CompileTarget,
) -> Result<CompiledContent, ContentError> {
    if target == CompileTarget::OrdinaryRelease {
        return Err(ContentError::FixtureOnlyReleaseRejected);
    }
    let graph = canonicalize(source, limits)?;
    let server_records = server_records(&graph)?;
    let client_records = client_records(&graph);
    let server_metadata =
        ArtifactMetadata::from_graph(&graph, ProjectionClass::ServerAuthoritative);
    let client_metadata = ArtifactMetadata::from_graph(&graph, ProjectionClass::ClientSafe);
    let server = encode_artifact(&server_metadata, &server_records, limits)?;
    let client = encode_artifact(&client_metadata, &client_records, limits)?;
    Ok(CompiledContent {
        graph,
        server_artifact: server.bytes,
        client_artifact: client.bytes,
        server_digest: server.digest,
        client_digest: client.digest,
    })
}

fn canonicalize(
    source: &FixtureSource,
    limits: &EvidenceLimits,
) -> Result<CanonicalGraph, ContentError> {
    if source.schema_version != FIXTURE_SCHEMA_VERSION {
        return Err(ContentError::InvalidArtifact("unsupported fixture schema"));
    }
    validate_source_strings(source, limits)?;
    validate_counts(source, limits)?;
    let mut canonical = source.clone();
    canonical
        .regions
        .sort_by(|left, right| left.key.cmp(&right.key));
    canonical
        .areas
        .sort_by(|left, right| left.key.cmp(&right.key));
    canonical
        .terrains
        .sort_by(|left, right| left.key.cmp(&right.key));
    canonical
        .cells
        .sort_by(|left, right| left.key.cmp(&right.key));
    canonical
        .relocations
        .sort_by(|left, right| left.key.cmp(&right.key));
    canonical
        .behaviors
        .sort_by(|left, right| left.key.cmp(&right.key));
    canonical
        .presentations
        .sort_by(|left, right| left.key.cmp(&right.key));
    canonical
        .creatures
        .sort_by(|left, right| left.key.cmp(&right.key));
    canonical
        .spawns
        .sort_by(|left, right| left.key.cmp(&right.key));
    canonical
        .formula_profiles
        .sort_by(|left, right| left.key.cmp(&right.key));
    canonical
        .effects
        .sort_by(|left, right| left.key.cmp(&right.key));
    canonical
        .abilities
        .sort_by(|left, right| left.key.cmp(&right.key));
    canonical
        .items
        .sort_by(|left, right| left.key.cmp(&right.key));
    canonical
        .loot_tables
        .sort_by(|left, right| left.key.cmp(&right.key));
    canonical
        .xp_definitions
        .sort_by(|left, right| left.key.cmp(&right.key));
    canonical.rng.purpose_keys.sort();
    for table in &mut canonical.loot_tables {
        table
            .entries
            .sort_by(|left, right| left.key.cmp(&right.key));
    }
    validate_semantics(&canonical)?;
    Ok(CanonicalGraph { source: canonical })
}

fn validate_source_strings(
    source: &FixtureSource,
    limits: &EvidenceLimits,
) -> Result<(), ContentError> {
    for presentation in &source.presentations {
        validate_graphic_string(
            &presentation.synthetic_asset_token,
            "synthetic asset token",
            limits,
        )?;
        if !presentation
            .synthetic_asset_token
            .starts_with("synthetic://")
        {
            return Err(ContentError::InvalidString("non-synthetic VSL asset token"));
        }
    }
    validate_graphic_string(
        &source.rng.synthetic_root_label,
        "rng fixture root label",
        limits,
    )
}

fn validate_graphic_string(
    value: &str,
    field: &'static str,
    limits: &EvidenceLimits,
) -> Result<(), ContentError> {
    limits.check("string bytes", value.len(), limits.max_string_bytes())?;
    if value.is_empty() || !value.is_ascii() || !value.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(ContentError::InvalidString(field));
    }
    Ok(())
}

fn validate_counts(source: &FixtureSource, limits: &EvidenceLimits) -> Result<(), ContentError> {
    limits.check("cells", source.cells.len(), limits.max_cells())?;
    let loot_entries = checked_sum(source.loot_tables.iter().map(|table| table.entries.len()))?;
    let definitions = checked_sum([
        source.regions.len(),
        source.areas.len(),
        source.terrains.len(),
        source.cells.len(),
        source.relocations.len(),
        source.behaviors.len(),
        source.presentations.len(),
        source.creatures.len(),
        source.spawns.len(),
        source.formula_profiles.len(),
        source.effects.len(),
        source.abilities.len(),
        source.items.len(),
        source.loot_tables.len(),
        loot_entries,
        source.xp_definitions.len(),
        source.rng.purpose_keys.len(),
    ])?;
    limits.check("definitions", definitions, limits.max_definitions())?;
    let server_records = definitions
        .checked_add(1)
        .ok_or(ContentError::InvalidSectionBounds)?;
    limits.check("records", server_records, limits.max_records())?;
    let client_records = checked_sum([
        source.presentations.len(),
        source.creatures.len(),
        source.abilities.len(),
        source.items.len(),
    ])?;
    limits.check("records", client_records, limits.max_records())?;

    let references = checked_sum([
        checked_mul(source.cells.len(), 3)?,
        checked_mul(source.relocations.len(), 2)?,
        checked_mul(source.creatures.len(), 2)?,
        checked_mul(source.spawns.len(), 3)?,
        source.effects.len(),
        checked_mul(source.abilities.len(), 2)?,
        source.items.len(),
        checked_mul(loot_entries, 3)?,
        source.xp_definitions.len(),
    ])?;
    limits.check("references", references, limits.max_references())
}

fn checked_sum<I>(values: I) -> Result<usize, ContentError>
where
    I: IntoIterator<Item = usize>,
{
    values.into_iter().try_fold(0_usize, |total, value| {
        total
            .checked_add(value)
            .ok_or(ContentError::InvalidSectionBounds)
    })
}

fn checked_mul(left: usize, right: usize) -> Result<usize, ContentError> {
    left.checked_mul(right)
        .ok_or(ContentError::InvalidSectionBounds)
}

fn validate_semantics(source: &FixtureSource) -> Result<(), ContentError> {
    if source.regions.is_empty()
        || source.areas.is_empty()
        || source.terrains.is_empty()
        || source.cells.is_empty()
        || source.relocations.is_empty()
        || source.behaviors.is_empty()
        || source.presentations.is_empty()
        || source.creatures.is_empty()
        || source.spawns.is_empty()
        || source.formula_profiles.is_empty()
        || source.effects.is_empty()
        || source.abilities.is_empty()
        || source.items.is_empty()
        || source.loot_tables.is_empty()
        || source.xp_definitions.is_empty()
        || source.rng.purpose_keys.is_empty()
    {
        return Err(ContentError::InvalidArtifact(
            "minimal VSL semantic set is incomplete",
        ));
    }
    if !source.items.iter().any(|item| item.materializable) {
        return Err(ContentError::InvalidArtifact(
            "VSL fixture has no materializable item",
        ));
    }
    if source
        .formula_profiles
        .iter()
        .any(|profile| !profile.fixture_only)
    {
        return Err(ContentError::FixtureOnlyReleaseRejected);
    }

    let mut definitions = BTreeSet::new();
    for definition in &source.regions {
        insert_key(&mut definitions, &definition.key)?;
    }
    for definition in &source.areas {
        insert_key(&mut definitions, &definition.key)?;
    }
    for definition in &source.terrains {
        insert_key(&mut definitions, &definition.key)?;
    }
    for definition in &source.cells {
        insert_key(&mut definitions, &definition.key)?;
    }
    for definition in &source.relocations {
        insert_key(&mut definitions, &definition.key)?;
    }
    for definition in &source.behaviors {
        insert_key(&mut definitions, &definition.key)?;
    }
    for definition in &source.presentations {
        insert_key(&mut definitions, &definition.key)?;
    }
    for definition in &source.creatures {
        insert_key(&mut definitions, &definition.key)?;
    }
    for definition in &source.spawns {
        insert_key(&mut definitions, &definition.key)?;
    }
    for definition in &source.formula_profiles {
        insert_key(&mut definitions, &definition.key)?;
    }
    for definition in &source.effects {
        insert_key(&mut definitions, &definition.key)?;
    }
    for definition in &source.abilities {
        insert_key(&mut definitions, &definition.key)?;
    }
    for definition in &source.items {
        insert_key(&mut definitions, &definition.key)?;
    }
    for definition in &source.loot_tables {
        insert_key(&mut definitions, &definition.key)?;
        for entry in &definition.entries {
            insert_key(&mut definitions, &entry.key)?;
        }
    }
    for definition in &source.xp_definitions {
        insert_key(&mut definitions, &definition.key)?;
    }
    for purpose in &source.rng.purpose_keys {
        insert_key(&mut definitions, purpose)?;
    }

    for cell in &source.cells {
        require_ref(&definitions, &cell.key, &cell.region_key)?;
        require_ref(&definitions, &cell.key, &cell.area_key)?;
        require_ref(&definitions, &cell.key, &cell.terrain_key)?;
    }
    for relocation in &source.relocations {
        require_ref(&definitions, &relocation.key, &relocation.from_cell)?;
        require_ref(&definitions, &relocation.key, &relocation.to_cell)?;
    }
    for creature in &source.creatures {
        if creature.fixture_max_hp == 0 {
            return Err(ContentError::InvalidArtifact(
                "creature fixture hp must be positive",
            ));
        }
        require_ref(&definitions, &creature.key, &creature.behavior_key)?;
        require_ref(&definitions, &creature.key, &creature.presentation_key)?;
    }
    for spawn in &source.spawns {
        if spawn.fixture_population_limit == 0 {
            return Err(ContentError::InvalidArtifact(
                "spawn population must be positive",
            ));
        }
        if spawn.multiplicity.is_none() || spawn.eligibility_scope.is_none() {
            return Err(ContentError::MissingSourceClassification(
                spawn.key.as_str().to_owned(),
            ));
        }
        require_ref(&definitions, &spawn.key, &spawn.creature_key)?;
        require_ref(&definitions, &spawn.key, &spawn.behavior_key)?;
        require_ref(&definitions, &spawn.key, &spawn.cell_key)?;
    }
    for effect in &source.effects {
        require_ref(&definitions, &effect.key, &effect.formula_profile_key)?;
    }
    for ability in &source.abilities {
        require_ref(&definitions, &ability.key, &ability.effect_key)?;
        require_ref(&definitions, &ability.key, &ability.presentation_key)?;
    }
    for item in &source.items {
        require_ref(&definitions, &item.key, &item.presentation_key)?;
    }
    for table in &source.loot_tables {
        if table.entries.is_empty() {
            return Err(ContentError::InvalidArtifact("loot table has no entries"));
        }
        for entry in &table.entries {
            if entry.fixture_weight == 0 {
                return Err(ContentError::InvalidArtifact(
                    "loot fixture weight must be positive",
                ));
            }
            require_ref(&definitions, &entry.key, &entry.item_key)?;
            require_ref(&definitions, &entry.key, &entry.rng_purpose_key)?;
        }
    }
    for xp in &source.xp_definitions {
        if xp.fixture_amount == 0 {
            return Err(ContentError::InvalidArtifact(
                "xp fixture amount must be positive",
            ));
        }
        require_ref(&definitions, &xp.key, &xp.formula_profile_key)?;
    }
    Ok(())
}

fn insert_key(definitions: &mut BTreeSet<String>, key: &ContentKey) -> Result<(), ContentError> {
    if !definitions.insert(key.as_str().to_owned()) {
        return Err(ContentError::DuplicateKey(key.as_str().to_owned()));
    }
    Ok(())
}

fn require_ref(
    definitions: &BTreeSet<String>,
    owner: &ContentKey,
    target: &ContentKey,
) -> Result<(), ContentError> {
    if !definitions.contains(target.as_str()) {
        return Err(ContentError::MissingReference {
            owner: owner.as_str().to_owned(),
            target: target.as_str().to_owned(),
        });
    }
    Ok(())
}

fn server_records(graph: &CanonicalGraph) -> Result<Vec<EvidenceRecord>, ContentError> {
    let source = &graph.source;
    let mut records = Vec::new();
    for value in &source.regions {
        records.push(record(RECORD_REGION, [value.key.as_str()]));
    }
    for value in &source.areas {
        records.push(record(RECORD_AREA, [value.key.as_str()]));
    }
    for value in &source.terrains {
        records.push(record(RECORD_TERRAIN, [value.key.as_str()]));
    }
    for value in &source.cells {
        records.push(EvidenceRecord::new(
            RECORD_CELL,
            vec![
                value.key.as_str().to_owned(),
                value.region_key.as_str().to_owned(),
                value.area_key.as_str().to_owned(),
                value.terrain_key.as_str().to_owned(),
                value.x.to_string(),
                value.y.to_string(),
                value.z.to_string(),
                value.collision.as_str().to_owned(),
            ],
        ));
    }
    for value in &source.relocations {
        records.push(record(
            RECORD_RELOCATION,
            [
                value.key.as_str(),
                value.from_cell.as_str(),
                value.to_cell.as_str(),
            ],
        ));
    }
    for value in &source.behaviors {
        records.push(record(
            RECORD_BEHAVIOR,
            [value.key.as_str(), value.policy_revision.as_str()],
        ));
    }
    for value in &source.presentations {
        records.push(record(
            RECORD_PRESENTATION,
            [value.key.as_str(), value.synthetic_asset_token.as_str()],
        ));
    }
    for value in &source.creatures {
        records.push(EvidenceRecord::new(
            RECORD_CREATURE,
            vec![
                value.key.as_str().to_owned(),
                value.behavior_key.as_str().to_owned(),
                value.presentation_key.as_str().to_owned(),
                value.fixture_max_hp.to_string(),
            ],
        ));
    }
    for value in &source.spawns {
        let multiplicity = value.multiplicity.ok_or_else(|| {
            ContentError::MissingSourceClassification(value.key.as_str().to_owned())
        })?;
        let eligibility = value.eligibility_scope.ok_or_else(|| {
            ContentError::MissingSourceClassification(value.key.as_str().to_owned())
        })?;
        records.push(EvidenceRecord::new(
            RECORD_SPAWN,
            vec![
                value.key.as_str().to_owned(),
                value.creature_key.as_str().to_owned(),
                value.behavior_key.as_str().to_owned(),
                value.cell_key.as_str().to_owned(),
                value.fixture_population_limit.to_string(),
                value.recovery.as_str().to_owned(),
                multiplicity.as_str().to_owned(),
                eligibility.as_str().to_owned(),
            ],
        ));
    }
    for value in &source.formula_profiles {
        records.push(record(
            RECORD_FORMULA,
            [
                value.key.as_str(),
                if value.fixture_only { "true" } else { "false" },
            ],
        ));
    }
    for value in &source.effects {
        records.push(record(
            RECORD_EFFECT,
            [
                value.key.as_str(),
                value.family.as_str(),
                value.formula_profile_key.as_str(),
            ],
        ));
    }
    for value in &source.abilities {
        records.push(record(
            RECORD_ABILITY,
            [
                value.key.as_str(),
                value.effect_key.as_str(),
                value.presentation_key.as_str(),
            ],
        ));
    }
    for value in &source.items {
        records.push(record(
            RECORD_ITEM,
            [
                value.key.as_str(),
                value.presentation_key.as_str(),
                if value.materializable {
                    "true"
                } else {
                    "false"
                },
            ],
        ));
    }
    for table in &source.loot_tables {
        records.push(record(RECORD_LOOT_TABLE, [table.key.as_str()]));
        for entry in &table.entries {
            records.push(EvidenceRecord::new(
                RECORD_LOOT_ENTRY,
                vec![
                    table.key.as_str().to_owned(),
                    entry.key.as_str().to_owned(),
                    entry.item_key.as_str().to_owned(),
                    entry.rng_purpose_key.as_str().to_owned(),
                    entry.fixture_weight.to_string(),
                ],
            ));
        }
    }
    for value in &source.xp_definitions {
        records.push(EvidenceRecord::new(
            RECORD_XP,
            vec![
                value.key.as_str().to_owned(),
                value.formula_profile_key.as_str().to_owned(),
                value.fixture_amount.to_string(),
            ],
        ));
    }
    records.push(record(
        RECORD_RNG_CONTEXT,
        [
            source.rng.profile_revision.as_str(),
            source.rng.synthetic_root_label.as_str(),
        ],
    ));
    for purpose in &source.rng.purpose_keys {
        records.push(record(RECORD_RNG_PURPOSE, [purpose.as_str()]));
    }
    Ok(records)
}

fn client_records(graph: &CanonicalGraph) -> Vec<EvidenceRecord> {
    let source = &graph.source;
    let mut records = Vec::new();
    for value in &source.presentations {
        records.push(record(
            RECORD_PRESENTATION,
            [value.key.as_str(), value.synthetic_asset_token.as_str()],
        ));
    }
    for value in &source.creatures {
        records.push(record(
            RECORD_CLIENT_CREATURE,
            [value.key.as_str(), value.presentation_key.as_str()],
        ));
    }
    for value in &source.abilities {
        records.push(record(
            RECORD_CLIENT_ABILITY,
            [value.key.as_str(), value.presentation_key.as_str()],
        ));
    }
    for value in &source.items {
        records.push(record(
            RECORD_CLIENT_ITEM,
            [value.key.as_str(), value.presentation_key.as_str()],
        ));
    }
    records
}

fn record<const N: usize>(kind: u8, fields: [&str; N]) -> EvidenceRecord {
    EvidenceRecord::new(kind, fields.into_iter().map(str::to_owned).collect())
}
