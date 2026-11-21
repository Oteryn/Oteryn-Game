//! Ordered phases of v2 project validation.

use super::*;

pub(super) fn validate(
    state: &ProjectV2State,
    records: &[ProjectReferenceRecord],
    imports: &[ImportBatch],
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    let identities = collect_definition_identities(state, records, limits)?;
    let require_ref = |reference: &ProjectV2DefinitionRef| -> Result<(), ProjectError> {
        reference.validate()?;
        if !identities.contains(reference) {
            return Err(ProjectError::InvalidProject(
                "unresolved v2 typed definition reference",
            ));
        }
        Ok(())
    };
    validate_declarations(state, records, &require_ref, limits)?;
    validate_authoring(state, records, &require_ref, limits)?;
    let worlds = validate_worlds(state, limits)?;
    validate_placements(state, &worlds, &require_ref, limits)?;
    validate_assets(state, &require_ref, limits)?;
    validate_sources(state, imports, &require_ref, limits)?;
    validate_editor(state, &require_ref, limits)?;
    Ok(())
}

fn collect_definition_identities(
    state: &ProjectV2State,
    records: &[ProjectReferenceRecord],
    limits: ProjectEvidenceLimits,
) -> Result<BTreeSet<ProjectV2DefinitionRef>, ProjectError> {
    let mut identities = BTreeSet::new();
    for record in records {
        let identity = record.identity();
        identities.insert(ProjectV2DefinitionRef {
            family: ProjectV2Family::from_reference(&identity.family)?,
            key: identity.key.clone(),
            revision: identity.revision.clone(),
        });
    }
    limits.check(
        "v2 declarations",
        state.declarations.len(),
        limits.max_reference_records,
    )?;
    let mut previous: Option<(ProjectV2Family, &str)> = None;
    for declaration in &state.declarations {
        let identity = declaration.identity();
        identity.validate()?;
        validate_v2_candidate_fields(declaration.fields())?;
        let key = (declaration.family(), identity.key.as_str());
        if previous.is_some_and(|prior| prior >= key) {
            return Err(ProjectError::InvalidProject(
                "v2 declarations are not identity sorted",
            ));
        }
        previous = Some(key);
        if !identities.insert(ProjectV2DefinitionRef {
            family: key.0,
            key: identity.key.clone(),
            revision: identity.revision.clone(),
        }) {
            return Err(ProjectError::InvalidProject(
                "duplicate v2 definition identity",
            ));
        }
    }
    Ok(identities)
}

fn validate_declarations(
    state: &ProjectV2State,
    records: &[ProjectReferenceRecord],
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    for declaration in &state.declarations {
        for (family, reference) in declaration.references() {
            if reference.family != family {
                return Err(ProjectError::InvalidProject(
                    "v2 definition reference family mismatch",
                ));
            }
            require_ref(reference)?;
        }
        validate_v2_declaration(declaration, require_ref, limits)?;
    }
    item_conversion::validate_item_conversions(state, records, limits)?;
    let parents = state
        .declarations
        .iter()
        .filter_map(|declaration| {
            if let ProjectV2Declaration::Area {
                identity, parent, ..
            } = declaration
            {
                Some((
                    identity.key.as_str(),
                    parent.as_ref().map(|parent| parent.key.as_str()),
                ))
            } else {
                None
            }
        })
        .collect();
    validate_parent_chains(&parents, "v2 Area parent chain contains a cycle")?;

    Ok(())
}

