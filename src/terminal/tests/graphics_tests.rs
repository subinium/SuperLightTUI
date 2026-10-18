//! Kitty and sprixel graphics flush tests.

use super::*;

#[test]
fn kitty_delete_all_requires_fresh_upload_and_placement() {
    let mut manager = KittyImageManager::new();
    let current = kitty_placements(1);
    let mut bytes = Vec::new();
    manager.flush(&mut bytes, &current, 0).unwrap();
    manager.delete_all(&mut bytes).unwrap();
    bytes.clear();
    manager.flush(&mut bytes, &current, 0).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains("a=t,"));
    assert!(text.contains("a=p,"));
}

#[test]
fn kitty_shape_and_crop_participate_in_identity() {
    let mut manager = KittyImageManager::new();
    let mut current = kitty_placements(1);
    let mut bytes = Vec::new();
    manager.flush(&mut bytes, &current, 0).unwrap();
    bytes.clear();
    current[0].src_width = 16;
    current[0].src_height = 4;
    manager.flush(&mut bytes, &current, 0).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("s=16,v=4"));
    bytes.clear();
    current[0].crop_y = 1;
    current[0].crop_h = 2;
    manager.flush(&mut bytes, &current, 3).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(!text.contains("a=t,"));
    assert!(text.contains("y=1,h=2"));
    bytes.clear();
    manager.flush(&mut bytes, &current, 3).unwrap();
    assert!(bytes.is_empty());
}

#[test]
fn kitty_failed_output_invalidates_uploads() {
    struct Fails;
    impl Write for Fails {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("injected"))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut manager = KittyImageManager::new();
    let mut placements = kitty_placements(1);
    manager.flush(&mut Vec::new(), &placements, 0).unwrap();
    placements[0].x += 1;
    assert!(manager.flush(&mut Fails, &placements, 0).is_err());
    assert!(manager.uploaded.is_empty());
    let mut bytes = Vec::new();
    manager.flush(&mut bytes, &placements, 0).unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("a=t,"));
}

/// Build `n` distinct 8x8 RGBA placements for kitty-flush golden tests.
fn kitty_placements(n: usize) -> Vec<KittyPlacement> {
    (0..n)
        .map(|i| {
            let mut rgba = vec![0u8; 256];
            rgba[0] = i as u8;
            let content_hash = crate::buffer::hash_rgba(&rgba);
            KittyPlacement {
                content_hash,
                rgba: std::sync::Arc::new(rgba),
                src_width: 8,
                src_height: 8,
                x: (i as u32) * 4,
                y: (i as u32) * 2,
                cols: 4,
                rows: 2,
                crop_y: 0,
                crop_h: 0,
            }
        })
        .collect()
}

#[test]
fn captured_sink_suppresses_kitty_graphics_bytes() {
    let mut term = Terminal::with_sink(8, 4, ColorDepth::TrueColor);
    term.graphics_support = GraphicsEmissionSupport {
        real_terminal: true,
        capabilities: Capabilities::default(),
        force_kitty: false,
        force_sixel: false,
        force_iterm: false,
    };
    for placement in kitty_placements(1) {
        term.buffer_mut().kitty_place(placement);
    }
    term.flush().unwrap();
    let bytes = term.take_sink_bytes();
    assert!(
        !contains_seq(&bytes, b"\x1b_G"),
        "captured sink must not emit Kitty APC bytes: {:?}",
        String::from_utf8_lossy(&bytes)
    );
}

