//! Issue #469: accessible linear output mode. Each test drives the real
//! frame kernel through a scripted line session and checks the transcript
//! and the widget state the answers produced.

use super::*;

use std::collections::VecDeque;

use crate::widgets::{
    FormField, MultiSelectState, RadioState, SelectState, TextInputState, TextareaState, validators,
};
use crate::{RunConfig, TestBackend};

/// Scripted input; output is a transcript with answers echoed the way a
/// terminal in cooked mode would (masked answers are not echoed).
struct Script {
    input: VecDeque<String>,
    out: String,
    masked_reads: usize,
}

impl Script {
    fn new(lines: &[&str]) -> Self {
        Self {
            input: lines.iter().map(|line| (*line).to_string()).collect(),
            out: String::new(),
            masked_reads: 0,
        }
    }
}

impl A11yIo for Script {
    fn write_str(&mut self, text: &str) -> io::Result<()> {
        self.out.push_str(text);
        Ok(())
    }

    fn ask(&mut self, prompt: &str, masked: bool) -> io::Result<Option<String>> {
        self.out.push_str(prompt);
        let line = self.input.pop_front();
        if masked {
            self.masked_reads += 1;
            if line.is_some() {
                self.out.push('\n');
            }
        } else if let Some(line) = &line {
            self.out.push_str(line);
            self.out.push('\n');
        }
        Ok(line)
    }
}

fn run(lines: &[&str], f: impl FnMut(&mut Context)) -> Script {
    let mut script = Script::new(lines);
    run_accessible_session(
        &mut script,
        &RunConfig::default(),
        (60, 20),
        f,
        &mut Vec::new,
    )
    .unwrap();
    script
}

fn count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

#[test]
fn env_value_parsing_matches_other_slt_switches() {
    for on in ["1", "true", "TRUE", "yes", "on", " 1 "] {
        assert!(env_truthy(on), "{on:?} should enable");
    }
    for off in ["", "0", "false", "no", "off", "2", "accessible"] {
        assert!(!env_truthy(off), "{off:?} should not enable");
    }
}

#[test]
fn explicit_builder_wins_over_the_default() {
    assert!(RunConfig::default().accessible(true).accessible);
    assert!(!RunConfig::default().accessible(false).accessible);
}

#[test]
fn visual_rendering_never_enters_accessible_mode() {
    let mut name = TextInputState::new();
    let mut seen = None;
    let mut tb = TestBackend::new(30, 6);
    tb.render(|ui| {
        seen = Some(ui.is_accessible());
        let _ = ui.a11y_label("Name").text_input(&mut name);
        assert!(ui.a11y_begin(true).is_none());
    });
    assert_eq!(seen, Some(false));
    tb.assert_contains("╭");
}

#[test]
fn text_input_reads_a_labelled_line_and_reports_submit() {
    let mut name = TextInputState::new();
    let mut submitted = 0;
    let script = run(&["Ada"], |ui| {
        ui.text("Username:");
        if ui.text_input(&mut name).submitted {
            submitted += 1;
            ui.quit();
        }
    });
    assert_eq!(name.value, "Ada");
    assert_eq!(submitted, 1);
    assert_eq!(
        script.out, "Focused: Username, text field\nUsername: Ada\n",
        "the preceding text names the field and is not printed twice"
    );
    assert!(!script.out.contains('\u{1b}'));
}

#[test]
fn empty_answer_keeps_the_current_value() {
    let mut city = TextInputState::with_placeholder("City");
    city.set_value("Seoul");
    let script = run(&[""], |ui| {
        if ui.text_input(&mut city).submitted {
            ui.quit();
        }
    });
    assert_eq!(city.value, "Seoul");
    assert!(script.out.contains("City [Seoul]: "), "{}", script.out);
}

#[test]
fn masked_input_is_read_without_echo_and_never_printed() {
    let mut password = TextInputState::password();
    let script = run(&["hunter2"], |ui| {
        let _ = ui.a11y_label("Password");
        if ui.text_input(&mut password).submitted {
            ui.quit();
        }
    });
    assert_eq!(password.value, "hunter2");
    assert_eq!(script.masked_reads, 1);
    assert!(!script.out.contains("hunter2"), "{}", script.out);
    assert!(script.out.contains("Focused: Password, password field"));
    assert!(script.out.contains("Password (hidden): "));
}