fn validate_authoring(
    state: &ProjectV2State,
    records: &[ProjectReferenceRecord],
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    limits.check(
        "v2 authoring profiles",
        state.authoring_profiles.len(),
        limits.max_reference_records,
    )?;
    if state
        .authoring_profiles
        .windows(2)
        .any(|pair| pair[0].target >= pair[1].target)
    {
        return Err(ProjectError::InvalidProject(
            "v2 authoring profiles are not target sorted and unique",
        ));
    }
    for profile in &state.authoring_profiles {
        validate_v2_authoring_profile(profile, require_ref, limits)?;
        if let ProjectV2AuthoringProfileData::Loot(loot) = &profile.data {
            let entries = records.iter().find_map(|record| match record {
                ProjectReferenceRecord::Loot {
                    identity, entries, ..
                } if identity.key == profile.target.key
                    && identity.revision == profile.target.revision =>
                {
                    Some(entries.len())
                }
                _ => None,
            });
            if entries != Some(loot.entries.len()) {
                return Err(ProjectError::InvalidProject(
                    "v2 Loot details must align with the entries of their Loot record",
                ));
            }
        }
    }

    validate_v2_encounter_bindings(&state.authoring_profiles)?;

    limits.check(
        "v2 item authoring",
        state.item_authoring.len(),
        limits.max_reference_records,
    )?;
    if state
        .item_authoring
        .windows(2)
        .any(|pair| pair[0].item >= pair[1].item)
    {
        return Err(ProjectError::InvalidProject(
            "v2 Item authoring is not identity sorted and unique",
        ));
    }
    for item in &state.item_authoring {
        validate_v2_item_authoring(item, require_ref, limits)?;
    }
    Ok(())
}

fn validate_worlds(
    state: &ProjectV2State,
    limits: ProjectEvidenceLimits,
) -> Result<BTreeMap<&String, &ProjectV2World>, ProjectError> {
    limits.check(
        "v2 worlds",
        state.worlds.len(),
        limits.max_reference_records,
    )?;
    let mut worlds = BTreeMap::new();
    let mut world_ids = BTreeSet::new();
    for world in &state.worlds {
        ProductionKey::new(&world.key)?;
        decode_world_id(&world.world_id)?;
        crate::content::CoordinateFrameRef::new(&world.coordinate_frame)?;
        let bounds = &world.bounds;
        if bounds.min_x < i64::from(i32::MIN)
            || bounds.min_y < i64::from(i32::MIN)
            || bounds.max_x_exclusive > i64::from(i32::MAX) + 1
            || bounds.max_y_exclusive > i64::from(i32::MAX) + 1
            || bounds.min_x >= bounds.max_x_exclusive
            || bounds.min_y >= bounds.max_y_exclusive
            || world.floors.is_empty()
            || world.floors.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(ProjectError::InvalidProject(
                "invalid v2 world bounds or floors",
            ));
        }
        limits.check(
            "v2 world floors",
            world.floors.len(),
            limits.max_reference_records,
        )?;
        if worlds.insert(&world.key, world).is_some() || !world_ids.insert(&world.world_id) {
            return Err(ProjectError::InvalidProject("duplicate v2 world identity"));
        }
    }
    if state
        .worlds
        .windows(2)
        .any(|pair| pair[0].key >= pair[1].key)
    {
        return Err(ProjectError::InvalidProject(
            "v2 worlds are not identity sorted",
        ));
    }
    Ok(worlds)
}