/// Issue #269: replacing the two per-frame `HashSet`s in
/// `KittyImageManager::flush` with reused `SmallVec` dedup scratch must not
/// change the emitted escape stream for the small placement counts (0, 1, 5)
/// the path actually sees. We assert structural invariants of the byte
/// stream rather than an opaque golden blob so the test documents intent.
#[test]
fn kitty_flush_smallvec_dedup_matches_for_small_n() {
    for n in [0usize, 1, 5] {
        let placements = kitty_placements(n);
        let mut mgr = KittyImageManager::new();

        // Frame 1: nothing previously placed → upload + place each image.
        let mut frame1: Vec<u8> = Vec::new();
        mgr.flush(&mut frame1, &placements, 0).unwrap();
        let s1 = String::from_utf8_lossy(&frame1);
        // One transmit (`a=t`) and one placement (`a=p`) per image.
        assert_eq!(
            s1.matches("a=t,").count(),
            n,
            "n={n}: expected {n} uploads in frame 1: {s1:?}"
        );
        assert_eq!(
            s1.matches("a=p,").count(),
            n,
            "n={n}: expected {n} placements in frame 1: {s1:?}"
        );

        // Frame 2: identical placements → fast path, zero output.
        let mut frame2: Vec<u8> = Vec::new();
        mgr.flush(&mut frame2, &placements, 0).unwrap();
        assert!(
            frame2.is_empty(),
            "n={n}: identical frame must hit the kitty fast path, got {} bytes",
            frame2.len()
        );

        // Frame 3: clear all placements → one delete (`a=d,d=i`) per image,
        // deduped by the reused SmallVec, plus image-data cleanup
        // (`a=d,d=I`) for every now-unused upload.
        let mut frame3: Vec<u8> = Vec::new();
        mgr.flush(&mut frame3, &[], 0).unwrap();
        let s3 = String::from_utf8_lossy(&frame3);
        assert_eq!(
            s3.matches("a=d,d=i,").count(),
            n,
            "n={n}: expected {n} placement deletes in frame 3: {s3:?}"
        );
        assert_eq!(
            s3.matches("a=d,d=I,").count(),
            n,
            "n={n}: expected {n} image-data deletes in frame 3: {s3:?}"
        );
    }
}

// ---- #265 sprixel damage matrix ----------------------------------------

use crate::buffer::{SprixelCell, SprixelPlacement};

/// Build a 2×2-cell sprixel at (1, 1) with the given footprint states.
fn make_sprixel(cells: Vec<SprixelCell>) -> SprixelPlacement {
    SprixelPlacement {
        content_hash: 0xABCD,
        seq: "<SIXEL>".into(),
        x: 1,
        y: 1,
        cols: 2,
        rows: 2,
        cells,
    }
}

#[test]
fn previous_only_sprixel_repaints_footprint_and_restores_overlap() {
    let area = Rect::new(0, 0, 10, 5);
    let removed = make_sprixel(vec![SprixelCell::Opaque; 4]);
    let mut stable = make_sprixel(vec![SprixelCell::Opaque; 4]);
    stable.x = 5;
    stable.content_hash = 0xBEEF;
    stable.seq = "<STABLE>".into();

    let mut current = Buffer::empty(area);
    current.sprixels.push(stable.clone());
    let mut previous = Buffer::empty(area);
    previous.sprixels.push(removed);
    previous.sprixels.push(stable);

    let rows = previous_only_sprixel_rows(&current, &previous, |_| true);
    assert_eq!(rows.as_slice(), &[1, 2]);

    let mut text = Vec::new();
    flush_buffer_diff_rows(
        &mut text,
        &current,
        &previous,
        ColorDepth::TrueColor,
        0,
        &mut String::new(),
        &rows,
        WidthPolicy::default(),
    )
    .unwrap();
    assert!(!text.is_empty(), "removed footprint rows must be repainted");

    let mut graphics = Vec::new();
    flush_sprixels_inner(&mut graphics, &current, &previous, 0, |_| true, &rows).unwrap();
    assert_eq!(
        String::from_utf8(graphics)
            .unwrap()
            .matches("<STABLE>")
            .count(),
        1,
        "row repaint must restore a surviving overlapping graphic"
    );
}

