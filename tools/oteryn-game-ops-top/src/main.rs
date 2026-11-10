//! Read-only live operations view for one local Oteryn node.

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use serde::Deserialize;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const REQUEST_LIMIT: u64 = 64 * 1024;

#[derive(Clone, Debug, Default, Deserialize)]
struct Health {
    readiness: bool,
    db_pool: String,
    platform: String,
    tick_ms: Option<u64>,
    connections: u64,
    #[serde(default)]
    stalled_tasks: Vec<Task>,
    #[serde(default)]
    last_errors: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Deserialize)]
struct Task {
    name: String,
    age_ms: u64,
}

#[derive(Clone, Debug, Deserialize)]
struct Diagnostic {
    ts: u64,
    code: String,
    module: String,
    event: String,
    #[serde(default)]
    trace: Option<String>,
}

struct App {
    socket: PathBuf,
    health: Health,
    status: String,
    code_filter: String,
    editing_filter: bool,
    diagnose: Vec<Diagnostic>,
}

fn request(socket: &Path, command: &str) -> Result<String, String> {
    let mut stream = UnixStream::connect(socket).map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    stream
        .write_all(format!("{command}\n").as_bytes())
        .map_err(|error| error.to_string())?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|error| error.to_string())?;
    let mut answer = String::new();
    stream
        .take(REQUEST_LIMIT)
        .read_to_string(&mut answer)
        .map_err(|error| error.to_string())?;
    Ok(answer)
}

impl App {
    fn refresh(&mut self) {
        match request(&self.socket, "health")
            .and_then(|value| serde_json::from_str(value.trim()).map_err(|error| error.to_string()))
        {
            Ok(health) => {
                self.health = health;
                self.status = "connected (q quit, d diagnose, / filter code)".into();
            }
            Err(error) => self.status = format!("unavailable: {error}"),
        }
    }

    fn diagnose(&mut self) {
        let command = if self.code_filter.is_empty() {
            "diagnose".to_owned()
        } else {
            format!("diagnose {}", self.code_filter)
        };
        match request(&self.socket, &command)
            .and_then(|value| serde_json::from_str(value.trim()).map_err(|error| error.to_string()))
        {
            Ok(lines) => self.diagnose = lines,
            Err(error) => self.status = format!("diagnose unavailable: {error}"),
        }
    }
}

fn draw(frame: &mut ratatui::Frame<'_>, app: &App) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(7),
            Constraint::Min(8),
        ])
        .split(frame.area());
    frame.render_widget(
        Paragraph::new(app.status.as_str()).block(
            Block::default()
                .title(" oteryn-game-ops top ")
                .borders(Borders::ALL),
        ),
        rows[0],
    );
    let tick = app
        .health
        .tick_ms
        .map_or_else(|| "n/a".into(), |value| format!("{value} ms"));
    let summary = format!(
        "health: {}\nDB pool: {}    Platform: {}\ntick: {}    connections: {}    stalled tasks: {}",
        if app.health.readiness {
            "ready"
        } else {
            "not ready"
        },
        app.health.db_pool,
        app.health.platform,
        tick,
        app.health.connections,
        app.health.stalled_tasks.len()
    );
    frame.render_widget(
        Paragraph::new(summary).block(Block::default().title(" node ").borders(Borders::ALL)),
        rows[1],
    );
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(rows[2]);
    let tasks = app
        .health
        .stalled_tasks
        .iter()
        .map(|task| ListItem::new(format!("{}  {} ms", task.name, task.age_ms)));
    frame.render_widget(
        List::new(tasks).block(
            Block::default()
                .title(" stalled tasks ")
                .borders(Borders::ALL),
        ),
        columns[0],
    );
    let source = if app.diagnose.is_empty() {
        &app.health.last_errors
    } else {
        &app.diagnose
    };
    let errors = source.iter().map(|item| {
        let indent = item.trace.as_ref().map_or("", |_| "  ");
        ListItem::new(Line::styled(
            format!(
                "{indent}{} {} {} {}",
                item.ts, item.code, item.module, item.event
            ),
            Style::default().fg(Color::LightRed),
        ))
    });
    let title = format!(
        " trace tree / last errors  filter={} ",
        if app.code_filter.is_empty() {
            "all"
        } else {
            &app.code_filter
        }
    );
    frame.render_widget(
        List::new(errors).block(Block::default().title(title).borders(Borders::ALL)),
        columns[1],
    );
}

fn run(socket: PathBuf) -> Result<(), String> {
    enable_raw_mode().map_err(|error| error.to_string())?;
    let mut output = std::io::stdout();
    execute!(output, EnterAlternateScreen).map_err(|error| error.to_string())?;
    let backend = CrosstermBackend::new(output);
    let mut terminal = Terminal::new(backend).map_err(|error| error.to_string())?;
    let mut app = App {
        socket,
        health: Health::default(),
        status: "connecting".into(),
        code_filter: String::new(),
        editing_filter: false,
        diagnose: Vec::new(),
    };
    let mut refresh_at = Instant::now();
    let result = loop {
        if Instant::now() >= refresh_at {
            app.refresh();
            refresh_at = Instant::now() + Duration::from_secs(1);
        }
        if let Err(error) = terminal.draw(|frame| draw(frame, &app)) {
            break Err(error.to_string());
        }
        if event::poll(Duration::from_millis(100)).map_err(|error| error.to_string())?
            && let Event::Key(key) = event::read().map_err(|error| error.to_string())?
            && key.kind == KeyEventKind::Press
        {
            if app.editing_filter {
                match key.code {
                    KeyCode::Enter => {
                        app.editing_filter = false;
                        app.diagnose();
                    }
                    KeyCode::Esc => app.editing_filter = false,
                    KeyCode::Backspace => {
                        app.code_filter.pop();
                    }
                    KeyCode::Char(character) if character.is_ascii_alphanumeric() => {
                        app.code_filter.push(character.to_ascii_uppercase())
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('q') => break Ok(()),
                    KeyCode::Char('d') => app.diagnose(),
                    KeyCode::Char('/') => app.editing_filter = true,
                    _ => {}
                }
            }
        }
    };
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    result
}

fn main() -> std::process::ExitCode {
    let mut arguments = std::env::args_os().skip(1);
    let Some(socket) = arguments.next() else {
        failure("usage");
        return std::process::ExitCode::from(2);
    };
    if arguments.next().is_some() {
        failure("usage");
        return std::process::ExitCode::from(2);
    }
    match run(PathBuf::from(socket)) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            failure(&error);
            std::process::ExitCode::from(1)
        }
    }
}

fn failure(detail: &str) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    let detail: String = detail
        .chars()
        .flat_map(char::escape_default)
        .take(512)
        .collect();
    eprintln!(
        "oteryn-game-ops-top ts={ts} level=error module=ops event=top_failed detail={detail}"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_response_is_private_and_bounded_data() -> Result<(), serde_json::Error> {
        let response = r#"{"readiness":true,"db_pool":"ready","platform":"available","tick_ms":9,"connections":3,"stalled_tasks":[],"last_errors":[{"ts":1,"code":"E4001","module":"runtime","event":"task_panic","trace":"trace-1"}]}"#;
        let health: Health = serde_json::from_str(response)?;
        assert!(health.readiness);
        assert_eq!(health.last_errors[0].code, "E4001");
        Ok(())
    }
}