fn validate_placements(
    state: &ProjectV2State,
    worlds: &BTreeMap<&String, &ProjectV2World>,
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    limits.check(
        "v2 placements",
        state.placements.len(),
        limits.max_reference_records,
    )?;
    let mut keys = BTreeSet::new();
    let mut orders = BTreeSet::new();
    for placement in &state.placements {
        ProductionKey::new(&placement.key)?;
        ProductionAtom::new("v2 map revision", &placement.map_revision)?;
        crate::content::CoordinateFrameRef::new(&placement.coordinate_frame)?;
        require_ref(&placement.definition)?;
        if let Some(area) = &placement.area {
            if area.family != ProjectV2Family::Area {
                return Err(ProjectError::InvalidProject(
                    "v2 placement area family mismatch",
                ));
            }
            require_ref(area)?;
        }
        if let Some(document) = &placement.document {
            if document.family != ProjectV2Family::Document {
                return Err(ProjectError::InvalidProject(
                    "v2 placement document family mismatch",
                ));
            }
            require_ref(document)?;
        }
        if let Some(parent) = &placement.parent_placement {
            ProductionKey::new(parent)?;
            if parent == &placement.key {
                return Err(ProjectError::InvalidProject(
                    "v2 placement cannot parent itself",
                ));
            }
        }
        let world = worlds
            .get(&placement.world)
            .ok_or(ProjectError::InvalidProject(
                "v2 placement world is missing",
            ))?;
        if world.coordinate_frame != placement.coordinate_frame
            || i64::from(placement.x) < world.bounds.min_x
            || i64::from(placement.x) >= world.bounds.max_x_exclusive
            || i64::from(placement.y) < world.bounds.min_y
            || i64::from(placement.y) >= world.bounds.max_y_exclusive
            || world.floors.binary_search(&placement.floor).is_err()
        {
            return Err(ProjectError::InvalidProject(
                "v2 placement world/frame/position mismatch",
            ));
        }
        if !orders.insert((
            &placement.world,
            placement.x,
            placement.y,
            placement.floor,
            &placement.presentation_order,
        )) {
            return Err(ProjectError::InvalidProject(
                "duplicate v2 presentation order at tile",
            ));
        }
        if !keys.insert(&placement.key) {
            return Err(ProjectError::InvalidProject(
                "duplicate v2 placement identity",
            ));
        }
    }
    if state
        .placements
        .windows(2)
        .any(|pair| pair[0].key >= pair[1].key)
    {
        return Err(ProjectError::InvalidProject(
            "v2 placements are not identity sorted",
        ));
    }
    let placements: BTreeMap<_, _> = state
        .placements
        .iter()
        .map(|p| (p.key.as_str(), p))
        .collect();
    for placement in &state.placements {
        if let Some(parent_key) = &placement.parent_placement {
            let parent =
                placements
                    .get(parent_key.as_str())
                    .ok_or(ProjectError::InvalidProject(
                        "v2 parent placement is missing",
                    ))?;
            if parent.world != placement.world || parent.map_revision != placement.map_revision {
                return Err(ProjectError::InvalidProject(
                    "v2 parent placement world or map revision mismatch",
                ));
            }
        }
    }
    let parents = state
        .placements
        .iter()
        .map(|p| (p.key.as_str(), p.parent_placement.as_deref()))
        .collect();
    validate_parent_chains(&parents, "v2 placement parent chain contains a cycle")?;

    Ok(())
}

fn validate_assets(
    state: &ProjectV2State,
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    limits.check(
        "v2 assets",
        state.assets.len(),
        limits.max_reference_records,
    )?;
    let mut assets = BTreeSet::new();
    for asset in &state.assets {
        ProductionKey::new(&asset.identity.key)?;
        DefinitionRevisionRef::new(&asset.identity.revision)?;
        Sha256HexDigest::new(&asset.sha256)?;
        if !assets.insert(&asset.identity) {
            return Err(ProjectError::InvalidProject("duplicate v2 asset"));
        }
    }
    if state
        .assets
        .windows(2)
        .any(|pair| pair[0].identity >= pair[1].identity)
    {
        return Err(ProjectError::InvalidProject(
            "v2 assets are not identity sorted",
        ));
    }
    limits.check(
        "v2 appearance bindings",
        state.appearance_bindings.len(),
        limits.max_reference_records,
    )?;
    let mut bound = BTreeSet::new();
    for binding in &state.appearance_bindings {
        if binding.presentation.family != ProjectV2Family::Presentation {
            return Err(ProjectError::InvalidProject(
                "appearance binding requires Presentation",
            ));
        }
        require_ref(&binding.presentation)?;
        if !assets.contains(&binding.asset) {
            return Err(ProjectError::InvalidProject("appearance asset is missing"));
        }
        if !bound.insert(&binding.presentation) {
            return Err(ProjectError::InvalidProject("duplicate appearance binding"));
        }
    }
    if state
        .appearance_bindings
        .windows(2)
        .any(|pair| pair[0].presentation >= pair[1].presentation)
    {
        return Err(ProjectError::InvalidProject(
            "appearance bindings are not identity sorted",
        ));
    }
    Ok(())
}

