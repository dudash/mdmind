use std::collections::HashSet;
use std::fs;
use std::io::{self, IsTerminal, Stdout};
use std::path::PathBuf;
use std::time::Duration;

use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::prelude::{Color, Line, Modifier, Span, Style};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Padding, Paragraph, Wrap,
};
use ratatui::{Frame, Terminal};

use crate::APP_VERSION;
use crate::app::{AppError, TargetRef, ensure_parseable, load_document};
use crate::editor::{Editor, default_focus_path, find_path_by_id, get_node};
use crate::model::{Document, Node};
use crate::query::{
    FilterQuery, find_matches, metadata_key_counts_for_filter, metadata_value_counts_for_filter,
    tag_counts_for_filter,
};
use crate::serializer::serialize_document;
use crate::session::{load_session_for, resolve_session_focus, save_session_for};
use crate::views::{SavedView, SavedViewsState, load_views_for, save_views_for};

const TICK_RATE: Duration = Duration::from_millis(150);

#[derive(Debug, Clone, Copy)]
struct Palette {
    background: Color,
    surface: Color,
    surface_alt: Color,
    border: Color,
    accent: Color,
    sky: Color,
    warn: Color,
    danger: Color,
    text: Color,
    muted: Color,
}

const PALETTE: Palette = Palette {
    background: Color::Rgb(8, 15, 24),
    surface: Color::Rgb(15, 25, 38),
    surface_alt: Color::Rgb(24, 39, 58),
    border: Color::Rgb(41, 65, 91),
    accent: Color::Rgb(67, 201, 176),
    sky: Color::Rgb(94, 191, 255),
    warn: Color::Rgb(248, 189, 94),
    danger: Color::Rgb(244, 114, 93),
    text: Color::Rgb(233, 241, 248),
    muted: Color::Rgb(129, 153, 178),
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatusTone {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
struct StatusMessage {
    tone: StatusTone,
    text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptMode {
    AddChild,
    AddSibling,
    AddRoot,
    Edit,
    SaveView,
    OpenId,
}

impl PromptMode {
    fn title(self) -> &'static str {
        match self {
            Self::AddChild => "Add Child",
            Self::AddSibling => "Add Sibling",
            Self::AddRoot => "Add Root",
            Self::Edit => "Edit Node",
            Self::SaveView => "Save Filter View",
            Self::OpenId => "Jump To Id",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Self::SaveView => "Give the current active filter a short name.",
            Self::OpenId => "Type a node id, then press Enter.",
            _ => "Use full node syntax: Label #tag @key:value [id:path/to/node]",
        }
    }
}

#[derive(Debug, Clone)]
struct ActiveFilter {
    query: FilterQuery,
    matches: Vec<Vec<usize>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchSection {
    Query,
    Facets,
    Views,
}

impl SearchSection {
    fn title(self) -> &'static str {
        match self {
            Self::Query => "Query",
            Self::Facets => "Facets",
            Self::Views => "Saved Views",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Query => Self::Facets,
            Self::Facets => Self::Views,
            Self::Views => Self::Query,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Query => Self::Views,
            Self::Facets => Self::Query,
            Self::Views => Self::Facets,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FacetTab {
    Tags,
    Keys,
    Values,
}

impl FacetTab {
    fn title(self) -> &'static str {
        match self {
            Self::Tags => "Tags",
            Self::Keys => "Keys",
            Self::Values => "Values",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Tags => Self::Keys,
            Self::Keys => Self::Values,
            Self::Values => Self::Tags,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Tags => Self::Values,
            Self::Keys => Self::Tags,
            Self::Values => Self::Keys,
        }
    }

    fn empty_message(self) -> &'static str {
        match self {
            Self::Tags => "No tags exist in the current scope.",
            Self::Keys => "No metadata keys exist in the current scope.",
            Self::Values => "No metadata values exist in the current scope.",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FacetItem {
    label: String,
    token: String,
    count: usize,
    detail: String,
}

#[derive(Debug, Clone)]
struct SearchOverlayState {
    section: SearchSection,
    draft_query: String,
    cursor: usize,
    facet_tab: FacetTab,
    facet_selected: usize,
    view_selected: usize,
}

impl SearchOverlayState {
    fn new(section: SearchSection, draft_query: String) -> Self {
        let cursor = draft_query.len();
        Self {
            section,
            draft_query,
            cursor,
            facet_tab: FacetTab::Tags,
            facet_selected: 0,
            view_selected: 0,
        }
    }

    fn move_left(&mut self) {
        self.cursor = previous_boundary(&self.draft_query, self.cursor);
    }

    fn move_right(&mut self) {
        self.cursor = next_boundary(&self.draft_query, self.cursor);
    }

    fn insert(&mut self, character: char) {
        self.draft_query.insert(self.cursor, character);
        self.cursor += character.len_utf8();
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let previous = previous_boundary(&self.draft_query, self.cursor);
        self.draft_query.replace_range(previous..self.cursor, "");
        self.cursor = previous;
    }

    fn delete(&mut self) {
        if self.cursor >= self.draft_query.len() {
            return;
        }
        let next = next_boundary(&self.draft_query, self.cursor);
        self.draft_query.replace_range(self.cursor..next, "");
    }

    fn query(&self) -> Option<FilterQuery> {
        FilterQuery::parse(&self.draft_query)
    }
}

#[derive(Debug, Clone)]
struct PromptState {
    mode: PromptMode,
    value: String,
    cursor: usize,
}

impl PromptState {
    fn new(mode: PromptMode, value: String) -> Self {
        let cursor = value.len();
        Self {
            mode,
            value,
            cursor,
        }
    }

    fn move_left(&mut self) {
        self.cursor = previous_boundary(&self.value, self.cursor);
    }

    fn move_right(&mut self) {
        self.cursor = next_boundary(&self.value, self.cursor);
    }

    fn insert(&mut self, character: char) {
        self.value.insert(self.cursor, character);
        self.cursor += character.len_utf8();
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let previous = previous_boundary(&self.value, self.cursor);
        self.value.replace_range(previous..self.cursor, "");
        self.cursor = previous;
    }

    fn delete(&mut self) {
        if self.cursor >= self.value.len() {
            return;
        }
        let next = next_boundary(&self.value, self.cursor);
        self.value.replace_range(self.cursor..next, "");
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VisibleRow {
    path: Vec<usize>,
    depth: usize,
    text: String,
    tags: Vec<String>,
    metadata: Vec<String>,
    id: Option<String>,
    line: usize,
    has_children: bool,
    expanded: bool,
    child_count: usize,
    matched: bool,
}

#[derive(Debug)]
struct TuiApp {
    map_path: PathBuf,
    editor: Editor,
    expanded: HashSet<Vec<usize>>,
    status: StatusMessage,
    prompt: Option<PromptState>,
    filter: Option<ActiveFilter>,
    search: Option<SearchOverlayState>,
    saved_views: SavedViewsState,
    help_open: bool,
    quit_armed: bool,
    delete_armed: bool,
    autosave: bool,
}

impl TuiApp {
    fn new(
        map_path: PathBuf,
        document: Document,
        focus_path: Vec<usize>,
        warning: Option<String>,
        autosave: bool,
        saved_views: SavedViewsState,
    ) -> Self {
        let mut expanded = initial_expanded_paths(&document);
        for ancestor in ancestor_paths(&focus_path) {
            expanded.insert(ancestor);
        }

        let status = match warning {
            Some(text) => StatusMessage {
                tone: StatusTone::Warning,
                text,
            },
            None => StatusMessage {
                tone: StatusTone::Info,
                text: "Arrows move. / opens search. Alt+arrows reshape. a adds. s saves."
                    .to_string(),
            },
        };

        Self {
            map_path,
            editor: Editor::new(document, focus_path),
            expanded,
            status,
            prompt: None,
            filter: None,
            search: None,
            saved_views,
            help_open: false,
            quit_armed: false,
            delete_armed: false,
            autosave,
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<bool, AppError> {
        if key.kind != KeyEventKind::Press {
            return Ok(true);
        }

        if self.prompt.is_some() {
            self.quit_armed = false;
            self.delete_armed = false;
            return self.handle_prompt_key(key);
        }

        if self.search.is_some() {
            self.quit_armed = false;
            self.delete_armed = false;
            return self.handle_search_key(key);
        }

        if self.help_open {
            match key.code {
                KeyCode::Esc | KeyCode::Char('?') => {
                    self.help_open = false;
                    self.set_status(StatusTone::Info, "Closed the keymap overlay.");
                }
                _ => {}
            }
            return Ok(true);
        }

        if key.modifiers.contains(KeyModifiers::ALT) {
            self.quit_armed = false;
            self.delete_armed = false;
            match key.code {
                KeyCode::Up => {
                    self.apply_edit(|editor| editor.move_node_up(), "Moved the node up.")?
                }
                KeyCode::Down => {
                    self.apply_edit(|editor| editor.move_node_down(), "Moved the node down.")?
                }
                KeyCode::Left => self.apply_edit(
                    |editor| editor.outdent_node(),
                    "Moved the node out one level.",
                )?,
                KeyCode::Right => self.apply_edit(
                    |editor| editor.indent_node(),
                    "Moved the node into the previous sibling.",
                )?,
                _ => {}
            }
            return Ok(true);
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.quit_armed = false;
                self.delete_armed = false;
                self.move_selection(-1)?;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.quit_armed = false;
                self.delete_armed = false;
                self.move_selection(1)?;
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.quit_armed = false;
                self.delete_armed = false;
                self.collapse_or_parent()?;
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.quit_armed = false;
                self.delete_armed = false;
                self.expand_or_child()?;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.quit_armed = false;
                self.delete_armed = false;
                self.toggle_branch()?;
            }
            KeyCode::Char('?') => {
                self.help_open = true;
                self.quit_armed = false;
                self.delete_armed = false;
            }
            KeyCode::Char('f') => {
                self.delete_armed = false;
                self.open_search_overlay(SearchSection::Facets);
            }
            KeyCode::Char('F') => {
                self.delete_armed = false;
                self.open_search_overlay(SearchSection::Views);
            }
            KeyCode::Char('a') => {
                self.delete_armed = false;
                self.begin_prompt(PromptMode::AddChild, String::new());
            }
            KeyCode::Char('A') => {
                self.delete_armed = false;
                self.begin_prompt(PromptMode::AddSibling, String::new());
            }
            KeyCode::Char('R') => {
                self.delete_armed = false;
                self.begin_prompt(PromptMode::AddRoot, String::new());
            }
            KeyCode::Char('e') => {
                self.delete_armed = false;
                let initial = self
                    .editor
                    .current()
                    .map(Node::display_line)
                    .unwrap_or_default();
                self.begin_prompt(PromptMode::Edit, initial);
            }
            KeyCode::Char('/') => {
                self.delete_armed = false;
                self.open_search_overlay(SearchSection::Query);
            }
            KeyCode::Char('o') => {
                self.delete_armed = false;
                self.begin_prompt(PromptMode::OpenId, String::new());
            }
            KeyCode::Char('g') => {
                self.editor.move_root()?;
                self.expand_focus_chain();
                self.persist_session()?;
                self.delete_armed = false;
                self.set_status(StatusTone::Info, "Jumped to the root.");
            }
            KeyCode::Char('s') => {
                self.save_to_disk()?;
                self.quit_armed = false;
                self.delete_armed = false;
            }
            KeyCode::Char('S') => {
                self.delete_armed = false;
                self.autosave = !self.autosave;
                let message = if self.autosave {
                    "Autosave enabled. Changes now write to disk immediately."
                } else {
                    "Autosave disabled. Press s to save changes manually."
                };
                self.set_status(StatusTone::Info, message);
            }
            KeyCode::Char('r') => {
                self.revert_from_disk()?;
                self.quit_armed = false;
                self.delete_armed = false;
            }
            KeyCode::Char('c') => {
                self.delete_armed = false;
                if self.clear_filter() {
                    self.set_status(StatusTone::Info, "Cleared the active filter.");
                } else {
                    self.set_status(StatusTone::Info, "No active filter to clear.");
                }
            }
            KeyCode::Char('n') => {
                self.delete_armed = false;
                self.move_match(1)?;
            }
            KeyCode::Char('N') => {
                self.delete_armed = false;
                self.move_match(-1)?;
            }
            KeyCode::Char('x') => {
                self.quit_armed = false;
                if self.delete_armed {
                    self.apply_edit(
                        |editor| editor.delete_current(),
                        "Deleted the selected node.",
                    )?;
                    self.delete_armed = false;
                } else {
                    self.delete_armed = true;
                    self.set_status(
                        StatusTone::Warning,
                        "Press x again to delete the selected node. Any other key will cancel.",
                    );
                }
            }
            KeyCode::Esc => {
                self.quit_armed = false;
                self.delete_armed = false;
                if self.clear_filter() {
                    self.set_status(StatusTone::Info, "Cleared the active filter.");
                } else {
                    self.set_status(
                        StatusTone::Info,
                        "Use q to quit. If there are unsaved changes, press q twice to discard them.",
                    );
                }
            }
            KeyCode::Char('q') => {
                self.delete_armed = false;
                if self.editor.dirty() && !self.quit_armed {
                    self.quit_armed = true;
                    self.set_status(
                        StatusTone::Warning,
                        "Unsaved changes. Press q again to discard, or press s to save first.",
                    );
                } else {
                    return Ok(false);
                }
            }
            _ => {
                self.delete_armed = false;
            }
        }

        Ok(true)
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> Result<bool, AppError> {
        let Some(mut search) = self.search.take() else {
            return Ok(true);
        };

        match key.code {
            KeyCode::Esc => {
                self.set_status(StatusTone::Info, "Closed search.");
                return Ok(true);
            }
            KeyCode::BackTab => {
                search.section = search.section.previous();
            }
            KeyCode::Tab => {
                search.section = search.section.next();
            }
            KeyCode::Char('c') => {
                self.clear_filter();
                search.draft_query.clear();
                search.cursor = 0;
                search.facet_selected = 0;
                search.view_selected = 0;
                self.set_status(StatusTone::Info, "Cleared the active filter.");
            }
            _ => match search.section {
                SearchSection::Query => {
                    let submitted = self.handle_search_query_key(&mut search, key)?;
                    if submitted {
                        return Ok(true);
                    }
                }
                SearchSection::Facets => {
                    let submitted = self.handle_search_facets_key(&mut search, key)?;
                    if submitted {
                        return Ok(true);
                    }
                }
                SearchSection::Views => {
                    let submitted = self.handle_search_views_key(&mut search, key)?;
                    if submitted {
                        return Ok(true);
                    }
                }
            },
        }

        self.search = Some(search);
        Ok(true)
    }

    fn handle_search_query_key(
        &mut self,
        search: &mut SearchOverlayState,
        key: KeyEvent,
    ) -> Result<bool, AppError> {
        match key.code {
            KeyCode::Enter => {
                self.apply_filter(&search.draft_query)?;
                return Ok(true);
            }
            KeyCode::Backspace => search.backspace(),
            KeyCode::Delete => search.delete(),
            KeyCode::Left => search.move_left(),
            KeyCode::Right => search.move_right(),
            KeyCode::Home => search.cursor = 0,
            KeyCode::End => search.cursor = search.draft_query.len(),
            KeyCode::Char(character)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                search.insert(character);
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_search_facets_key(
        &mut self,
        search: &mut SearchOverlayState,
        key: KeyEvent,
    ) -> Result<bool, AppError> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                search.facet_selected = search.facet_selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let items_len = self
                    .facet_items_for_query(search.facet_tab, search.query())
                    .len();
                if items_len > 0 {
                    search.facet_selected = (search.facet_selected + 1).min(items_len - 1);
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                search.facet_tab = search.facet_tab.previous();
                search.facet_selected = 0;
            }
            KeyCode::Right | KeyCode::Char('l') => {
                search.facet_tab = search.facet_tab.next();
                search.facet_selected = 0;
            }
            KeyCode::Enter => {
                let items = self.facet_items_for_query(search.facet_tab, search.query());
                if let Some(item) = items.get(search.facet_selected) {
                    search.draft_query = compose_query_with_token(&search.draft_query, &item.token);
                    search.cursor = search.draft_query.len();
                    self.apply_filter(&search.draft_query)?;
                    self.set_status(
                        StatusTone::Success,
                        format!("Applied facet {}.", item.label),
                    );
                } else {
                    self.set_status(StatusTone::Warning, search.facet_tab.empty_message());
                    self.search = Some(search.clone());
                }
                return Ok(true);
            }
            _ => {}
        }

        let items_len = self
            .facet_items_for_query(search.facet_tab, search.query())
            .len();
        if items_len == 0 {
            search.facet_selected = 0;
        } else {
            search.facet_selected = search.facet_selected.min(items_len - 1);
        }
        Ok(false)
    }

    fn handle_search_views_key(
        &mut self,
        search: &mut SearchOverlayState,
        key: KeyEvent,
    ) -> Result<bool, AppError> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                search.view_selected = search.view_selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let len = self.saved_views.views.len();
                if len > 0 {
                    search.view_selected = (search.view_selected + 1).min(len - 1);
                }
            }
            KeyCode::Char('a') => {
                if self.current_search_query_for_save().is_none() {
                    self.set_status(
                        StatusTone::Warning,
                        "Type or apply a filter first, then save it as a named view.",
                    );
                    self.search = Some(search.clone());
                } else {
                    self.search = Some(search.clone());
                    self.begin_prompt(PromptMode::SaveView, String::new());
                }
                return Ok(true);
            }
            KeyCode::Char('x') => {
                if self.saved_views.views.is_empty() {
                    self.set_status(StatusTone::Warning, "There are no saved views to delete.");
                } else {
                    let removed = self.saved_views.views.remove(search.view_selected);
                    self.persist_saved_views()?;
                    let len = self.saved_views.views.len();
                    if len == 0 {
                        search.view_selected = 0;
                    } else {
                        search.view_selected = search.view_selected.min(len - 1);
                    }
                    self.set_status(
                        StatusTone::Info,
                        format!("Deleted saved view '{}'.", removed.name),
                    );
                }
            }
            KeyCode::Enter => {
                if let Some(view) = self.saved_views.views.get(search.view_selected).cloned() {
                    self.apply_filter(&view.query)?;
                    self.set_status(
                        StatusTone::Success,
                        format!("Opened saved view '{}'.", view.name),
                    );
                } else {
                    self.set_status(StatusTone::Warning, "There are no saved views yet.");
                    self.search = Some(search.clone());
                }
                return Ok(true);
            }
            _ => {}
        }

        let len = self.saved_views.views.len();
        if len == 0 {
            search.view_selected = 0;
        } else {
            search.view_selected = search.view_selected.min(len - 1);
        }
        Ok(false)
    }

    fn handle_prompt_key(&mut self, key: KeyEvent) -> Result<bool, AppError> {
        let Some(mut prompt) = self.prompt.take() else {
            return Ok(true);
        };

        let mut submit = None;
        match key.code {
            KeyCode::Esc => {
                self.set_status(StatusTone::Info, "Cancelled input.");
            }
            KeyCode::Enter => {
                submit = Some((prompt.mode, prompt.value.trim().to_string()));
            }
            KeyCode::Backspace => prompt.backspace(),
            KeyCode::Delete => prompt.delete(),
            KeyCode::Left => prompt.move_left(),
            KeyCode::Right => prompt.move_right(),
            KeyCode::Home => prompt.cursor = 0,
            KeyCode::End => prompt.cursor = prompt.value.len(),
            KeyCode::Char(character)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                prompt.insert(character);
            }
            _ => {}
        }

        if let Some((mode, value)) = submit {
            self.submit_prompt(mode, &value)?;
        } else if !matches!(key.code, KeyCode::Esc) {
            self.prompt = Some(prompt);
        }

        Ok(true)
    }

    fn submit_prompt(&mut self, mode: PromptMode, value: &str) -> Result<(), AppError> {
        if value.is_empty() {
            self.set_status(StatusTone::Warning, "Input was empty; nothing changed.");
            return Ok(());
        }

        match mode {
            PromptMode::AddChild => self.editor.add_child(value)?,
            PromptMode::AddSibling => self.editor.add_sibling(value)?,
            PromptMode::AddRoot => self.editor.add_root(value)?,
            PromptMode::Edit => self.editor.edit_current(value)?,
            PromptMode::SaveView => {
                self.save_current_search_as(value)?;
                return Ok(());
            }
            PromptMode::OpenId => {
                self.editor.open_id(value)?;
                self.expand_focus_chain();
                self.persist_session()?;
                self.quit_armed = false;
                self.set_status(StatusTone::Success, "Jumped to the requested id.");
                return Ok(());
            }
        }
        let message = match mode {
            PromptMode::AddChild => "Added a child node.",
            PromptMode::AddSibling => "Added a sibling node.",
            PromptMode::AddRoot => "Added a new root node.",
            PromptMode::Edit => "Updated the selected node.",
            PromptMode::SaveView => unreachable!("save view returns early"),
            PromptMode::OpenId => unreachable!("open id returns early"),
        };
        self.after_edit(message)
    }

    fn visible_rows(&self) -> Vec<VisibleRow> {
        let mut rows = Vec::new();
        collect_visible_rows(
            &self.editor.document().nodes,
            &self.expanded,
            self.filter.as_ref(),
            &mut rows,
            Vec::new(),
            0,
        );
        rows
    }

    fn selected_index(&self, rows: &[VisibleRow]) -> usize {
        rows.iter()
            .position(|row| row.path == self.editor.focus_path())
            .or_else(|| {
                self.filter.as_ref().and_then(|filter| {
                    filter
                        .matches
                        .first()
                        .and_then(|path| rows.iter().position(|row| row.path == *path))
                })
            })
            .unwrap_or(0)
    }

    fn move_selection(&mut self, delta: isize) -> Result<(), AppError> {
        let rows = self.visible_rows();
        if rows.is_empty() {
            return Ok(());
        }

        let current_index = self.selected_index(&rows) as isize;
        let next_index = (current_index + delta).clamp(0, rows.len() as isize - 1) as usize;
        self.editor.set_focus_path(rows[next_index].path.clone())?;
        self.persist_session()?;
        if let Some(node) = self.editor.current() {
            self.set_status(StatusTone::Info, format!("Focused '{}'.", node.text));
        }
        Ok(())
    }

    fn collapse_or_parent(&mut self) -> Result<(), AppError> {
        let path = self.editor.focus_path().to_vec();
        if let Some(node) = self.editor.current()
            && !node.children.is_empty()
            && self.expanded.contains(&path)
        {
            self.expanded.remove(&path);
            self.set_status(StatusTone::Info, "Collapsed the branch.");
            return Ok(());
        }

        self.editor.move_parent()?;
        self.persist_session()?;
        self.set_status(StatusTone::Info, "Moved to the parent node.");
        Ok(())
    }

    fn expand_or_child(&mut self) -> Result<(), AppError> {
        let path = self.editor.focus_path().to_vec();
        let Some(node) = self.editor.current() else {
            return Ok(());
        };

        if node.children.is_empty() {
            self.set_status(StatusTone::Warning, "This node has no children to explore.");
            return Ok(());
        }

        if !self.expanded.contains(&path) {
            self.expanded.insert(path);
            self.set_status(StatusTone::Info, "Expanded the branch.");
            return Ok(());
        }

        self.editor.move_child(1)?;
        self.expand_focus_chain();
        self.persist_session()?;
        self.set_status(StatusTone::Info, "Moved into the first child.");
        Ok(())
    }

    fn toggle_branch(&mut self) -> Result<(), AppError> {
        let path = self.editor.focus_path().to_vec();
        let Some(node) = self.editor.current() else {
            return Ok(());
        };

        if node.children.is_empty() {
            self.set_status(StatusTone::Warning, "This node has no children.");
            return Ok(());
        }

        if self.expanded.contains(&path) {
            self.expanded.remove(&path);
            self.set_status(StatusTone::Info, "Collapsed the branch.");
        } else {
            self.expanded.insert(path);
            self.set_status(StatusTone::Info, "Expanded the branch.");
        }
        Ok(())
    }

    fn begin_prompt(&mut self, mode: PromptMode, value: String) {
        self.quit_armed = false;
        self.delete_armed = false;
        self.prompt = Some(PromptState::new(mode, value));
    }

    fn open_search_overlay(&mut self, section: SearchSection) {
        self.quit_armed = false;
        self.delete_armed = false;
        let draft_query = self
            .search
            .as_ref()
            .map(|search| search.draft_query.clone())
            .or_else(|| {
                self.filter
                    .as_ref()
                    .map(|filter| filter.query.raw().to_string())
            })
            .unwrap_or_default();
        self.search = Some(SearchOverlayState::new(section, draft_query));
        self.set_status(
            StatusTone::Info,
            "Search open. Tab switches sections. Enter applies the current selection.",
        );
    }

    fn facet_items_for_query(&self, tab: FacetTab, scope: Option<FilterQuery>) -> Vec<FacetItem> {
        let scope = scope.as_ref();
        match tab {
            FacetTab::Tags => tag_counts_for_filter(self.editor.document(), scope)
                .into_iter()
                .map(|entry| FacetItem {
                    label: entry.tag.clone(),
                    token: entry.tag,
                    count: entry.count,
                    detail: "tag".to_string(),
                })
                .collect(),
            FacetTab::Keys => metadata_key_counts_for_filter(self.editor.document(), scope)
                .into_iter()
                .map(|entry| FacetItem {
                    label: format!("@{}", entry.key),
                    token: format!("@{}", entry.key),
                    count: entry.count,
                    detail: "metadata key".to_string(),
                })
                .collect(),
            FacetTab::Values => metadata_value_counts_for_filter(self.editor.document(), scope)
                .into_iter()
                .map(|entry| FacetItem {
                    label: format!("@{}:{}", entry.key, entry.value),
                    token: format!("@{}:{}", entry.key, entry.value),
                    count: entry.count,
                    detail: format!("{} value", entry.key),
                })
                .collect(),
        }
    }

    fn apply_filter(&mut self, raw: &str) -> Result<(), AppError> {
        let Some(query) = FilterQuery::parse(raw) else {
            self.filter = None;
            self.set_status(StatusTone::Info, "Cleared the active filter.");
            return Ok(());
        };

        let matches = collect_match_paths(self.editor.document(), &query);
        let count = matches.len();
        self.filter = Some(ActiveFilter { query, matches });
        if count == 0 {
            self.set_status(StatusTone::Warning, "Filter applied, but no nodes matched.");
        } else {
            self.move_focus_to_first_match()?;
            self.set_status(
                StatusTone::Success,
                format!("Filter applied with {count} matches."),
            );
        }
        Ok(())
    }

    fn clear_filter(&mut self) -> bool {
        let had_filter = self.filter.is_some();
        self.filter = None;
        had_filter
    }

    fn save_current_search_as(&mut self, name: &str) -> Result<(), AppError> {
        let Some(query) = self.current_search_query_for_save() else {
            return Err(AppError::new(
                "There is no active or drafted filter to save as a named view.",
            ));
        };
        let trimmed_name = name.trim();
        if trimmed_name.is_empty() {
            return Err(AppError::new("Saved view name cannot be empty."));
        }

        if let Some(existing) = self
            .saved_views
            .views
            .iter_mut()
            .find(|view| view.name.eq_ignore_ascii_case(trimmed_name))
        {
            existing.name = trimmed_name.to_string();
            existing.query = query.clone();
        } else {
            self.saved_views.views.push(SavedView {
                name: trimmed_name.to_string(),
                query: query.clone(),
            });
        }
        self.saved_views
            .views
            .sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
        let selected = self
            .saved_views
            .views
            .iter()
            .position(|view| view.name.eq_ignore_ascii_case(trimmed_name))
            .unwrap_or(0);
        if let Some(search) = &mut self.search {
            search.section = SearchSection::Views;
            search.view_selected = selected;
        } else {
            let mut search = SearchOverlayState::new(SearchSection::Views, query.clone());
            search.view_selected = selected;
            self.search = Some(search);
        }
        self.persist_saved_views()?;
        self.set_status(
            StatusTone::Success,
            format!("Saved view '{}'.", trimmed_name),
        );
        Ok(())
    }

    fn current_search_query_for_save(&self) -> Option<String> {
        self.search
            .as_ref()
            .map(|search| search.draft_query.trim().to_string())
            .filter(|query| !query.is_empty())
            .or_else(|| {
                self.filter
                    .as_ref()
                    .map(|filter| filter.query.raw().trim().to_string())
                    .filter(|query| !query.is_empty())
            })
    }

    fn move_match(&mut self, delta: isize) -> Result<(), AppError> {
        let Some((matches, total_matches)) = self
            .filter
            .as_ref()
            .map(|filter| (filter.matches.clone(), filter.matches.len()))
        else {
            return Err(AppError::new(
                "No active filter. Press / to search the map.",
            ));
        };
        if matches.is_empty() {
            return Err(AppError::new("The active filter has no matches."));
        }

        let current_index = matches
            .iter()
            .position(|path| *path == self.editor.focus_path())
            .unwrap_or(0) as isize;
        let len = matches.len() as isize;
        let next_index = (current_index + delta).rem_euclid(len) as usize;
        self.editor.set_focus_path(matches[next_index].clone())?;
        self.expand_focus_chain();
        self.persist_session()?;
        self.set_status(
            StatusTone::Info,
            format!("Match {}/{}.", next_index + 1, total_matches),
        );
        Ok(())
    }

    fn move_focus_to_first_match(&mut self) -> Result<(), AppError> {
        let Some(filter) = &self.filter else {
            return Ok(());
        };
        if let Some(path) = filter.matches.first() {
            self.editor.set_focus_path(path.clone())?;
            self.expand_focus_chain();
            self.persist_session()?;
        }
        Ok(())
    }

    fn save_to_disk(&mut self) -> Result<(), AppError> {
        self.editor.save()?;
        fs::write(&self.map_path, serialize_document(self.editor.document())).map_err(|error| {
            AppError::new(format!(
                "Could not write '{}': {error}",
                self.map_path.display()
            ))
        })?;
        self.editor.mark_clean();
        self.persist_session()?;
        self.set_status(
            StatusTone::Success,
            format!("Saved '{}'.", self.map_path.display()),
        );
        Ok(())
    }

    fn revert_from_disk(&mut self) -> Result<(), AppError> {
        let focus_id = self.editor.current().and_then(|node| node.id.clone());
        let focus_path = self.editor.focus_path().to_vec();
        let map_arg = self.map_path.display().to_string();
        let loaded = load_document(&map_arg)?;
        ensure_parseable(&loaded)?;

        let next_focus = if let Some(id) = &focus_id {
            find_path_by_id(&loaded.document.nodes, id)
        } else {
            None
        }
        .or_else(|| {
            if get_node(&loaded.document.nodes, &focus_path).is_some() {
                Some(focus_path.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| default_focus_path(&loaded.document));

        let mut expanded = initial_expanded_paths(&loaded.document);
        for ancestor in ancestor_paths(&next_focus) {
            expanded.insert(ancestor);
        }

        self.editor = Editor::new(loaded.document, next_focus);
        self.expanded = expanded;
        self.persist_session()?;
        self.set_status(
            StatusTone::Warning,
            format!(
                "Reverted '{}' to the last saved version.",
                self.map_path.display()
            ),
        );
        Ok(())
    }

    fn persist_session(&self) -> Result<(), AppError> {
        save_session_for(&self.map_path, &self.editor.session_state())
    }

    fn persist_saved_views(&self) -> Result<(), AppError> {
        save_views_for(&self.map_path, &self.saved_views)
    }

    fn expand_focus_chain(&mut self) {
        for ancestor in ancestor_paths(self.editor.focus_path()) {
            self.expanded.insert(ancestor);
        }
    }

    fn set_status(&mut self, tone: StatusTone, text: impl Into<String>) {
        self.status = StatusMessage {
            tone,
            text: text.into(),
        };
    }

    fn after_edit(&mut self, message: impl Into<String>) -> Result<(), AppError> {
        self.expand_focus_chain();
        self.persist_session()?;
        self.quit_armed = false;
        let message = message.into();
        if self.autosave {
            self.save_to_disk()?;
            self.set_status(StatusTone::Success, format!("{message} Autosaved."));
        } else {
            self.set_status(StatusTone::Success, message);
        }
        Ok(())
    }

    fn apply_edit<F>(&mut self, edit: F, message: impl Into<String>) -> Result<(), AppError>
    where
        F: FnOnce(&mut Editor) -> Result<(), AppError>,
    {
        edit(&mut self.editor)?;
        self.after_edit(message)
    }
}

pub fn run_interactive(target: &str, autosave: bool) -> Result<(), AppError> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(AppError::new(
            "mdmind needs an interactive terminal. Use `--preview` for a static view.",
        ));
    }

    let loaded = load_document(target)?;
    ensure_parseable(&loaded)?;
    let focus_path = resolve_initial_focus(&loaded.target, &loaded.document)?;
    let warning = if loaded.validation_diagnostics.is_empty() {
        None
    } else {
        Some(format!(
            "{} validation warning(s) are present. Run `mdm validate {}` for details.",
            loaded.validation_diagnostics.len(),
            loaded.target.path.display()
        ))
    };
    let saved_views = load_views_for(&loaded.target.path)?;
    let mut app = TuiApp::new(
        loaded.target.path.clone(),
        loaded.document,
        focus_path,
        warning,
        autosave,
        saved_views,
    );
    app.persist_session()?;

    let mut terminal = setup_terminal()?;
    let result = run_event_loop(&mut terminal, &mut app);
    restore_terminal(&mut terminal)?;
    result
}

fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut TuiApp,
) -> Result<(), AppError> {
    loop {
        terminal
            .draw(|frame| render(frame, app))
            .map_err(|error| AppError::new(format!("Could not draw the TUI: {error}")))?;

        if event::poll(TICK_RATE)
            .map_err(|error| AppError::new(format!("Could not poll terminal events: {error}")))?
        {
            if let Event::Key(key) = event::read()
                .map_err(|error| AppError::new(format!("Could not read terminal input: {error}")))?
            {
                match app.handle_key(key) {
                    Ok(should_continue) => {
                        if !should_continue {
                            break;
                        }
                    }
                    Err(error) => {
                        app.set_status(StatusTone::Error, error.message().to_string());
                    }
                }
            }
        }
    }

    Ok(())
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

fn render(frame: &mut Frame, app: &TuiApp) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(PALETTE.background)),
        area,
    );

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(16),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(area);

    render_header(frame, outer[0], app);
    render_body(frame, outer[1], app);
    render_status(frame, outer[2], app);
    render_keybar(frame, outer[3]);

    if app.help_open {
        render_help_overlay(frame, centered_rect(70, 70, area));
    }

    if let Some(search) = &app.search {
        render_search_overlay(frame, centered_rect(78, 80, area), app, search);
    }

    if let Some(prompt) = &app.prompt {
        render_prompt_overlay(frame, centered_rect(68, 30, area), prompt, app);
    }
}

fn render_header(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(PALETTE.border))
        .style(Style::default().bg(PALETTE.surface))
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let breadcrumb = if app.editor.breadcrumb().is_empty() {
        "(no focus)".to_string()
    } else {
        app.editor.breadcrumb().join("  /  ")
    };
    let badge = if app.editor.dirty() {
        "MODIFIED"
    } else {
        "SAVED"
    };
    let badge_color = if app.editor.dirty() {
        PALETTE.warn
    } else {
        PALETTE.accent
    };
    let autosave_badge = if app.autosave {
        (" AUTOSAVE ", PALETTE.sky)
    } else {
        (" MANUAL ", PALETTE.border)
    };
    let filter_badge = app
        .filter
        .as_ref()
        .map(|filter| format!(" FILTER {} ", filter.matches.len()));
    let mut header_spans = vec![
        Span::styled(
            format!(" mdmind v{APP_VERSION} "),
            Style::default()
                .fg(PALETTE.background)
                .bg(PALETTE.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            app.map_path.display().to_string(),
            Style::default()
                .fg(PALETTE.text)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!(" {badge} "),
            Style::default()
                .fg(PALETTE.background)
                .bg(badge_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            autosave_badge.0,
            Style::default()
                .fg(PALETTE.background)
                .bg(autosave_badge.1)
                .add_modifier(Modifier::BOLD),
        ),
    ];
    if let Some(filter_badge) = filter_badge {
        header_spans.push(Span::raw("  "));
        header_spans.push(Span::styled(
            filter_badge,
            Style::default()
                .fg(PALETTE.background)
                .bg(PALETTE.warn)
                .add_modifier(Modifier::BOLD),
        ));
    }

    let lines = vec![
        Line::from(header_spans),
        Line::from(vec![
            Span::styled("focus ", Style::default().fg(PALETTE.muted)),
            Span::styled(breadcrumb, Style::default().fg(PALETTE.sky)),
        ]),
    ];

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn render_body(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_outline(frame, columns[0], app);
    render_focus_cluster(frame, columns[1], app);
}

fn render_outline(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let rows = app.visible_rows();
    let selected_index = app.selected_index(&rows);

    let items: Vec<ListItem> = rows
        .iter()
        .map(|row| {
            let mut spans = Vec::new();
            spans.push(Span::raw(" ".repeat(row.depth * 2)));
            let icon = if row.has_children {
                if row.expanded { "▾ " } else { "▸ " }
            } else {
                "• "
            };
            let icon_color = if row.has_children {
                PALETTE.accent
            } else {
                PALETTE.muted
            };
            spans.push(Span::styled(icon, Style::default().fg(icon_color)));
            spans.push(Span::styled(
                row.text.clone(),
                Style::default()
                    .fg(if row.matched {
                        PALETTE.sky
                    } else {
                        PALETTE.text
                    })
                    .add_modifier(Modifier::BOLD),
            ));
            if !row.tags.is_empty() {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    row.tags.join(" "),
                    Style::default().fg(PALETTE.accent),
                ));
            }
            if !row.metadata.is_empty() {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    row.metadata.join(" "),
                    Style::default().fg(PALETTE.warn),
                ));
            }
            if let Some(id) = &row.id {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    format!("[id:{id}]"),
                    Style::default().fg(PALETTE.muted),
                ));
            }
            if row.has_children {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    format!("({})", row.child_count),
                    Style::default().fg(PALETTE.sky),
                ));
            }
            if row.matched {
                spans.push(Span::raw(" "));
                spans.push(Span::styled("●", Style::default().fg(PALETTE.warn)));
            }
            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(styled_title(
                    if app.filter.is_some() {
                        "Map · Filtered"
                    } else {
                        "Map"
                    },
                    PALETTE.accent,
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(PALETTE.accent))
                .style(Style::default().bg(PALETTE.surface)),
        )
        .highlight_style(
            Style::default()
                .bg(PALETTE.surface_alt)
                .fg(PALETTE.text)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    if !rows.is_empty() {
        state.select(Some(selected_index));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_focus_cluster(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let focus_height = if app.filter.is_some() { 9 } else { 8 };
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(focus_height), Constraint::Min(8)])
        .split(area);

    render_focus_card(frame, sections[0], app);

    let lanes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(28),
            Constraint::Percentage(32),
            Constraint::Percentage(40),
        ])
        .split(sections[1]);

    render_parent_lane(frame, lanes[0], app);
    render_peer_lane(frame, lanes[1], app);
    render_children_lane(frame, lanes[2], app);
}

