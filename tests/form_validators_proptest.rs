//! Property tests for the handwritten built-in form validators (#252).
//!
//! Focus: the length validators must agree with `chars().count()` over
//! arbitrary Unicode input, and `min_len`/`max_len` must be exact complements
//! of their bound.

use proptest::prelude::*;
use slt::validators;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// `min_len(n)` accepts exactly the strings with at least `n` chars.
    #[test]
    fn min_len_matches_char_count(s in ".*", n in 0usize..32) {
        let v = validators::min_len(n, "too short");
        let count = s.chars().count();
        let result = v(&s);
        if count >= n {
            prop_assert!(result.is_ok(), "len {count} >= {n} should pass: {s:?}");
        } else {
            prop_assert_eq!(result, Err("too short".to_string()));
        }
    }

    /// `max_len(n)` accepts exactly the strings with at most `n` chars.
    #[test]
    fn max_len_matches_char_count(s in ".*", n in 0usize..32) {
        let v = validators::max_len(n, "too long");
        let count = s.chars().count();
        let result = v(&s);
        if count <= n {
            prop_assert!(result.is_ok(), "len {count} <= {n} should pass: {s:?}");
        } else {
            prop_assert_eq!(result, Err("too long".to_string()));
        }
    }

    /// For any non-empty char-count range `[lo, hi]`, a string whose length is
    /// inside the range passes both bounds, and one outside fails the relevant
    /// bound. Counts characters (Unicode scalar values), not bytes.
    #[test]
    fn min_and_max_len_are_complementary(s in ".*", lo in 0usize..16, span in 0usize..16) {
        let hi = lo + span;
        let count = s.chars().count();
        let min = validators::min_len(lo, "min");
        let max = validators::max_len(hi, "max");
        let in_range = count >= lo && count <= hi;
        let passes_both = min(&s).is_ok() && max(&s).is_ok();
        prop_assert_eq!(in_range, passes_both, "len {} range [{},{}]", count, lo, hi);
    }

    /// `required` rejects exactly the strings that are empty after trimming.
    #[test]
    fn required_rejects_blank(s in ".*") {
        let v = validators::required("required");
        let blank = s.trim().is_empty();
        prop_assert_eq!(v(&s).is_err(), blank);
    }
}
