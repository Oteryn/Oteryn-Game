//! Authored native altar counterpart: real drops through the existing floor crafting owner.
use super::TestResult;
use super::l6_floor_native_fixtures::run;

#[test]
fn l6_native_altar_assembly() -> TestResult {
    run("l6-floor-assembly-native")
}
#[test]
fn l6_native_altar_missing() -> TestResult {
    run("l6-floor-assembly-native-missing")
}
#[test]
fn l6_native_altar_unbound() -> TestResult {
    run("l6-floor-assembly-native-unbound")
}
#[test]
fn l6_native_altar_ruby_exact_one() -> TestResult {
    run("l6-floor-assembly-native-ruby-stack")
}
#[test]
fn l6_native_altar_ruby_unbound() -> TestResult {
    run("l6-floor-assembly-native-ruby-unbound")
}
