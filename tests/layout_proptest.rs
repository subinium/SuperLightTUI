use proptest::prelude::*;
use slt::TestBackend;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn layout_never_panics_with_arbitrary_dimensions(
        w in 1u32..=300,
        h in 1u32..=100,
        gap in 0u32..=20,
        padding in 0u32..=10,
        children in 0usize..=50,
    ) {
        let mut tb = TestBackend::new(w, h);
        tb.render(|ui| {
            let _ = ui.container().gap(gap).p(padding).col(|ui| {
                for i in 0..children {
                    ui.text(format!("item {i}"));
                }
            });
        });
    }

    #[test]
    fn row_layout_with_extreme_grow(
        w in 1u32..=200,
        h in 1u32..=50,
        grow_a in 0u16..=100,
        grow_b in 0u16..=100,
        grow_c in 0u16..=100,
    ) {
        let mut tb = TestBackend::new(w, h);
        tb.render(|ui| {
            let _ = ui.row(|ui| {
                let _ = ui.container().grow(grow_a).col(|ui| { ui.text("a"); });
                let _ = ui.container().grow(grow_b).col(|ui| { ui.text("b"); });
                let _ = ui.container().grow(grow_c).col(|ui| { ui.text("c"); });
            });
        });
    }

    #[test]
    fn deeply_nested_containers(
        w in 10u32..=120,
        h in 10u32..=40,
        depth in 1usize..=30,
    ) {
        let mut tb = TestBackend::new(w, h);
        tb.render(|ui| {
            fn nest(ui: &mut slt::Context, remaining: usize) {
                if remaining == 0 {
                    ui.text("leaf");
                    return;
                }
                let _ = ui.container().p(1).col(|ui| {
                    nest(ui, remaining - 1);
                });
            }
            nest(ui, depth);
        });
    }

    #[test]
    fn grid_layout_arbitrary(
        w in 1u32..=200,
        h in 1u32..=50,
        cols in 1u32..=20,
        items in 0usize..=40,
    ) {
        let mut tb = TestBackend::new(w, h);
        tb.render(|ui| {
            let _ = ui.grid(cols, |ui| {
                for i in 0..items {
                    ui.text(format!("{i}"));
                }
            });
        });
    }

    #[test]
    fn percentage_sizing_boundaries(
        w in 1u32..=200,
        h in 1u32..=100,
        w_pct in 1u8..=100,
        h_pct in 1u8..=100,
    ) {
        let mut tb = TestBackend::new(w, h);
        tb.render(|ui| {
            let _ = ui.container().w_pct(w_pct).h_pct(h_pct).col(|ui| {
                ui.text("content");
            });
        });
    }
}
