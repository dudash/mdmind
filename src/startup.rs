use std::fs;
use std::io::{self, IsTerminal, Stdout};
use std::path::{Path, PathBuf};

use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::{Color, Line, Modifier, Span, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::{Frame, Terminal};

use crate::app::{AppError, create_from_template};
use crate::examples::{ExampleAsset, all as bundled_examples};
use crate::templates::TemplateKind;

const BLANK_MAP_CONTENTS: &str = "- Untitled Map [id:root]\n";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartupAction {
    OpenExisting,
    CreateBlank,
    StartFromTemplate,
    TryExample,
}

impl StartupAction {
    fn all() -> &'static [Self] {
        &[
            Self::OpenExisting,
            Self::CreateBlank,
            Self::StartFromTemplate,
            Self::TryExample,
        ]
    }

    fn title(self) -> &'static str {
        match self {
            Self::OpenExisting => "Open Existing File",
            Self::CreateBlank => "Create New Blank Map",
            Self::StartFromTemplate => "Start From Template",
            Self::TryExample => "Try An Example",
        }
    }

    fn subtitle(self) -> &'static str {
        match self {
            Self::OpenExisting => "Use a markdown file already in this directory",
            Self::CreateBlank => "Create a fresh map with one starter node",
            Self::StartFromTemplate => "Choose a scaffold for product, feature, prompts, and more",
            Self::TryExample => "Copy a bundled example map into the current directory",
        }
    }

    fn empty_state(self) -> &'static str {
        match self {
            Self::OpenExisting => "No .md files were found in the current directory.",
            Self::CreateBlank => {
                "Blank maps skip the middle pane. Edit the file path and press Enter."
            }
            Self::StartFromTemplate => "Templates are bundled with mdmind.",
            Self::TryExample => "Examples are bundled with mdmind.",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartupFocus {
    Actions,
    Items,
    Path,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatusTone {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
struct StartupStatus {
    tone: StatusTone,
    text: String,
}

#[derive(Debug, Clone)]
struct StartupState {
    current_dir: PathBuf,
    action_selected: usize,
    focus: StartupFocus,
    existing_files: Vec<PathBuf>,
    existing_selected: usize,
    template_selected: usize,
    template_path: String,
    example_selected: usize,
    example_path: String,
    blank_path: String,
    path_cursor: usize,
    status: StartupStatus,
}

impl StartupState {
    fn new(current_dir: PathBuf, existing_files: Vec<PathBuf>) -> Self {
        let blank_path = suggested_startup_path("mind.md")
            .to_string_lossy()
            .into_owned();
        let template_path = suggested_startup_path(TemplateKind::all()[0].default_file_name())
            .to_string_lossy()
            .into_owned();
        let example_path = suggested_startup_path(bundled_examples()[0].file_name)
            .to_string_lossy()
            .into_owned();
        Self {
            current_dir,
            action_selected: 0,
            focus: StartupFocus::Actions,
            existing_files,
            existing_selected: 0,
            template_selected: 0,
            template_path,
            example_selected: 0,
            example_path,
            blank_path: blank_path.clone(),
            path_cursor: blank_path.len(),
            status: StartupStatus {
                tone: StatusTone::Info,
                text: "Choose a starting point. Tab moves between panes. Enter opens it."
                    .to_string(),
            },
        }
    }

    fn active_action(&self) -> StartupAction {
        StartupAction::all()[self.action_selected]
    }

    fn available_focuses(&self) -> &'static [StartupFocus] {
        match self.active_action() {
            StartupAction::OpenExisting => &[StartupFocus::Actions, StartupFocus::Items],
            StartupAction::CreateBlank => &[StartupFocus::Actions, StartupFocus::Path],
            StartupAction::StartFromTemplate | StartupAction::TryExample => &[
                StartupFocus::Actions,
                StartupFocus::Items,
                StartupFocus::Path,
            ],
        }
    }

    fn set_status(&mut self, tone: StatusTone, text: impl Into<String>) {
        self.status = StartupStatus {
            tone,
            text: text.into(),
        };
    }

    fn sync_focus_and_cursor(&mut self) {
        if !self.available_focuses().contains(&self.focus) {
            self.focus = StartupFocus::Actions;
        }
        self.path_cursor = self
            .current_path()
            .map_or(0, str::len)
            .min(self.path_cursor);
        if self.focus == StartupFocus::Path {
            self.path_cursor = self.current_path().map_or(0, str::len);
        }
    }

    fn move_focus_forward(&mut self) {
        let focuses = self.available_focuses();
        let current = focuses
            .iter()
            .position(|focus| *focus == self.focus)
            .unwrap_or(0);
        self.focus = focuses[(current + 1) % focuses.len()];
        if self.focus == StartupFocus::Path {
            self.path_cursor = self.current_path().map_or(0, str::len);
        }
    }

    fn move_focus_backward(&mut self) {
        let focuses = self.available_focuses();
        let current = focuses
            .iter()
            .position(|focus| *focus == self.focus)
            .unwrap_or(0);
        self.focus = focuses[(current + focuses.len() - 1) % focuses.len()];
        if self.focus == StartupFocus::Path {
            self.path_cursor = self.current_path().map_or(0, str::len);
        }
    }

    fn move_action(&mut self, delta: isize) {
        let len = StartupAction::all().len() as isize;
        let next = (self.action_selected as isize + delta).clamp(0, len - 1) as usize;
        if next != self.action_selected {
            self.action_selected = next;
            self.sync_focus_and_cursor();
            self.set_status(
                StatusTone::Info,
                format!("{} selected.", self.active_action().title()),
            );
        }
    }

    fn move_item(&mut self, delta: isize) {
        match self.active_action() {
            StartupAction::OpenExisting => {
                if self.existing_files.is_empty() {
                    self.set_status(
                        StatusTone::Warning,
                        "No existing markdown files are available here.",
                    );
                    return;
                }
                self.existing_selected =
                    offset_selection(self.existing_selected, self.existing_files.len(), delta);
            }
            StartupAction::StartFromTemplate => {
                let previous = TemplateKind::all()[self.template_selected];
                self.template_selected =
                    offset_selection(self.template_selected, TemplateKind::all().len(), delta);
                if self.template_selected
                    != TemplateKind::all()
                        .iter()
                        .position(|item| *item == previous)
                        .unwrap_or(0)
                {
                    self.template_path = suggested_startup_path(
                        TemplateKind::all()[self.template_selected].default_file_name(),
                    )
                    .to_string_lossy()
                    .into_owned();
                    if self.focus == StartupFocus::Path {
                        self.path_cursor = self.template_path.len();
                    }
                }
            }
            StartupAction::TryExample => {
                let previous = bundled_examples()[self.example_selected].file_name;
                self.example_selected =
                    offset_selection(self.example_selected, bundled_examples().len(), delta);
                if bundled_examples()[self.example_selected].file_name != previous {
                    self.example_path =
                        suggested_startup_path(bundled_examples()[self.example_selected].file_name)
                            .to_string_lossy()
                            .into_owned();
                    if self.focus == StartupFocus::Path {
                        self.path_cursor = self.example_path.len();
                    }
                }
            }
            StartupAction::CreateBlank => {}
        }
    }

    fn current_path(&self) -> Option<&str> {
        match self.active_action() {
            StartupAction::OpenExisting => None,
            StartupAction::CreateBlank => Some(&self.blank_path),
            StartupAction::StartFromTemplate => Some(&self.template_path),
            StartupAction::TryExample => Some(&self.example_path),
        }
    }

    fn current_path_mut(&mut self) -> Option<&mut String> {
        match self.active_action() {
            StartupAction::OpenExisting => None,
            StartupAction::CreateBlank => Some(&mut self.blank_path),
            StartupAction::StartFromTemplate => Some(&mut self.template_path),
            StartupAction::TryExample => Some(&mut self.example_path),
        }
    }

    fn path_label(&self) -> Option<&'static str> {
        match self.active_action() {
            StartupAction::OpenExisting => None,
            StartupAction::CreateBlank => Some("New File"),
            StartupAction::StartFromTemplate => Some("Template File"),
            StartupAction::TryExample => Some("Example File"),
        }
    }

    fn insert_path_char(&mut self, character: char) {
        let cursor = self.path_cursor;
        if let Some(path) = self.current_path_mut() {
            path.insert(cursor, character);
            self.path_cursor += character.len_utf8();
        }
    }

    fn backspace_path(&mut self) {
        let cursor = self.path_cursor;
        if let Some(path) = self.current_path_mut() {
            if cursor == 0 {
                return;
            }
            let previous = previous_boundary(path, cursor);
            path.replace_range(previous..cursor, "");
            self.path_cursor = previous;
        }
    }

    fn delete_path(&mut self) {
        let cursor = self.path_cursor;
        if let Some(path) = self.current_path_mut() {
            if cursor >= path.len() {
                return;
            }
            let next = next_boundary(path, cursor);
            path.replace_range(cursor..next, "");
        }
    }

    fn move_path_left(&mut self) {
        if let Some(path) = self.current_path() {
            self.path_cursor = previous_boundary(path, self.path_cursor);
        }
    }

    fn move_path_right(&mut self) {
        if let Some(path) = self.current_path() {
            self.path_cursor = next_boundary(path, self.path_cursor);
        }
    }

    fn item_title(&self) -> &'static str {
        match self.active_action() {
            StartupAction::OpenExisting => "Files In This Directory",
            StartupAction::CreateBlank => "Blank Map",
            StartupAction::StartFromTemplate => "Templates",
            StartupAction::TryExample => "Examples",
        }
    }

    fn preview_title(&self) -> &'static str {
        match self.active_action() {
            StartupAction::OpenExisting => "Preview",
            StartupAction::CreateBlank => "Blank Map Preview",
            StartupAction::StartFromTemplate => "Template Preview",
            StartupAction::TryExample => "Example Preview",
        }
    }

    fn preview_text(&self) -> String {
        match self.active_action() {
            StartupAction::OpenExisting => {
                let Some(path) = self.existing_files.get(self.existing_selected) else {
                    return "No markdown files are available in the current directory yet.\n\nChoose Create New Blank Map, Start From Template, or Try An Example to create one."
                        .to_string();
                };
                let file_name = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("map.md");
                format!(
                    "Open `{file_name}` in mdmind.\n\n{}\n\n{}",
                    path.display(),
                    preview_excerpt_from_file(path)
                )
            }
            StartupAction::CreateBlank => {
                let path = self.blank_path.trim();
                format!(
                    "Create a fresh map in the current directory.\n\nFile: `{}`\n\nStarter contents:\n{}\n\nUse this when you want the lightest possible starting point.",
                    if path.is_empty() { "mind.md" } else { path },
                    BLANK_MAP_CONTENTS.trim_end()
                )
            }
            StartupAction::StartFromTemplate => {
                let template = TemplateKind::all()[self.template_selected];
                format!(
                    "{}\n\n{}\n\nDefault file: `{}`\n\n{}",
                    template.name(),
                    template.description(),
                    template.default_file_name(),
                    preview_excerpt(template.file_contents())
                )
            }
            StartupAction::TryExample => {
                let asset = &bundled_examples()[self.example_selected];
                format!(
                    "{}\n\n{}\n\nDefault file: `{}`\n\n{}",
                    asset.name,
                    asset.description,
                    asset.file_name,
                    preview_excerpt(asset.contents)
                )
            }
        }
    }

    fn activate(&mut self) -> Result<Option<PathBuf>, AppError> {
        match self.active_action() {
            StartupAction::OpenExisting => {
                let Some(path) = self.existing_files.get(self.existing_selected).cloned() else {
                    return Err(AppError::new(
                        "No markdown files were found in the current directory.",
                    ));
                };
                Ok(Some(path))
            }
            StartupAction::CreateBlank => {
                let path = normalize_requested_path(&self.blank_path)?;
                create_blank_map(&path)?;
                Ok(Some(path))
            }
            StartupAction::StartFromTemplate => {
                let path = normalize_requested_path(&self.template_path)?;
                create_template_map(TemplateKind::all()[self.template_selected], &path)?;
                Ok(Some(path))
            }
            StartupAction::TryExample => {
                let path = normalize_requested_path(&self.example_path)?;
                create_example_map(&bundled_examples()[self.example_selected], &path)?;
                Ok(Some(path))
            }
        }
    }
}

