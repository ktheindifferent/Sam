#[test]
fn test_simple() {
    assert_eq!(2 + 2, 4);
}

#[test]
fn test_failing() {
    assert_eq!(2 + 2, 5, "This test should fail: 2 + 2 != 5");
}
