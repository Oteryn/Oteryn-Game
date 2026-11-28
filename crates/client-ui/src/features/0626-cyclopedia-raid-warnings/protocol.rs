use oteryn_client_runtime::cyclopedia_map::{
    MapWireError,
    raid_warnings::{Warning, decode},
};

/// The most recent valid warning snapshot for the current session scope.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Warnings(Vec<Warning>);

pub fn apply(warnings: &mut Warnings, payload: &[u8]) -> Result<(), MapWireError> {
    warnings.0 = decode(payload)?;
    Ok(())
}

impl Warnings {
    pub fn area_warned(&self, area: u32) -> bool {
        let start = self.0.partition_point(|warning| warning.area < area);
        self.0
            .get(start)
            .is_some_and(|warning| warning.area == area)
    }

    pub fn subarea_warned(&self, area: u32, subarea: u32) -> bool {
        self.0
            .binary_search(&Warning {
                area,
                subarea: Some(subarea),
            })
            .is_ok()
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
