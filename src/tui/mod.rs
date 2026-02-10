pub mod input;
pub mod state;
pub mod widgets;

use std::io;

use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, widgets::*};

use self::state::{AppState, HumanAction};

/// Run the TUI application.
pub fn run(initial_state: AppState) -> io::Result<Option<HumanAction>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = initial_state;
    let result = run_loop(&mut terminal, &mut state);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &mut AppState,
) -> io::Result<Option<HumanAction>> {
    loop {
        terminal.draw(|f| ui(f, state))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match input::handle_key(key.code, state) {
                        input::InputResult::Continue => {}
                        input::InputResult::Quit => return Ok(None),
                        input::InputResult::Action(action) => return Ok(Some(action)),
                    }
                }
            }
        }
    }
}

fn ui(frame: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(5), // Recommendation badge
            Constraint::Min(10),   // Evidence panel
            Constraint::Length(3), // Help bar
        ])
        .split(frame.area());

    // Header
    let header = Block::default()
        .borders(Borders::ALL)
        .title(" canary-gate ");
    let header_text = Paragraph::new(format!(
        "Deployment: {} | Cycle: {}",
        state.deployment_id, state.total_cycles
    ))
    .block(header);
    frame.render_widget(header_text, chunks[0]);

    // Recommendation
    widgets::recommendation::render(frame, chunks[1], state);

    // Evidence panel
    widgets::evidence::render(frame, chunks[2], state);

    // Help bar
    let help = Paragraph::new(" [p] Promote  [r] Rollback  [h] Hold  [q] Quit ")
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL).title(" Actions "));
    frame.render_widget(help, chunks[3]);
}