fn validate_sources(
    state: &ProjectV2State,
    imports: &[ImportBatch],
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    limits.check("v2 sources", state.sources.len(), limits.max_import_records)?;
    let mut source_keys = BTreeSet::new();
    for source in &state.sources {
        ProductionKey::new(&source.key)?;
        ProductionAtom::new("v2 source revision", &source.revision)?;
        Sha256HexDigest::new(&source.sha256)?;
        let batch = imports
            .iter()
            .find(|batch| batch.batch_id == source.import_batch_id)
            .ok_or(ProjectError::InvalidProject(
                "v2 source import batch is missing",
            ))?;
        if batch.source_revision != source.revision || batch.source_artifact_sha256 != source.sha256
        {
            return Err(ProjectError::InvalidProject(
                "v2 source disagrees with import batch",
            ));
        }
        if !source_keys.insert((&source.key, &source.revision)) {
            return Err(ProjectError::InvalidProject("duplicate v2 source"));
        }
    }
    if state
        .sources
        .windows(2)
        .any(|pair| (&pair[0].key, &pair[0].revision) >= (&pair[1].key, &pair[1].revision))
    {
        return Err(ProjectError::InvalidProject(
            "v2 sources are not identity sorted",
        ));
    }
    limits.check(
        "v2 source identity bindings",
        state.source_identity_bindings.len(),
        limits.max_decoded_fields,
    )?;
    let mut binding_keys = BTreeSet::new();
    for binding in &state.source_identity_bindings {
        ProductionKey::new(&binding.source_key)?;
        ProductionAtom::new("v2 source revision", &binding.source_revision)?;
        validate_v2_source_identity_namespace(&binding.identity_namespace, limits)?;
        validate_v2_source_text("v2 external source identity", &binding.external_id, limits)?;
        if !state.sources.iter().any(|source| {
            source.key == binding.source_key && source.revision == binding.source_revision
        }) {
            return Err(ProjectError::InvalidProject(
                "v2 source identity binding has no exact source revision",
            ));
        }
        require_ref(&binding.target)?;
        if binding.identity_namespace == "client/appearance_id"
            && binding.target.family != ProjectV2Family::Presentation
        {
            return Err(ProjectError::InvalidProject(
                "client appearance identity must target Presentation",
            ));
        }
        if !binding_keys.insert((
            &binding.source_key,
            &binding.source_revision,
            &binding.identity_namespace,
            &binding.external_id,
        )) {
            return Err(ProjectError::InvalidProject(
                "duplicate or conflicting v2 source identity binding",
            ));
        }
    }
    if state.source_identity_bindings.windows(2).any(|pair| {
        (
            &pair[0].source_key,
            &pair[0].source_revision,
            &pair[0].identity_namespace,
            &pair[0].external_id,
        ) >= (
            &pair[1].source_key,
            &pair[1].source_revision,
            &pair[1].identity_namespace,
            &pair[1].external_id,
        )
    }) {
        return Err(ProjectError::InvalidProject(
            "v2 source identity bindings are not identity sorted",
        ));
    }
    Ok(())
}

fn validate_editor(
    state: &ProjectV2State,
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    limits.check(
        "v2 editor entries",
        state.editor.len(),
        limits.max_reference_records,
    )?;
    let mut alias_keys = BTreeSet::new();
    let mut targets = BTreeSet::new();
    for entry in &state.editor {
        require_ref(&entry.target)?;
        if !targets.insert(&entry.target) {
            return Err(ProjectError::InvalidProject("duplicate v2 editor target"));
        }
        if entry.aliases.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(ProjectError::InvalidProject(
                "v2 editor aliases are not sorted and unique",
            ));
        }
        for alias in &entry.aliases {
            if alias.trim() != alias
                || alias.is_empty()
                || alias.chars().any(char::is_control)
                || !alias_keys.insert((entry.target.family, alias))
            {
                return Err(ProjectError::InvalidProject(
                    "invalid or duplicate v2 alias",
                ));
            }
        }
        for tag in &entry.tags {
            ProductionKey::new(tag)?;
        }
        if entry.tags.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(ProjectError::InvalidProject(
                "v2 editor tags are not sorted and unique",
            ));
        }
    }
    if state
        .editor
        .windows(2)
        .any(|pair| pair[0].target >= pair[1].target)
    {
        return Err(ProjectError::InvalidProject(
            "v2 editor entries are not identity sorted",
        ));
    }
    Ok(())
}