pub fn choose_startup_target() -> Result<Option<String>, AppError> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(AppError::new(
            "No target was provided. Run `mdmind path/to/map.md`, or start `mdmind` in an interactive terminal to create one.",
        ));
    }

    let current_dir = std::env::current_dir()
        .map_err(|error| AppError::new(format!("Could not read the current directory: {error}")))?;
    let existing_files = discover_markdown_files(&current_dir)?;
    let mut state = StartupState::new(current_dir, existing_files);
    let mut terminal = setup_terminal()?;
    let result = run_startup_loop(&mut terminal, &mut state);
    restore_terminal(&mut terminal)?;
    result.map(|path| path.map(|path| path.to_string_lossy().into_owned()))
}

fn run_startup_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    state: &mut StartupState,
) -> Result<Option<PathBuf>, AppError> {
    loop {
        terminal
            .draw(|frame| render_startup(frame, state))
            .map_err(|error| {
                AppError::new(format!("Could not draw the startup screen: {error}"))
            })?;

        if let Event::Key(key) = event::read()
            .map_err(|error| AppError::new(format!("Could not read terminal input: {error}")))?
        {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match handle_startup_key(state, key)? {
                Some(result) => return Ok(result),
                None => continue,
            }
        }
    }
}

fn handle_startup_key(
    state: &mut StartupState,
    key: KeyEvent,
) -> Result<Option<Option<PathBuf>>, AppError> {
    match key.code {
        KeyCode::Esc => return Ok(Some(None)),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            return Ok(Some(None));
        }
        KeyCode::Char('q') if key.modifiers == KeyModifiers::NONE => return Ok(Some(None)),
        KeyCode::Tab => {
            state.move_focus_forward();
        }
        KeyCode::BackTab => {
            state.move_focus_backward();
        }
        KeyCode::Left => {
            if state.focus == StartupFocus::Path {
                state.move_path_left();
            } else {
                state.move_focus_backward();
            }
        }
        KeyCode::Right => {
            if state.focus == StartupFocus::Path {
                state.move_path_right();
            } else {
                state.move_focus_forward();
            }
        }
        KeyCode::Up => match state.focus {
            StartupFocus::Actions => state.move_action(-1),
            StartupFocus::Items => state.move_item(-1),
            StartupFocus::Path => {}
        },
        KeyCode::Down => match state.focus {
            StartupFocus::Actions => state.move_action(1),
            StartupFocus::Items => state.move_item(1),
            StartupFocus::Path => {}
        },
        KeyCode::Home if state.focus == StartupFocus::Path => state.path_cursor = 0,
        KeyCode::End if state.focus == StartupFocus::Path => {
            state.path_cursor = state.current_path().map_or(0, str::len)
        }
        KeyCode::Backspace if state.focus == StartupFocus::Path => state.backspace_path(),
        KeyCode::Delete if state.focus == StartupFocus::Path => state.delete_path(),
        KeyCode::Char(character)
            if state.focus == StartupFocus::Path && key.modifiers == KeyModifiers::NONE =>
        {
            state.insert_path_char(character);
        }
        KeyCode::Enter => match state.activate() {
            Ok(path) => return Ok(Some(path)),
            Err(error) => state.set_status(StatusTone::Error, error.message().to_string()),
        },
        _ => {}
    }

    Ok(None)
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, AppError> {
    enable_raw_mode()
        .map_err(|error| AppError::new(format!("Could not enable raw mode: {error}")))?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, Hide)
        .map_err(|error| AppError::new(format!("Could not enter the alternate screen: {error}")))?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
        .map_err(|error| AppError::new(format!("Could not start the TUI: {error}")))
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), AppError> {
    disable_raw_mode()
        .map_err(|error| AppError::new(format!("Could not disable raw mode: {error}")))?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, Show)
        .map_err(|error| AppError::new(format!("Could not restore the terminal: {error}")))?;
    terminal
        .show_cursor()
        .map_err(|error| AppError::new(format!("Could not restore the cursor: {error}")))
}