fn render_focus_card(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let block = Block::default()
        .title(styled_title("Focus", PALETTE.sky))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(PALETTE.sky))
        .style(Style::default().bg(PALETTE.surface))
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = match app.editor.current() {
        Some(node) => {
            let mut lines = Vec::new();
            lines.push(Line::from(vec![Span::styled(
                node.text.clone(),
                Style::default()
                    .fg(PALETTE.text)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )]));

            if !node.tags.is_empty() || !node.metadata.is_empty() {
                let mut meta = Vec::new();
                if !node.tags.is_empty() {
                    meta.push(Span::styled(
                        node.tags.join(" "),
                        Style::default().fg(PALETTE.accent),
                    ));
                }
                if !node.metadata.is_empty() {
                    if !meta.is_empty() {
                        meta.push(Span::raw("   "));
                    }
                    meta.push(Span::styled(
                        node.metadata
                            .iter()
                            .map(|entry| format!("@{}:{}", entry.key, entry.value))
                            .collect::<Vec<_>>()
                            .join(" "),
                        Style::default().fg(PALETTE.warn),
                    ));
                }
                lines.push(Line::from(meta));
            }

            let breadcrumb = if app.editor.breadcrumb().is_empty() {
                "(no focus)".to_string()
            } else {
                app.editor.breadcrumb().join(" / ")
            };
            lines.push(Line::from(vec![
                Span::styled("path ", Style::default().fg(PALETTE.muted)),
                Span::styled(breadcrumb, Style::default().fg(PALETTE.sky)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("id ", Style::default().fg(PALETTE.muted)),
                Span::styled(
                    node.id.clone().unwrap_or_else(|| "none".to_string()),
                    Style::default().fg(PALETTE.text),
                ),
                Span::raw("   "),
                Span::styled("line ", Style::default().fg(PALETTE.muted)),
                Span::styled(node.line.to_string(), Style::default().fg(PALETTE.text)),
                Span::raw("   "),
                Span::styled("children ", Style::default().fg(PALETTE.muted)),
                Span::styled(
                    node.children.len().to_string(),
                    Style::default().fg(PALETTE.text),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("mind map cue ", Style::default().fg(PALETTE.muted)),
                Span::styled(
                    summarize_relationships(app),
                    Style::default().fg(PALETTE.text),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("save mode ", Style::default().fg(PALETTE.muted)),
                Span::styled(
                    if app.autosave {
                        "autosave after each structural edit"
                    } else {
                        "manual save only"
                    },
                    Style::default().fg(PALETTE.text),
                ),
            ]));
            if let Some(filter) = &app.filter {
                let is_direct_match = current_node_matches_filter(app);
                lines.push(Line::from(vec![
                    Span::styled("filter ", Style::default().fg(PALETTE.muted)),
                    Span::styled(
                        format!("{} ({})", filter.query.raw(), filter.matches.len()),
                        Style::default().fg(PALETTE.warn),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        if is_direct_match {
                            "direct match"
                        } else {
                            "context ancestor"
                        },
                        Style::default().fg(if is_direct_match {
                            PALETTE.accent
                        } else {
                            PALETTE.muted
                        }),
                    ),
                ]));
            }
            lines
        }
        None => vec![Line::from(Span::styled(
            "This map is empty. Press Shift+R to add a root node.",
            Style::default().fg(PALETTE.muted),
        ))],
    };

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn render_parent_lane(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let title = styled_title("Parent", PALETTE.warn);
    render_simple_lane(
        frame,
        area,
        title,
        parent_lines(app),
        Style::default().bg(PALETTE.surface),
    );
}

fn render_peer_lane(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let title = styled_title("Peers", PALETTE.accent);
    render_simple_lane(
        frame,
        area,
        title,
        peer_lines(app),
        Style::default().bg(PALETTE.surface),
    );
}

fn render_children_lane(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let title = styled_title("Children", PALETTE.sky);
    render_simple_lane(
        frame,
        area,
        title,
        child_lines(app),
        Style::default().bg(PALETTE.surface),
    );
}

fn render_simple_lane(
    frame: &mut Frame,
    area: Rect,
    title: Line<'static>,
    lines: Vec<Line<'static>>,
    style: Style,
) {
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(PALETTE.border))
        .style(style)
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn render_help_card(
    frame: &mut Frame,
    area: Rect,
    title: &'static str,
    color: Color,
    lines: Vec<Line<'static>>,
) {
    let block = Block::default()
        .title(styled_title(title, color))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
        .style(Style::default().bg(PALETTE.surface))
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn render_status(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let (label, color) = match app.status.tone {
        StatusTone::Info => ("INFO", PALETTE.sky),
        StatusTone::Success => ("SAVED", PALETTE.accent),
        StatusTone::Warning => ("WARN", PALETTE.warn),
        StatusTone::Error => ("ERROR", PALETTE.danger),
    };
    let line = Line::from(vec![
        Span::styled(
            format!(" {label} "),
            Style::default()
                .fg(PALETTE.background)
                .bg(color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(app.status.text.clone(), Style::default().fg(PALETTE.text)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(PALETTE.border))
        .style(Style::default().bg(PALETTE.surface));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(line), inner);
}

fn render_keybar(frame: &mut Frame, area: Rect) {
    let line = Line::from(vec![
        key_hint("↑↓", "move"),
        Span::raw(" · "),
        key_hint("←→", "tree"),
        Span::raw(" · "),
        key_hint("⌥←→", "nest"),
        Span::raw(" · "),
        key_hint("⌥↑↓", "swap"),
        Span::raw(" · "),
        key_hint("a", "add"),
        Span::raw(" · "),
        key_hint("e", "edit"),
        Span::raw(" · "),
        key_hint("x", "delete"),
        Span::raw(" · "),
        key_hint("/", "filter"),
        Span::raw(" · "),
        key_hint("f", "facets"),
        Span::raw(" · "),
        key_hint("F", "views"),
        Span::raw(" · "),
        key_hint("n", "next"),
        Span::raw(" · "),
        key_hint("s", "save"),
        Span::raw(" · "),
        key_hint("r", "revert"),
        Span::raw(" · "),
        key_hint("?", "help"),
    ]);
    frame.render_widget(
        Paragraph::new(line)
            .style(Style::default().bg(PALETTE.background))
            .alignment(Alignment::Center),
        area,
    );
}

fn render_help_overlay(frame: &mut Frame, area: Rect) {
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(styled_title("Mindmap Guide", PALETTE.accent))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(PALETTE.accent))
        .style(Style::default().bg(PALETTE.surface_alt))
        .padding(Padding::uniform(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    "Visual Mindmap Mode",
                    Style::default()
                        .fg(PALETTE.text)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
                Span::raw("  "),
                Span::styled(
                    format!("v{APP_VERSION}"),
                    Style::default().fg(PALETTE.accent).add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(
                    "Outline on the left, context on the right. Shape the tree without leaving the map.",
                    Style::default().fg(PALETTE.muted),
                ),
            ]),
            Line::from(Span::styled(
                "The selected node is the center of gravity for the whole screen.",
                Style::default().fg(PALETTE.sky),
            )),
        ]),
        sections[0],
    );

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(sections[1]);

    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(36),
            Constraint::Percentage(28),
            Constraint::Percentage(36),
        ])
        .split(columns[0]);
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(34),
            Constraint::Percentage(30),
            Constraint::Percentage(36),
        ])
        .split(columns[1]);

    render_help_card(
        frame,
        left[0],
        "Navigate",
        PALETTE.sky,
        vec![
            Line::from("↑ / ↓   move through visible nodes"),
            Line::from("←       collapse branch or go to parent"),
            Line::from("→       expand branch or enter first child"),
            Line::from("Enter   toggle expanded/collapsed"),
            Line::from("/       open search"),
            Line::from("f       open search on facets"),
            Line::from("F       open search on saved views"),
            Line::from("o       jump directly to a node id"),
        ],
    );
    render_help_card(
        frame,
        left[1],
        "Create / Edit",
        PALETTE.warn,
        vec![
            Line::from("a       add child"),
            Line::from("A       add sibling"),
            Line::from("Shift+R add root"),
            Line::from("e       edit selected node"),
            Line::from("x       press twice to delete"),
        ],
    );
    render_help_card(
        frame,
        left[2],
        "Reshape Tree",
        PALETTE.accent,
        vec![
            Line::from("Alt+↑   move node up"),
            Line::from("Alt+↓   move node down"),
            Line::from("Alt+←   move node out one level"),
            Line::from("Alt+→   indent into previous sibling"),
            Line::from("g       jump back to root"),
        ],
    );
    render_help_card(
        frame,
        right[0],
        "Find / Recover",
        PALETTE.accent,
        vec![
            Line::from("/       search by text, #tag, or @key:value"),
            Line::from("Tab     switch Query / Facets / Saved Views"),
            Line::from("← / →   switch Tags / Keys / Values in Facets"),
            Line::from("n / N   next or previous match"),
            Line::from("c       clear active filter"),
            Line::from("s       save to disk now"),
            Line::from("S       toggle autosave"),
            Line::from("r       reload from disk"),
            Line::from("q       quit, warns if dirty"),
        ],
    );
    render_help_card(
        frame,
        right[1],
        "Inline Syntax",
        PALETTE.sky,
        vec![
            Line::from("#tag              topic or workflow marker"),
            Line::from("@key:value        structured metadata"),
            Line::from("[id:path/to/node] deep link target"),
            Line::from("Combine them on the same line."),
        ],
    );
    render_help_card(
        frame,
        right[2],
        "Example Node",
        PALETTE.warn,
        vec![
            Line::from("API Design #backend"),
            Line::from("@status:todo @owner:jason"),
            Line::from("[id:product/api-design]"),
            Line::from(""),
            Line::from(format!(
                "Esc or ? closes this overlay. Running mdmind v{APP_VERSION}."
            )),
        ],
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Tip ", Style::default().fg(PALETTE.warn).add_modifier(Modifier::BOLD)),
            Span::styled(
                "Use the right-side parent, peer, and child panels as your mental compass while editing.",
                Style::default().fg(PALETTE.muted),
            ),
        ])),
        sections[2],
    );
}

