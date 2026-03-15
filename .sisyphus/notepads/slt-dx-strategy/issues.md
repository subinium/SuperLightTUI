# Issues & Gotchas

## [2026-03-15] Session ses_30f1110cfffeHLJwhBf9HHmFmD

### Worktree
- git worktree add fails with signal 10 (SIGBUS) — working directly on dx/wave-1-trust branch
- Push also had signal 10 issue earlier — may need retry

### slt_warn is a function not macro
- Momus noted: "slt_warn is currently a function, not a macro"
- When extending, keep consistent (can convert to macro or keep as function)

### widget_path storage
- Momus noted: may need owned Strings if titles are included (bordered().title("X"))
- Plan says Vec<&'static str> but "X" from title() is runtime data → needs Vec<String>