fn render_startup(frame: &mut Frame, state: &StartupState) {
    let area = frame.area();
    let background = Color::Rgb(12, 16, 20);
    let surface = Color::Rgb(22, 28, 34);
    let border = Color::Rgb(72, 86, 98);
    let accent = Color::Rgb(116, 193, 255);
    let text = Color::Rgb(234, 239, 244);
    let muted = Color::Rgb(155, 168, 180);
    let warning = Color::Rgb(255, 191, 106);
    let error = Color::Rgb(255, 121, 121);

    frame.render_widget(
        Block::default().style(Style::default().bg(background)),
        area,
    );

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(18),
            Constraint::Length(3),
        ])
        .split(area);

    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                "mdmind",
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                "Start A Map",
                Style::default().fg(text).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Current directory: ", Style::default().fg(muted)),
            Span::styled(
                state.current_dir.display().to_string(),
                Style::default().fg(text),
            ),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border))
            .style(Style::default().bg(surface)),
    );
    frame.render_widget(header, outer[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(28),
            Constraint::Percentage(30),
            Constraint::Percentage(42),
        ])
        .split(outer[1]);

    render_actions(frame, body[0], state, surface, border, accent, text, muted);
    render_items(frame, body[1], state, surface, border, accent, text, muted);
    render_preview(
        frame, body[2], state, surface, border, accent, text, muted, warning, error,
    );

    let status_color = match state.status.tone {
        StatusTone::Info => muted,
        StatusTone::Warning => warning,
        StatusTone::Error => error,
    };
    let footer = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            state.status.text.clone(),
            Style::default().fg(status_color),
        )]),
        Line::from(vec![
            Span::styled("Tab", Style::default().fg(accent)),
            Span::styled(" move focus  ", Style::default().fg(muted)),
            Span::styled("Arrows", Style::default().fg(accent)),
            Span::styled(" browse  ", Style::default().fg(muted)),
            Span::styled("Enter", Style::default().fg(accent)),
            Span::styled(" open/create  ", Style::default().fg(muted)),
            Span::styled("Esc", Style::default().fg(accent)),
            Span::styled(" cancel", Style::default().fg(muted)),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border))
            .style(Style::default().bg(surface)),
    );
    frame.render_widget(footer, outer[2]);
}