fn render_search_overlay(frame: &mut Frame, area: Rect, app: &TuiApp, search: &SearchOverlayState) {
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(styled_title("Find", PALETTE.accent))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(PALETTE.accent))
        .style(Style::default().bg(PALETTE.surface_alt))
        .padding(Padding::uniform(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(inner);

    let scope_line = current_scope_label(app, Some(search));
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    "One Search Surface",
                    Style::default()
                        .fg(PALETTE.text)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
                Span::raw("  "),
                Span::styled(
                    "Query, browse facets, and reopen saved working sets without leaving the map.",
                    Style::default().fg(PALETTE.muted),
                ),
            ]),
            Line::from(Span::styled(scope_line, Style::default().fg(PALETTE.sky))),
        ]),
        sections[0],
    );

    render_search_section_tabs(frame, sections[1], search);

    match search.section {
        SearchSection::Query => render_search_query_section(frame, sections[2], app, search),
        SearchSection::Facets => render_search_facets_section(frame, sections[2], app, search),
        SearchSection::Views => render_search_views_section(frame, sections[2], app, search),
    }

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            key_hint("Tab", "sections"),
            Span::raw(" · "),
            key_hint("↑↓", "select"),
            Span::raw(" · "),
            key_hint("Enter", "apply"),
            Span::raw(" · "),
            key_hint("c", "clear"),
            Span::raw(" · "),
            key_hint("Esc", "close"),
        ]))
        .alignment(Alignment::Center)
        .style(Style::default().fg(PALETTE.muted)),
        sections[3],
    );
}