#[test]
fn max_length_is_enforced_before_applying() {
    let mut code = TextInputState::with_placeholder("Code").max_length(3);
    let script = run(&["12345", "123"], |ui| {
        if ui.text_input(&mut code).submitted {
            ui.quit();
        }
    });
    assert_eq!(code.value, "123");
    assert!(script.out.contains("Invalid: must be at most 3 characters"));
}

#[test]
fn select_prints_numbered_options_and_reprompts_invalid_numbers() {
    let mut color = SelectState::new(vec!["Red", "Green", "Blue"]);
    let mut changed = false;
    let script = run(&["7", "x", "3"], |ui| {
        let _ = ui.a11y_label("Color");
        if ui.select(&mut color).changed {
            changed = true;
            ui.quit();
        }
    });
    assert_eq!(color.selected, 2);
    assert!(changed);
    assert!(
        script.out.contains(
            "Focused: Color, select, 3 options\n1. Red (selected)\n2. Green\n3. Blue\n\
             Enter a number between 1 and 3 [1]: 7\n\
             Invalid: must be a number between 1 and 3\n\
             Enter a number between 1 and 3 [1]: x\n"
        ),
        "{}",
        script.out
    );
    assert_eq!(count(&script.out, "Invalid:"), 2);
    assert!(script.out.contains("Selected: Blue"));
}

#[test]
fn radio_and_multi_select_answer_by_number() {
    let mut size = RadioState::new(vec!["Small", "Large"]);
    let mut toppings = MultiSelectState::new(vec!["Cheese", "Olives", "Basil"]);
    let script = run(&["2", "1", "3", "0", ""], |ui| {
        ui.text("Size");
        let _ = ui.radio(&mut size);
        ui.text("Toppings");
        let _ = ui.multi_select(&mut toppings);
        if ui.button("Order").clicked {
            ui.quit();
        }
    });
    assert_eq!(size.selected, 1);
    assert_eq!(toppings.selected_items(), vec!["Cheese", "Basil"]);
    let out = &script.out;
    assert!(
        out.contains("Focused: Size, radio group, 2 options\n1. Small (selected)\n2. Large\n"),
        "{out}"
    );
    assert!(
        out.contains("Focused: Toppings, multi-select list, 3 options"),
        "{out}"
    );
    assert!(
        out.contains("1. Cheese (selected)\n2. Olives\n3. Basil (selected)\n0. Done"),
        "{out}"
    );
    assert!(out.contains("Focused: Order, button"), "{out}");
}

#[test]
fn checkbox_toggle_and_confirm_read_yes_or_no() {
    let mut terms = false;
    let mut dark = true;
    let mut sure = false;
    let mut answered = false;
    let script = run(&["maybe", "y", "n", ""], |ui| {
        let _ = ui.checkbox("Accept terms", &mut terms);
        let _ = ui.toggle("Dark mode", &mut dark);
        if ui.confirm("Create account?", &mut sure).clicked {
            answered = true;
            ui.quit();
        }
    });
    assert!(terms);
    assert!(!dark);
    assert!(answered && !sure, "empty answer takes the default (No)");
    assert!(script.out.contains("Focused: Accept terms, check box, not checked\nAccept terms [y/N]: maybe\nInvalid: please answer y or n\n"));
    assert!(script.out.contains("Accept terms: checked"));
    assert!(
        script
            .out
            .contains("Focused: Dark mode, switch, on\nDark mode [Y/n]: n\nDark mode: off")
    );
    assert!(
        script
            .out
            .contains("Focused: Create account?, yes or no question\nCreate account? [y/N]: ")
    );
}

#[test]
fn buttons_activate_on_enter_and_can_be_skipped() {
    let mut cancel = 0;
    let mut save = 0;
    let script = run(&["s", ""], |ui| {
        if ui.button("Cancel").clicked {
            cancel += 1;
        }
        if ui.button("Save").clicked {
            save += 1;
            ui.quit();
        }
    });
    assert_eq!((cancel, save), (0, 1));
    assert!(script.out.contains("Focused: Cancel, button\nPress Enter to activate Cancel, or type s to skip: s\nFocused: Save, button\n"));
}

