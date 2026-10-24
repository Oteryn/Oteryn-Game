/// [`test_source`] with a spawn of `population` placements on the last `population` cells.
#[cfg(test)]
pub fn test_source_with_population(
    cell_count: usize,
    population: u16,
) -> Result<FirstProductionContentSource, ContentError> {
    let mut source = test_source(cell_count)?;
    let placements = usize::from(population);
    source.spawns[0].population_limit = population;
    source.spawns[0].cell_keys = source.cells[cell_count.saturating_sub(placements)..]
        .iter()
        .map(|cell| cell.key.clone())
        .collect();
    Ok(source)
}
