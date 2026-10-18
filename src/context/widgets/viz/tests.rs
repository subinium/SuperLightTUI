use super::*;

#[cfg(all(test, feature = "qrcode"))]
#[test]
fn test_qr_code() {
    let mut backend = crate::TestBackend::new(60, 30);
    backend.render(|ui| {
        let _ = ui.qr_code("hello");
    });

    let output = backend.to_string();
    assert!(output.contains('▀') || output.contains('█'));
}

#[test]
fn treemap_cjk_label_no_panic() {
    use super::TreemapItem;
    use crate::style::Color;
    let mut backend = crate::TestBackend::new(20, 10);
    backend.render(|ui| {
        let _ = ui.treemap(&[
            TreemapItem::new("한글파일", 100.0, Color::Cyan),
            TreemapItem::new("English", 50.0, Color::Yellow),
            TreemapItem::new("🎉파티", 30.0, Color::Green),
        ]);
    });
    backend.assert_contains("한글파일");
    backend.assert_contains("English");
}

#[test]
fn viz_widgets_return_outer_warm_frame_rects() {
    let mut sparkline_backend = crate::TestBackend::new(20, 4);
    let mut sparkline_response = Response::none();
    sparkline_backend.render(|ui| sparkline_response = ui.sparkline(&[1.0, 2.0, 3.0], 8));
    sparkline_backend.render(|ui| sparkline_response = ui.sparkline(&[1.0, 2.0, 3.0], 8));
    assert_eq!(sparkline_response.rect.width, 8);
    assert_eq!(sparkline_response.rect.height, 1);

    let mut chart_backend = crate::TestBackend::new(30, 10);
    let mut chart_response = Response::none();
    chart_backend.render(|ui| {
        chart_response = ui.line_chart(&[1.0, 3.0, 2.0], 12, 4).show();
    });
    chart_backend.render(|ui| {
        chart_response = ui.line_chart(&[1.0, 3.0, 2.0], 12, 4).show();
    });
    assert_eq!(chart_response.rect.width, 12);
    assert_eq!(chart_response.rect.height, 4);
}

#[test]
fn viz_non_finite_inputs_follow_missing_or_zero_policy() {
    let mut backend = crate::TestBackend::new(40, 12);
    backend.render(|ui| {
        let _ = ui.bar_chart(&[("nan", f64::NAN), ("inf", f64::INFINITY), ("ok", 2.0)], 8);
        let _ = ui.sparkline(&[f64::NEG_INFINITY, 1.0, f64::NAN, 2.0], 8);
    });
    let output = backend.to_string();
    assert!(!output.contains("NaN"));
    assert!(!output.contains("inf\u{221e}"));
}

#[test]
fn extreme_finite_viz_ratios_remain_bounded() {
    for value in [-f64::MAX, 0.0, f64::MAX] {
        let ratio = unit_ratio(value, -f64::MAX, f64::MAX);
        assert!(ratio.is_finite());
        assert!((0.0..=1.0).contains(&ratio));
    }
}