#[test]
fn terminal_flush_erases_removed_sprixel_with_text_redraw() {
    let mut term = Terminal::with_sink(10, 5, ColorDepth::TrueColor);
    term.graphics_support = GraphicsEmissionSupport {
        real_terminal: true,
        capabilities: Capabilities {
            sixel: true,
            ..Default::default()
        },
        force_kitty: false,
        force_sixel: false,
        force_iterm: false,
    };
    let mut placement = make_sprixel(vec![SprixelCell::Opaque; 4]);
    placement.seq = "\x1bPqpayload\x1b\\".into();
    term.current.sprixels.push(placement);
    term.flush().unwrap();
    assert!(String::from_utf8_lossy(&term.take_sink_bytes()).contains("payload"));

    term.flush().unwrap();
    let erase = term.take_sink_bytes();
    assert!(!erase.is_empty());
    assert!(!String::from_utf8_lossy(&erase).contains("payload"));
}

#[test]
fn checked_sprixel_flush_suppresses_mux_without_ack_or_force() {
    let area = Rect::new(0, 0, 10, 5);
    let mut placement = make_sprixel(vec![SprixelCell::Opaque; 4]);
    placement.seq = "\x1bPqpayload\x1b\\".into();

    let mut current = Buffer::empty(area);
    current.sprixels.push(placement);
    let previous = Buffer::empty(area);
    let support = GraphicsEmissionSupport {
        real_terminal: true,
        capabilities: Capabilities::default(),
        force_kitty: false,
        force_sixel: false,
        force_iterm: false,
    };

    let mut out = Vec::new();
    flush_sprixels_checked(&mut out, &current, &previous, 0, support).unwrap();
    assert!(
        out.is_empty(),
        "tmux/screen without ack must not emit Sixel"
    );
}

#[test]
fn checked_sprixel_flush_allows_mux_with_probe_ack() {
    let area = Rect::new(0, 0, 10, 5);
    let mut placement = make_sprixel(vec![SprixelCell::Opaque; 4]);
    placement.seq = "\x1bPqpayload\x1b\\".into();

    let mut current = Buffer::empty(area);
    current.sprixels.push(placement);
    let previous = Buffer::empty(area);
    let support = GraphicsEmissionSupport {
        real_terminal: true,
        capabilities: Capabilities {
            sixel: true,
            ..Default::default()
        },
        force_kitty: false,
        force_sixel: false,
        force_iterm: false,
    };

    let mut out = Vec::new();
    flush_sprixels_checked(&mut out, &current, &previous, 0, support).unwrap();
    assert!(
        contains_seq(&out, b"\x1bPqpayload\x1b\\"),
        "probe-acked Sixel should emit: {:?}",
        String::from_utf8_lossy(&out)
    );
}

#[test]
fn sprixel_no_text_change_emits_zero_bytes() {
    // A frame identical to the previous one must emit no sprixel bytes.
    let area = Rect::new(0, 0, 10, 5);
    let placement = make_sprixel(vec![SprixelCell::Opaque; 4]);

    let mut current = Buffer::empty(area);
    current.sprixels.push(placement.clone());
    let mut previous = Buffer::empty(area);
    previous.sprixels.push(placement);

    let mut out: Vec<u8> = Vec::new();
    flush_sprixels(&mut out, &current, &previous, 0).unwrap();
    assert!(out.is_empty(), "stable frame should emit no sprixel bytes");
}

#[test]
fn sprixel_first_frame_blits_once() {
    // No previous placement -> the graphic must be emitted exactly once.
    let area = Rect::new(0, 0, 10, 5);
    let mut current = Buffer::empty(area);
    current
        .sprixels
        .push(make_sprixel(vec![SprixelCell::Opaque; 4]));
    let previous = Buffer::empty(area);

    let mut out: Vec<u8> = Vec::new();
    flush_sprixels(&mut out, &current, &previous, 0).unwrap();
    let s = String::from_utf8(out).unwrap();
    assert_eq!(s.matches("<SIXEL>").count(), 1);
}

