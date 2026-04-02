// Property-Based Tests using proptest
// Tests invariants and properties that should hold for all inputs
// Added: April 2, 2026

use proptest::prelude::*;

// Property: Collection length should equal number of items added
#[test]
fn prop_vector_length_matches_additions() {
    proptest!(|(items in prop::collection::vec(0i32..1000, 0..100))| {
        let mut vec = Vec::new();
        for item in &items {
            vec.push(*item);
        }
        prop_assert_eq!(vec.len(), items.len());
    });
}

// Property: Sorting should not change the collection size
#[test]
fn prop_sort_preserves_length() {
    proptest!(|(mut items in prop::collection::vec(0i32..1000, 1..100))| {
        let original_len = items.len();
        items.sort();
        prop_assert_eq!(items.len(), original_len);
    });
}

// Property: All items in sorted array should be <= next item
#[test]
fn prop_sorted_array_is_ordered() {
    proptest!(|(mut items in prop::collection::vec(0i32..1000, 1..100))| {
        items.sort();
        for i in 0..items.len()-1 {
            prop_assert!(items[i] <= items[i+1]);
        }
    });
}

// Property: String concatenation length equals sum of input lengths
#[test]
fn prop_string_concat_length() {
    proptest!(|(s1 in "\\PC*", s2 in "\\PC*")| {
        let combined = format!("{}{}", s1, s2);
        prop_assert_eq!(combined.len(), s1.len() + s2.len());
    });
}

// Property: Number parsing roundtrip
#[test]
fn prop_number_parsing_roundtrip() {
    proptest!(|(n in 0i32..i32::MAX)| {
        let s = n.to_string();
        let parsed = s.parse::<i32>().unwrap();
        prop_assert_eq!(n, parsed);
    });
}

// Property: Vec push and pop should be inverse operations
#[test]
fn prop_vec_push_pop_inverse() {
    proptest!(|(items in prop::collection::vec(0i32..1000, 1..50), val in 0i32..1000)| {
        let mut vec = items.clone();
        vec.push(val);
        let popped = vec.pop();
        prop_assert_eq!(popped, Some(val));
        prop_assert_eq!(vec.len(), items.len());
    });
}

// Property: HashMap contains key after insertion
#[test]
fn prop_hashmap_contains_inserted_key() {
    proptest!(|(key in "\\PC{1,10}", value in "[a-z]{1,10}")| {
        let mut map = std::collections::HashMap::new();
        map.insert(key.clone(), value.clone());
        prop_assert!(map.contains_key(&key));
        prop_assert_eq!(map.get(&key), Some(&value));
    });
}

// Property: Option Some and None are mutually exclusive
#[test]
fn prop_option_some_none_exclusive() {
    proptest!(|(val in 0i32..1000)| {
        let opt_some: Option<i32> = Some(val);
        let opt_none: Option<i32> = None;

        prop_assert!(opt_some.is_some());
        prop_assert!(!opt_some.is_none());
        prop_assert!(opt_none.is_none());
        prop_assert!(!opt_none.is_some());
    });
}

// Property: Result Ok and Err are mutually exclusive
#[test]
fn prop_result_ok_err_exclusive() {
    proptest!(|(val in 0i32..1000)| {
        let result_ok: Result<i32, String> = Ok(val);
        let result_err: Result<i32, String> = Err("error".to_string());

        prop_assert!(result_ok.is_ok());
        prop_assert!(!result_ok.is_err());
        prop_assert!(result_err.is_err());
        prop_assert!(!result_err.is_ok());
    });
}

// Property: Range iteration covers all values
#[test]
fn prop_range_iteration_coverage() {
    proptest!(|(start in 0i32..100, len in 1i32..50)| {
        let range = start..(start + len);
        let mut count = 0;
        for _item in range {
            count += 1;
        }
        prop_assert_eq!(count, len);
    });
}

// Property: Filter should reduce or maintain size, never increase
#[test]
fn prop_filter_maintains_invariant() {
    proptest!(|(items in prop::collection::vec(0i32..1000, 0..100))| {
        let original_len = items.len();
        let filtered = items.iter().filter(|x| *x % 2 == 0).count();
        prop_assert!(filtered <= original_len);
    });
}

// Property: Map should preserve length
#[test]
fn prop_map_preserves_length() {
    proptest!(|(items in prop::collection::vec(0i32..1000, 0..100))| {
        let original_len = items.len();
        let mapped: Vec<_> = items.iter().map(|x| x * 2).collect();
        prop_assert_eq!(mapped.len(), original_len);
    });
}

// Property: Duplicate removal reduces or maintains size
#[test]
fn prop_duplicate_removal_maintains_invariant() {
    proptest!(|(mut items in prop::collection::vec(0i32..100, 0..100))| {
        let original_len = items.len();
        items.sort();
        items.dedup();
        prop_assert!(items.len() <= original_len);
    });
}

// Property: Reverse applied twice returns original
#[test]
fn prop_reverse_is_involution() {
    proptest!(|(mut items in prop::collection::vec(0i32..1000, 0..100))| {
        let original = items.clone();
        items.reverse();
        items.reverse();
        prop_assert_eq!(items, original);
    });
}

// Property: Addition is commutative
#[test]
fn prop_addition_commutative() {
    proptest!(|(a in 0i32..i32::MAX / 2, b in 0i32..i32::MAX / 2)| {
        prop_assert_eq!(a + b, b + a);
    });
}

// Property: Multiplication is commutative
#[test]
fn prop_multiplication_commutative() {
    proptest!(|(a in 0i32..1000, b in 0i32..1000)| {
        prop_assert_eq!(a * b, b * a);
    });
}

// Property: Integer division roundtrip
#[test]
fn prop_division_roundtrip() {
    proptest!(|(numerator in 1i32..10000, denominator in 1i32..100)| {
        let result = (numerator / denominator) * denominator + (numerator % denominator);
        prop_assert_eq!(result, numerator);
    });
}

// Property: Logarithm and exponentiation are related
#[test]
fn prop_log_exp_relationship() {
    proptest!(|(n in 1.0f64..1000.0)| {
        let log_n = n.log10();
        let exp_result = 10.0_f64.powf(log_n);
        prop_assert!((exp_result - n).abs() < 0.0001);
    });
}

// Property: Collection iteration should be deterministic
#[test]
fn prop_iteration_deterministic() {
    proptest!(|(items in prop::collection::vec(0i32..1000, 0..100))| {
        let iter1: Vec<_> = items.iter().copied().collect();
        let iter2: Vec<_> = items.iter().copied().collect();
        prop_assert_eq!(iter1, iter2);
    });
}

// Property: String trim removes only whitespace
#[test]
fn prop_trim_only_whitespace() {
    proptest!(|(s in "[ \\t\\n\\r]{1,20}test[ \\t\\n\\r]{1,20}")| {
        let trimmed = s.trim();
        prop_assert_eq!(trimmed, "test");
    });
}

// Property: Parse and format roundtrip
#[test]
fn prop_parse_format_roundtrip() {
    proptest!(|(n in 0u32..1000000)| {
        let s = format!("{}", n);
        let parsed = s.parse::<u32>().unwrap();
        prop_assert_eq!(n, parsed);
    });
}
