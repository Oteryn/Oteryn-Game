use oteryn_client_domain::movement::StepDir;
/// Native sprites retain the source's four facings after an eight-way step.
pub fn facing(direction: StepDir) -> u32 {
    match direction {
        StepDir::North => 0,
        StepDir::East | StepDir::NorthEast | StepDir::SouthEast => 1,
        StepDir::South => 2,
        StepDir::West | StepDir::NorthWest | StepDir::SouthWest => 3,
    }
}