#[test]
fn sprixel_text_in_opaque_cell_reblits_once() {
    // A text write over an opaque footprint cell annihilates the graphic.
    let area = Rect::new(0, 0, 10, 5);
    let placement = make_sprixel(vec![SprixelCell::Opaque; 4]);

    let mut current = Buffer::empty(area);
    current.sprixels.push(placement.clone());
    // Write a glyph over the top-left footprint cell (1, 1).
    current.set_char(1, 1, 'X', Style::new());

    let mut previous = Buffer::empty(area);
    previous.sprixels.push(placement);

    let mut out: Vec<u8> = Vec::new();
    flush_sprixels(&mut out, &current, &previous, 0).unwrap();
    let s = String::from_utf8(out).unwrap();
    assert_eq!(
        s.matches("<SIXEL>").count(),
        1,
        "opaque-cell text write must re-blit the graphic exactly once"
    );
}

#[test]
fn sprixel_text_in_transparent_cell_does_not_reblit() {
    // The footprint marks (1, 1) transparent; a text write there must NOT
    // re-blit the graphic (the core #265 win).
    let area = Rect::new(0, 0, 10, 5);
    let cells = vec![
        SprixelCell::Transparent, // (1, 1)
        SprixelCell::Opaque,      // (2, 1)
        SprixelCell::Opaque,      // (1, 2)
        SprixelCell::Opaque,      // (2, 2)
    ];
    let placement = make_sprixel(cells);

    let mut current = Buffer::empty(area);
    current.sprixels.push(placement.clone());
    current.set_char(1, 1, 'X', Style::new());

    let mut previous = Buffer::empty(area);
    previous.sprixels.push(placement);

    let mut out: Vec<u8> = Vec::new();
    flush_sprixels(&mut out, &current, &previous, 0).unwrap();
    assert!(
        out.is_empty(),
        "text in a transparent footprint cell must emit zero sprixel bytes"
    );
}

#[test]
fn sprixel_text_outside_footprint_does_not_reblit() {
    // A text write adjacent to (but outside) the footprint is free.
    let area = Rect::new(0, 0, 10, 5);
    let placement = make_sprixel(vec![SprixelCell::Opaque; 4]);

    let mut current = Buffer::empty(area);
    current.sprixels.push(placement.clone());
    // (5, 0) is well outside the (1,1)-(2,2) footprint.
    current.set_char(5, 0, 'Z', Style::new());

    let mut previous = Buffer::empty(area);
    previous.sprixels.push(placement);

    let mut out: Vec<u8> = Vec::new();
    flush_sprixels(&mut out, &current, &previous, 0).unwrap();
    assert!(
        out.is_empty(),
        "text outside the footprint must not re-blit the graphic"
    );
}

#[test]
fn sprixel_position_change_reblits() {
    // Moving the graphic (same content, new x/y) must re-blit.
    let area = Rect::new(0, 0, 10, 5);
    let mut moved = make_sprixel(vec![SprixelCell::Opaque; 4]);
    let original = moved.clone();
    moved.x = 4;

    let mut current = Buffer::empty(area);
    current.sprixels.push(moved);
    let mut previous = Buffer::empty(area);
    previous.sprixels.push(original);

    let mut out: Vec<u8> = Vec::new();
    flush_sprixels(&mut out, &current, &previous, 0).unwrap();
    let s = String::from_utf8(out).unwrap();
    assert_eq!(s.matches("<SIXEL>").count(), 1);
}

#[test]
fn sprixel_content_change_reblits() {
    // Same position, different content hash -> re-blit.
    let area = Rect::new(0, 0, 10, 5);
    let mut recolored = make_sprixel(vec![SprixelCell::Opaque; 4]);
    let original = recolored.clone();
    recolored.content_hash = 0x1234;
    recolored.seq = "<SIXEL2>".into();

    let mut current = Buffer::empty(area);
    current.sprixels.push(recolored);
    let mut previous = Buffer::empty(area);
    previous.sprixels.push(original);

    let mut out: Vec<u8> = Vec::new();
    flush_sprixels(&mut out, &current, &previous, 0).unwrap();
    let s = String::from_utf8(out).unwrap();
    assert_eq!(s.matches("<SIXEL2>").count(), 1);
}

