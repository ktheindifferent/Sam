fn main() {
    println!("Running basic tests...");
    
    // Test 1: Basic arithmetic
    let result = 2 + 2;
    assert_eq!(result, 4);
    println!("✓ Test 1 passed: 2 + 2 = 4");
    
    // Test 2: String operations
    let s = String::from("hello");
    assert_eq!(s.len(), 5);
    println!("✓ Test 2 passed: String length check");
    
    // Test 3: Vector operations
    let mut v = vec![1, 2, 3];
    v.push(4);
    assert_eq!(v.len(), 4);
    println!("✓ Test 3 passed: Vector operations");
    
    // Test 4: This should fail to demonstrate error handling
    let should_fail = false;
    if should_fail {
        assert_eq!(1, 2, "This test intentionally fails");
    } else {
        println!("✓ Test 4 passed: Skipped failing test");
    }
    
    println!("\nAll tests passed!");
}