#[allow(clippy::too_many_arguments)]
fn render_actions(
    frame: &mut Frame,
    area: Rect,
    state: &StartupState,
    surface: Color,
    border: Color,
    accent: Color,
    text: Color,
    muted: Color,
) {
    let items = StartupAction::all()
        .iter()
        .map(|action| {
            ListItem::new(vec![
                Line::from(Span::styled(
                    action.title(),
                    Style::default().fg(text).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(action.subtitle(), Style::default().fg(muted))),
            ])
        })
        .collect::<Vec<_>>();

    let mut list_state = ListState::default();
    list_state.select(Some(state.action_selected));
    let title = if state.focus == StartupFocus::Actions {
        "Actions • Active"
    } else {
        "Actions"
    };
    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(
                    Style::default().fg(if state.focus == StartupFocus::Actions {
                        accent
                    } else {
                        border
                    }),
                )
                .style(Style::default().bg(surface)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Rgb(32, 44, 56))
                .fg(text)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");
    frame.render_stateful_widget(list, area, &mut list_state);
}

#[allow(clippy::too_many_arguments)]
fn render_items(
    frame: &mut Frame,
    area: Rect,
    state: &StartupState,
    surface: Color,
    border: Color,
    accent: Color,
    text: Color,
    muted: Color,
) {
    let title = if state.focus == StartupFocus::Items {
        format!("{} • Active", state.item_title())
    } else {
        state.item_title().to_string()
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if state.focus == StartupFocus::Items {
            accent
        } else {
            border
        }))
        .style(Style::default().bg(surface));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    match state.active_action() {
        StartupAction::OpenExisting => {
            if state.existing_files.is_empty() {
                frame.render_widget(
                    Paragraph::new(state.active_action().empty_state())
                        .style(Style::default().fg(muted))
                        .wrap(Wrap { trim: true }),
                    inner,
                );
                return;
            }
            let items = state
                .existing_files
                .iter()
                .map(|path| {
                    let file_name = path
                        .file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or("map.md");
                    ListItem::new(vec![
                        Line::from(Span::styled(
                            file_name,
                            Style::default().fg(text).add_modifier(Modifier::BOLD),
                        )),
                        Line::from(Span::styled(
                            path.display().to_string(),
                            Style::default().fg(muted),
                        )),
                    ])
                })
                .collect::<Vec<_>>();
            let mut list_state = ListState::default();
            list_state.select(Some(
                state.existing_selected.min(state.existing_files.len() - 1),
            ));
            let list = List::new(items)
                .highlight_style(
                    Style::default()
                        .bg(Color::Rgb(32, 44, 56))
                        .fg(text)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("› ");
            frame.render_stateful_widget(list, inner, &mut list_state);
        }
        StartupAction::CreateBlank => {
            frame.render_widget(
                Paragraph::new(state.active_action().empty_state())
                    .style(Style::default().fg(muted))
                    .wrap(Wrap { trim: true }),
                inner,
            );
        }
        StartupAction::StartFromTemplate => {
            let items = TemplateKind::all()
                .iter()
                .map(|template| {
                    ListItem::new(vec![
                        Line::from(Span::styled(
                            template.name(),
                            Style::default().fg(text).add_modifier(Modifier::BOLD),
                        )),
                        Line::from(Span::styled(
                            template.description(),
                            Style::default().fg(muted),
                        )),
                    ])
                })
                .collect::<Vec<_>>();
            let mut list_state = ListState::default();
            list_state.select(Some(
                state.template_selected.min(TemplateKind::all().len() - 1),
            ));
            let list = List::new(items)
                .highlight_style(
                    Style::default()
                        .bg(Color::Rgb(32, 44, 56))
                        .fg(text)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("› ");
            frame.render_stateful_widget(list, inner, &mut list_state);
        }
        StartupAction::TryExample => {
            let items = bundled_examples()
                .iter()
                .map(|asset| {
                    ListItem::new(vec![
                        Line::from(Span::styled(
                            asset.name,
                            Style::default().fg(text).add_modifier(Modifier::BOLD),
                        )),
                        Line::from(Span::styled(asset.description, Style::default().fg(muted))),
                    ])
                })
                .collect::<Vec<_>>();
            let mut list_state = ListState::default();
            list_state.select(Some(
                state.example_selected.min(bundled_examples().len() - 1),
            ));
            let list = List::new(items)
                .highlight_style(
                    Style::default()
                        .bg(Color::Rgb(32, 44, 56))
                        .fg(text)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("› ");
            frame.render_stateful_widget(list, inner, &mut list_state);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_preview(
    frame: &mut Frame,
    area: Rect,
    state: &StartupState,
    surface: Color,
    border: Color,
    accent: Color,
    text: Color,
    muted: Color,
    warning: Color,
    error: Color,
) {
    let path_height = if state.current_path().is_some() { 5 } else { 0 };
    let columns = if path_height > 0 {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(12), Constraint::Length(path_height)])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(12)])
            .split(area)
    };

    let preview = Paragraph::new(state.preview_text())
        .style(Style::default().fg(text))
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .title(state.preview_title())
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border))
                .style(Style::default().bg(surface)),
        );
    frame.render_widget(preview, columns[0]);

    if let Some(path_label) = state.path_label() {
        let title = if state.focus == StartupFocus::Path {
            format!("{path_label} • Active")
        } else {
            path_label.to_string()
        };
        let path = state.current_path().unwrap_or_default();
        let lines = vec![
            render_path_line(
                path,
                state.path_cursor,
                state.focus == StartupFocus::Path,
                text,
                accent,
            ),
            Line::from(vec![Span::styled(
                "Press Enter to create the file and open it in mdmind.",
                Style::default().fg(match state.status.tone {
                    StatusTone::Info => muted,
                    StatusTone::Warning => warning,
                    StatusTone::Error => error,
                }),
            )]),
        ];
        let input = Paragraph::new(lines).wrap(Wrap { trim: false }).block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if state.focus == StartupFocus::Path {
                    accent
                } else {
                    border
                }))
                .style(Style::default().bg(surface)),
        );
        frame.render_widget(input, columns[1]);
    }
}