#[test]
fn sprixel_reblit_count_invariant_over_single_cell_writes() {
    // Invariant (issue #265 proptest spirit, exhaustive here): for a write
    // to a single footprint cell, the number of re-emitted sprixels is 0
    // iff that cell is Transparent, else 1.
    let area = Rect::new(0, 0, 10, 5);
    for (idx, (col, row)) in [(0u32, 0u32), (1, 0), (0, 1), (1, 1)]
        .into_iter()
        .enumerate()
    {
        for state in [
            SprixelCell::Opaque,
            SprixelCell::Mixed,
            SprixelCell::Transparent,
        ] {
            let mut cells = vec![SprixelCell::Opaque; 4];
            cells[idx] = state;
            let placement = make_sprixel(cells);

            let mut current = Buffer::empty(area);
            current.sprixels.push(placement.clone());
            current.set_char(1 + col, 1 + row, 'A', Style::new());

            let mut previous = Buffer::empty(area);
            previous.sprixels.push(placement);

            let mut out: Vec<u8> = Vec::new();
            flush_sprixels(&mut out, &current, &previous, 0).unwrap();
            let count = String::from_utf8(out).unwrap().matches("<SIXEL>").count();
            let expected = if matches!(state, SprixelCell::Transparent) {
                0
            } else {
                1
            };
            assert_eq!(
                count, expected,
                "cell ({col},{row}) state {state:?}: expected {expected} re-blits"
            );
        }
    }
}

// ---- Sprixel reblit-scan optimization regression -----------------------
//
// These drive the hashed-key position lookup and the per-row written-span
// shortcut (#486), proving the optimization preserves the exact #265 re-blit
// semantics.

#[test]
fn sprixel_unchanged_with_span_shortcut_emits_zero_bytes() {
    // Regression: a steady frame (identical to previous) with per-row
    // digests refreshed must NOT re-blit. This exercises the per-row
    // clean+hash shortcut: every footprint row is clean and hash-matched, so
    // the per-cell scan is skipped and nothing is emitted.
    let area = Rect::new(0, 0, 10, 5);
    let placement = make_sprixel(vec![SprixelCell::Opaque; 4]);

    let mut current = Buffer::empty(area);
    current.sprixels.push(placement.clone());
    let mut previous = Buffer::empty(area);
    previous.sprixels.push(placement);

    // Sanity: the footprint rows are provably identical, so the shortcut is
    // the path actually taken.
    assert!(crate::buffer::same_layout(&current, &previous));
    assert_eq!(crate::buffer::row_diff_range(&current, &previous, 1), None);
    assert_eq!(crate::buffer::row_diff_range(&current, &previous, 2), None);

    let mut out: Vec<u8> = Vec::new();
    flush_sprixels(&mut out, &current, &previous, 0).unwrap();
    assert!(
        out.is_empty(),
        "unchanged sprixel must not be re-blitted (per-row shortcut)"
    );
}

#[test]
fn sprixel_changed_text_with_span_shortcut_reblits_once() {
    // Regression: a text write over an opaque footprint cell must still
    // re-blit exactly once even with digests refreshed. The touched row is
    // dirty (or hash-mismatched), so the shortcut correctly does NOT skip it
    // and the per-cell annihilation scan fires.
    let area = Rect::new(0, 0, 10, 5);
    let placement = make_sprixel(vec![SprixelCell::Opaque; 4]);

    let mut current = Buffer::empty(area);
    current.sprixels.push(placement.clone());
    current.set_char(1, 1, 'X', Style::new());
    let mut previous = Buffer::empty(area);
    previous.sprixels.push(placement);

    // The footprint's top row differs from the previous frame.
    assert_eq!(
        crate::buffer::row_diff_range(&current, &previous, 1),
        Some((1, 2))
    );

    let mut out: Vec<u8> = Vec::new();
    flush_sprixels(&mut out, &current, &previous, 0).unwrap();
    let s = String::from_utf8(out).unwrap();
    assert_eq!(
        s.matches("<SIXEL>").count(),
        1,
        "annihilating text write must re-blit exactly once"
    );
}