fn render_search_section_tabs(frame: &mut Frame, area: Rect, search: &SearchOverlayState) {
    let spans = [
        SearchSection::Query,
        SearchSection::Facets,
        SearchSection::Views,
    ]
    .into_iter()
    .flat_map(|section| {
        let is_active = section == search.section;
        let mut spans = vec![Span::styled(
            format!(" {} ", section.title()),
            Style::default()
                .fg(if is_active {
                    PALETTE.background
                } else {
                    PALETTE.text
                })
                .bg(if is_active {
                    PALETTE.accent
                } else {
                    PALETTE.surface
                })
                .add_modifier(Modifier::BOLD),
        )];
        if section != SearchSection::Views {
            spans.push(Span::raw(" "));
        }
        spans
    })
    .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_search_query_section(
    frame: &mut Frame,
    area: Rect,
    app: &TuiApp,
    search: &SearchOverlayState,
) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(8),
        ])
        .split(area);

    let input_block = Block::default()
        .title(styled_title("Query", PALETTE.warn))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(PALETTE.warn))
        .style(Style::default().bg(PALETTE.surface))
        .padding(Padding::horizontal(1));
    let input_inner = input_block.inner(sections[0]);
    frame.render_widget(input_block, sections[0]);
    frame.render_widget(
        Paragraph::new(search.draft_query.clone())
            .style(Style::default().fg(PALETTE.text))
            .wrap(Wrap { trim: false }),
        input_inner,
    );
    frame.set_cursor_position((input_inner.x + search.cursor as u16, input_inner.y));

    let preview = query_preview_matches(app, &search.draft_query);
    let helper = if search.draft_query.trim().is_empty() {
        "Type text, #tag, or @key:value. Enter applies the query. Empty input clears the filter."
            .to_string()
    } else {
        format!(
            "{} live match(es). Enter applies the query to the map.",
            preview.len()
        )
    };
    frame.render_widget(
        Paragraph::new(helper).style(Style::default().fg(PALETTE.sky)),
        sections[1],
    );

    if preview.is_empty() {
        frame.render_widget(
            Paragraph::new("No matches yet.")
                .block(
                    Block::default()
                        .title(styled_title("Preview", PALETTE.sky))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(PALETTE.border))
                        .style(Style::default().bg(PALETTE.surface))
                        .padding(Padding::horizontal(1)),
                )
                .style(Style::default().fg(PALETTE.muted))
                .wrap(Wrap { trim: false }),
            sections[2],
        );
        return;
    }

    let items = preview
        .iter()
        .take(8)
        .map(|entry| {
            ListItem::new(Line::from(vec![
                Span::styled(entry.0.clone(), Style::default().fg(PALETTE.text)),
                Span::raw("  "),
                Span::styled(entry.1.clone(), Style::default().fg(PALETTE.muted)),
            ]))
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        List::new(items).block(
            Block::default()
                .title(styled_title("Preview", PALETTE.sky))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(PALETTE.sky))
                .style(Style::default().bg(PALETTE.surface)),
        ),
        sections[2],
    );
}

