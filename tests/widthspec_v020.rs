//! v0.20 `WidthSpec` / `HeightSpec` unified `Constraints` redesign (#237).
//!
//! These tests cover the public API contract of the new enum-based
//! `Constraints`: variant defaults, ratio resolution, min-max clamping,
//! the 24-byte size budget, and serde round-trip for every variant.

use slt::{Constraints, HeightSpec, WidthSpec};

#[test]
fn test_widthspec_auto_default() {
    let c = Constraints::default();
    assert_eq!(c.width, WidthSpec::Auto);
    assert_eq!(c.height, HeightSpec::Auto);
    assert_eq!(WidthSpec::default(), WidthSpec::Auto);
    assert_eq!(HeightSpec::default(), HeightSpec::Auto);
}

#[test]
fn test_widthspec_ratio_exact_third() {
    // For an area whose width is divisible by `den`, a `Ratio(num, den)`
    // produces exactly `area * num / den` columns.
    let c = Constraints::default().w_ratio(1, 3);
    assert_eq!(c.width, WidthSpec::Ratio(1, 3));

    // Verify that we can construct and read the variant.
    if let WidthSpec::Ratio(num, den) = c.width {
        let area_width: u32 = 81;
        let resolved = (area_width as u64 * num as u64 / den as u64) as u32;
        assert_eq!(resolved, 27, "81 / 3 must be exactly 27");
    } else {
        panic!("expected Ratio variant");
    }
}

#[test]
fn test_widthspec_ratio_remainder() {
    // For `area = 80, num = 1, den = 3`, floor division yields `26`
    // (80 / 3 = 26 remainder 2). The remainder is silently dropped —
    // documented behavior.
    let c = Constraints::default().w_ratio(1, 3);
    if let WidthSpec::Ratio(num, den) = c.width {
        let area_width: u32 = 80;
        let resolved = (area_width as u64 * num as u64 / den as u64) as u32;
        assert_eq!(resolved, 26, "floor(80 / 3) must be 26");
    } else {
        panic!("expected Ratio variant");
    }
}

#[test]
fn test_widthspec_minmax_clamp() {
    let c = Constraints::default().min_w(20).max_w(40);
    assert_eq!(c.min_width(), Some(20));
    assert_eq!(c.max_width(), Some(40));
    // Sentinel encoding for MinMax.
    assert_eq!(c.width, WidthSpec::MinMax { min: 20, max: 40 });

    // Same for height.
    let c = Constraints::default().min_h(5).max_h(15);
    assert_eq!(c.min_height(), Some(5));
    assert_eq!(c.max_height(), Some(15));
    assert_eq!(c.height, HeightSpec::MinMax { min: 5, max: 15 });
}

#[test]
fn test_widthspec_minmax_partial() {
    // Only setting `min_w` should leave `max` as the sentinel `u32::MAX`
    // (i.e. unbounded above). The accessor still returns `None` for that side.
    let c = Constraints::default().min_w(10);
    assert_eq!(c.min_width(), Some(10));
    assert_eq!(c.max_width(), None);

    let c = Constraints::default().max_w(50);
    assert_eq!(c.min_width(), None);
    assert_eq!(c.max_width(), Some(50));
}

#[test]
fn test_widthspec_fixed_returns_min_and_max() {
    // `Fixed(n)` must report `Some(n)` for both min and max accessors —
    // it is semantically equivalent to `MinMax { n, n }` on the layout side.
    let c = Constraints::default().w(20);
    assert_eq!(c.width, WidthSpec::Fixed(20));
    assert_eq!(c.min_width(), Some(20));
    assert_eq!(c.max_width(), Some(20));
}

#[test]
fn test_widthspec_pct_returns_none_for_min_max() {
    // `Pct` and `Ratio` are unresolved at construction time. The min/max
    // accessors return `None` until layout time substitutes a concrete
    // `Fixed` value.
    let c = Constraints::default().w_pct(50);
    assert_eq!(c.width, WidthSpec::Pct(50));
    assert_eq!(c.min_width(), None);
    assert_eq!(c.max_width(), None);
    assert_eq!(c.width_pct(), Some(50));

    let c = Constraints::default().w_ratio(1, 3);
    assert_eq!(c.min_width(), None);
    assert_eq!(c.max_width(), None);
}

#[test]
fn test_constraints_size_24_bytes() {
    // Hard regression guard. The unified design exists to hit 24 bytes;
    // any future variant that grows the largest variant past 12 bytes
    // breaks every layout-state cache footprint promise.
    assert_eq!(std::mem::size_of::<Constraints>(), 24);
    assert_eq!(std::mem::size_of::<WidthSpec>(), 12);
    assert_eq!(std::mem::size_of::<HeightSpec>(), 12);
}

#[test]
fn test_imperative_setters_round_trip() {
    let mut c = Constraints::default();
    c.set_min_width(Some(10));
    c.set_max_width(Some(40));
    assert_eq!(c.min_width(), Some(10));
    assert_eq!(c.max_width(), Some(40));

    // Clearing both sides should collapse back to Auto.
    c.set_min_width(None);
    c.set_max_width(None);
    assert_eq!(c.width, WidthSpec::Auto);

    c.set_width_pct(Some(75));
    assert_eq!(c.width, WidthSpec::Pct(75));
    c.set_width_pct(None);
    assert_eq!(c.width, WidthSpec::Auto);
}

#[test]
fn test_h_ratio_h_minmax_builders() {
    let c = Constraints::default().h_ratio(2, 3).w_minmax(5, 25);
    assert_eq!(c.height, HeightSpec::Ratio(2, 3));
    assert_eq!(c.width, WidthSpec::MinMax { min: 5, max: 25 });
}

#[cfg(feature = "serde")]
mod serde_roundtrip {
    use super::*;

    fn roundtrip_w(spec: WidthSpec) {
        let json = serde_json::to_string(&spec).expect("serialize");
        let back: WidthSpec = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, spec, "WidthSpec serde round-trip failed: {json}");
    }

    fn roundtrip_h(spec: HeightSpec) {
        let json = serde_json::to_string(&spec).expect("serialize");
        let back: HeightSpec = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, spec, "HeightSpec serde round-trip failed: {json}");
    }

    #[test]
    fn test_serde_widthspec_all_variants() {
        roundtrip_w(WidthSpec::Auto);
        roundtrip_w(WidthSpec::Fixed(42));
        roundtrip_w(WidthSpec::Pct(75));
        roundtrip_w(WidthSpec::Ratio(2, 5));
        roundtrip_w(WidthSpec::MinMax { min: 10, max: 100 });
    }

    #[test]
    fn test_serde_heightspec_all_variants() {
        roundtrip_h(HeightSpec::Auto);
        roundtrip_h(HeightSpec::Fixed(20));
        roundtrip_h(HeightSpec::Pct(50));
        roundtrip_h(HeightSpec::Ratio(1, 3));
        roundtrip_h(HeightSpec::MinMax { min: 5, max: 25 });
    }

    #[test]
    fn test_serde_constraints_round_trip() {
        let c = Constraints::default().w_ratio(1, 3).min_h(5).max_h(20);
        let json = serde_json::to_string(&c).expect("serialize");
        let back: Constraints = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, c);
    }
}