fn render_path_line(
    path: &str,
    cursor: usize,
    focused: bool,
    text: Color,
    accent: Color,
) -> Line<'static> {
    let bounded = cursor.min(path.len());
    let previous = &path[..bounded];
    let next = &path[bounded..];
    let cursor_char = if focused {
        next.chars()
            .next()
            .map(|character| character.to_string())
            .unwrap_or_else(|| " ".to_string())
    } else {
        String::new()
    };
    let remainder = if focused {
        next.chars().skip(1).collect::<String>()
    } else {
        next.to_string()
    };

    let mut spans = vec![Span::styled(
        previous.to_string(),
        Style::default().fg(text),
    )];
    if focused {
        spans.push(Span::styled(
            cursor_char,
            Style::default().fg(Color::Rgb(12, 16, 20)).bg(accent),
        ));
        spans.push(Span::styled(remainder, Style::default().fg(text)));
    } else {
        spans.push(Span::styled(next.to_string(), Style::default().fg(text)));
    }
    if path.is_empty() {
        spans.push(Span::styled(
            "mind.md",
            Style::default().fg(Color::DarkGray),
        ));
    }
    Line::from(spans)
}

fn normalize_requested_path(raw: &str) -> Result<PathBuf, AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::new("Choose a file path before continuing."));
    }
    Ok(PathBuf::from(trimmed))
}

