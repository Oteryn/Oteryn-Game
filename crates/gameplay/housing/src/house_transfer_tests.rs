use super::*;
#[test]
fn house_transfer_batches_are_bounded_sorted_and_have_exact_subjects() {
    let id = |n| {
        let mut id = [0; 16];
        id[6] = 0x70;
        id[8] = 0x80;
        id[15] = n;
        id
    };
    let batch: Vec<_> = (1..=100).map(|n| (id(n), id(n % 2 + 1))).collect();
    assert!(valid_step(&batch));
    let mut too_many = batch.clone();
    too_many.push((id(101), id(1)));
    assert!(!valid_step(&too_many));
    let mut duplicate = batch.clone();
    duplicate[1] = duplicate[0];
    assert!(!valid_step(&duplicate));
    let mut unordered = batch.clone();
    unordered.swap(0, 1);
    assert!(!valid_step(&unordered));
    assert!(!valid_step(&[(id(1), [0; 16])]));
    assert!(valid_step(&[]));
}