#[test]
fn form_fields_prompt_in_order_and_print_validation_messages() {
    let mut fields = [
        FormField::new("Email").validate(validators::email()),
        FormField::new("Password").password(),
    ];
    let mut submitted = None;
    let script = run(&["not-an-email", "ada@example.com", "secret", ""], |ui| {
        ui.text("Sign in");
        for field in &mut fields {
            ui.form_field(field);
        }
        if ui.form_submit("Sign in now").clicked {
            submitted = Some((fields[0].input.value.clone(), fields[1].input.value.clone()));
            ui.quit();
        }
    });
    assert_eq!(
        submitted,
        Some(("ada@example.com".to_string(), "secret".to_string()))
    );
    let out = &script.out;
    assert!(
        out.starts_with("Sign in\nFocused: Email, text field\nEmail: not-an-email\nError: "),
        "{out}"
    );
    assert!(out.contains("Email [not-an-email]: ada@example.com\nFocused: Password, password field\nPassword (hidden): \n"), "{out}");
    assert!(
        out.contains("Focused: Sign in now, button\nPress Enter to activate Sign in now"),
        "{out}"
    );
    assert!(!out.contains("secret"), "{out}");
    assert_eq!(count(out, "Sign in\n"), 1, "{out}");
}

#[test]
fn display_text_is_printed_once_and_again_when_it_changes() {
    let mut name = TextInputState::new();
    let script = run(&["a", "b", "c"], |ui| {
        ui.text("Welcome");
        let _ = ui.row(|ui| {
            ui.text("Typed:");
            ui.text(name.value.len().to_string());
        });
        ui.text("────────");
        let _ = ui.a11y_label("Word").text_input(&mut name);
    });
    let out = &script.out;
    assert_eq!(count(out, "Welcome"), 1, "{out}");
    assert!(out.starts_with("Welcome\nTyped: 0\nFocused: Word"), "{out}");
    assert!(out.contains("Word: a\nTyped: 1\n"), "{out}");
    assert!(
        out.contains("Word [a]: b\nTyped: 1\n") || out.contains("Word [a]: b\n"),
        "{out}"
    );
    assert!(!out.contains('─'), "decorative rules are dropped: {out}");
}

#[test]
fn textarea_reads_lines_until_an_empty_line() {
    let mut notes = TextareaState::new();
    let mut changed = false;
    let script = run(&["first", "second", ""], |ui| {
        let _ = ui.a11y_label("Notes");
        if ui.textarea(&mut notes, 3).changed {
            changed = true;
            ui.quit();
        }
    });
    assert!(changed);
    assert_eq!(notes.lines, vec!["first", "second"]);
    assert!(
        script
            .out
            .contains("Notes (end with an empty line): first\n> second\n> \n")
    );
}

#[test]
fn static_log_and_toasts_are_plain_lines() {
    let mut done = TextInputState::with_placeholder("Command");
    let script = run(&["go"], |ui| {
        if ui.text_input(&mut done).submitted {
            ui.static_log("ran go");
            ui.notify("Saved", crate::ToastLevel::Success);
            ui.quit();
        }
    });
    assert!(
        script.out.ends_with("Command: go\nran go\nSaved\n"),
        "{}",
        script.out
    );
}

#[test]
fn without_prompts_lines_are_sent_as_keys() {
    let mut pressed = String::new();
    let script = run(&["x", "q"], |ui| {
        ui.text("Press q to quit");
        for event in ui.events() {
            if let crate::Event::Key(key) = event
                && let crate::KeyCode::Char(ch) = key.code
            {
                pressed.push(ch);
            }
        }
        if ui.key('q') {
            ui.quit();
        }
    });
    assert_eq!(pressed, "xq");
    assert!(
        script
            .out
            .starts_with("Press q to quit\nNo prompt is active."),
        "{}",
        script.out
    );
}

#[test]
fn end_of_input_ends_the_session() {
    let mut name = TextInputState::new();
    let script = run(&[], |ui| {
        let _ = ui.text_input(&mut name);
    });
    assert!(script.out.ends_with("Input: "), "{}", script.out);
}

#[test]
fn app_driven_screen_change_is_prompted_without_skipping() {
    let mut step = 0;
    let mut first = TextInputState::new();
    let mut second = TextInputState::new();
    let script = run(&["one", "two"], |ui| {
        if step == 0 {
            if ui.a11y_label("First").text_input(&mut first).submitted {
                step = 1;
            }
        } else if ui.a11y_label("Second").text_input(&mut second).submitted {
            ui.quit();
        }
    });
    assert_eq!(
        (first.value.as_str(), second.value.as_str()),
        ("one", "two")
    );
    assert!(
        script.out.contains("Focused: Second, text field"),
        "{}",
        script.out
    );
}