fn render_search_facets_section(
    frame: &mut Frame,
    area: Rect,
    app: &TuiApp,
    search: &SearchOverlayState,
) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(8)])
        .split(area);

    let tabs = [FacetTab::Tags, FacetTab::Keys, FacetTab::Values]
        .into_iter()
        .flat_map(|tab| {
            let is_active = tab == search.facet_tab;
            let mut spans = vec![Span::styled(
                format!(" {} ", tab.title()),
                Style::default()
                    .fg(if is_active {
                        PALETTE.background
                    } else {
                        PALETTE.text
                    })
                    .bg(if is_active {
                        PALETTE.sky
                    } else {
                        PALETTE.surface
                    })
                    .add_modifier(Modifier::BOLD),
            )];
            if tab != FacetTab::Values {
                spans.push(Span::raw(" "));
            }
            spans
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(Line::from(tabs)), sections[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .split(sections[1]);

    let items = app.facet_items_for_query(search.facet_tab, search.query());
    if items.is_empty() {
        frame.render_widget(
            Paragraph::new(search.facet_tab.empty_message())
                .block(
                    Block::default()
                        .title(styled_title(search.facet_tab.title(), PALETTE.sky))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(PALETTE.border))
                        .style(Style::default().bg(PALETTE.surface))
                        .padding(Padding::horizontal(1)),
                )
                .style(Style::default().fg(PALETTE.muted))
                .wrap(Wrap { trim: false }),
            body[0],
        );
    } else {
        let list_items = items
            .iter()
            .map(|item| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        item.label.clone(),
                        Style::default()
                            .fg(PALETTE.text)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("{} nodes", item.count),
                        Style::default().fg(PALETTE.warn),
                    ),
                ]))
            })
            .collect::<Vec<_>>();
        let mut state = ListState::default();
        state.select(Some(search.facet_selected.min(items.len() - 1)));
        frame.render_stateful_widget(
            List::new(list_items)
                .block(
                    Block::default()
                        .title(styled_title(search.facet_tab.title(), PALETTE.sky))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(PALETTE.sky))
                        .style(Style::default().bg(PALETTE.surface)),
                )
                .highlight_style(
                    Style::default()
                        .bg(PALETTE.surface_alt)
                        .fg(PALETTE.text)
                        .add_modifier(Modifier::BOLD),
                ),
            body[0],
            &mut state,
        );
    }

    let selection = items.get(search.facet_selected);
    let preview_lines = match selection {
        Some(item) => vec![
            Line::from(vec![
                Span::styled("selected ", Style::default().fg(PALETTE.muted)),
                Span::styled(item.label.clone(), Style::default().fg(PALETTE.text)),
            ]),
            Line::from(vec![
                Span::styled("apply ", Style::default().fg(PALETTE.muted)),
                Span::styled(
                    compose_query_with_token(&search.draft_query, &item.token),
                    Style::default().fg(PALETTE.accent),
                ),
            ]),
            Line::from(Span::styled(
                "Enter applies this facet. Left and right switch Tags / Keys / Values.",
                Style::default().fg(PALETTE.muted),
            )),
        ],
        None => vec![Line::from(Span::styled(
            search.facet_tab.empty_message(),
            Style::default().fg(PALETTE.muted),
        ))],
    };
    frame.render_widget(
        Paragraph::new(preview_lines)
            .block(
                Block::default()
                    .title(styled_title("Preview", PALETTE.warn))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(PALETTE.warn))
                    .style(Style::default().bg(PALETTE.surface))
                    .padding(Padding::horizontal(1)),
            )
            .wrap(Wrap { trim: false }),
        body[1],
    );
}

