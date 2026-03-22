use super::*;

mod collections;
mod events;
mod rich_markdown;
mod selection;
mod tree_widgets;

/// Inline markdown segment — text, link, or image reference.
enum MdInline {
    Text(String),
    Link { text: String, url: String },
    Image { alt: String },
}

fn calendar_month_name(month: u32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "???",
    }
}

fn calendar_move_cursor_by_days(state: &mut CalendarState, delta: i32) {
    let mut remaining = delta;
    while remaining != 0 {
        let days = CalendarState::days_in_month(state.year, state.month);
        if remaining > 0 {
            let forward = days.saturating_sub(state.cursor_day) as i32;
            if remaining <= forward {
                state.cursor_day += remaining as u32;
                return;
            }

            remaining -= forward + 1;
            if state.month == 12 {
                state.month = 1;
                state.year += 1;
            } else {
                state.month += 1;
            }
            state.cursor_day = 1;
        } else {
            let backward = state.cursor_day.saturating_sub(1) as i32;
            if -remaining <= backward {
                state.cursor_day -= (-remaining) as u32;
                return;
            }

            remaining += backward + 1;
            if state.month == 1 {
                state.month = 12;
                state.year -= 1;
            } else {
                state.month -= 1;
            }
            state.cursor_day = CalendarState::days_in_month(state.year, state.month);
        }
    }
}
