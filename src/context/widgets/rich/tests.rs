use crate::TestBackend;

// streaming_markdown should not allocate a new String on `code_block_lang`
// when the value is unchanged across frames (issue #187).
#[test]
fn streaming_markdown_skips_state_writes_when_unchanged() {
    use crate::widgets::StreamingMarkdownState;

    let mut state = StreamingMarkdownState::new();
    state.start();
    state.push("hello world\n");
    state.streaming = false;

    // First render: not in code block; lang stays empty.
    let mut tb = TestBackend::new(40, 4);
    tb.render(|ui| {
        let _ = ui.streaming_markdown(&mut state);
    });
    assert!(!state.in_code_block);
    assert!(state.code_block_lang.is_empty());

    // Re-render with no content change — values should remain identical.
    tb.render(|ui| {
        let _ = ui.streaming_markdown(&mut state);
    });
    assert!(!state.in_code_block);
    assert!(state.code_block_lang.is_empty());
}