fn render_search_views_section(
    frame: &mut Frame,
    area: Rect,
    app: &TuiApp,
    search: &SearchOverlayState,
) {
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(area);

    if app.saved_views.views.is_empty() {
        frame.render_widget(
            Paragraph::new("No saved views yet. Type or apply a query, then press a to save it.")
                .block(
                    Block::default()
                        .title(styled_title("Views", PALETTE.sky))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(PALETTE.border))
                        .style(Style::default().bg(PALETTE.surface))
                        .padding(Padding::horizontal(1)),
                )
                .style(Style::default().fg(PALETTE.muted))
                .wrap(Wrap { trim: false }),
            body[0],
        );
    } else {
        let items = app
            .saved_views
            .views
            .iter()
            .map(|view| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        view.name.clone(),
                        Style::default()
                            .fg(PALETTE.text)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(view.query.clone(), Style::default().fg(PALETTE.warn)),
                ]))
            })
            .collect::<Vec<_>>();
        let mut state = ListState::default();
        state.select(Some(
            search.view_selected.min(app.saved_views.views.len() - 1),
        ));
        frame.render_stateful_widget(
            List::new(items)
                .block(
                    Block::default()
                        .title(styled_title("Views", PALETTE.sky))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(PALETTE.sky))
                        .style(Style::default().bg(PALETTE.surface)),
                )
                .highlight_style(
                    Style::default()
                        .bg(PALETTE.surface_alt)
                        .fg(PALETTE.text)
                        .add_modifier(Modifier::BOLD),
                ),
            body[0],
            &mut state,
        );
    }

    let preview_lines = if let Some(view) = app.saved_views.views.get(search.view_selected) {
        vec![
            Line::from(vec![
                Span::styled("name ", Style::default().fg(PALETTE.muted)),
                Span::styled(view.name.clone(), Style::default().fg(PALETTE.text)),
            ]),
            Line::from(vec![
                Span::styled("query ", Style::default().fg(PALETTE.muted)),
                Span::styled(view.query.clone(), Style::default().fg(PALETTE.warn)),
            ]),
            Line::from(Span::styled(
                "Enter opens this view. a saves the current query. x deletes the selected view.",
                Style::default().fg(PALETTE.muted),
            )),
        ]
    } else {
        let views_path = crate::views::views_path_for(&app.map_path)
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| "(unavailable)".to_string());
        vec![
            Line::from(Span::styled(
                "Saved views live in a local sidecar next to the map.",
                Style::default().fg(PALETTE.muted),
            )),
            Line::from(Span::styled(views_path, Style::default().fg(PALETTE.sky))),
        ]
    };
    frame.render_widget(
        Paragraph::new(preview_lines)
            .block(
                Block::default()
                    .title(styled_title("Preview", PALETTE.warn))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(PALETTE.warn))
                    .style(Style::default().bg(PALETTE.surface))
                    .padding(Padding::horizontal(1)),
            )
            .wrap(Wrap { trim: false }),
        body[1],
    );
}

