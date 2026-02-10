use ratatui::{prelude::*, widgets::*};

use crate::behavior::{TestEvaluation, TestResult};

/// Render a config criteria status showing which tests pass/fail.
pub fn render(frame: &mut Frame, area: Rect, evaluations: &[TestEvaluation]) {
    let items: Vec<ListItem> = evaluations
        .iter()
        .map(|eval| {
            let (icon, style) = match eval.result {
                TestResult::Pass => ("✓", Style::default().fg(Color::Green)),
                TestResult::Fail => ("✗", Style::default().fg(Color::Red)),
                TestResult::Unknown => ("?", Style::default().fg(Color::Yellow)),
            };
            ListItem::new(format!(" {} {}", icon, eval.test_name)).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Config Criteria "),
    );

    frame.render_widget(list, area);
}
