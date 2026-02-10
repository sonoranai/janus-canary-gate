use ratatui::{prelude::*, widgets::*};

use crate::behavior::TestResult;
use crate::tui::state::AppState;

/// Render the evidence panel showing test results and reasoning.
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Test results table
    let rows: Vec<Row> = state
        .test_results
        .iter()
        .map(|eval| {
            let result_style = match eval.result {
                TestResult::Pass => Style::default().fg(Color::Green),
                TestResult::Fail => Style::default().fg(Color::Red),
                TestResult::Unknown => Style::default().fg(Color::Yellow),
            };
            Row::new(vec![
                Cell::from(eval.test_name.clone()),
                Cell::from(format!("{:?}", eval.result)).style(result_style),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [Constraint::Percentage(70), Constraint::Percentage(30)],
    )
    .header(
        Row::new(vec!["Test", "Result"])
            .style(Style::default().bold())
            .bottom_margin(1),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Test Results "),
    );

    frame.render_widget(table, chunks[0]);

    // Reasoning panel
    let reasoning_text: Vec<Line> = state
        .reasoning
        .iter()
        .map(|r| Line::from(format!("  - {}", r)))
        .collect();

    let reasoning = Paragraph::new(reasoning_text)
        .block(Block::default().borders(Borders::ALL).title(" Reasoning "))
        .wrap(Wrap { trim: true });

    frame.render_widget(reasoning, chunks[1]);
}