pub(crate) fn discover_markdown_files(directory: &Path) -> Result<Vec<PathBuf>, AppError> {
    let mut files = fs::read_dir(directory)
        .map_err(|error| {
            AppError::new(format!("Could not read '{}': {error}", directory.display()))
        })?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("md"))
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.file_name().cmp(&right.file_name()));
    Ok(files)
}

pub(crate) fn suggested_startup_path(file_name: &str) -> PathBuf {
    let preferred = PathBuf::from(file_name);
    if !preferred.exists() {
        return preferred;
    }
    unique_path_in_dir(Path::new("."), file_name)
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn unique_path_in_dir(directory: &Path, file_name: &str) -> PathBuf {
    let preferred = directory.join(file_name);
    if !preferred.exists() {
        return preferred;
    }

    let file_path = Path::new(file_name);
    let stem = file_path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("mind");
    let extension = file_path.extension().and_then(|value| value.to_str());

    for index in 2.. {
        let candidate_name = match extension {
            Some(extension) => format!("{stem}-{index}.{extension}"),
            None => format!("{stem}-{index}"),
        };
        let candidate = directory.join(candidate_name);
        if !candidate.exists() {
            return candidate;
        }
    }

    unreachable!("numeric suffix search should always find a file name");
}

fn ensure_parent_dir(path: &Path) -> Result<(), AppError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| {
            AppError::new(format!(
                "Could not create parent directory '{}': {error}",
                parent.display()
            ))
        })?;
    }
    Ok(())
}