fn current_scope_label(app: &TuiApp, search: Option<&SearchOverlayState>) -> String {
    if let Some(search) = search
        && !search.draft_query.trim().is_empty()
    {
        let count = find_matches(app.editor.document(), &search.draft_query).len();
        return format!(
            "Draft query: '{}' ({} matching nodes)",
            search.draft_query.trim(),
            count
        );
    }

    if let Some(filter) = &app.filter {
        return format!(
            "Active filter: '{}' ({} matching nodes)",
            filter.query.raw(),
            filter.matches.len()
        );
    }

    format!(
        "Whole map ({} nodes)",
        count_nodes(&app.editor.document().nodes)
    )
}

fn query_preview_matches(app: &TuiApp, raw: &str) -> Vec<(String, String)> {
    find_matches(app.editor.document(), raw)
        .into_iter()
        .map(|entry| {
            let detail = entry.id.unwrap_or_else(|| entry.breadcrumb);
            (entry.text, detail)
        })
        .collect()
}

fn render_prompt_overlay(frame: &mut Frame, area: Rect, prompt: &PromptState, _app: &TuiApp) {
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(styled_title(prompt.mode.title(), PALETTE.warn))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(PALETTE.warn))
        .style(Style::default().bg(PALETTE.surface_alt))
        .padding(Padding::uniform(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(1),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(prompt.mode.hint())
            .style(Style::default().fg(PALETTE.muted))
            .wrap(Wrap { trim: false }),
        chunks[0],
    );

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(PALETTE.accent))
        .style(Style::default().bg(PALETTE.surface));
    let input_inner = input_block.inner(chunks[1]);
    frame.render_widget(input_block, chunks[1]);
    frame.render_widget(
        Paragraph::new(prompt.value.clone())
            .style(Style::default().fg(PALETTE.text))
            .wrap(Wrap { trim: false }),
        input_inner,
    );
    frame.set_cursor_position((input_inner.x + prompt.cursor as u16, input_inner.y));

    frame.render_widget(
        Paragraph::new("Enter saves the action. Esc cancels.")
            .style(Style::default().fg(PALETTE.sky)),
        chunks[2],
    );
}

fn resolve_initial_focus(target: &TargetRef, document: &Document) -> Result<Vec<usize>, AppError> {
    if let Some(anchor) = &target.anchor {
        return find_path_by_id(&document.nodes, anchor)
            .ok_or_else(|| AppError::new(format!("No node id matches anchor '{anchor}'.")));
    }

    if let Some(session) = load_session_for(&target.path)?
        && let Some(path) = resolve_session_focus(document, &session)
    {
        return Ok(path);
    }

    Ok(default_focus_path(document))
}

fn initial_expanded_paths(document: &Document) -> HashSet<Vec<usize>> {
    let mut expanded = HashSet::new();
    expand_to_depth(&document.nodes, &mut expanded, Vec::new(), 0, 1);
    expanded
}

fn expand_to_depth(
    nodes: &[Node],
    expanded: &mut HashSet<Vec<usize>>,
    prefix: Vec<usize>,
    depth: usize,
    limit: usize,
) {
    if depth > limit {
        return;
    }

    for (index, node) in nodes.iter().enumerate() {
        let mut path = prefix.clone();
        path.push(index);
        if !node.children.is_empty() && depth < limit {
            expanded.insert(path.clone());
        }
        expand_to_depth(&node.children, expanded, path, depth + 1, limit);
    }
}

fn ancestor_paths(path: &[usize]) -> Vec<Vec<usize>> {
    let mut ancestors = Vec::new();
    for index in 0..path.len() {
        ancestors.push(path[..=index].to_vec());
    }
    ancestors
}

fn collect_visible_rows(
    nodes: &[Node],
    expanded: &HashSet<Vec<usize>>,
    filter: Option<&ActiveFilter>,
    rows: &mut Vec<VisibleRow>,
    prefix: Vec<usize>,
    depth: usize,
) -> bool {
    let mut subtree_has_visible_match = false;
    for (index, node) in nodes.iter().enumerate() {
        let mut path = prefix.clone();
        path.push(index);
        let matched =
            filter.is_some_and(|filter| filter.matches.iter().any(|candidate| *candidate == path));
        let include_children_by_expansion = expanded.contains(&path);
        let mut child_rows = Vec::new();
        let child_has_match = collect_visible_rows(
            &node.children,
            expanded,
            filter,
            &mut child_rows,
            path.clone(),
            depth + 1,
        );
        let include_row = match filter {
            Some(_) => matched || child_has_match,
            None => true,
        };

        if !include_row {
            continue;
        }

        rows.push(VisibleRow {
            path: path.clone(),
            depth,
            text: node.text.clone(),
            tags: node.tags.clone(),
            metadata: node
                .metadata
                .iter()
                .map(|entry| format!("@{}:{}", entry.key, entry.value))
                .collect(),
            id: node.id.clone(),
            line: node.line,
            has_children: !node.children.is_empty(),
            expanded: expanded.contains(&path),
            child_count: node.children.len(),
            matched,
        });

        if filter.is_some() {
            rows.extend(child_rows);
        } else if include_children_by_expansion {
            rows.extend(child_rows);
        }
        subtree_has_visible_match = true;
    }
    subtree_has_visible_match
}

fn collect_match_paths(document: &Document, query: &FilterQuery) -> Vec<Vec<usize>> {
    let mut paths = Vec::new();
    collect_match_paths_from_nodes(&document.nodes, query, &mut paths, Vec::new());
    paths
}

fn collect_match_paths_from_nodes(
    nodes: &[Node],
    query: &FilterQuery,
    paths: &mut Vec<Vec<usize>>,
    prefix: Vec<usize>,
) {
    for (index, node) in nodes.iter().enumerate() {
        let mut path = prefix.clone();
        path.push(index);
        if query.matches(node) {
            paths.push(path.clone());
        }
        collect_match_paths_from_nodes(&node.children, query, paths, path);
    }
}

fn styled_title(title: &'static str, color: Color) -> Line<'static> {
    Line::from(Span::styled(
        format!(" {title} "),
        Style::default()
            .fg(PALETTE.background)
            .bg(color)
            .add_modifier(Modifier::BOLD),
    ))
}

fn key_hint<'a>(key: &'a str, meaning: &'a str) -> Span<'a> {
    Span::styled(
        format!("{key}:{meaning}"),
        Style::default().fg(PALETTE.muted),
    )
}

fn summarize_relationships(app: &TuiApp) -> String {
    let parent = parent_node(app);
    let peers = peer_nodes(app);
    let children = app
        .editor
        .current()
        .map(|node| node.children.len())
        .unwrap_or(0);

    let parent_label = parent
        .map(|node| node.text.clone())
        .unwrap_or_else(|| "root".to_string());
    format!(
        "{parent_label} -> {} peers -> {children} children",
        peers.len()
    )
}

fn current_node_matches_filter(app: &TuiApp) -> bool {
    app.filter.as_ref().is_some_and(|filter| {
        filter
            .matches
            .iter()
            .any(|path| *path == app.editor.focus_path())
    })
}

fn count_nodes(nodes: &[Node]) -> usize {
    nodes
        .iter()
        .map(|node| 1 + count_nodes(&node.children))
        .sum()
}

fn compose_query_with_token(raw: &str, token: &str) -> String {
    let mut terms = raw
        .split_whitespace()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if !terms.iter().any(|term| term.eq_ignore_ascii_case(token)) {
        terms.push(token.to_string());
    }
    terms.join(" ")
}

fn parent_node(app: &TuiApp) -> Option<&Node> {
    let path = app.editor.focus_path();
    if path.len() <= 1 {
        return None;
    }
    get_node(
        app.editor.document().nodes.as_slice(),
        &path[..path.len() - 1],
    )
}

fn peer_nodes(app: &TuiApp) -> Vec<&Node> {
    let path = app.editor.focus_path();
    if path.is_empty() {
        return Vec::new();
    }

    let siblings = if path.len() == 1 {
        app.editor.document().nodes.as_slice()
    } else {
        match get_node(
            app.editor.document().nodes.as_slice(),
            &path[..path.len() - 1],
        ) {
            Some(parent) => parent.children.as_slice(),
            None => &[],
        }
    };

    siblings
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            if Some(&index) == path.last() {
                None
            } else {
                Some(node)
            }
        })
        .collect()
}

fn parent_lines(app: &TuiApp) -> Vec<Line<'static>> {
    match parent_node(app) {
        Some(parent) => vec![
            Line::from(Span::styled(
                parent.text.clone(),
                Style::default()
                    .fg(PALETTE.text)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                parent.id.clone().unwrap_or_else(|| "no id".to_string()),
                Style::default().fg(PALETTE.muted),
            )),
        ],
        None => vec![Line::from(Span::styled(
            "This node is at the root level.",
            Style::default().fg(PALETTE.muted),
        ))],
    }
}

fn peer_lines(app: &TuiApp) -> Vec<Line<'static>> {
    let peers = peer_nodes(app);
    if peers.is_empty() {
        return vec![Line::from(Span::styled(
            "No peer nodes beside the current selection.",
            Style::default().fg(PALETTE.muted),
        ))];
    }

    peers
        .into_iter()
        .map(|node| {
            Line::from(vec![
                Span::styled("• ", Style::default().fg(PALETTE.accent)),
                Span::styled(node.text.clone(), Style::default().fg(PALETTE.text)),
            ])
        })
        .collect()
}