#[test]
fn sprixel_changed_text_in_transparent_cell_with_span_shortcut_does_not_reblit() {
    // Regression edge case: even though the touched row is dirty/hash-mismatched
    // (so the per-row shortcut does NOT skip it), a write landing only on a
    // Transparent footprint cell must still emit zero bytes — the per-cell
    // damage matrix governs, exactly as in the unoptimized path.
    let area = Rect::new(0, 0, 10, 5);
    let cells = vec![
        SprixelCell::Transparent, // (1, 1)
        SprixelCell::Opaque,      // (2, 1)
        SprixelCell::Opaque,      // (1, 2)
        SprixelCell::Opaque,      // (2, 2)
    ];
    let placement = make_sprixel(cells);

    let mut current = Buffer::empty(area);
    current.sprixels.push(placement.clone());
    current.set_char(1, 1, 'X', Style::new());
    let mut previous = Buffer::empty(area);
    previous.sprixels.push(placement);

    let mut out: Vec<u8> = Vec::new();
    flush_sprixels(&mut out, &current, &previous, 0).unwrap();
    assert!(
        out.is_empty(),
        "transparent-cell text write must not re-blit even with hashes engaged"
    );
}

#[test]
fn sprixel_key_matches_partial_eq_contract() {
    // The hashed identity key must agree with `SprixelPlacement: PartialEq`:
    // equal placements share a key; any field the PartialEq compares
    // produces a distinct key.
    let base = make_sprixel(vec![SprixelCell::Opaque; 4]);
    assert_eq!(sprixel_key(&base), sprixel_key(&base.clone()));

    let mut moved = base.clone();
    moved.x = 7;
    assert_ne!(sprixel_key(&base), sprixel_key(&moved));

    let mut recolored = base.clone();
    recolored.content_hash = 0x9999;
    assert_ne!(sprixel_key(&base), sprixel_key(&recolored));

    // The damage matrix is excluded from both PartialEq and the key.
    let mut annihilated = base.clone();
    annihilated.cells = vec![SprixelCell::Annihilated; 4];
    assert_eq!(sprixel_key(&base), sprixel_key(&annihilated));
    assert_eq!(base, annihilated);
}

#[test]
fn sprixel_multi_placement_only_changed_one_reblits() {
    // With several stacked sprixels, moving one must re-blit only that one;
    // the others (clean, hash-matched) stay silent. Exercises the hash-set
    // position lookup across multiple placements.
    let area = Rect::new(0, 0, 10, 9);
    let mut current = Buffer::empty(area);
    let mut previous = Buffer::empty(area);
    for i in 0..3u32 {
        let p = SprixelPlacement {
            content_hash: 0x100 + i as u64,
            seq: format!("<S{i}>").into(),
            x: 0,
            y: i * 3,
            cols: 2,
            rows: 2,
            cells: vec![SprixelCell::Opaque; 4],
        };
        current.sprixels.push(p.clone());
        previous.sprixels.push(p);
    }
    // Move only the middle sprixel.
    current.sprixels[1].x = 5;

    let mut out: Vec<u8> = Vec::new();
    flush_sprixels(&mut out, &current, &previous, 0).unwrap();
    let s = String::from_utf8(out).unwrap();
    assert_eq!(s.matches("<S0>").count(), 0);
    assert_eq!(
        s.matches("<S1>").count(),
        1,
        "only the moved sprixel reblits"
    );
    assert_eq!(s.matches("<S2>").count(), 0);
}

#[test]
fn bench_sprixel_fixture_steady_state_emits_nothing() {
    // The bench fixture must represent a steady frame (no re-blit) so it
    // measures the no-damage scan cost. Guards against the wrapper silently
    // emitting work.
    let fixture = __bench_new_sprixel_fixture(4);
    assert_eq!(fixture.len(), 4);
    assert!(!fixture.is_empty());
    let mut out: Vec<u8> = Vec::new();
    fixture.flush(&mut out, 0).unwrap();
    assert!(
        out.is_empty(),
        "steady-state bench fixture re-blits nothing"
    );
}
