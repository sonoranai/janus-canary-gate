use ratatui::{prelude::*, widgets::*};

use crate::recommendation::Recommendation;

/// Render a cycle timeline showing pass/fail history.
pub fn render(frame: &mut Frame, area: Rect, history: &[(u32, Recommendation)]) {
    let spans: Vec<Span> = history
        .iter()
        .map(|(cycle, rec)| {
            let (symbol, style) = match rec {
                Recommendation::Promote => ("●", Style::default().fg(Color::Green)),
                Recommendation::Hold => ("◐", Style::default().fg(Color::Yellow)),
                Recommendation::Rollback => ("●", Style::default().fg(Color::Red)),
            };
            Span::styled(format!(" C{}: {} ", cycle, symbol), style)
        })
        .collect();

    let line = Line::from(spans);
    let paragraph = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Cycle Timeline "),
    );

    frame.render_widget(paragraph, area);
}