fn validate_parent_chains(
    parents: &BTreeMap<&str, Option<&str>>,
    error: &'static str,
) -> Result<(), ProjectError> {
    let mut done = BTreeSet::new();
    for &start in parents.keys() {
        let mut path = BTreeSet::new();
        let mut current = Some(start);
        while let Some(key) = current {
            if done.contains(key) {
                break;
            }
            if !path.insert(key) {
                return Err(ProjectError::InvalidProject(error));
            }
            current = parents.get(key).copied().flatten();
        }
        done.extend(path);
    }
    Ok(())
}

#[cfg(test)]
mod regression_tests {
    use super::*;

    #[test]
    fn deep_parent_chains_have_bounded_validation_work() -> Result<(), ProjectError> {
        let limits = ProjectEvidenceLimits {
            max_documents: 10000,
            max_document_bytes: 1000000,
            max_total_bytes: 10000000,
            max_json_depth: 64,
            max_decoded_fields: 100000,
            max_string_bytes: 10000,
            max_locator_bytes: 10000,
            max_locator_segments: 100,
            max_reference_records: 10000,
            max_import_records: 10000,
            max_reimport_states: 10000,
        };
        let mut state = ProjectV2State::default();
        for index in 0..2048 {
            state.declarations.push(ProjectV2Declaration::Area {
                identity: ProjectV2Identity {
                    key: format!("oteryn:area/{index:04}"),
                    revision: "r1".into(),
                },
                visibility: PublicVisibility::Public,
                parent: (index > 0).then(|| ProjectV2DefinitionRef {
                    family: ProjectV2Family::Area,
                    key: format!("oteryn:area/{:04}", index - 1),
                    revision: "r1".into(),
                }),
                fields: Vec::new(),
            });
        }
        let started = std::time::Instant::now();
        validate_declarations(&state, &[], &|_| Ok(()), limits)?;
        assert!(
            started.elapsed() < std::time::Duration::from_secs(2),
            "deep parent validation exceeded two seconds"
        );
        state.worlds.push(ProjectV2World {
            key: "oteryn:world/chain".into(),
            world_id: "unused".into(),
            coordinate_frame: "oteryn:frame/chain".into(),
            bounds: ProjectV2Bounds {
                min_x: 0,
                min_y: 0,
                max_x_exclusive: 1,
                max_y_exclusive: 1,
            },
            floors: vec![0],
        });
        for index in 0..2048 {
            state.placements.push(ProjectV2Placement {
                key: format!("oteryn:placement/{index:04}"),
                world: "oteryn:world/chain".into(),
                map_revision: "r1".into(),
                definition: ProjectV2DefinitionRef {
                    family: ProjectV2Family::Area,
                    key: "oteryn:area/0000".into(),
                    revision: "r1".into(),
                },
                area: None,
                document: None,
                parent_placement: (index > 0).then(|| format!("oteryn:placement/{:04}", index - 1)),
                coordinate_frame: "oteryn:frame/chain".into(),
                x: 0,
                y: 0,
                floor: 0,
                presentation_order: ProjectV2PresentationOrder {
                    plane: 0,
                    order: index,
                },
                disposition: ProjectV2Disposition::CandidateOnly,
            });
        }
        let worlds = state
            .worlds
            .iter()
            .map(|world| (&world.key, world))
            .collect();
        let started = std::time::Instant::now();
        validate_placements(&state, &worlds, &|_| Ok(()), limits)?;
        assert!(
            started.elapsed() < std::time::Duration::from_secs(2),
            "deep placement validation exceeded two seconds"
        );
        state.placements[0].parent_placement = Some("oteryn:placement/2047".into());
        assert!(validate_placements(&state, &worlds, &|_| Ok(()), limits).is_err());
        if let ProjectV2Declaration::Area { parent, .. } = &mut state.declarations[0] {
            *parent = Some(ProjectV2DefinitionRef {
                family: ProjectV2Family::Area,
                key: "oteryn:area/2047".into(),
                revision: "r1".into(),
            });
        }
        assert!(validate_declarations(&state, &[], &|_| Ok(()), limits).is_err());
        Ok(())
    }
}
