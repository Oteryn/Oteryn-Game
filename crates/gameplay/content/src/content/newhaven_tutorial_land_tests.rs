use super::*;

#[test]
fn tutorial_land_is_exactly_the_two_accepted_components() {
    let mut count = 0;
    let mut islet = 0;
    for y in 32405..=32522 {
        for x in 32506..=32634 {
            if contains(x, y, 7) {
                count += 1;
                if (32532..=32539).contains(&x) && (32511..=32518).contains(&y) {
                    islet += 1;
                }
            }
        }
    }
    assert_eq!((count, islet), (9239, 50));
    assert!(contains(32534, 32513, 7));
    assert!(contains(32544, 32515, 7));
    assert!(!contains(32534, 32513, 6));
    assert!(!contains(32506, 32522, 7));
    assert!(!contains(32369, 32238, 7));
}
