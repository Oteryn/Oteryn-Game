use super::resource_owner::BlockingJobOwner;

#[test]
fn generic_blocking_spawn_requires_a_pre_spawn_owner() {
    let owner = BlockingJobOwner::for_test_funded();
    let _job = super::spawn_blocking_owned(owner, || 1usize);
}
