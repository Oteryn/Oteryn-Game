pub type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

pub fn run_matrix() -> TestResult<Vec<String>> {
    Ok(Vec::new())
}
