use ratatui::{prelude::*, widgets::*};

use crate::recommendation::Recommendation;
use crate::tui::state::AppState;

/// Render the recommendation badge with color-coded status.
pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let (text, style) = match state.recommendation {
        Recommendation::Promote => (
            " RECOMMEND PROMOTE ",
            Style::default().fg(Color::Black).bg(Color::Green).bold(),
        ),
        Recommendation::Hold => (
            " RECOMMEND HOLD ",
            Style::default().fg(Color::Black).bg(Color::Yellow).bold(),
        ),
        Recommendation::Rollback => (
            " RECOMMEND ROLLBACK ",
            Style::default().fg(Color::White).bg(Color::Red).bold(),
        ),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Recommendation ");

    let paragraph = Paragraph::new(Line::from(Span::styled(text, style)))
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(paragraph, area);
}