fn child_lines(app: &TuiApp) -> Vec<Line<'static>> {
    let Some(node) = app.editor.current() else {
        return vec![Line::from(Span::styled(
            "No selected node.",
            Style::default().fg(PALETTE.muted),
        ))];
    };

    if node.children.is_empty() {
        return vec![Line::from(Span::styled(
            "No children yet. Press a to grow this branch.",
            Style::default().fg(PALETTE.muted),
        ))];
    }

    node.children
        .iter()
        .enumerate()
        .map(|(index, child)| {
            let mut spans = vec![
                Span::styled(format!("{}. ", index + 1), Style::default().fg(PALETTE.sky)),
                Span::styled(
                    child.text.clone(),
                    Style::default()
                        .fg(PALETTE.text)
                        .add_modifier(Modifier::BOLD),
                ),
            ];
            if !child.tags.is_empty() {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    child.tags.join(" "),
                    Style::default().fg(PALETTE.accent),
                ));
            }
            Line::from(spans)
        })
        .collect()
}

fn centered_rect(horizontal_percent: u16, vertical_percent: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - vertical_percent) / 2),
            Constraint::Percentage(vertical_percent),
            Constraint::Percentage((100 - vertical_percent) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - horizontal_percent) / 2),
            Constraint::Percentage(horizontal_percent),
            Constraint::Percentage((100 - horizontal_percent) / 2),
        ])
        .split(vertical[1])[1]
}

fn previous_boundary(value: &str, index: usize) -> usize {
    value[..index]
        .char_indices()
        .last()
        .map(|(position, _)| position)
        .unwrap_or(0)
}

fn next_boundary(value: &str, index: usize) -> usize {
    if index >= value.len() {
        return value.len();
    }
    value[index..]
        .char_indices()
        .nth(1)
        .map(|(offset, _)| index + offset)
        .unwrap_or(value.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_document;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn sample_document() -> Document {
        parse_document(
            "- Product Idea [id:product]\n  - Direction #idea [id:product/direction]\n    - CLI-first MVP\n  - Tasks #todo @status:active [id:product/tasks]\n    - Build parser\n    - Ship tests\n",
        )
        .document
    }

    fn temp_map_path(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("mdmind-tui-{nonce}-{name}"))
    }

    #[test]
    fn tree_navigation_moves_with_arrow_keys() {
        let map_path = temp_map_path("map.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
            .expect("down should work");
        assert_eq!(app.editor.focus_path(), &[0, 0]);

        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE))
            .expect("right should work");
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE))
            .expect("second right should work");
        assert_eq!(app.editor.focus_path(), &[0, 0, 0]);

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn prompt_submission_adds_a_child() {
        let map_path = temp_map_path("map.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );
        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
            .expect("down should work");
        app.begin_prompt(
            PromptMode::AddChild,
            "New Branch #todo [id:product/direction/new]".to_string(),
        );
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should submit");

        let current = app.editor.current().expect("current node should exist");
        assert_eq!(current.text, "New Branch");
        assert_eq!(current.id.as_deref(), Some("product/direction/new"));

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn autosave_writes_after_a_structural_edit() {
        let map_path = temp_map_path("autosave.md");
        let source = "- Product Idea [id:product]\n  - Tasks #todo [id:product/tasks]\n";
        std::fs::write(&map_path, source).expect("fixture map should be writable");
        let document = parse_document(source).document;
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            true,
            SavedViewsState::default(),
        );

        app.begin_prompt(
            PromptMode::AddChild,
            "New Branch #todo [id:product/new]".to_string(),
        );
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should submit");

        let saved = std::fs::read_to_string(&map_path).expect("autosaved map should be readable");
        assert!(saved.contains("New Branch #todo [id:product/new]"));

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
        std::fs::remove_file(map_path).ok();
    }

    #[test]
    fn revert_reloads_the_last_saved_map() {
        let map_path = temp_map_path("revert.md");
        let source = "- Product Idea [id:product]\n  - Tasks #todo [id:product/tasks]\n";
        std::fs::write(&map_path, source).expect("fixture map should be writable");
        let document = parse_document(source).document;
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.begin_prompt(
            PromptMode::AddChild,
            "Unsaved Idea #todo [id:product/unsaved]".to_string(),
        );
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should submit");
        assert!(app.editor.dirty(), "edit should mark the editor dirty");

        app.handle_key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE))
            .expect("revert should succeed");

        assert!(!app.editor.dirty(), "revert should clear the dirty state");
        let rendered = serialize_document(app.editor.document());
        assert!(!rendered.contains("Unsaved Idea"));

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
        std::fs::remove_file(map_path).ok();
    }

    #[test]
    fn filter_highlights_matches_and_cycles_between_them() {
        let map_path = temp_map_path("filter.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.open_search_overlay(SearchSection::Query);
        app.search.as_mut().expect("search should open").draft_query = "#todo".to_string();
        app.search.as_mut().expect("search should open").cursor = "#todo".len();
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should apply the filter");

        assert_eq!(
            app.filter.as_ref().map(|filter| filter.matches.len()),
            Some(1)
        );
        assert_eq!(app.editor.focus_path(), &[0, 1]);

        let rows = app.visible_rows();
        assert!(rows.iter().any(|row| row.matched && row.text == "Tasks"));
        assert!(rows.iter().any(|row| row.text == "Product Idea"));

        app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE))
            .expect("cycling a single match should still work");
        assert_eq!(app.editor.focus_path(), &[0, 1]);

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn esc_clears_the_active_filter_before_showing_generic_status() {
        let map_path = temp_map_path("filter-esc.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.open_search_overlay(SearchSection::Query);
        app.search.as_mut().expect("search should open").draft_query = "#todo".to_string();
        app.search.as_mut().expect("search should open").cursor = "#todo".len();
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should apply the filter");
        assert!(app.filter.is_some(), "filter should be active");

        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .expect("escape should be handled");

        assert!(
            app.filter.is_none(),
            "escape should clear the active filter"
        );
        assert_eq!(app.status.text, "Cleared the active filter.");

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn filter_state_distinguishes_direct_matches_from_context_ancestors() {
        let map_path = temp_map_path("filter-context.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.open_search_overlay(SearchSection::Query);
        app.search.as_mut().expect("search should open").draft_query = "#todo".to_string();
        app.search.as_mut().expect("search should open").cursor = "#todo".len();
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should apply the filter");
        assert!(
            current_node_matches_filter(&app),
            "the initial focused match should be marked as a direct match"
        );

        app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))
            .expect("up should move to the ancestor kept as filter context");
        assert_eq!(app.editor.focus_path(), &[0]);
        assert!(
            !current_node_matches_filter(&app),
            "ancestor context should not be treated as a direct match"
        );

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn facet_browser_applies_the_selected_tag_filter() {
        let map_path = temp_map_path("facet-tag.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE))
            .expect("f should open search on facets");
        assert!(
            app.search
                .as_ref()
                .is_some_and(|search| search.section == SearchSection::Facets),
            "search should open on facets"
        );

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should apply the selected facet");

        assert_eq!(
            app.filter.as_ref().map(|filter| filter.query.raw()),
            Some("#idea")
        );
        assert_eq!(app.editor.focus_path(), &[0, 0]);

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn facet_browser_compounds_the_active_filter_scope() {
        let map_path = temp_map_path("facet-scope.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.open_search_overlay(SearchSection::Query);
        app.search.as_mut().expect("search should open").draft_query = "#todo".to_string();
        app.search.as_mut().expect("search should open").cursor = "#todo".len();
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should apply the filter");
        assert_eq!(
            app.filter.as_ref().map(|filter| filter.matches.len()),
            Some(1)
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE))
            .expect("f should open search on facets");
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE))
            .expect("right should move to keys");
        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE))
            .expect("right should move to values");
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should apply the selected metadata value facet");

        assert_eq!(
            app.filter.as_ref().map(|filter| filter.query.raw()),
            Some("#todo @status:active")
        );

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn saved_views_in_search_persist_and_reopen_a_named_filter() {
        let map_path = temp_map_path("saved-views.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.open_search_overlay(SearchSection::Query);
        app.search.as_mut().expect("search should open").draft_query = "#todo".to_string();
        app.search.as_mut().expect("search should open").cursor = "#todo".len();
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should apply the filter");

        app.handle_key(KeyEvent::new(KeyCode::Char('F'), KeyModifiers::NONE))
            .expect("F should open saved views");
        app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE))
            .expect("a should begin save-view prompt");
        app.handle_key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE))
            .expect("t should type");
        app.handle_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE))
            .expect("o should type");
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE))
            .expect("d should type");
        app.handle_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE))
            .expect("o should type");
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should save the named view");

        assert_eq!(app.saved_views.views.len(), 1);
        assert_eq!(app.saved_views.views[0].name, "todo");
        assert_eq!(app.saved_views.views[0].query, "#todo");

        let views_path =
            crate::views::views_path_for(&map_path).expect("views path should be derivable");
        assert!(
            views_path.exists(),
            "saving a view should write the sidecar"
        );

        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .expect("escape should close the unified search overlay");
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .expect("a second escape should clear the active filter in the main TUI");
        assert!(app.filter.is_none(), "filter should be cleared");

        app.handle_key(KeyEvent::new(KeyCode::Char('F'), KeyModifiers::NONE))
            .expect("F should reopen saved views");
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should reopen the saved view");

        assert_eq!(
            app.filter.as_ref().map(|filter| filter.query.raw()),
            Some("#todo")
        );

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
        if views_path.exists() {
            std::fs::remove_file(views_path).ok();
        }
    }

    #[test]
    fn delete_requires_confirmation_and_removes_the_node() {
        let map_path = temp_map_path("delete.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0, 1],
            None,
            false,
            SavedViewsState::default(),
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE))
            .expect("first x should arm delete");
        assert!(app.delete_armed, "delete should be armed after first x");

        app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE))
            .expect("second x should delete");
        assert!(!app.delete_armed, "delete should disarm after deletion");
        assert_eq!(app.editor.focus_path(), &[0, 0]);
        assert_eq!(
            app.editor.current().expect("focus should exist").text,
            "Direction"
        );

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }
}