pub(crate) fn create_blank_map(path: &Path) -> Result<(), AppError> {
    ensure_parent_dir(path)?;
    if path.exists() {
        return Err(AppError::new(format!(
            "'{}' already exists. Choose another path.",
            path.display()
        )));
    }
    fs::write(path, BLANK_MAP_CONTENTS)
        .map_err(|error| AppError::new(format!("Could not write '{}': {error}", path.display())))
}

pub(crate) fn create_example_map(asset: &ExampleAsset, path: &Path) -> Result<(), AppError> {
    ensure_parent_dir(path)?;
    if path.exists() {
        return Err(AppError::new(format!(
            "'{}' already exists. Choose another path.",
            path.display()
        )));
    }
    fs::write(path, asset.contents)
        .map_err(|error| AppError::new(format!("Could not write '{}': {error}", path.display())))
}

pub(crate) fn create_template_map(template: TemplateKind, path: &Path) -> Result<(), AppError> {
    create_from_template(path, template, false)
}

fn preview_excerpt_from_file(path: &Path) -> String {
    match fs::read_to_string(path) {
        Ok(contents) => preview_excerpt(&contents),
        Err(error) => format!("Could not read preview: {error}"),
    }
}

fn preview_excerpt(contents: &str) -> String {
    let excerpt = contents
        .lines()
        .take(12)
        .map(str::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    if excerpt.is_empty() {
        "(This file is currently empty.)".to_string()
    } else {
        excerpt
    }
}

fn offset_selection(current: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    (current as isize + delta).clamp(0, len as isize - 1) as usize
}

fn previous_boundary(value: &str, index: usize) -> usize {
    value
        .char_indices()
        .take_while(|(offset, _)| *offset < index)
        .map(|(offset, _)| offset)
        .last()
        .unwrap_or(0)
}

fn next_boundary(value: &str, index: usize) -> usize {
    value
        .char_indices()
        .map(|(offset, _)| offset)
        .find(|offset| *offset > index)
        .unwrap_or(value.len())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::examples::find as find_example;

    use super::{
        create_blank_map, create_example_map, create_template_map, discover_markdown_files,
        unique_path_in_dir,
    };
    use crate::templates::TemplateKind;

    fn temp_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("mdmind-startup-{nonce}-{name}"));
        fs::create_dir_all(&path).expect("temp directory should be created");
        path
    }

    #[test]
    fn unique_path_in_dir_adds_a_numeric_suffix_when_needed() {
        let directory = temp_dir("unique-path");
        let first = directory.join("mind.md");
        let second = directory.join("mind-2.md");
        fs::write(&first, "").expect("first file should be created");
        fs::write(&second, "").expect("second file should be created");

        let candidate = unique_path_in_dir(&directory, "mind.md");
        assert_eq!(candidate, directory.join("mind-3.md"));

        fs::remove_file(first).expect("first file should be removable");
        fs::remove_file(second).expect("second file should be removable");
        fs::remove_dir(directory).expect("temp directory should be removable");
    }

    #[test]
    fn create_blank_map_writes_a_starter_node() {
        let directory = temp_dir("blank-map");
        let path = directory.join("mind.md");

        create_blank_map(&path).expect("blank map should be created");

        assert!(Path::new(&path).is_file());
        assert_eq!(
            fs::read_to_string(&path).expect("file should be readable"),
            "- Untitled Map [id:root]\n"
        );

        fs::remove_file(path).expect("file should be removable");
        fs::remove_dir(directory).expect("temp directory should be removable");
    }

    #[test]
    fn create_example_map_writes_bundled_example_contents() {
        let directory = temp_dir("example-map");
        let path = directory.join("demo.md");
        let asset = find_example("demo").expect("demo example should exist");

        create_example_map(asset, &path).expect("example map should be created");

        let contents = fs::read_to_string(&path).expect("example map should be readable");
        assert!(contents.contains("- mdmind Demo [id:demo]"));

        fs::remove_file(path).expect("file should be removable");
        fs::remove_dir(directory).expect("temp directory should be removable");
    }

    #[test]
    fn create_template_map_writes_selected_template_contents() {
        let directory = temp_dir("template-map");
        let path = directory.join("product.md");

        create_template_map(TemplateKind::Product, &path).expect("template map should be created");

        let contents = fs::read_to_string(&path).expect("template map should be readable");
        assert!(contents.contains("- Product Roadmap [id:product]"));

        fs::remove_file(path).expect("file should be removable");
        fs::remove_dir(directory).expect("temp directory should be removable");
    }

    #[test]
    fn discover_markdown_files_lists_current_directory_markdown_files() {
        let directory = temp_dir("discover-files");
        fs::write(directory.join("b.md"), "- B\n").expect("markdown file should be created");
        fs::write(directory.join("a.md"), "- A\n").expect("markdown file should be created");
        fs::write(directory.join("notes.txt"), "ignore").expect("text file should be created");

        let files = discover_markdown_files(&directory).expect("files should be discoverable");
        let names = files
            .iter()
            .filter_map(|path| path.file_name().and_then(|value| value.to_str()))
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["a.md", "b.md"]);

        fs::remove_file(directory.join("a.md")).expect("a.md should be removable");
        fs::remove_file(directory.join("b.md")).expect("b.md should be removable");
        fs::remove_file(directory.join("notes.txt")).expect("notes.txt should be removable");
        fs::remove_dir(directory).expect("temp directory should be removable");
    }
}
