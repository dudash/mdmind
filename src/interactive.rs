use std::cell::Cell;
use std::collections::HashSet;
use std::fs;
use std::io::{self, IsTerminal, Stdout};
use std::path::PathBuf;
use std::time::Duration;

use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, size,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Margin, Rect};
use ratatui::prelude::{Color, Line, Modifier, Span, Style};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Padding, Paragraph, Wrap,
};
use ratatui::{Frame, Terminal};

use crate::APP_VERSION;
use crate::app::{AppError, TargetRef, ensure_parseable, load_document};
use crate::checkpoints::{
    Checkpoint, CheckpointAnchor, CheckpointViewMode, CheckpointsState, load_checkpoints_for,
    save_checkpoints_for,
};
use crate::editor::{Editor, EditorState, default_focus_path, find_path_by_id, get_node};
use crate::mindmap::{
    MindmapWidget, Scene as MindmapScene, Theme as MindmapTheme, default_export_path, export_png,
};
use crate::model::{Document, Node};
use crate::query::{
    FilterQuery, find_matches, metadata_key_counts_for_filter, metadata_value_counts_for_filter,
    tag_counts_for_filter,
};
use crate::serializer::serialize_document;
use crate::session::{load_session_for, resolve_session_focus, save_session_for};
use crate::ui_settings::{ThemeId, UiSettings, load_ui_settings_for, save_ui_settings_for};
use crate::views::{SavedView, SavedViewsState, load_views_for, save_views_for};

const TICK_RATE: Duration = Duration::from_millis(150);
const HISTORY_LIMIT: usize = 64;
const CHECKPOINT_LIMIT: usize = 12;

type Palette = MindmapTheme;

thread_local! {
    static ACTIVE_PALETTE: Cell<Palette> = Cell::new(ThemeId::Workbench.theme());
    static ACTIVE_ASCII_ACCENTS: Cell<bool> = const { Cell::new(false) };
    static ACTIVE_MOTION_TARGET: Cell<Option<MotionTarget>> = const { Cell::new(None) };
    static ACTIVE_MOTION_LEVEL: Cell<u8> = const { Cell::new(0) };
}

const MOTION_CUE_DURATION: u8 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatusTone {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    FullMap,
    FocusBranch,
    SubtreeOnly,
    FilteredFocus,
}

impl ViewMode {
    fn label(self) -> &'static str {
        match self {
            Self::FullMap => "Full Map",
            Self::FocusBranch => "Focus Branch",
            Self::SubtreeOnly => "Subtree Only",
            Self::FilteredFocus => "Filtered Focus",
        }
    }

    fn status_label(self) -> &'static str {
        match self {
            Self::FullMap => "full map",
            Self::FocusBranch => "focus branch",
            Self::SubtreeOnly => "subtree only",
            Self::FilteredFocus => "filtered focus",
        }
    }

    fn next(self, has_filter: bool) -> Self {
        match (self, has_filter) {
            (Self::FullMap, _) => Self::FocusBranch,
            (Self::FocusBranch, _) => Self::SubtreeOnly,
            (Self::SubtreeOnly, true) => Self::FilteredFocus,
            (Self::SubtreeOnly, false) => Self::FullMap,
            (Self::FilteredFocus, _) => Self::FullMap,
        }
    }

    fn previous(self, has_filter: bool) -> Self {
        match (self, has_filter) {
            (Self::FullMap, true) => Self::FilteredFocus,
            (Self::FullMap, false) => Self::SubtreeOnly,
            (Self::FocusBranch, _) => Self::FullMap,
            (Self::SubtreeOnly, _) => Self::FocusBranch,
            (Self::FilteredFocus, _) => Self::SubtreeOnly,
        }
    }
}

impl From<ViewMode> for CheckpointViewMode {
    fn from(value: ViewMode) -> Self {
        match value {
            ViewMode::FullMap => Self::FullMap,
            ViewMode::FocusBranch => Self::FocusBranch,
            ViewMode::SubtreeOnly => Self::SubtreeOnly,
            ViewMode::FilteredFocus => Self::FilteredFocus,
        }
    }
}

impl From<CheckpointViewMode> for ViewMode {
    fn from(value: CheckpointViewMode) -> Self {
        match value {
            CheckpointViewMode::FullMap => Self::FullMap,
            CheckpointViewMode::FocusBranch => Self::FocusBranch,
            CheckpointViewMode::SubtreeOnly => Self::SubtreeOnly,
            CheckpointViewMode::FilteredFocus => Self::FilteredFocus,
        }
    }
}

#[derive(Debug, Clone)]
struct StatusMessage {
    tone: StatusTone,
    text: String,
}

#[derive(Debug, Clone)]
struct StatusModel {
    tone: StatusTone,
    message: String,
    focus_label: String,
    focus_id: Option<String>,
    filter_summary: Option<String>,
    scope_label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MotionTarget {
    Focus,
    FilterResult,
    Scope,
    PaletteInput,
    HelpInput,
    SearchActive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MotionCue {
    target: MotionTarget,
    ticks_remaining: u8,
}

impl MotionCue {
    fn new(target: MotionTarget) -> Self {
        Self {
            target,
            ticks_remaining: MOTION_CUE_DURATION,
        }
    }

    fn tick(self) -> Option<Self> {
        if self.ticks_remaining <= 1 {
            None
        } else {
            Some(Self {
                target: self.target,
                ticks_remaining: self.ticks_remaining - 1,
            })
        }
    }

    fn emphasis_level(self) -> u8 {
        match self.ticks_remaining {
            4..=u8::MAX => 3,
            2..=3 => 2,
            1 => 1,
            _ => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptMode {
    AddChild,
    AddSibling,
    AddRoot,
    Edit,
    SaveView,
    SaveCheckpoint,
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
            Self::SaveCheckpoint => "Save Checkpoint",
            Self::OpenId => "Jump To Id",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Self::SaveView => "Give the current active filter a short name.",
            Self::SaveCheckpoint => "Name a local snapshot of the current map state.",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaletteItemKind {
    Action,
    Setting,
    Node,
    SavedView,
    History,
    Checkpoint,
    Safety,
    Help,
    Theme,
}

impl PaletteItemKind {
    fn label(self) -> &'static str {
        match self {
            Self::Action => "Action",
            Self::Setting => "Setting",
            Self::Node => "Node",
            Self::SavedView => "View",
            Self::History => "History",
            Self::Checkpoint => "Checkpoint",
            Self::Safety => "Safety",
            Self::Help => "Help",
            Self::Theme => "Theme",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HelpTopic {
    Navigation,
    Editing,
    Search,
    Views,
    Themes,
    Mindmap,
    Syntax,
}

impl HelpTopic {
    fn title(self) -> &'static str {
        match self {
            Self::Navigation => "Navigation",
            Self::Editing => "Editing",
            Self::Search => "Search And Filters",
            Self::Views => "View Modes",
            Self::Themes => "Themes",
            Self::Mindmap => "Visual Mindmap",
            Self::Syntax => "Inline Syntax",
        }
    }

    fn summary(self) -> &'static str {
        match self {
            Self::Navigation => "Move through the tree, jump quickly, and open major overlays.",
            Self::Editing => "Add, rename, delete, and reshape branches without leaving the map.",
            Self::Search => {
                "Filter by text, tags, metadata, and saved views from one search surface."
            }
            Self::Views => "Switch between full-map and focused working modes.",
            Self::Themes => "Change the visual surface without leaving the map.",
            Self::Mindmap => "Inspect the current working set visually and export it as a PNG.",
            Self::Syntax => "Write labels, tags, metadata, and deep-link ids inline on one node.",
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Self::Navigation => "Move through the visible tree, jump, and open overlays.",
            Self::Editing => "Add, edit, delete, and reorder branches from the keyboard.",
            Self::Search => "Search by content, browse facets, and reopen saved views.",
            Self::Views => "Cycle between full-map and branch-focused working modes.",
            Self::Themes => "Open the palette, preview surface changes live, and commit them.",
            Self::Mindmap => "Open the visual map, pan it, and export a PNG.",
            Self::Syntax => "Use tags, metadata, and deep-link ids inline on the same node.",
        }
    }

    fn keywords(self) -> &'static str {
        match self {
            Self::Navigation => "navigate movement arrows focus jump root open id palette hotkeys",
            Self::Editing => {
                "edit add delete reshape move indent outdent sibling child root write undo redo checkpoint history"
            }
            Self::Search => {
                "search filter query facets saved views tags metadata matches key keys value values"
            }
            Self::Views => "views focus branch subtree filtered focus isolate branch presentation",
            Self::Themes => {
                "theme themes paper blueprint calm terminal neon workbench palette ui settings motion ascii accents"
            }
            Self::Mindmap => "mindmap visual bubble canvas png export pan recenter map overlay",
            Self::Syntax => {
                "syntax tags metadata key keys value values ids deep links format example inline node"
            }
        }
    }

    fn body(self) -> &'static [&'static str] {
        match self {
            Self::Navigation => &[
                "↑ / ↓ move through visible nodes",
                "← collapses a branch or moves to the parent",
                "→ expands a branch or enters the first child",
                "Enter or Space toggles expanded and collapsed state",
                "g jumps to the map root, or the subtree root in Subtree Only",
                ": or Ctrl+P opens the command palette",
                "o jumps directly to a node id",
                "m opens the visual mindmap overlay",
            ],
            Self::Editing => &[
                "a adds a child under the current node",
                "A adds a sibling next to the current node",
                "Shift+R adds a root-level branch",
                "e edits the selected node inline",
                "x deletes after a second confirmation press",
                "u undoes the last structural change",
                "U redoes the last undone change",
                "Alt+↑ and Alt+↓ reorder a node among siblings",
                "Alt+← moves the node out one level",
                "Alt+→ indents the node into the previous sibling",
                "The palette can create manual checkpoints and restore named snapshots",
                "Large structural edits also capture automatic safety checkpoints",
                "The palette can browse recent actions and jump back through history",
            ],
            Self::Search => &[
                "/ opens query search",
                "f opens facets for tags and metadata",
                "F opens saved views",
                "Tab switches Query, Facets, and Saved Views",
                "Enter applies the current query or selection",
                "Applying a query, facet, or saved view lands on the first matching node",
                "n and N move between matches in the tree",
                "c clears the active filter",
                "Search supports plain text, #tag, and @key:value",
            ],
            Self::Views => &[
                "v and V cycle Full Map, Focus Branch, Subtree Only, and Filtered Focus",
                "Focus Branch keeps ancestors and sibling context visible",
                "Subtree Only isolates the current branch as a rooted workspace",
                "In Subtree Only, g returns to the subtree root",
                "In Subtree Only, ← never climbs above the subtree root",
                "Filtered Focus blends filter results with local context",
                "The visual mindmap follows the current view mode too",
            ],
            Self::Themes => &[
                ": or Ctrl+P opens the command palette",
                "Type 'theme' or a theme name like 'paper' or 'blueprint'",
                "Selecting a theme previews it immediately inside the palette",
                "Enter applies the selected theme and persists it locally",
                "Esc closes the palette and restores the previous theme or surface settings",
                "Search 'motion' to toggle attention-guiding focus, filter, and input motion",
                "Search 'ascii' to toggle terminal-style separators and title marks",
                "Themes change the header, outline, overlays, status line, and mindmap surface",
                "The selected theme lives in a local UI settings sidecar next to the map",
            ],
            Self::Mindmap => &[
                "m opens the visual mindmap overlay",
                "The map recenters on the focused node",
                "Arrow keys pan the canvas",
                "0 recenters the camera",
                "p exports a PNG from the current visual map",
                "The mindmap respects the active view mode and filter scope",
            ],
            Self::Syntax => &[
                "#tag adds a topic or workflow marker",
                "@key:value adds structured metadata",
                "[id:path/to/node] adds a deep-link target id",
                "Combine label, tags, metadata, and id on one line",
                "",
                "Example:",
                "API Design #backend @status:todo [id:product/api-design]",
            ],
        }
    }

    fn search_text(self) -> String {
        format!(
            "{} {} {} {} {}",
            self.title(),
            self.summary(),
            self.hint(),
            self.keywords(),
            self.body().join(" ")
        )
    }
}

#[derive(Debug, Clone)]
enum PaletteAction {
    Undo,
    Redo,
    AddChild,
    AddSibling,
    AddRoot,
    EditNode,
    JumpToId,
    JumpToRoot,
    OpenSearch,
    OpenFacets,
    OpenSavedViews,
    OpenMindmap,
    SaveNow,
    ToggleAutosave,
    RevertFromDisk,
    CreateCheckpoint,
    RestoreLatestCheckpoint,
    ClearFilter,
    CycleViewMode,
    ShowHelp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SurfaceSetting {
    Motion(bool),
    AsciiAccents(bool),
}

impl SurfaceSetting {
    fn label(self) -> &'static str {
        match self {
            Self::Motion(true) => "Motion: On",
            Self::Motion(false) => "Motion: Off",
            Self::AsciiAccents(true) => "ASCII Accents: On",
            Self::AsciiAccents(false) => "ASCII Accents: Off",
        }
    }

    fn subtitle(self) -> &'static str {
        match self {
            Self::Motion(true) => {
                "Enable attention-guiding motion for focus, filter results, scope, and active inputs"
            }
            Self::Motion(false) => "Disable attention-guiding motion and keep the interface static",
            Self::AsciiAccents(true) => "Turn on ASCII separators and small terminal-style marks",
            Self::AsciiAccents(false) => {
                "Turn off ASCII separators and return to the default chrome"
            }
        }
    }

    fn preview(self) -> &'static str {
        match self {
            Self::Motion(true) => {
                "Preview attention-guiding motion. Enter commits it; Esc restores the previous motion preference."
            }
            Self::Motion(false) => {
                "Preview a fully static surface. Enter commits it; Esc restores the previous motion preference."
            }
            Self::AsciiAccents(true) => {
                "Preview ASCII-flavored titles, separators, and help chrome. Enter commits it; Esc restores the previous surface."
            }
            Self::AsciiAccents(false) => {
                "Preview the quieter default chrome without ASCII marks. Enter commits it; Esc restores the previous surface."
            }
        }
    }

    fn keywords(self) -> &'static str {
        match self {
            Self::Motion(true) => "motion on animate guidance focus filter scope input",
            Self::Motion(false) => "motion off static reduce disable animation guidance",
            Self::AsciiAccents(true) => "ascii accents on terminal art separators chrome",
            Self::AsciiAccents(false) => "ascii accents off terminal art separators chrome default",
        }
    }
}

#[derive(Debug, Clone)]
enum PaletteTarget {
    Action(PaletteAction),
    Setting(SurfaceSetting),
    NodePath(Vec<usize>),
    SavedView { name: String, query: String },
    UndoSteps(usize),
    RedoSteps(usize),
    Checkpoint(usize),
    HelpTopic(HelpTopic),
    Theme(ThemeId),
}

impl PaletteTarget {
    fn is_previewable(&self) -> bool {
        matches!(self, Self::Theme(_) | Self::Setting(_))
    }

    fn preview_ui_settings(&self, baseline: &UiSettings) -> Option<UiSettings> {
        let mut preview = baseline.clone();
        match self {
            Self::Theme(theme) => {
                preview.theme = *theme;
                Some(preview)
            }
            Self::Setting(SurfaceSetting::Motion(enabled)) => {
                preview.motion_enabled = *enabled;
                Some(preview)
            }
            Self::Setting(SurfaceSetting::AsciiAccents(enabled)) => {
                preview.ascii_accents = *enabled;
                Some(preview)
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct PaletteItem {
    kind: PaletteItemKind,
    title: String,
    subtitle: String,
    preview: String,
    score: i64,
    target: PaletteTarget,
}

#[derive(Debug, Clone)]
struct PaletteState {
    query: String,
    cursor: usize,
    selected: usize,
}

impl PaletteState {
    fn new() -> Self {
        Self {
            query: String::new(),
            cursor: 0,
            selected: 0,
        }
    }

    fn move_left(&mut self) {
        self.cursor = previous_boundary(&self.query, self.cursor);
    }

    fn move_right(&mut self) {
        self.cursor = next_boundary(&self.query, self.cursor);
    }

    fn insert(&mut self, character: char) {
        self.query.insert(self.cursor, character);
        self.cursor += character.len_utf8();
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let previous = previous_boundary(&self.query, self.cursor);
        self.query.replace_range(previous..self.cursor, "");
        self.cursor = previous;
    }

    fn delete(&mut self) {
        if self.cursor >= self.query.len() {
            return;
        }
        let next = next_boundary(&self.query, self.cursor);
        self.query.replace_range(self.cursor..next, "");
    }
}

#[derive(Debug, Clone)]
struct HelpOverlayState {
    query: String,
    cursor: usize,
    selected: usize,
}

impl HelpOverlayState {
    fn new(topic: Option<HelpTopic>) -> Self {
        let query = topic
            .map(|topic| topic.title().to_string())
            .unwrap_or_default();
        let cursor = query.len();
        Self {
            query,
            cursor,
            selected: 0,
        }
    }

    fn move_left(&mut self) {
        self.cursor = previous_boundary(&self.query, self.cursor);
    }

    fn move_right(&mut self) {
        self.cursor = next_boundary(&self.query, self.cursor);
    }

    fn insert(&mut self, character: char) {
        self.query.insert(self.cursor, character);
        self.cursor += character.len_utf8();
    }

    fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let previous = previous_boundary(&self.query, self.cursor);
        self.query.replace_range(previous..self.cursor, "");
        self.cursor = previous;
    }

    fn delete(&mut self) {
        if self.cursor >= self.query.len() {
            return;
        }
        let next = next_boundary(&self.query, self.cursor);
        self.query.replace_range(self.cursor..next, "");
    }
}

#[derive(Debug, Clone)]
struct PromptState {
    mode: PromptMode,
    value: String,
    cursor: usize,
}

#[derive(Debug, Clone, Default)]
struct MindmapOverlayState {
    pan_x: i32,
    pan_y: i32,
}

impl MindmapOverlayState {
    fn pan(&mut self, delta_x: i32, delta_y: i32) {
        self.pan_x += delta_x;
        self.pan_y += delta_y;
    }

    fn recenter(&mut self) {
        self.pan_x = 0;
        self.pan_y = 0;
    }
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
    dimmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PathAnchor {
    path: Vec<usize>,
    id: Option<String>,
}

#[derive(Debug, Clone)]
struct WorkspaceSnapshot {
    editor: EditorState,
    expanded: HashSet<Vec<usize>>,
    view_mode: ViewMode,
    subtree_root: Option<PathAnchor>,
    filter_query: Option<String>,
}

#[derive(Debug, Clone)]
struct HistoryEntry {
    label: String,
    snapshot: WorkspaceSnapshot,
}

#[derive(Debug, Clone, Copy)]
struct ViewProjection<'a> {
    filter: Option<&'a ActiveFilter>,
    filter_visible_paths: Option<&'a HashSet<Vec<usize>>>,
    focus_path: &'a [usize],
    view_mode: ViewMode,
}

#[derive(Debug)]
struct TuiApp {
    map_path: PathBuf,
    editor: Editor,
    expanded: HashSet<Vec<usize>>,
    view_mode: ViewMode,
    subtree_root: Option<PathAnchor>,
    status: StatusMessage,
    prompt: Option<PromptState>,
    palette: Option<PaletteState>,
    filter: Option<ActiveFilter>,
    search: Option<SearchOverlayState>,
    saved_views: SavedViewsState,
    checkpoints: CheckpointsState,
    mindmap: Option<MindmapOverlayState>,
    help: Option<HelpOverlayState>,
    palette_preview_base: Option<UiSettings>,
    quit_armed: bool,
    delete_armed: bool,
    autosave: bool,
    motion_cue: Option<MotionCue>,
    ui_settings: UiSettings,
    undo_history: Vec<HistoryEntry>,
    redo_history: Vec<HistoryEntry>,
}

impl TuiApp {
    #[cfg_attr(not(test), allow(dead_code))]
    fn new(
        map_path: PathBuf,
        document: Document,
        focus_path: Vec<usize>,
        warning: Option<String>,
        autosave: bool,
        saved_views: SavedViewsState,
    ) -> Self {
        Self::new_with_settings(
            map_path,
            document,
            focus_path,
            warning,
            autosave,
            saved_views,
            UiSettings::default(),
        )
    }

    fn new_with_settings(
        map_path: PathBuf,
        document: Document,
        focus_path: Vec<usize>,
        warning: Option<String>,
        autosave: bool,
        saved_views: SavedViewsState,
        ui_settings: UiSettings,
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
                text: "Arrows move. : opens palette. ? opens help. v changes view. a adds."
                    .to_string(),
            },
        };

        Self {
            map_path,
            editor: Editor::new(document, focus_path),
            expanded,
            view_mode: ViewMode::FullMap,
            subtree_root: None,
            status,
            prompt: None,
            palette: None,
            filter: None,
            search: None,
            saved_views,
            checkpoints: CheckpointsState::default(),
            mindmap: None,
            help: None,
            palette_preview_base: None,
            quit_armed: false,
            delete_armed: false,
            autosave,
            motion_cue: None,
            ui_settings,
            undo_history: Vec::new(),
            redo_history: Vec::new(),
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

        if self.palette.is_some() {
            self.quit_armed = false;
            self.delete_armed = false;
            return self.handle_palette_key(key);
        }

        if self.help.is_some() {
            self.quit_armed = false;
            self.delete_armed = false;
            return self.handle_help_key(key);
        }

        if self.search.is_some() {
            self.quit_armed = false;
            self.delete_armed = false;
            return self.handle_search_key(key);
        }

        if self.mindmap.is_some() {
            self.quit_armed = false;
            self.delete_armed = false;
            return self.handle_mindmap_key(key);
        }

        if key.modifiers.contains(KeyModifiers::ALT) {
            self.quit_armed = false;
            self.delete_armed = false;
            match key.code {
                KeyCode::Up => self.apply_edit(
                    |editor| editor.move_node_up(),
                    "Moved the node up.",
                    Some(self.automatic_checkpoint_label("swap")),
                )?,
                KeyCode::Down => self.apply_edit(
                    |editor| editor.move_node_down(),
                    "Moved the node down.",
                    Some(self.automatic_checkpoint_label("swap")),
                )?,
                KeyCode::Left => self.apply_edit(
                    |editor| editor.outdent_node(),
                    "Moved the node out one level.",
                    Some(self.automatic_checkpoint_label("reparent")),
                )?,
                KeyCode::Right => self.apply_edit(
                    |editor| editor.indent_node(),
                    "Moved the node into the previous sibling.",
                    Some(self.automatic_checkpoint_label("reparent")),
                )?,
                _ => {}
            }
            return Ok(true);
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('p') {
            self.delete_armed = false;
            self.open_palette();
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
                self.open_help(None);
                self.quit_armed = false;
                self.delete_armed = false;
            }
            KeyCode::Char(':') => {
                self.delete_armed = false;
                self.open_palette();
            }
            KeyCode::Char('f') => {
                self.delete_armed = false;
                self.open_search_overlay(SearchSection::Facets);
            }
            KeyCode::Char('F') => {
                self.delete_armed = false;
                self.open_search_overlay(SearchSection::Views);
            }
            KeyCode::Char('m') => {
                self.delete_armed = false;
                self.mindmap = Some(MindmapOverlayState::default());
                let scene = self.current_mindmap_scene();
                self.set_status(
                    StatusTone::Info,
                    format!(
                        "Mindmap open. {}. Arrow keys pan, 0 recenters, p exports PNG.",
                        scene.describe()
                    ),
                );
            }
            KeyCode::Char('v') => {
                self.delete_armed = false;
                self.cycle_view_mode(true);
            }
            KeyCode::Char('V') => {
                self.delete_armed = false;
                self.cycle_view_mode(false);
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
            KeyCode::Char('u') => {
                self.delete_armed = false;
                self.undo()?;
            }
            KeyCode::Char('U') => {
                self.delete_armed = false;
                self.redo()?;
            }
            KeyCode::Char('g') => {
                if self.view_mode == ViewMode::SubtreeOnly {
                    if let Some(path) = self.subtree_root_path() {
                        self.editor.set_focus_path(path)?;
                        self.expand_focus_chain();
                        self.persist_session()?;
                        self.trigger_motion(MotionTarget::Focus);
                        self.delete_armed = false;
                        self.set_status(StatusTone::Info, "Returned to the subtree root.");
                    } else {
                        self.set_status(StatusTone::Warning, "No subtree root is available.");
                    }
                } else {
                    self.editor.move_root()?;
                    self.expand_focus_chain();
                    self.persist_session()?;
                    self.trigger_motion(MotionTarget::Focus);
                    self.delete_armed = false;
                    self.set_status(StatusTone::Info, "Jumped to the root.");
                }
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
                        Some(self.automatic_checkpoint_label("delete")),
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
                if self.view_mode != ViewMode::FullMap {
                    self.set_view_mode(ViewMode::FullMap);
                    self.set_status(StatusTone::Info, "Returned to the full map view.");
                } else if self.clear_filter() {
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

    fn handle_mindmap_key(&mut self, key: KeyEvent) -> Result<bool, AppError> {
        let Some(mut overlay) = self.mindmap.take() else {
            return Ok(true);
        };

        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('m') => {
                self.set_status(StatusTone::Info, "Closed the visual mindmap.");
                return Ok(true);
            }
            KeyCode::Char('0') => {
                overlay.recenter();
                self.set_status(StatusTone::Info, "Recentered the visual mindmap.");
            }
            KeyCode::Up | KeyCode::Char('k') => overlay.pan(0, -3),
            KeyCode::Down | KeyCode::Char('j') => overlay.pan(0, 3),
            KeyCode::Left | KeyCode::Char('h') => overlay.pan(-6, 0),
            KeyCode::Right | KeyCode::Char('l') => overlay.pan(6, 0),
            KeyCode::Char('p') => {
                self.export_current_mindmap_png(&overlay)?;
                return Ok(true);
            }
            _ => {}
        }

        self.mindmap = Some(overlay);
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

    fn handle_palette_key(&mut self, key: KeyEvent) -> Result<bool, AppError> {
        let Some(mut palette) = self.palette.take() else {
            return Ok(true);
        };

        match key.code {
            KeyCode::Esc => {
                self.close_palette(false)?;
                self.set_status(StatusTone::Info, "Closed the command palette.");
                return Ok(true);
            }
            KeyCode::BackTab => {
                let items = self.palette_items(&palette.query);
                palette.selected = previous_palette_group_index(&items, palette.selected);
            }
            KeyCode::Tab => {
                let items = self.palette_items(&palette.query);
                palette.selected = next_palette_group_index(&items, palette.selected);
            }
            KeyCode::Up => {
                palette.selected = palette.selected.saturating_sub(1);
            }
            KeyCode::Down => {
                let len = self.palette_items(&palette.query).len();
                if len > 0 {
                    palette.selected = (palette.selected + 1).min(len - 1);
                }
            }
            KeyCode::Enter => {
                let items = self.palette_items(&palette.query);
                if let Some(item) = items.get(palette.selected).cloned() {
                    if item.target.is_previewable() {
                        self.commit_preview_target(item.target)?;
                    } else {
                        self.close_palette(false)?;
                        self.execute_palette_target(item.target)?;
                    }
                    return Ok(true);
                }
                self.set_status(
                    StatusTone::Warning,
                    "No palette result is selected yet. Type to search actions and nodes.",
                );
                return Ok(true);
            }
            KeyCode::Backspace => palette.backspace(),
            KeyCode::Delete => palette.delete(),
            KeyCode::Left => palette.move_left(),
            KeyCode::Right => palette.move_right(),
            KeyCode::Home => palette.cursor = 0,
            KeyCode::End => palette.cursor = palette.query.len(),
            KeyCode::Char(character)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                palette.insert(character);
            }
            _ => {}
        }

        let len = self.palette_items(&palette.query).len();
        if len == 0 {
            palette.selected = 0;
        } else {
            palette.selected = palette.selected.min(len - 1);
        }
        self.sync_palette_preview(&palette);
        self.palette = Some(palette);
        Ok(true)
    }

    fn handle_help_key(&mut self, key: KeyEvent) -> Result<bool, AppError> {
        let Some(mut help) = self.help.take() else {
            return Ok(true);
        };

        match key.code {
            KeyCode::Esc | KeyCode::Char('?') => {
                self.set_status(StatusTone::Info, "Closed searchable help.");
                return Ok(true);
            }
            KeyCode::Up => {
                help.selected = help.selected.saturating_sub(1);
            }
            KeyCode::Down => {
                let len = self.help_topics(&help.query).len();
                if len > 0 {
                    help.selected = (help.selected + 1).min(len - 1);
                }
            }
            KeyCode::Backspace => help.backspace(),
            KeyCode::Delete => help.delete(),
            KeyCode::Left => help.move_left(),
            KeyCode::Right => help.move_right(),
            KeyCode::Home => help.cursor = 0,
            KeyCode::End => help.cursor = help.query.len(),
            KeyCode::Enter => {
                if let Some(topic) = self.help_topics(&help.query).get(help.selected).copied() {
                    self.set_status(
                        StatusTone::Info,
                        format!("Showing help for {}.", topic.title()),
                    );
                }
            }
            KeyCode::Char(character)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                help.insert(character);
            }
            _ => {}
        }

        let len = self.help_topics(&help.query).len();
        if len == 0 {
            help.selected = 0;
        } else {
            help.selected = help.selected.min(len - 1);
        }
        self.help = Some(help);
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
                    self.apply_search_facet(&item.label, &search.draft_query)?;
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
                    self.open_saved_view(&view.name, &view.query)?;
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
            PromptMode::AddChild => self.apply_edit(
                |editor| editor.add_child(value),
                "Added a child node.",
                None,
            ),
            PromptMode::AddSibling => self.apply_edit(
                |editor| editor.add_sibling(value),
                "Added a sibling node.",
                None,
            ),
            PromptMode::AddRoot => self.apply_edit(
                |editor| editor.add_root(value),
                "Added a new root node.",
                None,
            ),
            PromptMode::Edit => self.apply_edit(
                |editor| editor.edit_current(value),
                "Updated the selected node.",
                None,
            ),
            PromptMode::SaveView => {
                self.save_current_search_as(value)?;
                Ok(())
            }
            PromptMode::SaveCheckpoint => {
                self.save_checkpoint(value)?;
                Ok(())
            }
            PromptMode::OpenId => {
                self.editor.open_id(value)?;
                self.expand_focus_chain();
                self.persist_session()?;
                self.quit_armed = false;
                self.set_status(StatusTone::Success, "Jumped to the requested id.");
                Ok(())
            }
        }
    }

    fn visible_rows(&self) -> Vec<VisibleRow> {
        let mut rows = Vec::new();
        let filter_visible_paths = self.filter.as_ref().map(filter_visible_paths);
        let projection_focus_path = self.projection_focus_path();
        let projection = ViewProjection {
            filter: self.filter.as_ref(),
            filter_visible_paths: filter_visible_paths.as_ref(),
            focus_path: &projection_focus_path,
            view_mode: self.view_mode,
        };
        collect_visible_rows(
            &self.editor.document().nodes,
            &self.expanded,
            projection,
            &mut rows,
            Vec::new(),
        );
        rows
    }

    fn subtree_root_path(&self) -> Option<Vec<usize>> {
        let anchor = self.subtree_root.as_ref()?;
        if let Some(id) = &anchor.id
            && let Some(path) = find_path_by_id(&self.editor.document().nodes, id)
        {
            return Some(path);
        }
        if get_node(&self.editor.document().nodes, &anchor.path).is_some() {
            return Some(anchor.path.clone());
        }
        self.editor
            .current()
            .map(|_| self.editor.focus_path().to_vec())
    }

    fn projection_focus_path(&self) -> Vec<usize> {
        if self.view_mode == ViewMode::SubtreeOnly {
            self.subtree_root_path()
                .unwrap_or_else(|| self.editor.focus_path().to_vec())
        } else {
            self.editor.focus_path().to_vec()
        }
    }

    fn subtree_root_node(&self) -> Option<&Node> {
        let path = self.subtree_root_path()?;
        get_node(&self.editor.document().nodes, &path)
    }

    fn set_view_mode(&mut self, next: ViewMode) {
        self.view_mode = next;
        if next == ViewMode::SubtreeOnly {
            self.subtree_root = self.editor.current().map(|node| PathAnchor {
                path: self.editor.focus_path().to_vec(),
                id: node.id.clone(),
            });
        } else {
            self.subtree_root = None;
        }
        self.trigger_motion(MotionTarget::Scope);
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
        self.trigger_motion(MotionTarget::Focus);
        if let Some(node) = self.editor.current() {
            self.set_status(StatusTone::Info, format!("Focused '{}'.", node.text));
        }
        Ok(())
    }

    fn collapse_or_parent(&mut self) -> Result<(), AppError> {
        let path = self.editor.focus_path().to_vec();
        let subtree_root = self.subtree_root_path();
        if let Some(node) = self.editor.current()
            && !node.children.is_empty()
            && self.expanded.contains(&path)
        {
            self.expanded.remove(&path);
            self.set_status(StatusTone::Info, "Collapsed the branch.");
            return Ok(());
        }

        if self.view_mode == ViewMode::SubtreeOnly
            && subtree_root.as_deref() == Some(path.as_slice())
        {
            self.set_status(
                StatusTone::Info,
                "Already at the subtree root. Press Esc to leave the isolated branch.",
            );
            return Ok(());
        }

        self.editor.move_parent()?;
        self.persist_session()?;
        self.trigger_motion(MotionTarget::Focus);
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
        self.trigger_motion(MotionTarget::Focus);
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

    fn open_palette(&mut self) {
        self.quit_armed = false;
        self.delete_armed = false;
        self.palette_preview_base = Some(self.ui_settings.clone());
        self.palette = Some(PaletteState::new());
        self.trigger_motion(MotionTarget::PaletteInput);
        self.set_status(
            StatusTone::Info,
            "Palette open. Type to search actions, recent history, manual checkpoints, safety snapshots, themes, settings, nodes, saved views, and help.",
        );
    }

    fn open_help(&mut self, topic: Option<HelpTopic>) {
        self.quit_armed = false;
        self.delete_armed = false;
        self.help = Some(HelpOverlayState::new(topic));
        self.trigger_motion(MotionTarget::HelpInput);
        let message = match topic {
            Some(topic) => format!("Help open on {}. Type to refine topics.", topic.title()),
            None => "Help open. Type to search topics and shortcuts.".to_string(),
        };
        self.set_status(StatusTone::Info, message);
    }

    fn cycle_view_mode(&mut self, forward: bool) {
        let has_filter = self.filter.is_some();
        let next = if forward {
            self.view_mode.next(has_filter)
        } else {
            self.view_mode.previous(has_filter)
        };
        self.set_view_mode(next);
        self.set_status(
            StatusTone::Info,
            format!("View mode: {}.", self.view_mode.status_label()),
        );
    }

    fn execute_palette_target(&mut self, target: PaletteTarget) -> Result<(), AppError> {
        match target {
            PaletteTarget::Action(action) => match action {
                PaletteAction::Undo => self.undo()?,
                PaletteAction::Redo => self.redo()?,
                PaletteAction::AddChild => self.begin_prompt(PromptMode::AddChild, String::new()),
                PaletteAction::AddSibling => {
                    self.begin_prompt(PromptMode::AddSibling, String::new())
                }
                PaletteAction::AddRoot => self.begin_prompt(PromptMode::AddRoot, String::new()),
                PaletteAction::EditNode => {
                    let initial = self
                        .editor
                        .current()
                        .map(Node::display_line)
                        .unwrap_or_default();
                    self.begin_prompt(PromptMode::Edit, initial);
                }
                PaletteAction::JumpToId => self.begin_prompt(PromptMode::OpenId, String::new()),
                PaletteAction::JumpToRoot => {
                    if self.view_mode == ViewMode::SubtreeOnly {
                        if let Some(path) = self.subtree_root_path() {
                            self.editor.set_focus_path(path)?;
                            self.expand_focus_chain();
                            self.persist_session()?;
                            self.trigger_motion(MotionTarget::Focus);
                            self.set_status(StatusTone::Info, "Returned to the subtree root.");
                        }
                    } else {
                        self.editor.move_root()?;
                        self.expand_focus_chain();
                        self.persist_session()?;
                        self.trigger_motion(MotionTarget::Focus);
                        self.set_status(StatusTone::Info, "Jumped to the root.");
                    }
                }
                PaletteAction::OpenSearch => self.open_search_overlay(SearchSection::Query),
                PaletteAction::OpenFacets => self.open_search_overlay(SearchSection::Facets),
                PaletteAction::OpenSavedViews => self.open_search_overlay(SearchSection::Views),
                PaletteAction::OpenMindmap => {
                    self.mindmap = Some(MindmapOverlayState::default());
                    let scene = self.current_mindmap_scene();
                    self.set_status(
                        StatusTone::Info,
                        format!(
                            "Mindmap open. {}. Arrow keys pan, 0 recenters, p exports PNG.",
                            scene.describe()
                        ),
                    );
                }
                PaletteAction::SaveNow => {
                    self.save_to_disk()?;
                }
                PaletteAction::ToggleAutosave => {
                    self.autosave = !self.autosave;
                    let message = if self.autosave {
                        "Autosave enabled. Changes now write to disk immediately."
                    } else {
                        "Autosave disabled. Press s to save changes manually."
                    };
                    self.set_status(StatusTone::Info, message);
                }
                PaletteAction::RevertFromDisk => {
                    self.revert_from_disk()?;
                }
                PaletteAction::CreateCheckpoint => {
                    self.begin_prompt(PromptMode::SaveCheckpoint, self.suggest_checkpoint_name());
                }
                PaletteAction::RestoreLatestCheckpoint => {
                    self.restore_latest_checkpoint()?;
                }
                PaletteAction::ClearFilter => {
                    if self.clear_filter() {
                        self.set_status(StatusTone::Info, "Cleared the active filter.");
                    } else {
                        self.set_status(StatusTone::Info, "No active filter to clear.");
                    }
                }
                PaletteAction::CycleViewMode => {
                    self.cycle_view_mode(true);
                }
                PaletteAction::ShowHelp => {
                    self.open_help(None);
                }
            },
            PaletteTarget::Setting(setting) => {
                self.apply_surface_setting(setting)?;
            }
            PaletteTarget::NodePath(path) => {
                self.editor.set_focus_path(path)?;
                self.expand_focus_chain();
                self.persist_session()?;
                self.trigger_motion(MotionTarget::Focus);
                if let Some(node) = self.editor.current() {
                    self.set_status(StatusTone::Success, format!("Jumped to '{}'.", node.text));
                }
            }
            PaletteTarget::SavedView { name, query } => {
                self.open_saved_view(&name, &query)?;
            }
            PaletteTarget::UndoSteps(steps) => {
                self.undo_steps(steps)?;
            }
            PaletteTarget::RedoSteps(steps) => {
                self.redo_steps(steps)?;
            }
            PaletteTarget::Checkpoint(index) => {
                self.restore_checkpoint(index)?;
            }
            PaletteTarget::HelpTopic(topic) => {
                self.open_help(Some(topic));
            }
            PaletteTarget::Theme(theme) => {
                self.set_theme(theme)?;
            }
        }

        Ok(())
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
        self.trigger_motion(MotionTarget::SearchActive);
        self.set_status(
            StatusTone::Info,
            "Search open. Tab switches sections. Enter applies the current selection.",
        );
    }

    fn current_mindmap_scene(&self) -> MindmapScene {
        let visible_paths = self
            .visible_rows()
            .into_iter()
            .map(|row| row.path)
            .collect::<HashSet<_>>();
        MindmapScene::build(
            self.editor.document(),
            self.editor.focus_path(),
            &self.expanded,
            self.filter.as_ref().map(|filter| filter.matches.as_slice()),
            Some(&visible_paths),
        )
    }

    fn mindmap_theme(&self) -> MindmapTheme {
        self.theme_colors()
    }

    fn export_current_mindmap_png(
        &mut self,
        overlay: &MindmapOverlayState,
    ) -> Result<(), AppError> {
        let (terminal_width, terminal_height) = size()
            .map_err(|error| AppError::new(format!("Could not measure the terminal: {error}")))?;
        let frame_area = Rect::new(0, 0, terminal_width, terminal_height);
        let overlay_area = centered_rect(92, 88, frame_area);
        let inner = overlay_area.inner(Margin::new(1, 1));
        let scene = self.current_mindmap_scene();
        let camera = scene.camera(
            inner.width.max(1),
            inner.height.max(1),
            overlay.pan_x,
            overlay.pan_y,
        );
        let path = default_export_path(&self.map_path);
        let exported =
            export_png(&scene, camera, self.mindmap_theme(), &path).map_err(AppError::new)?;
        self.set_status(
            StatusTone::Success,
            format!("Exported the visual mindmap to '{}'.", exported.display()),
        );
        self.mindmap = Some(overlay.clone());
        Ok(())
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

    fn palette_items(&self, raw: &str) -> Vec<PaletteItem> {
        let query = raw.trim().to_lowercase();
        let mut items = Vec::new();
        items.extend(self.palette_action_items(&query));
        items.extend(self.palette_theme_items(&query));
        items.extend(self.palette_setting_items(&query));
        items.extend(self.palette_saved_view_items(&query));
        items.extend(self.palette_history_items(&query));
        items.extend(self.palette_checkpoint_items(&query));
        items.extend(self.palette_help_items(&query));
        if !query.is_empty() {
            items.extend(self.palette_node_items(&query));
        }
        items.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| palette_kind_rank(left.kind).cmp(&palette_kind_rank(right.kind)))
                .then_with(|| left.title.to_lowercase().cmp(&right.title.to_lowercase()))
        });
        items.truncate(16);
        items
    }

    fn palette_action_items(&self, query: &str) -> Vec<PaletteItem> {
        let actions = vec![
            (
                "Undo",
                "Restore the previous structural map state",
                "undo revert last edit change history",
                PaletteAction::Undo,
            ),
            (
                "Redo",
                "Reapply the last undone structural change",
                "redo restore undone edit change history",
                PaletteAction::Redo,
            ),
            (
                "Add Child",
                "Create a child under the current node",
                "add child new branch child",
                PaletteAction::AddChild,
            ),
            (
                "Add Sibling",
                "Create a sibling next to the current node",
                "add sibling peer branch",
                PaletteAction::AddSibling,
            ),
            (
                "Add Root",
                "Create a new root-level branch",
                "add root top level branch",
                PaletteAction::AddRoot,
            ),
            (
                "Edit Node",
                "Edit the selected node inline syntax",
                "edit rename node current",
                PaletteAction::EditNode,
            ),
            (
                "Jump To Id",
                "Open the jump-to-id prompt",
                "jump open id deep link",
                PaletteAction::JumpToId,
            ),
            (
                "Jump To Root",
                "Move focus to the map root or subtree root",
                "jump root go top subtree root",
                PaletteAction::JumpToRoot,
            ),
            (
                "Open Search",
                "Search by text, tags, metadata, and ids",
                "search query filter find slash",
                PaletteAction::OpenSearch,
            ),
            (
                "Browse Facets",
                "Open the facet browser for tags and metadata",
                "facets tags metadata browse",
                PaletteAction::OpenFacets,
            ),
            (
                "Saved Views",
                "Open saved filter views",
                "saved views working sets filters",
                PaletteAction::OpenSavedViews,
            ),
            (
                "Open Mindmap",
                "Open the visual mindmap overlay",
                "mindmap visual bubble overlay canvas",
                PaletteAction::OpenMindmap,
            ),
            (
                "Cycle View Mode",
                "Switch between full map and focused views",
                "view mode focus subtree filtered focus",
                PaletteAction::CycleViewMode,
            ),
            (
                "Save Now",
                "Write the current document to disk",
                "save write disk",
                PaletteAction::SaveNow,
            ),
            (
                "Toggle Autosave",
                "Toggle immediate writes after edits",
                "autosave automatic save toggle",
                PaletteAction::ToggleAutosave,
            ),
            (
                "Revert From Disk",
                "Discard in-memory changes and reload",
                "revert reload discard reset",
                PaletteAction::RevertFromDisk,
            ),
            (
                "Create Checkpoint",
                "Save a local snapshot of the current map state",
                "checkpoint snapshot save safety local",
                PaletteAction::CreateCheckpoint,
            ),
            (
                "Restore Latest Checkpoint",
                "Return to the newest local checkpoint",
                "checkpoint restore latest local snapshot",
                PaletteAction::RestoreLatestCheckpoint,
            ),
            (
                "Clear Filter",
                "Clear the active query filter",
                "clear filter query search",
                PaletteAction::ClearFilter,
            ),
            (
                "Show Help",
                "Open the built-in help overlay",
                "help guide docs shortcuts",
                PaletteAction::ShowHelp,
            ),
        ];

        actions
            .into_iter()
            .filter_map(|(title, subtitle, keywords, action)| {
                palette_match_score(query, title, &format!("{title} {subtitle} {keywords}")).map(
                    |score| PaletteItem {
                        kind: PaletteItemKind::Action,
                        title: title.to_string(),
                        subtitle: subtitle.to_string(),
                        preview: subtitle.to_string(),
                        score: score + 500,
                        target: PaletteTarget::Action(action),
                    },
                )
            })
            .collect()
    }

    fn palette_theme_items(&self, query: &str) -> Vec<PaletteItem> {
        let baseline_theme = self.palette_surface_baseline().theme;
        ThemeId::ALL
            .into_iter()
            .filter_map(|theme| {
                let title = format!("Theme: {}", theme.label());
                let haystack = format!("{title} {} {}", theme.summary(), theme.keywords());
                palette_match_score(query, &title, &haystack).map(|score| PaletteItem {
                    kind: PaletteItemKind::Theme,
                    title,
                    subtitle: theme.summary().to_string(),
                    preview: format!(
                        "{}{}{}",
                        theme.summary(),
                        if baseline_theme == theme {
                            " Current theme."
                        } else {
                            ""
                        },
                        if self.palette_preview_base.is_some() {
                            " Moves preview immediately; Enter commits and Esc reverts."
                        } else {
                            ""
                        }
                    ),
                    score: score + 400,
                    target: PaletteTarget::Theme(theme),
                })
            })
            .collect()
    }

    fn palette_setting_items(&self, query: &str) -> Vec<PaletteItem> {
        let settings = [
            SurfaceSetting::Motion(true),
            SurfaceSetting::Motion(false),
            SurfaceSetting::AsciiAccents(true),
            SurfaceSetting::AsciiAccents(false),
        ];

        settings
            .into_iter()
            .filter_map(|setting| {
                let title = setting.label().to_string();
                let haystack = format!("{title} {} {}", setting.subtitle(), setting.keywords());
                let is_current = self.setting_matches_surface(setting);
                palette_match_score(query, &title, &haystack).map(|score| PaletteItem {
                    kind: PaletteItemKind::Setting,
                    title,
                    subtitle: setting.subtitle().to_string(),
                    preview: format!(
                        "{}{}",
                        setting.preview(),
                        if is_current { " Current setting." } else { "" }
                    ),
                    score: score + if is_current { 365 } else { 385 },
                    target: PaletteTarget::Setting(setting),
                })
            })
            .collect()
    }

    fn palette_saved_view_items(&self, query: &str) -> Vec<PaletteItem> {
        self.saved_views
            .views
            .iter()
            .filter_map(|view| {
                palette_match_score(query, &view.name, &format!("{} {}", view.name, view.query))
                    .map(|score| PaletteItem {
                        kind: PaletteItemKind::SavedView,
                        title: view.name.clone(),
                        subtitle: view.query.clone(),
                        preview: format!("Apply saved filter '{}'.", view.query),
                        score: score + 350,
                        target: PaletteTarget::SavedView {
                            name: view.name.clone(),
                            query: view.query.clone(),
                        },
                    })
            })
            .collect()
    }

    fn palette_history_items(&self, query: &str) -> Vec<PaletteItem> {
        let current = self.snapshot_workspace();
        let undo_items = self
            .undo_history
            .iter()
            .rev()
            .enumerate()
            .filter_map(|(depth, entry)| {
                let title = format!("Undo · {}", entry.label);
                let haystack = format!("{title} undo history recent action {}", entry.label);
                palette_match_score(query, &title, &haystack).map(|score| PaletteItem {
                    kind: PaletteItemKind::History,
                    title,
                    subtitle: format!(
                        "{} step{} back",
                        depth + 1,
                        if depth == 0 { "" } else { "s" }
                    ),
                    preview: history_entry_preview("Undo", depth + 1, &current, entry),
                    score: score + 345,
                    target: PaletteTarget::UndoSteps(depth + 1),
                })
            });

        let redo_items = self
            .redo_history
            .iter()
            .rev()
            .enumerate()
            .filter_map(|(depth, entry)| {
                let title = format!("Redo · {}", entry.label);
                let haystack = format!("{title} redo history recent action {}", entry.label);
                palette_match_score(query, &title, &haystack).map(|score| PaletteItem {
                    kind: PaletteItemKind::History,
                    title,
                    subtitle: format!(
                        "{} step{} forward",
                        depth + 1,
                        if depth == 0 { "" } else { "s" }
                    ),
                    preview: history_entry_preview("Redo", depth + 1, &current, entry),
                    score: score + 344,
                    target: PaletteTarget::RedoSteps(depth + 1),
                })
            });

        undo_items.chain(redo_items).collect()
    }

    fn palette_checkpoint_items(&self, query: &str) -> Vec<PaletteItem> {
        let current = self.snapshot_workspace();
        self.checkpoints
            .checkpoints
            .iter()
            .enumerate()
            .filter_map(|(index, checkpoint)| {
                let automatic = is_automatic_checkpoint(checkpoint);
                let title = checkpoint_palette_title(checkpoint, automatic);
                let haystack = format!(
                    "{} checkpoint safety snapshot {} {}",
                    title,
                    checkpoint.filter_query.as_deref().unwrap_or("no filter"),
                    match checkpoint.view_mode {
                        CheckpointViewMode::FullMap => "full map",
                        CheckpointViewMode::FocusBranch => "focus branch",
                        CheckpointViewMode::SubtreeOnly => "subtree only",
                        CheckpointViewMode::FilteredFocus => "filtered focus",
                    }
                );
                palette_match_score(query, &title, &haystack).map(|score| PaletteItem {
                    kind: if automatic {
                        PaletteItemKind::Safety
                    } else {
                        PaletteItemKind::Checkpoint
                    },
                    title,
                    subtitle: checkpoint_palette_subtitle(checkpoint, automatic),
                    preview: checkpoint_preview(&current, checkpoint),
                    score: score + if automatic { 338 } else { 342 },
                    target: PaletteTarget::Checkpoint(index),
                })
            })
            .collect()
    }

    fn palette_help_items(&self, query: &str) -> Vec<PaletteItem> {
        self.help_topics(query)
            .into_iter()
            .filter_map(|topic| {
                palette_match_score(query, topic.title(), &topic.search_text()).map(|score| {
                    PaletteItem {
                        kind: PaletteItemKind::Help,
                        title: topic.title().to_string(),
                        subtitle: topic.summary().to_string(),
                        preview: topic.hint().to_string(),
                        score: score + 250,
                        target: PaletteTarget::HelpTopic(topic),
                    }
                })
            })
            .collect()
    }

    fn help_topics(&self, raw: &str) -> Vec<HelpTopic> {
        let query = raw.trim().to_lowercase();
        let mut topics = [
            HelpTopic::Navigation,
            HelpTopic::Editing,
            HelpTopic::Search,
            HelpTopic::Views,
            HelpTopic::Themes,
            HelpTopic::Mindmap,
            HelpTopic::Syntax,
        ]
        .into_iter()
        .filter_map(|topic| {
            if query.is_empty() {
                Some((topic, 0_i64))
            } else {
                palette_match_score(&query, topic.title(), &topic.search_text())
                    .map(|score| (topic, score))
            }
        })
        .collect::<Vec<_>>();

        topics.sort_by(|left, right| {
            right
                .1
                .cmp(&left.1)
                .then_with(|| left.0.title().cmp(right.0.title()))
        });
        topics.into_iter().map(|entry| entry.0).collect()
    }

    fn palette_node_items(&self, query: &str) -> Vec<PaletteItem> {
        collect_palette_nodes(self.editor.document())
            .into_iter()
            .filter_map(|entry| {
                palette_match_score(query, &entry.primary, &entry.haystack).map(|score| {
                    PaletteItem {
                        kind: PaletteItemKind::Node,
                        title: entry.primary,
                        subtitle: entry.secondary,
                        preview: entry.preview,
                        score,
                        target: PaletteTarget::NodePath(entry.path),
                    }
                })
            })
            .collect()
    }

    fn apply_filter(&mut self, raw: &str) -> Result<(), AppError> {
        let Some(query) = FilterQuery::parse(raw) else {
            self.filter = None;
            if self.view_mode == ViewMode::FilteredFocus {
                self.set_view_mode(ViewMode::FocusBranch);
            }
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
            self.trigger_motion(MotionTarget::FilterResult);
            self.set_status(
                StatusTone::Success,
                format!("Filter applied with {count} matches."),
            );
        }
        Ok(())
    }

    fn active_filter_match_count(&self) -> usize {
        self.filter
            .as_ref()
            .map_or(0, |filter| filter.matches.len())
    }

    fn apply_search_facet(&mut self, label: &str, query: &str) -> Result<(), AppError> {
        self.apply_filter(query)?;
        let count = self.active_filter_match_count();
        if count == 0 {
            self.set_status(
                StatusTone::Warning,
                format!("Applied facet {label}, but no nodes matched."),
            );
        } else {
            self.set_status(
                StatusTone::Success,
                format!("Applied facet {label} and landed on the first of {count} matches."),
            );
        }
        Ok(())
    }

    fn open_saved_view(&mut self, name: &str, query: &str) -> Result<(), AppError> {
        self.apply_filter(query)?;
        let count = self.active_filter_match_count();
        if count == 0 {
            self.set_status(
                StatusTone::Warning,
                format!("Opened saved view '{name}', but no nodes matched."),
            );
        } else {
            self.set_status(
                StatusTone::Success,
                format!("Opened saved view '{name}' and landed on the first of {count} matches."),
            );
        }
        Ok(())
    }

    fn snapshot_workspace(&self) -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            editor: self.editor.state(),
            expanded: self.expanded.clone(),
            view_mode: self.view_mode,
            subtree_root: self.subtree_root.clone(),
            filter_query: self
                .filter
                .as_ref()
                .map(|filter| filter.query.raw().to_string()),
        }
    }

    fn push_history_entry(
        entries: &mut Vec<HistoryEntry>,
        label: impl Into<String>,
        snapshot: WorkspaceSnapshot,
    ) {
        entries.push(HistoryEntry {
            label: label.into(),
            snapshot,
        });
        if entries.len() > HISTORY_LIMIT {
            entries.remove(0);
        }
    }

    fn remember_undo_state(&mut self, label: impl Into<String>) {
        let snapshot = self.snapshot_workspace();
        Self::push_history_entry(&mut self.undo_history, label, snapshot);
        self.redo_history.clear();
    }

    fn active_filter_from_raw(&self, raw: &str) -> Option<ActiveFilter> {
        let query = FilterQuery::parse(raw)?;
        let matches = collect_match_paths(self.editor.document(), &query);
        Some(ActiveFilter { query, matches })
    }

    fn apply_workspace_snapshot(&mut self, snapshot: WorkspaceSnapshot) -> Result<(), AppError> {
        self.editor.restore_state(snapshot.editor)?;
        self.expanded = snapshot.expanded;
        self.view_mode = snapshot.view_mode;
        self.subtree_root = snapshot.subtree_root;
        self.filter = snapshot
            .filter_query
            .as_deref()
            .and_then(|raw| self.active_filter_from_raw(raw));
        if self.filter.is_none() && self.view_mode == ViewMode::FilteredFocus {
            self.view_mode = ViewMode::FocusBranch;
        }
        self.expand_focus_chain();
        self.prompt = None;
        self.palette = None;
        self.help = None;
        self.search = None;
        self.mindmap = None;
        self.palette_preview_base = None;
        self.quit_armed = false;
        self.delete_armed = false;
        Ok(())
    }

    fn restore_workspace(
        &mut self,
        snapshot: WorkspaceSnapshot,
        status_tone: StatusTone,
        status_text: impl Into<String>,
    ) -> Result<(), AppError> {
        let status_text = status_text.into();
        self.apply_workspace_snapshot(snapshot)?;
        self.persist_session()?;
        if self.autosave {
            self.write_editor_to_disk()?;
            self.set_status(status_tone, format!("{status_text} Autosaved."));
        } else {
            self.set_status(status_tone, status_text);
        }
        self.trigger_motion(MotionTarget::Focus);
        Ok(())
    }

    fn step_undo_once(&mut self) -> Result<Option<HistoryEntry>, AppError> {
        let Some(entry) = self.undo_history.pop() else {
            return Ok(None);
        };
        let snapshot = self.snapshot_workspace();
        Self::push_history_entry(&mut self.redo_history, entry.label.clone(), snapshot);
        self.apply_workspace_snapshot(entry.snapshot.clone())?;
        Ok(Some(entry))
    }

    fn step_redo_once(&mut self) -> Result<Option<HistoryEntry>, AppError> {
        let Some(entry) = self.redo_history.pop() else {
            return Ok(None);
        };
        let snapshot = self.snapshot_workspace();
        Self::push_history_entry(&mut self.undo_history, entry.label.clone(), snapshot);
        self.apply_workspace_snapshot(entry.snapshot.clone())?;
        Ok(Some(entry))
    }

    fn undo(&mut self) -> Result<(), AppError> {
        self.undo_steps(1)
    }

    fn redo(&mut self) -> Result<(), AppError> {
        self.redo_steps(1)
    }

    fn undo_steps(&mut self, steps: usize) -> Result<(), AppError> {
        let mut count = 0_usize;
        let mut label = None;
        while count < steps {
            let Some(entry) = self.step_undo_once()? else {
                break;
            };
            label = Some(entry.label);
            count += 1;
        }

        if count == 0 {
            self.set_status(StatusTone::Info, "Nothing to undo yet.");
            return Ok(());
        }

        self.persist_session()?;
        if self.autosave {
            self.write_editor_to_disk()?;
        }
        let label = label.expect("count > 0 should set a label");
        let label = label.trim_end_matches('.');
        let message = if count == 1 {
            format!("Undid: {label}.")
        } else {
            format!("Undid {count} actions through: {label}.")
        };
        if self.autosave {
            self.set_status(StatusTone::Success, format!("{message} Autosaved."));
        } else {
            self.set_status(StatusTone::Success, message);
        }
        self.trigger_motion(MotionTarget::Focus);
        Ok(())
    }

    fn redo_steps(&mut self, steps: usize) -> Result<(), AppError> {
        let mut count = 0_usize;
        let mut label = None;
        while count < steps {
            let Some(entry) = self.step_redo_once()? else {
                break;
            };
            label = Some(entry.label);
            count += 1;
        }

        if count == 0 {
            self.set_status(StatusTone::Info, "Nothing to redo yet.");
            return Ok(());
        }

        self.persist_session()?;
        if self.autosave {
            self.write_editor_to_disk()?;
        }
        let label = label.expect("count > 0 should set a label");
        let label = label.trim_end_matches('.');
        let message = if count == 1 {
            format!("Redid: {label}.")
        } else {
            format!("Redid {count} actions through: {label}.")
        };
        if self.autosave {
            self.set_status(StatusTone::Success, format!("{message} Autosaved."));
        } else {
            self.set_status(StatusTone::Success, message);
        }
        self.trigger_motion(MotionTarget::Focus);
        Ok(())
    }

    fn clear_filter(&mut self) -> bool {
        let had_filter = self.filter.is_some();
        self.filter = None;
        if had_filter && self.view_mode == ViewMode::FilteredFocus {
            self.set_view_mode(ViewMode::FocusBranch);
        }
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
        self.trigger_motion(MotionTarget::FilterResult);
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
            self.trigger_motion(MotionTarget::Focus);
        }
        Ok(())
    }

    fn checkpoint_for_current_state(&self, name: impl Into<String>) -> Checkpoint {
        let snapshot = self.snapshot_workspace();
        Checkpoint {
            name: name.into(),
            document: snapshot.editor.document,
            focus_path: snapshot.editor.focus_path,
            dirty: snapshot.editor.dirty,
            expanded_paths: snapshot.expanded.into_iter().collect(),
            view_mode: snapshot.view_mode.into(),
            subtree_root: snapshot.subtree_root.map(|anchor| CheckpointAnchor {
                path: anchor.path,
                id: anchor.id,
            }),
            filter_query: snapshot.filter_query,
        }
    }

    fn save_checkpoint(&mut self, name: &str) -> Result<(), AppError> {
        let trimmed_name = name.trim();
        if trimmed_name.is_empty() {
            return Err(AppError::new("Checkpoint name cannot be empty."));
        }

        self.checkpoints
            .checkpoints
            .retain(|checkpoint| !checkpoint.name.eq_ignore_ascii_case(trimmed_name));
        self.checkpoints.checkpoints.insert(
            0,
            self.checkpoint_for_current_state(trimmed_name.to_string()),
        );
        if self.checkpoints.checkpoints.len() > CHECKPOINT_LIMIT {
            self.checkpoints.checkpoints.truncate(CHECKPOINT_LIMIT);
        }
        self.persist_checkpoints()?;
        self.set_status(
            StatusTone::Success,
            format!("Saved checkpoint '{}'.", trimmed_name),
        );
        Ok(())
    }

    fn save_automatic_checkpoint(&mut self, name: impl Into<String>) -> Result<(), AppError> {
        self.checkpoints
            .checkpoints
            .insert(0, self.checkpoint_for_current_state(name));
        if self.checkpoints.checkpoints.len() > CHECKPOINT_LIMIT {
            self.checkpoints.checkpoints.truncate(CHECKPOINT_LIMIT);
        }
        self.persist_checkpoints()
    }

    fn restore_latest_checkpoint(&mut self) -> Result<(), AppError> {
        if self.checkpoints.checkpoints.is_empty() {
            self.set_status(StatusTone::Info, "No checkpoints are available yet.");
            return Ok(());
        }
        self.restore_checkpoint(0)
    }

    fn restore_checkpoint(&mut self, index: usize) -> Result<(), AppError> {
        let Some(checkpoint) = self.checkpoints.checkpoints.get(index).cloned() else {
            self.set_status(
                StatusTone::Warning,
                "That checkpoint is no longer available.",
            );
            return Ok(());
        };

        self.remember_undo_state(format!("restore checkpoint '{}'", checkpoint.name));
        let snapshot = WorkspaceSnapshot {
            editor: EditorState {
                document: checkpoint.document,
                focus_path: checkpoint.focus_path,
                dirty: checkpoint.dirty,
            },
            expanded: checkpoint.expanded_paths.into_iter().collect(),
            view_mode: checkpoint.view_mode.into(),
            subtree_root: checkpoint.subtree_root.map(|anchor| PathAnchor {
                path: anchor.path,
                id: anchor.id,
            }),
            filter_query: checkpoint.filter_query,
        };
        self.restore_workspace(
            snapshot,
            StatusTone::Success,
            format!("Restored checkpoint '{}'.", checkpoint.name),
        )
    }

    fn suggest_checkpoint_name(&self) -> String {
        self.editor
            .current()
            .map(|node| {
                if node.text.is_empty() {
                    "Checkpoint".to_string()
                } else {
                    format!("{} checkpoint", node.text)
                }
            })
            .unwrap_or_else(|| "Checkpoint".to_string())
    }

    fn write_editor_to_disk(&mut self) -> Result<(), AppError> {
        self.editor.save()?;
        fs::write(&self.map_path, serialize_document(self.editor.document())).map_err(|error| {
            AppError::new(format!(
                "Could not write '{}': {error}",
                self.map_path.display()
            ))
        })?;
        self.editor.mark_clean();
        Ok(())
    }

    fn save_to_disk(&mut self) -> Result<(), AppError> {
        self.write_editor_to_disk()?;
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
        self.remember_undo_state("revert from disk");

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
        self.filter = None;
        if self.view_mode == ViewMode::FilteredFocus {
            self.view_mode = ViewMode::FocusBranch;
        }
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

    fn persist_checkpoints(&self) -> Result<(), AppError> {
        save_checkpoints_for(&self.map_path, &self.checkpoints)
    }

    fn persist_ui_settings(&self) -> Result<(), AppError> {
        save_ui_settings_for(&self.map_path, &self.ui_settings)
    }

    fn theme_colors(&self) -> Palette {
        self.ui_settings.theme.theme()
    }

    fn palette_surface_baseline(&self) -> &UiSettings {
        self.palette_preview_base
            .as_ref()
            .unwrap_or(&self.ui_settings)
    }

    fn setting_matches_surface(&self, setting: SurfaceSetting) -> bool {
        let baseline = self.palette_surface_baseline();
        match setting {
            SurfaceSetting::Motion(enabled) => baseline.motion_enabled == enabled,
            SurfaceSetting::AsciiAccents(enabled) => baseline.ascii_accents == enabled,
        }
    }

    fn tick(&mut self) {
        self.motion_cue = self.motion_cue.and_then(MotionCue::tick);
    }

    fn trigger_motion(&mut self, target: MotionTarget) {
        if self.ui_settings.motion_enabled {
            self.motion_cue = Some(MotionCue::new(target));
        }
    }

    fn set_theme(&mut self, theme: ThemeId) -> Result<(), AppError> {
        if self.ui_settings.theme == theme {
            self.set_status(
                StatusTone::Info,
                format!("Theme already set to {}.", theme.label()),
            );
            return Ok(());
        }

        self.ui_settings.theme = theme;
        self.persist_ui_settings()?;
        self.set_status(
            StatusTone::Success,
            format!("Theme set to {}.", theme.label()),
        );
        Ok(())
    }

    fn apply_surface_setting(&mut self, setting: SurfaceSetting) -> Result<(), AppError> {
        let changed = match setting {
            SurfaceSetting::Motion(enabled) => {
                let changed = self.ui_settings.motion_enabled != enabled;
                self.ui_settings.motion_enabled = enabled;
                if !enabled {
                    self.motion_cue = None;
                }
                changed
            }
            SurfaceSetting::AsciiAccents(enabled) => {
                let changed = self.ui_settings.ascii_accents != enabled;
                self.ui_settings.ascii_accents = enabled;
                changed
            }
        };

        self.persist_ui_settings()?;

        let (tone, message) = match setting {
            SurfaceSetting::Motion(true) if changed => (
                StatusTone::Success,
                "Motion enabled. Focus changes, scope changes, and active inputs will guide attention.",
            ),
            SurfaceSetting::Motion(true) => (StatusTone::Info, "Motion is already enabled."),
            SurfaceSetting::Motion(false) if changed => (
                StatusTone::Success,
                "Motion disabled. The interface now stays fully static.",
            ),
            SurfaceSetting::Motion(false) => (StatusTone::Info, "Motion is already disabled."),
            SurfaceSetting::AsciiAccents(true) if changed => (
                StatusTone::Success,
                "ASCII accents enabled. Titles and separators now use the terminal-styled chrome.",
            ),
            SurfaceSetting::AsciiAccents(true) => {
                (StatusTone::Info, "ASCII accents are already enabled.")
            }
            SurfaceSetting::AsciiAccents(false) if changed => (
                StatusTone::Success,
                "ASCII accents disabled. The default chrome is back.",
            ),
            SurfaceSetting::AsciiAccents(false) => {
                (StatusTone::Info, "ASCII accents are already disabled.")
            }
        };
        self.set_status(tone, message);
        Ok(())
    }

    fn sync_palette_preview(&mut self, palette: &PaletteState) {
        let Some(baseline) = self.palette_preview_base.clone() else {
            return;
        };

        let preview = self
            .palette_items(&palette.query)
            .get(palette.selected)
            .and_then(|item| item.target.preview_ui_settings(&baseline));

        self.ui_settings = preview.unwrap_or(baseline);
        if !self.ui_settings.motion_enabled {
            self.motion_cue = None;
        }
    }

    fn close_palette(&mut self, commit_preview: bool) -> Result<(), AppError> {
        self.palette = None;
        if let Some(baseline) = self.palette_preview_base.take() {
            if commit_preview {
                if self.ui_settings != baseline {
                    self.persist_ui_settings()?;
                }
            } else {
                self.ui_settings = baseline;
                if !self.ui_settings.motion_enabled {
                    self.motion_cue = None;
                }
            }
        }
        Ok(())
    }

    fn commit_preview_target(&mut self, target: PaletteTarget) -> Result<(), AppError> {
        let baseline = self.palette_preview_base.clone();
        self.close_palette(true)?;
        let changed = baseline.is_some_and(|settings| settings != self.ui_settings);
        match target {
            PaletteTarget::Theme(theme) => {
                if changed {
                    self.set_status(
                        StatusTone::Success,
                        format!("Theme set to {}.", theme.label()),
                    );
                } else {
                    self.set_status(
                        StatusTone::Info,
                        format!("Theme already set to {}.", theme.label()),
                    );
                }
            }
            PaletteTarget::Setting(setting) => {
                let (tone, message) = match setting {
                    SurfaceSetting::Motion(true) if changed => (
                        StatusTone::Success,
                        "Motion enabled. Focus changes, scope changes, and active inputs will guide attention.",
                    ),
                    SurfaceSetting::Motion(true) => {
                        (StatusTone::Info, "Motion is already enabled.")
                    }
                    SurfaceSetting::Motion(false) if changed => (
                        StatusTone::Success,
                        "Motion disabled. The interface now stays fully static.",
                    ),
                    SurfaceSetting::Motion(false) => {
                        (StatusTone::Info, "Motion is already disabled.")
                    }
                    SurfaceSetting::AsciiAccents(true) if changed => (
                        StatusTone::Success,
                        "ASCII accents enabled. Titles and separators now use the terminal-styled chrome.",
                    ),
                    SurfaceSetting::AsciiAccents(true) => {
                        (StatusTone::Info, "ASCII accents are already enabled.")
                    }
                    SurfaceSetting::AsciiAccents(false) if changed => (
                        StatusTone::Success,
                        "ASCII accents disabled. The default chrome is back.",
                    ),
                    SurfaceSetting::AsciiAccents(false) => {
                        (StatusTone::Info, "ASCII accents are already disabled.")
                    }
                };
                self.set_status(tone, message);
            }
            _ => {}
        }
        Ok(())
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

    fn status_model(&self) -> StatusModel {
        let focus_label = self
            .editor
            .current()
            .map(|node| {
                if node.text.is_empty() {
                    "(empty)".to_string()
                } else {
                    node.text.clone()
                }
            })
            .unwrap_or_else(|| "(no focus)".to_string());
        let focus_id = self.editor.current().and_then(|node| node.id.clone());
        let filter_summary = self
            .filter
            .as_ref()
            .map(|filter| format!("{} ({})", filter.query.raw(), filter.matches.len()));
        let scope_label = if let Some(search) = &self.search {
            if !search.draft_query.trim().is_empty() {
                format!("Draft '{}'", search.draft_query.trim())
            } else {
                current_scope_label(self, Some(search))
            }
        } else {
            current_scope_label(self, None)
        };

        StatusModel {
            tone: self.status.tone,
            message: self.status.text.clone(),
            focus_label,
            focus_id,
            filter_summary,
            scope_label,
        }
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

    fn automatic_checkpoint_label(&self, action: &str) -> String {
        let label = self
            .editor
            .current()
            .map(|node| {
                if node.text.is_empty() {
                    "(empty)".to_string()
                } else {
                    node.text.clone()
                }
            })
            .unwrap_or_else(|| "current node".to_string());
        format!("Safety checkpoint: {action} · {label}")
    }

    fn apply_edit<F>(
        &mut self,
        edit: F,
        message: impl Into<String>,
        checkpoint_label: Option<String>,
    ) -> Result<(), AppError>
    where
        F: FnOnce(&mut Editor) -> Result<(), AppError>,
    {
        let message = message.into();
        if let Some(checkpoint_label) = checkpoint_label {
            self.save_automatic_checkpoint(checkpoint_label)?;
        }
        self.remember_undo_state(message.clone());
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
    let checkpoints = load_checkpoints_for(&loaded.target.path)?;
    let ui_settings = load_ui_settings_for(&loaded.target.path)?;
    let mut app = TuiApp::new_with_settings(
        loaded.target.path.clone(),
        loaded.document,
        focus_path,
        warning,
        autosave,
        saved_views,
        ui_settings,
    );
    app.checkpoints = checkpoints;
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
        } else {
            app.tick();
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

#[allow(non_snake_case)]
fn render(frame: &mut Frame, app: &TuiApp) {
    let PALETTE = app.theme_colors();
    ACTIVE_PALETTE.with(|palette| palette.set(PALETTE));
    ACTIVE_ASCII_ACCENTS.with(|enabled| enabled.set(app.ui_settings.ascii_accents));
    ACTIVE_MOTION_TARGET.with(|target| target.set(app.motion_cue.map(|cue| cue.target)));
    ACTIVE_MOTION_LEVEL
        .with(|level| level.set(app.motion_cue.map_or(0, MotionCue::emphasis_level)));
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
            Constraint::Length(4),
            Constraint::Length(1),
        ])
        .split(area);

    render_header(frame, outer[0], app);
    render_body(frame, outer[1], app);
    render_status(frame, outer[2], app);
    render_keybar(frame, outer[3]);

    if let Some(help) = &app.help {
        render_help_overlay(frame, centered_rect(78, 80, area), app, help);
    }

    if let Some(overlay) = &app.mindmap {
        render_mindmap_overlay(frame, centered_rect(92, 88, area), app, overlay);
    }

    if let Some(search) = &app.search {
        render_search_overlay(frame, centered_rect(78, 80, area), app, search);
    }

    if let Some(palette) = &app.palette {
        render_palette_overlay(frame, centered_rect(78, 76, area), app, palette);
    }

    if let Some(prompt) = &app.prompt {
        render_prompt_overlay(frame, centered_rect(68, 30, area), prompt, app);
    }
}

#[allow(non_snake_case)]
fn render_header(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let PALETTE = app.theme_colors();
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
    let view_badge = format!(" {} ", app.view_mode.label());
    let filter_badge = app
        .filter
        .as_ref()
        .map(|filter| format!(" FILTER {} ", filter.matches.len()));
    let mut header_spans = vec![
        Span::styled(
            if ascii_accents_enabled() {
                format!(" // mdmind v{APP_VERSION} // ")
            } else {
                format!(" mdmind v{APP_VERSION} ")
            },
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
        Span::raw("  "),
        Span::styled(
            view_badge,
            Style::default()
                .fg(PALETTE.background)
                .bg(PALETTE.warn)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        status_chip(
            "THEME",
            app.ui_settings.theme.label(),
            PALETTE.border,
            PALETTE.text,
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

#[allow(non_snake_case)]
fn render_outline(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let PALETTE = app.theme_colors();
    let rows = app.visible_rows();
    let selected_index = app.selected_index(&rows);
    let scope_root = app.projection_focus_path();
    let scope_attention = motion_level(MotionTarget::Scope);
    let filter_attention = motion_level(MotionTarget::FilterResult);
    let selected_highlight_style = match (
        motion_level(MotionTarget::Focus),
        filter_attention,
        scope_attention,
    ) {
        (_, level, _) if level >= 2 => Style::default()
            .bg(PALETTE.warn)
            .fg(PALETTE.background)
            .add_modifier(Modifier::BOLD),
        (_, 1, _) => Style::default()
            .bg(PALETTE.surface_alt)
            .fg(PALETTE.warn)
            .add_modifier(Modifier::BOLD),
        (level, _, _) if level >= 2 => Style::default()
            .bg(PALETTE.accent)
            .fg(PALETTE.background)
            .add_modifier(Modifier::BOLD),
        (1, _, _) => Style::default()
            .bg(PALETTE.surface_alt)
            .fg(PALETTE.text)
            .add_modifier(Modifier::BOLD),
        (_, _, level) if level >= 2 => Style::default()
            .bg(PALETTE.warn)
            .fg(PALETTE.background)
            .add_modifier(Modifier::BOLD),
        (_, _, 1) => Style::default()
            .bg(PALETTE.surface_alt)
            .fg(PALETTE.text)
            .add_modifier(Modifier::BOLD),
        _ => Style::default()
            .bg(PALETTE.surface_alt)
            .fg(PALETTE.text)
            .add_modifier(Modifier::BOLD),
    };

    let items: Vec<ListItem> = rows
        .iter()
        .map(|row| {
            let mut spans = Vec::new();
            let scope_role = if scope_attention > 0 {
                scope_handoff_role(&row.path, &scope_root, app.view_mode)
            } else {
                ScopeHandoffRole::None
            };
            spans.push(Span::raw(" ".repeat(row.depth * 2)));
            let icon = if row.has_children {
                if row.expanded { "▾ " } else { "▸ " }
            } else {
                "• "
            };
            let icon_color = if (filter_attention > 0 && row.matched)
                || (scope_attention > 0 && scope_role == ScopeHandoffRole::Root)
            {
                PALETTE.warn
            } else if scope_attention > 0 && scope_role == ScopeHandoffRole::Branch && !row.dimmed {
                PALETTE.sky
            } else if row.has_children {
                if row.dimmed {
                    PALETTE.border
                } else {
                    PALETTE.accent
                }
            } else {
                PALETTE.muted
            };
            spans.push(Span::styled(icon, Style::default().fg(icon_color)));
            let mut text_style = Style::default()
                .fg(if row.dimmed {
                    PALETTE.muted
                } else if row.matched {
                    PALETTE.sky
                } else {
                    PALETTE.text
                })
                .add_modifier(if row.dimmed {
                    Modifier::empty()
                } else {
                    Modifier::BOLD
                });
            if scope_attention > 0 {
                match scope_role {
                    ScopeHandoffRole::Root => {
                        text_style = text_style
                            .fg(PALETTE.warn)
                            .add_modifier(Modifier::UNDERLINED | Modifier::BOLD);
                    }
                    ScopeHandoffRole::Branch if !row.dimmed => {
                        text_style = text_style.fg(PALETTE.sky);
                    }
                    _ => {}
                }
            }
            if filter_attention > 0 && row.matched {
                text_style = text_style
                    .fg(PALETTE.warn)
                    .add_modifier(Modifier::UNDERLINED | Modifier::BOLD);
            }
            spans.push(Span::styled(row.text.clone(), text_style));
            if !row.tags.is_empty() {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    row.tags.join(" "),
                    Style::default().fg(if row.dimmed {
                        PALETTE.border
                    } else {
                        PALETTE.accent
                    }),
                ));
            }
            if !row.metadata.is_empty() {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    row.metadata.join(" "),
                    Style::default().fg(if row.dimmed {
                        PALETTE.muted
                    } else {
                        PALETTE.warn
                    }),
                ));
            }
            if let Some(id) = &row.id {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    format!("[id:{id}]"),
                    Style::default().fg(if row.dimmed {
                        PALETTE.border
                    } else {
                        PALETTE.muted
                    }),
                ));
            }
            if row.has_children {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    format!("({})", row.child_count),
                    Style::default().fg(if row.dimmed {
                        PALETTE.border
                    } else {
                        PALETTE.sky
                    }),
                ));
            }
            if row.matched {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    "●",
                    Style::default().fg(if filter_attention > 0 {
                        PALETTE.accent
                    } else {
                        PALETTE.warn
                    }),
                ));
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
        .highlight_style(selected_highlight_style);

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

#[allow(non_snake_case)]
fn render_focus_card(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let PALETTE = app.theme_colors();
    let scope_root_path = app.projection_focus_path();
    let filter_surface = attention_fill(
        PALETTE.surface,
        PALETTE.surface_alt,
        PALETTE.surface_alt,
        MotionTarget::FilterResult,
    );
    let focus_surface = attention_fill(
        filter_surface,
        PALETTE.surface_alt,
        PALETTE.surface_alt,
        MotionTarget::Focus,
    );
    let scope_surface = attention_fill(
        focus_surface,
        PALETTE.surface_alt,
        PALETTE.surface_alt,
        MotionTarget::Scope,
    );
    let block = Block::default()
        .title(styled_title("Focus", PALETTE.sky))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(attention_border(
            attention_border(
                attention_border(
                    PALETTE.sky,
                    PALETTE.warn,
                    PALETTE.warn,
                    MotionTarget::FilterResult,
                ),
                PALETTE.sky,
                PALETTE.accent,
                MotionTarget::Focus,
            ),
            PALETTE.warn,
            PALETTE.warn,
            MotionTarget::Scope,
        )))
        .style(Style::default().bg(scope_surface))
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
            if motion_level(MotionTarget::Scope) > 0
                && let Some(scope_root) =
                    get_node(&app.editor.document().nodes, scope_root_path.as_slice())
            {
                let label = match app.view_mode {
                    ViewMode::SubtreeOnly => "scope root ",
                    ViewMode::FocusBranch => "scope branch ",
                    ViewMode::FilteredFocus => "scope focus ",
                    ViewMode::FullMap => "scope ",
                };
                let detail = if scope_root.text.is_empty() {
                    "(empty)"
                } else {
                    scope_root.text.as_str()
                };
                lines.push(Line::from(vec![
                    Span::styled(label, Style::default().fg(PALETTE.muted)),
                    Span::styled(
                        detail,
                        Style::default()
                            .fg(PALETTE.warn)
                            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                    ),
                    Span::raw("  "),
                    Span::styled("view settles here", Style::default().fg(PALETTE.sky)),
                ]));
            }
            if app.view_mode == ViewMode::SubtreeOnly
                && let Some(root) = app.subtree_root_node()
            {
                lines.push(Line::from(vec![
                    Span::styled("subtree root ", Style::default().fg(PALETTE.muted)),
                    Span::styled(
                        if root.text.is_empty() {
                            "(empty)"
                        } else {
                            root.text.as_str()
                        },
                        Style::default().fg(PALETTE.warn),
                    ),
                    Span::raw("  "),
                    Span::styled("g", Style::default().fg(PALETTE.accent)),
                    Span::raw(" returns here"),
                ]));
            }
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
                        Style::default().fg(if motion_level(MotionTarget::FilterResult) > 0 {
                            PALETTE.accent
                        } else {
                            PALETTE.warn
                        }),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        if is_direct_match {
                            if motion_level(MotionTarget::FilterResult) > 0 {
                                "landing match"
                            } else {
                                "direct match"
                            }
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

#[allow(non_snake_case)]
fn render_parent_lane(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let PALETTE = app.theme_colors();
    let title = styled_title("Parent", PALETTE.warn);
    render_simple_lane(
        frame,
        area,
        title,
        parent_lines(app),
        Style::default().bg(PALETTE.surface),
    );
}

#[allow(non_snake_case)]
fn render_peer_lane(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let PALETTE = app.theme_colors();
    let title = styled_title("Peers", PALETTE.accent);
    render_simple_lane(
        frame,
        area,
        title,
        peer_lines(app),
        Style::default().bg(PALETTE.surface),
    );
}

#[allow(non_snake_case)]
fn render_children_lane(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let PALETTE = app.theme_colors();
    let title = styled_title("Children", PALETTE.sky);
    render_simple_lane(
        frame,
        area,
        title,
        child_lines(app),
        Style::default().bg(PALETTE.surface),
    );
}

#[allow(non_snake_case)]
fn render_simple_lane(
    frame: &mut Frame,
    area: Rect,
    title: Line<'static>,
    lines: Vec<Line<'static>>,
    style: Style,
) {
    let PALETTE = active_palette();
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

#[allow(non_snake_case)]
fn render_status(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let PALETTE = app.theme_colors();
    let status = app.status_model();
    let (label, color) = match status.tone {
        StatusTone::Info => ("INFO", PALETTE.sky),
        StatusTone::Success => ("SAVED", PALETTE.accent),
        StatusTone::Warning => ("WARN", PALETTE.warn),
        StatusTone::Error => ("ERROR", PALETTE.danger),
    };
    let mut headline = vec![
        Span::styled(
            format!(" {label} "),
            Style::default()
                .fg(PALETTE.background)
                .bg(color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(status.message, Style::default().fg(PALETTE.text)),
    ];
    if let Some(filter) = &status.filter_summary {
        headline.push(Span::raw(" "));
        headline.push(status_chip(
            "FILTER",
            filter,
            if motion_level(MotionTarget::FilterResult) >= 2 {
                PALETTE.warn
            } else {
                PALETTE.accent
            },
            PALETTE.background,
        ));
    }

    let mut context = vec![status_chip(
        "FOCUS",
        &status.focus_label,
        if motion_level(MotionTarget::FilterResult) >= 2 {
            PALETTE.warn
        } else if motion_level(MotionTarget::Focus) >= 2 {
            PALETTE.accent
        } else {
            PALETTE.sky
        },
        PALETTE.background,
    )];
    if let Some(id) = &status.focus_id {
        context.push(Span::raw(" "));
        context.push(status_chip("ID", id, PALETTE.border, PALETTE.text));
    }
    context.push(Span::raw(" "));
    context.push(status_chip(
        "SCOPE",
        &status.scope_label,
        if motion_level(MotionTarget::Scope) >= 2 {
            PALETTE.warn
        } else if motion_level(MotionTarget::Scope) == 1 {
            PALETTE.sky
        } else {
            PALETTE.border
        },
        if motion_level(MotionTarget::Scope) >= 2 {
            PALETTE.background
        } else {
            PALETTE.text
        },
    ));

    let lines = vec![Line::from(headline), Line::from(context)];

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(attention_border(
            PALETTE.border,
            PALETTE.border,
            PALETTE.warn,
            MotionTarget::Scope,
        )))
        .style(Style::default().bg(PALETTE.surface));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

#[allow(non_snake_case)]
fn render_keybar(frame: &mut Frame, area: Rect) {
    let PALETTE = active_palette();
    let line = Line::from(vec![
        key_hint("↑↓", "move"),
        separator_span(),
        key_hint("←→", "tree"),
        separator_span(),
        key_hint("⌥←→", "nest"),
        separator_span(),
        key_hint("⌥↑↓", "swap"),
        separator_span(),
        key_hint("a", "add"),
        separator_span(),
        key_hint("e", "edit"),
        separator_span(),
        key_hint("x", "delete"),
        separator_span(),
        key_hint("u/U", "undo"),
        separator_span(),
        key_hint("m", "mindmap"),
        separator_span(),
        key_hint(":", "palette"),
        separator_span(),
        key_hint("v/V", "mode"),
        separator_span(),
        key_hint("/", "find"),
        separator_span(),
        key_hint("f/F", "find"),
        separator_span(),
        key_hint("n", "next"),
        separator_span(),
        key_hint("s", "save"),
        separator_span(),
        key_hint("r", "revert"),
        separator_span(),
        key_hint("?", "help"),
    ]);
    frame.render_widget(
        Paragraph::new(line)
            .style(Style::default().bg(PALETTE.background))
            .alignment(Alignment::Center),
        area,
    );
}

#[allow(non_snake_case)]
fn render_help_overlay(frame: &mut Frame, area: Rect, app: &TuiApp, help: &HelpOverlayState) {
    let PALETTE = app.theme_colors();
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(styled_title("mdmind Help", PALETTE.accent))
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
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    if app.ui_settings.ascii_accents {
                        "/\\/\\  Searchable Built-In Help"
                    } else {
                        "Searchable Built-In Help"
                    },
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
                    "Filter topics for navigation, search, views, mindmap behavior, syntax, and surface settings without leaving the terminal.",
                    Style::default().fg(PALETTE.muted),
                ),
            ]),
            Line::from(Span::styled(
                format!("Current view: {}.", app.view_mode.label()),
                Style::default().fg(PALETTE.sky),
            )),
        ]),
        sections[0],
    );

    let input_block = Block::default()
        .title(styled_title(
            "Query",
            attention_border(
                PALETTE.warn,
                PALETTE.sky,
                PALETTE.accent,
                MotionTarget::HelpInput,
            ),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(attention_border(
            PALETTE.warn,
            PALETTE.sky,
            PALETTE.accent,
            MotionTarget::HelpInput,
        )))
        .style(Style::default().bg(attention_fill(
            PALETTE.surface,
            PALETTE.surface_alt,
            PALETTE.surface_alt,
            MotionTarget::HelpInput,
        )))
        .padding(Padding::horizontal(1));
    let input_inner = input_block.inner(sections[1]);
    frame.render_widget(input_block, sections[1]);
    frame.render_widget(
        Paragraph::new(help.query.clone())
            .style(Style::default().fg(PALETTE.text))
            .wrap(Wrap { trim: false }),
        input_inner,
    );
    frame.set_cursor_position((input_inner.x + help.cursor as u16, input_inner.y));

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
        .split(sections[2]);

    let topics = app.help_topics(&help.query);
    if topics.is_empty() {
        frame.render_widget(
            Paragraph::new("No help topics match yet. Try 'views', 'search', or 'syntax'.")
                .block(
                    Block::default()
                        .title(styled_title("Topics", PALETTE.sky))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(PALETTE.border))
                        .style(Style::default().bg(PALETTE.surface))
                        .padding(Padding::horizontal(1)),
                )
                .style(Style::default().fg(PALETTE.muted))
                .wrap(Wrap { trim: false }),
            columns[0],
        );
    } else {
        let items = topics
            .iter()
            .map(|topic| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        topic.title(),
                        Style::default()
                            .fg(PALETTE.text)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(topic.summary(), Style::default().fg(PALETTE.muted)),
                ]))
            })
            .collect::<Vec<_>>();
        let mut state = ListState::default();
        state.select(Some(help.selected.min(topics.len() - 1)));
        frame.render_stateful_widget(
            List::new(items)
                .block(
                    Block::default()
                        .title(styled_title("Topics", PALETTE.sky))
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
            columns[0],
            &mut state,
        );
    }

    let preview_lines = if let Some(topic) = topics.get(help.selected).copied() {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("topic ", Style::default().fg(PALETTE.muted)),
                Span::styled(
                    topic.title(),
                    Style::default()
                        .fg(PALETTE.sky)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(
                topic.summary(),
                Style::default().fg(PALETTE.text),
            )),
            Line::from(""),
        ];
        lines.extend(topic.body().iter().map(|line| {
            if line.is_empty() {
                Line::from("")
            } else {
                Line::from(Span::styled(*line, Style::default().fg(PALETTE.text)))
            }
        }));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("context ", Style::default().fg(PALETTE.muted)),
            Span::styled(
                help_context_line(app, topic),
                Style::default().fg(PALETTE.warn),
            ),
        ]));
        lines
    } else {
        vec![Line::from(Span::styled(
            "Type to search the built-in guide by topic or capability.",
            Style::default().fg(PALETTE.muted),
        ))]
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
        columns[1],
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            key_hint("type", "filter"),
            separator_span(),
            key_hint("↑↓", "select"),
            separator_span(),
            key_hint("Esc", "close"),
            separator_span(),
            key_hint("?", "close"),
        ]))
        .alignment(Alignment::Center)
        .style(Style::default().fg(PALETTE.muted)),
        sections[3],
    );
}

#[allow(non_snake_case)]
fn render_mindmap_overlay(
    frame: &mut Frame,
    area: Rect,
    app: &TuiApp,
    overlay: &MindmapOverlayState,
) {
    let PALETTE = app.theme_colors();
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(styled_title("Visual Mindmap", PALETTE.accent))
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
            Constraint::Min(12),
            Constraint::Length(2),
        ])
        .split(inner);

    let scene = app.current_mindmap_scene();
    let camera = scene.camera(
        sections[1].width.max(1),
        sections[1].height.max(1),
        overlay.pan_x,
        overlay.pan_y,
    );

    let headline = if let Some(node) = app.editor.current() {
        format!(
            "Centered on '{}' · respects the current view mode, expanded branches, and active filter context.",
            if node.text.is_empty() {
                "(empty)"
            } else {
                &node.text
            }
        )
    } else {
        "No focused node yet.".to_string()
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    "Bubble View",
                    Style::default()
                        .fg(PALETTE.text)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
                Span::raw("  "),
                Span::styled(headline, Style::default().fg(PALETTE.muted)),
            ]),
            Line::from(vec![
                Span::styled(scene.describe(), Style::default().fg(PALETTE.sky)),
                Span::raw("  "),
                Span::styled(
                    format!("pan {}, {}", overlay.pan_x, overlay.pan_y),
                    Style::default().fg(PALETTE.warn),
                ),
            ]),
        ]),
        sections[0],
    );

    let canvas_block = Block::default()
        .title(styled_title("Map Canvas", PALETTE.sky))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(PALETTE.sky))
        .style(Style::default().bg(PALETTE.background));
    let canvas_inner = canvas_block.inner(sections[1]);
    frame.render_widget(canvas_block, sections[1]);
    frame.render_widget(
        MindmapWidget::new(&scene, camera, app.mindmap_theme()),
        canvas_inner,
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            key_hint("↑↓←→", "pan"),
            separator_span(),
            key_hint("0", "recenter"),
            separator_span(),
            key_hint("p", "export png"),
            separator_span(),
            key_hint("Esc", "close"),
            separator_span(),
            key_hint("Enter", "close"),
        ]))
        .alignment(Alignment::Center)
        .style(Style::default().fg(PALETTE.muted)),
        sections[2],
    );
}

#[allow(non_snake_case)]
fn render_palette_overlay(frame: &mut Frame, area: Rect, app: &TuiApp, palette: &PaletteState) {
    let PALETTE = app.theme_colors();
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(styled_title("Command Palette", PALETTE.accent))
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
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(2),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    if app.ui_settings.ascii_accents {
                        "// One Entry Point //"
                    } else {
                        "One Entry Point"
                    },
                    Style::default()
                        .fg(PALETTE.text)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
                Span::raw("  "),
                Span::styled(
                    "Jump to nodes, browse recent actions, restore manual checkpoints or safety snapshots, preview themes and surface settings, open saved views, or find the right help topic.",
                    Style::default().fg(PALETTE.muted),
                ),
            ]),
            Line::from(Span::styled(
                format!("Current view: {}.", app.view_mode.label()),
                Style::default().fg(PALETTE.sky),
            )),
        ]),
        sections[0],
    );

    let input_block = Block::default()
        .title(styled_title(
            "Query",
            attention_border(
                PALETTE.warn,
                PALETTE.sky,
                PALETTE.accent,
                MotionTarget::PaletteInput,
            ),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(attention_border(
            PALETTE.warn,
            PALETTE.sky,
            PALETTE.accent,
            MotionTarget::PaletteInput,
        )))
        .style(Style::default().bg(attention_fill(
            PALETTE.surface,
            PALETTE.surface_alt,
            PALETTE.surface_alt,
            MotionTarget::PaletteInput,
        )))
        .padding(Padding::horizontal(1));
    let input_inner = input_block.inner(sections[1]);
    frame.render_widget(input_block, sections[1]);
    frame.render_widget(
        Paragraph::new(palette.query.clone())
            .style(Style::default().fg(PALETTE.text))
            .wrap(Wrap { trim: false }),
        input_inner,
    );
    frame.set_cursor_position((input_inner.x + palette.cursor as u16, input_inner.y));

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(sections[2]);

    let items = app.palette_items(&palette.query);
    if items.is_empty() {
        frame.render_widget(
            Paragraph::new(
                "No matches yet. Try 'save', 'paper', 'ascii', 'motion', or a node label like 'tasks'.",
            )
            .block(
                Block::default()
                    .title(styled_title("Results", PALETTE.sky))
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
        let group_starts = palette_group_starts(&items);
        let list_items = items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let mut lines = Vec::new();
                if group_starts[index] {
                    lines.push(palette_group_header_line(item.kind, PALETTE));
                }
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("[{}]", item.kind.label()),
                        Style::default().fg(PALETTE.warn),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        item.title.clone(),
                        Style::default()
                            .fg(PALETTE.text)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(item.subtitle.clone(), Style::default().fg(PALETTE.muted)),
                ]));
                ListItem::new(lines)
            })
            .collect::<Vec<_>>();
        let mut state = ListState::default();
        state.select(Some(palette.selected.min(items.len() - 1)));
        frame.render_stateful_widget(
            List::new(list_items)
                .block(
                    Block::default()
                        .title(styled_title("Results", PALETTE.sky))
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

    let preview_lines = if let Some(item) = items.get(palette.selected) {
        vec![
            Line::from(vec![
                Span::styled("groups ", Style::default().fg(PALETTE.muted)),
                Span::styled(
                    palette_group_summary(&items, palette.selected),
                    Style::default().fg(PALETTE.accent),
                ),
            ]),
            Line::from(vec![
                Span::styled("kind ", Style::default().fg(PALETTE.muted)),
                Span::styled(item.kind.label(), Style::default().fg(PALETTE.warn)),
            ]),
            Line::from(vec![
                Span::styled("title ", Style::default().fg(PALETTE.muted)),
                Span::styled(item.title.clone(), Style::default().fg(PALETTE.text)),
            ]),
            Line::from(vec![
                Span::styled("detail ", Style::default().fg(PALETTE.muted)),
                Span::styled(item.subtitle.clone(), Style::default().fg(PALETTE.sky)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                item.preview.clone(),
                Style::default().fg(PALETTE.text),
            )),
        ]
    } else {
        vec![Line::from(Span::styled(
            "Actions appear first by default. Type to search nodes, recent actions, checkpoints, safety snapshots, saved views, and help.",
            Style::default().fg(PALETTE.muted),
        ))]
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

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            key_hint("↑↓", "select"),
            separator_span(),
            key_hint("Tab", "group"),
            separator_span(),
            key_hint("⇧Tab", "group"),
            separator_span(),
            key_hint("Enter", "run"),
            separator_span(),
            key_hint("Esc", "close"),
            separator_span(),
            key_hint(":", "open"),
            separator_span(),
            key_hint("^P", "open"),
        ]))
        .alignment(Alignment::Center)
        .style(Style::default().fg(PALETTE.muted)),
        sections[3],
    );
}

#[allow(non_snake_case)]
fn render_search_overlay(frame: &mut Frame, area: Rect, app: &TuiApp, search: &SearchOverlayState) {
    let PALETTE = app.theme_colors();
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
            separator_span(),
            key_hint("↑↓", "select"),
            separator_span(),
            key_hint("Enter", "apply"),
            separator_span(),
            key_hint("c", "clear"),
            separator_span(),
            key_hint("Esc", "close"),
        ]))
        .alignment(Alignment::Center)
        .style(Style::default().fg(PALETTE.muted)),
        sections[3],
    );
}

#[allow(non_snake_case)]
fn render_search_section_tabs(frame: &mut Frame, area: Rect, search: &SearchOverlayState) {
    let PALETTE = active_palette();
    let search_attention = motion_level(MotionTarget::SearchActive);
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
                    if search_attention >= 2 {
                        PALETTE.warn
                    } else {
                        PALETTE.accent
                    }
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

#[allow(non_snake_case)]
fn render_search_query_section(
    frame: &mut Frame,
    area: Rect,
    app: &TuiApp,
    search: &SearchOverlayState,
) {
    let PALETTE = app.theme_colors();
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(8),
        ])
        .split(area);

    let input_block = Block::default()
        .title(styled_title(
            "Query",
            attention_border(
                PALETTE.warn,
                PALETTE.sky,
                PALETTE.accent,
                MotionTarget::SearchActive,
            ),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(attention_border(
            PALETTE.warn,
            PALETTE.sky,
            PALETTE.accent,
            MotionTarget::SearchActive,
        )))
        .style(Style::default().bg(attention_fill(
            PALETTE.surface,
            PALETTE.surface_alt,
            PALETTE.surface_alt,
            MotionTarget::SearchActive,
        )))
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

#[allow(non_snake_case)]
fn render_search_facets_section(
    frame: &mut Frame,
    area: Rect,
    app: &TuiApp,
    search: &SearchOverlayState,
) {
    let PALETTE = app.theme_colors();
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

#[allow(non_snake_case)]
fn render_search_views_section(
    frame: &mut Frame,
    area: Rect,
    app: &TuiApp,
    search: &SearchOverlayState,
) {
    let PALETTE = app.theme_colors();
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
            "Draft query: '{}' ({} matching nodes) · view {}",
            search.draft_query.trim(),
            count,
            app.view_mode.status_label()
        );
    }

    if let Some(filter) = &app.filter {
        return format!(
            "Active filter: '{}' ({} matching nodes) · view {}",
            filter.query.raw(),
            filter.matches.len(),
            app.view_mode.status_label()
        );
    }

    if app.view_mode == ViewMode::SubtreeOnly
        && let Some(root) = app.subtree_root_node()
    {
        return format!(
            "Subtree rooted at '{}' · view {}",
            if root.text.is_empty() {
                "(empty)"
            } else {
                root.text.as_str()
            },
            app.view_mode.status_label()
        );
    }

    format!(
        "Whole map ({} nodes) · view {}",
        count_nodes(&app.editor.document().nodes),
        app.view_mode.status_label()
    )
}

fn help_context_line(app: &TuiApp, topic: HelpTopic) -> String {
    match topic {
        HelpTopic::Navigation => {
            let breadcrumb = if app.editor.breadcrumb().is_empty() {
                "(no focus)".to_string()
            } else {
                app.editor.breadcrumb().join(" / ")
            };
            format!("focus path {breadcrumb}")
        }
        HelpTopic::Editing => {
            if app.autosave {
                "autosave is on, so structural edits write immediately".to_string()
            } else {
                "manual save mode is active, so press s after edits".to_string()
            }
        }
        HelpTopic::Search => match &app.filter {
            Some(filter) => format!(
                "active filter {} with {} direct matches",
                filter.query.raw(),
                filter.matches.len()
            ),
            None => "no active filter yet".to_string(),
        },
        HelpTopic::Views => {
            let mut context = format!("current mode {}", app.view_mode.label());
            if app.view_mode == ViewMode::SubtreeOnly
                && let Some(root) = app.subtree_root_node()
            {
                context.push_str(" rooted at ");
                context.push_str(if root.text.is_empty() {
                    "(empty)"
                } else {
                    root.text.as_str()
                });
            }
            context
        }
        HelpTopic::Themes => format!(
            "current theme {}, motion {}, accents {} stored next to the map",
            app.ui_settings.theme.label(),
            if app.ui_settings.motion_enabled {
                "on"
            } else {
                "off"
            },
            if app.ui_settings.ascii_accents {
                "ascii"
            } else {
                "default"
            }
        ),
        HelpTopic::Mindmap => {
            if app.filter.is_some() {
                "the visual map follows the current filtered working set".to_string()
            } else {
                format!(
                    "the visual map follows the current {} projection",
                    app.view_mode.label()
                )
            }
        }
        HelpTopic::Syntax => app
            .editor
            .current()
            .map(|node| {
                format!(
                    "current node line {} is ready for inline syntax edits",
                    node.line
                )
            })
            .unwrap_or_else(|| "create a node first, then add inline tags or ids".to_string()),
    }
}

#[derive(Debug, Clone)]
struct PaletteNodeEntry {
    path: Vec<usize>,
    primary: String,
    secondary: String,
    preview: String,
    haystack: String,
}

fn palette_kind_rank(kind: PaletteItemKind) -> u8 {
    match kind {
        PaletteItemKind::Action => 0,
        PaletteItemKind::Theme => 1,
        PaletteItemKind::Setting => 2,
        PaletteItemKind::History => 3,
        PaletteItemKind::Checkpoint => 4,
        PaletteItemKind::Safety => 5,
        PaletteItemKind::Node => 6,
        PaletteItemKind::SavedView => 7,
        PaletteItemKind::Help => 8,
    }
}

fn palette_group_starts(items: &[PaletteItem]) -> Vec<bool> {
    let mut starts = Vec::with_capacity(items.len());
    let mut previous_kind = None;
    for item in items {
        let starts_group = previous_kind != Some(item.kind);
        starts.push(starts_group);
        previous_kind = Some(item.kind);
    }
    starts
}

fn next_palette_group_index(items: &[PaletteItem], selected: usize) -> usize {
    if items.is_empty() {
        return 0;
    }
    let group_starts = palette_group_starts(items);
    let selected = selected.min(items.len() - 1);
    for (index, starts_group) in group_starts.iter().enumerate().skip(selected + 1) {
        if *starts_group {
            return index;
        }
    }
    selected
}

fn previous_palette_group_index(items: &[PaletteItem], selected: usize) -> usize {
    if items.is_empty() {
        return 0;
    }
    let group_starts = palette_group_starts(items);
    let selected = selected.min(items.len() - 1);
    for index in (0..selected).rev() {
        if group_starts[index] {
            return index;
        }
    }
    0
}

fn palette_group_summary(items: &[PaletteItem], selected: usize) -> String {
    if items.is_empty() {
        return "No groups".to_string();
    }

    let selected = selected.min(items.len() - 1);
    let mut groups = Vec::new();
    for item in items {
        if groups.last().copied() != Some(item.kind) {
            groups.push(item.kind);
        }
    }

    let current_kind = items[selected].kind;
    groups
        .into_iter()
        .map(|kind| {
            if kind == current_kind {
                format!("[{}]", kind.label())
            } else {
                kind.label().to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

fn palette_group_header_line(kind: PaletteItemKind, palette: Palette) -> Line<'static> {
    let label = if ascii_accents_enabled() {
        format!("// {} //", kind.label())
    } else {
        kind.label().to_uppercase()
    };
    let divider = if ascii_accents_enabled() {
        "----------------"
    } else {
        "················"
    };
    Line::from(vec![
        Span::styled(
            label,
            Style::default()
                .fg(palette.sky)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(divider, Style::default().fg(palette.border)),
    ])
}

fn palette_match_score(query: &str, primary: &str, haystack: &str) -> Option<i64> {
    if query.is_empty() {
        return Some(100);
    }

    let primary = primary.to_lowercase();
    let haystack = haystack.to_lowercase();
    if !query
        .split_whitespace()
        .all(|token| haystack.contains(token))
    {
        return None;
    }

    let mut score = 200;
    if primary == query {
        score += 600;
    } else if primary.starts_with(query) {
        score += 450;
    } else if primary.contains(query) {
        score += 300;
    }
    if haystack.contains(query) {
        score += 120;
    }
    score -= primary.len() as i64 / 8;
    Some(score)
}

fn collect_palette_nodes(document: &Document) -> Vec<PaletteNodeEntry> {
    let mut entries = Vec::new();
    collect_palette_nodes_from(&document.nodes, &mut entries, Vec::new(), &mut Vec::new());
    entries
}

fn collect_palette_nodes_from(
    nodes: &[Node],
    entries: &mut Vec<PaletteNodeEntry>,
    prefix: Vec<usize>,
    breadcrumb: &mut Vec<String>,
) {
    for (index, node) in nodes.iter().enumerate() {
        let mut path = prefix.clone();
        path.push(index);
        let label = if node.text.is_empty() {
            "(empty)".to_string()
        } else {
            node.text.clone()
        };
        breadcrumb.push(label.clone());
        let breadcrumb_text = breadcrumb.join(" / ");
        let metadata = node
            .metadata
            .iter()
            .map(|entry| format!("@{}:{}", entry.key, entry.value))
            .collect::<Vec<_>>()
            .join(" ");
        let id = node.id.clone().unwrap_or_default();
        let secondary = if id.is_empty() {
            breadcrumb_text.clone()
        } else {
            id.clone()
        };
        let haystack = format!(
            "{} {} {} {} {}",
            label,
            breadcrumb_text,
            id,
            node.tags.join(" "),
            metadata
        );
        entries.push(PaletteNodeEntry {
            path: path.clone(),
            primary: label,
            secondary,
            preview: breadcrumb_text,
            haystack,
        });
        collect_palette_nodes_from(&node.children, entries, path, breadcrumb);
        breadcrumb.pop();
    }
}

fn node_label_for_document(document: &Document, path: &[usize]) -> String {
    get_node(&document.nodes, path)
        .map(|node| {
            if node.text.is_empty() {
                "(empty)".to_string()
            } else {
                node.text.clone()
            }
        })
        .unwrap_or_else(|| "(missing focus)".to_string())
}

fn is_automatic_checkpoint(checkpoint: &Checkpoint) -> bool {
    checkpoint.name.starts_with("Safety checkpoint:")
        || checkpoint.name.starts_with("Before ")
        || checkpoint.name.starts_with("Safety · ")
}

fn checkpoint_palette_title(checkpoint: &Checkpoint, automatic: bool) -> String {
    if automatic {
        let detail = checkpoint
            .name
            .strip_prefix("Safety checkpoint:")
            .unwrap_or(&checkpoint.name)
            .trim();
        format!("Safety · {detail}")
    } else {
        format!("Checkpoint · {}", checkpoint.name)
    }
}

fn checkpoint_palette_subtitle(checkpoint: &Checkpoint, automatic: bool) -> String {
    let origin = if automatic {
        "automatic safety snapshot"
    } else {
        "manual snapshot"
    };
    format!(
        "{origin} · {}{}",
        ViewMode::from(checkpoint.view_mode).label(),
        checkpoint
            .filter_query
            .as_ref()
            .map(|filter| format!(" · {filter}"))
            .unwrap_or_default()
    )
}

fn snapshot_preview(snapshot: &WorkspaceSnapshot) -> String {
    let focus = node_label_for_document(&snapshot.editor.document, &snapshot.editor.focus_path);
    let filter = snapshot.filter_query.as_deref().unwrap_or("none");
    let dirty = if snapshot.editor.dirty {
        "modified"
    } else {
        "saved"
    };
    let mut lines = vec![
        "Restores this recent map state.".to_string(),
        format!("Focus lands on '{focus}'."),
        format!(
            "View {} · filter {} · state {} · {} nodes.",
            snapshot.view_mode.label(),
            filter,
            dirty,
            count_nodes(&snapshot.editor.document.nodes)
        ),
    ];
    if snapshot.view_mode == ViewMode::SubtreeOnly
        && let Some(root) = &snapshot.subtree_root
    {
        lines.push(format!(
            "Subtree root '{}'.",
            node_label_for_document(&snapshot.editor.document, &root.path)
        ));
    }
    lines.join("\n")
}

fn workspace_change_summary(current: &WorkspaceSnapshot, target: &WorkspaceSnapshot) -> String {
    let current_focus =
        node_label_for_document(&current.editor.document, &current.editor.focus_path);
    let target_focus = node_label_for_document(&target.editor.document, &target.editor.focus_path);
    let current_filter = current.filter_query.as_deref().unwrap_or("none");
    let target_filter = target.filter_query.as_deref().unwrap_or("none");

    let focus_change = if current_focus == target_focus {
        format!("focus stays '{current_focus}'")
    } else {
        format!("focus '{current_focus}' -> '{target_focus}'")
    };
    let view_change = if current.view_mode == target.view_mode {
        format!("view stays {}", current.view_mode.label())
    } else {
        format!(
            "view {} -> {}",
            current.view_mode.label(),
            target.view_mode.label()
        )
    };
    let filter_change = if current_filter == target_filter {
        format!("filter stays {current_filter}")
    } else {
        format!("filter {current_filter} -> {target_filter}")
    };

    format!("Changes from current: {focus_change}; {view_change}; {filter_change}.")
}

fn history_entry_preview(
    direction: &str,
    steps: usize,
    current: &WorkspaceSnapshot,
    entry: &HistoryEntry,
) -> String {
    format!(
        "{direction} {steps} step{} to '{}'.\n{}\n{}\nThe current state stays recoverable from the opposite history stack.",
        if steps == 1 { "" } else { "s" },
        entry.label,
        snapshot_preview(&entry.snapshot),
        workspace_change_summary(current, &entry.snapshot)
    )
}

fn checkpoint_preview(current: &WorkspaceSnapshot, checkpoint: &Checkpoint) -> String {
    let automatic = is_automatic_checkpoint(checkpoint);
    let focus = node_label_for_document(&checkpoint.document, &checkpoint.focus_path);
    let filter = checkpoint.filter_query.as_deref().unwrap_or("none");
    let dirty = if checkpoint.dirty {
        "modified"
    } else {
        "saved"
    };
    let mut lines = vec![
        if automatic {
            "Restore this automatic safety checkpoint.".to_string()
        } else {
            "Restore this manual checkpoint.".to_string()
        },
        format!("Focus lands on '{focus}'."),
        format!(
            "View {} · filter {} · state {} · {} nodes.",
            ViewMode::from(checkpoint.view_mode).label(),
            filter,
            dirty,
            count_nodes(&checkpoint.document.nodes)
        ),
    ];
    if checkpoint.view_mode == CheckpointViewMode::SubtreeOnly
        && let Some(root) = &checkpoint.subtree_root
    {
        lines.push(format!(
            "Subtree root '{}'.",
            node_label_for_document(&checkpoint.document, &root.path)
        ));
    }
    let target = WorkspaceSnapshot {
        editor: EditorState {
            document: checkpoint.document.clone(),
            focus_path: checkpoint.focus_path.clone(),
            dirty: checkpoint.dirty,
        },
        expanded: checkpoint.expanded_paths.iter().cloned().collect(),
        view_mode: checkpoint.view_mode.into(),
        subtree_root: checkpoint.subtree_root.as_ref().map(|anchor| PathAnchor {
            path: anchor.path.clone(),
            id: anchor.id.clone(),
        }),
        filter_query: checkpoint.filter_query.clone(),
    };
    lines.push(workspace_change_summary(current, &target));
    lines.push(if automatic {
        "This snapshot was captured automatically before a structural change.".to_string()
    } else {
        "This snapshot was named intentionally and kept as a manual checkpoint.".to_string()
    });
    lines.push("The current state can still be undone after restore.".to_string());
    lines.join("\n")
}

fn query_preview_matches(app: &TuiApp, raw: &str) -> Vec<(String, String)> {
    find_matches(app.editor.document(), raw)
        .into_iter()
        .map(|entry| {
            let detail = entry.id.unwrap_or(entry.breadcrumb);
            (entry.text, detail)
        })
        .collect()
}

#[allow(non_snake_case)]
fn render_prompt_overlay(frame: &mut Frame, area: Rect, prompt: &PromptState, app: &TuiApp) {
    let PALETTE = app.theme_colors();
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

fn filter_visible_paths(filter: &ActiveFilter) -> HashSet<Vec<usize>> {
    let mut visible = HashSet::new();
    for path in &filter.matches {
        for index in 0..path.len() {
            visible.insert(path[..=index].to_vec());
        }
    }
    visible
}

fn is_path_prefix(prefix: &[usize], path: &[usize]) -> bool {
    prefix.len() <= path.len() && prefix == &path[..prefix.len()]
}

fn is_sibling_of(path: &[usize], target: &[usize]) -> bool {
    path.len() == target.len()
        && !path.is_empty()
        && path != target
        && path[..path.len() - 1] == target[..target.len() - 1]
}

fn is_focus_branch_context(path: &[usize], focus_path: &[usize]) -> bool {
    if focus_path.is_empty() {
        return true;
    }

    if is_path_prefix(path, focus_path) || is_path_prefix(focus_path, path) {
        return true;
    }

    ancestor_paths(focus_path)
        .into_iter()
        .any(|ancestor| is_sibling_of(path, &ancestor))
}

fn visible_in_view_mode(
    path: &[usize],
    focus_path: &[usize],
    filter_visible_paths: Option<&HashSet<Vec<usize>>>,
    view_mode: ViewMode,
) -> bool {
    let in_filter_scope = filter_visible_paths.is_none_or(|paths| paths.contains(path));

    match view_mode {
        ViewMode::FullMap => in_filter_scope,
        ViewMode::FocusBranch => in_filter_scope && is_focus_branch_context(path, focus_path),
        ViewMode::SubtreeOnly => in_filter_scope && is_path_prefix(focus_path, path),
        ViewMode::FilteredFocus => match filter_visible_paths {
            Some(paths) => paths.contains(path) || is_focus_branch_context(path, focus_path),
            None => is_focus_branch_context(path, focus_path),
        },
    }
}

fn visible_row_depth(path: &[usize], focus_path: &[usize], view_mode: ViewMode) -> usize {
    match view_mode {
        ViewMode::SubtreeOnly => path.len().saturating_sub(focus_path.len()),
        _ => path.len().saturating_sub(1),
    }
}

fn row_is_dimmed(path: &[usize], focus_path: &[usize], view_mode: ViewMode) -> bool {
    match view_mode {
        ViewMode::FullMap | ViewMode::SubtreeOnly => false,
        ViewMode::FocusBranch | ViewMode::FilteredFocus => {
            !(is_path_prefix(path, focus_path) || is_path_prefix(focus_path, path))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScopeHandoffRole {
    None,
    Branch,
    Root,
}

fn scope_handoff_role(
    path: &[usize],
    scope_root: &[usize],
    view_mode: ViewMode,
) -> ScopeHandoffRole {
    match view_mode {
        ViewMode::FullMap => ScopeHandoffRole::None,
        ViewMode::SubtreeOnly => {
            if path == scope_root {
                ScopeHandoffRole::Root
            } else if is_path_prefix(scope_root, path) {
                ScopeHandoffRole::Branch
            } else {
                ScopeHandoffRole::None
            }
        }
        ViewMode::FocusBranch | ViewMode::FilteredFocus => {
            if path == scope_root {
                ScopeHandoffRole::Root
            } else if is_path_prefix(path, scope_root) || is_path_prefix(scope_root, path) {
                ScopeHandoffRole::Branch
            } else {
                ScopeHandoffRole::None
            }
        }
    }
}

fn collect_visible_rows(
    nodes: &[Node],
    expanded: &HashSet<Vec<usize>>,
    projection: ViewProjection<'_>,
    rows: &mut Vec<VisibleRow>,
    prefix: Vec<usize>,
) -> bool {
    let mut subtree_has_visible_rows = false;
    for (index, node) in nodes.iter().enumerate() {
        let mut path = prefix.clone();
        path.push(index);
        let matched = projection
            .filter
            .is_some_and(|filter| filter.matches.contains(&path));
        let include_children_by_expansion = expanded.contains(&path);
        let mut child_rows = Vec::new();
        let child_has_visible_rows = collect_visible_rows(
            &node.children,
            expanded,
            projection,
            &mut child_rows,
            path.clone(),
        );
        let include_row = visible_in_view_mode(
            &path,
            projection.focus_path,
            projection.filter_visible_paths,
            projection.view_mode,
        );

        if include_row {
            rows.push(VisibleRow {
                path: path.clone(),
                depth: visible_row_depth(&path, projection.focus_path, projection.view_mode),
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
                dimmed: row_is_dimmed(&path, projection.focus_path, projection.view_mode),
            });
        }

        let reveal_child_rows = if include_row {
            match projection.view_mode {
                ViewMode::SubtreeOnly => {
                    include_children_by_expansion || path == projection.focus_path
                }
                _ => {
                    projection.filter.is_some()
                        || include_children_by_expansion
                        || is_path_prefix(&path, projection.focus_path)
                }
            }
        } else {
            child_has_visible_rows
        };

        if reveal_child_rows {
            rows.extend(child_rows);
        }
        subtree_has_visible_rows |= include_row || child_has_visible_rows;
    }
    subtree_has_visible_rows
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
    let palette = active_palette();
    Line::from(Span::styled(
        if ascii_accents_enabled() {
            format!(" // {title} // ")
        } else {
            format!(" {title} ")
        },
        Style::default()
            .fg(palette.background)
            .bg(color)
            .add_modifier(Modifier::BOLD),
    ))
}

fn key_hint<'a>(key: &'a str, meaning: &'a str) -> Span<'a> {
    let palette = active_palette();
    Span::styled(
        if ascii_accents_enabled() {
            format!("[{key}] {meaning}")
        } else {
            format!("{key}:{meaning}")
        },
        Style::default().fg(palette.muted),
    )
}

fn status_chip(label: &str, value: &str, bg: Color, fg: Color) -> Span<'static> {
    Span::styled(
        format!(" {label} {value} "),
        Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
    )
}

fn active_palette() -> Palette {
    ACTIVE_PALETTE.with(|palette| palette.get())
}

fn ascii_accents_enabled() -> bool {
    ACTIVE_ASCII_ACCENTS.with(|enabled| enabled.get())
}

fn motion_level(target: MotionTarget) -> u8 {
    let active_target = ACTIVE_MOTION_TARGET.with(|motion_target| motion_target.get());
    if active_target == Some(target) {
        ACTIVE_MOTION_LEVEL.with(|level| level.get())
    } else {
        0
    }
}

fn attention_border(base: Color, medium: Color, strong: Color, target: MotionTarget) -> Color {
    match motion_level(target) {
        0 => base,
        1 => medium,
        _ => strong,
    }
}

fn attention_fill(base: Color, soft: Color, strong: Color, target: MotionTarget) -> Color {
    match motion_level(target) {
        0 => base,
        1 => soft,
        _ => strong,
    }
}

fn separator_span() -> Span<'static> {
    Span::raw(if ascii_accents_enabled() {
        " | "
    } else {
        " · "
    })
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

#[allow(non_snake_case)]
fn parent_lines(app: &TuiApp) -> Vec<Line<'static>> {
    let PALETTE = app.theme_colors();
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

#[allow(non_snake_case)]
fn peer_lines(app: &TuiApp) -> Vec<Line<'static>> {
    let PALETTE = app.theme_colors();
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

#[allow(non_snake_case)]
fn child_lines(app: &TuiApp) -> Vec<Line<'static>> {
    let PALETTE = app.theme_colors();
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
    use std::path::Path;
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

    fn cleanup_sidecars(map_path: &Path) {
        let settings_path = crate::ui_settings::ui_settings_path_for(map_path)
            .expect("settings path should be derivable");
        if settings_path.exists() {
            std::fs::remove_file(settings_path).ok();
        }
        let session_path =
            crate::session::session_path_for(map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
        let checkpoints_path = crate::checkpoints::checkpoints_path_for(map_path)
            .expect("checkpoints path should be derivable");
        if checkpoints_path.exists() {
            std::fs::remove_file(checkpoints_path).ok();
        }
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
    fn command_palette_opens_and_jumps_to_a_node() {
        let map_path = temp_map_path("palette-node.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE))
            .expect("colon should open the command palette");
        assert!(app.palette.is_some(), "palette should be open");
        for character in "product/tasks".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE))
                .expect("typing should update the palette query");
        }
        let items = app.palette_items(
            &app.palette
                .as_ref()
                .expect("palette should still be open")
                .query,
        );
        assert!(
            !items.is_empty(),
            "palette should produce at least one matching result"
        );
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should execute the selected palette item");

        assert!(
            app.palette.is_none(),
            "palette should close after execution"
        );
        assert_eq!(app.editor.focus_path(), &[0, 1]);

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn command_palette_can_run_an_action() {
        let map_path = temp_map_path("palette-action.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL))
            .expect("ctrl+p should open the command palette");
        for character in "cycle view".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE))
                .expect("typing should update the palette query");
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should run the selected action");

        assert_eq!(app.view_mode, ViewMode::FocusBranch);

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn command_palette_can_open_a_saved_view() {
        let map_path = temp_map_path("palette-view.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState {
                views: vec![SavedView {
                    name: "todo focus".to_string(),
                    query: "#todo".to_string(),
                }],
            },
        );

        app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE))
            .expect("colon should open the command palette");
        for character in "todo focus".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE))
                .expect("typing should update the palette query");
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should open the saved view");

        assert_eq!(
            app.filter.as_ref().map(|filter| filter.query.raw()),
            Some("#todo")
        );
        assert_eq!(
            app.motion_cue.map(|cue| cue.target),
            Some(MotionTarget::FilterResult)
        );
        assert_eq!(
            app.status.text,
            "Opened saved view 'todo focus' and landed on the first of 1 matches."
        );

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn command_palette_can_open_help_topics() {
        let map_path = temp_map_path("palette-help.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE))
            .expect("colon should open the command palette");
        for character in "syntax".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE))
                .expect("typing should update the palette query");
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should open help for the matching topic");

        assert!(app.help.is_some(), "help should open from the palette");
        let topics = app.help_topics(&app.help.as_ref().expect("help should be open").query);
        assert_eq!(topics.first().copied(), Some(HelpTopic::Syntax));
        assert!(app.status.text.contains("Inline Syntax"));

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn searchable_help_filters_to_matching_topic() {
        let map_path = temp_map_path("help-search.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE))
            .expect("question mark should open help");
        for character in "mind".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE))
                .expect("typing should filter help topics");
        }

        let topics = app.help_topics(&app.help.as_ref().expect("help should stay open").query);
        assert_eq!(topics.first().copied(), Some(HelpTopic::Mindmap));

        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .expect("escape should close help");
        assert!(app.help.is_none(), "help should close on escape");

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn searchable_help_indexes_body_text_for_key_value_queries() {
        let map_path = temp_map_path("help-key-value.md");
        let document = sample_document();
        let app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        let key_topics = app.help_topics("key");
        assert!(
            key_topics.contains(&HelpTopic::Syntax),
            "searching for 'key' should find the syntax topic"
        );

        let value_topics = app.help_topics("value");
        assert!(
            value_topics.contains(&HelpTopic::Syntax),
            "searching for 'value' should find the syntax topic"
        );

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn command_palette_can_apply_and_persist_a_theme() {
        let map_path = temp_map_path("palette-theme.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document.clone(),
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE))
            .expect("colon should open the command palette");
        for character in "paper".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE))
                .expect("typing should search themes");
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should apply the theme");

        assert_eq!(app.ui_settings.theme, ThemeId::Paper);
        let loaded_settings =
            load_ui_settings_for(&map_path).expect("ui settings should load after theme change");
        assert_eq!(loaded_settings.theme, ThemeId::Paper);

        let reloaded = TuiApp::new_with_settings(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
            loaded_settings,
        );
        assert_eq!(
            reloaded.theme_colors().background,
            ThemeId::Paper.theme().background
        );

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn command_palette_theme_preview_reverts_on_escape() {
        let map_path = temp_map_path("palette-theme-preview.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE))
            .expect("colon should open the command palette");
        for character in "paper".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE))
                .expect("typing should preview themes");
        }
        assert_eq!(app.ui_settings.theme, ThemeId::Paper);

        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .expect("escape should close the palette");

        assert!(app.palette.is_none(), "palette should close on escape");
        assert_eq!(app.ui_settings.theme, ThemeId::Workbench);
        let loaded_settings =
            load_ui_settings_for(&map_path).expect("ui settings should still load");
        assert_eq!(loaded_settings.theme, ThemeId::Workbench);

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn command_palette_can_toggle_ascii_accents_and_persist_them() {
        let map_path = temp_map_path("palette-ascii.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE))
            .expect("colon should open the command palette");
        for character in "ascii".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE))
                .expect("typing should search surface settings");
        }
        assert!(
            app.ui_settings.ascii_accents,
            "selection should preview accents"
        );

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should commit the setting");

        assert!(app.ui_settings.ascii_accents);
        let loaded_settings =
            load_ui_settings_for(&map_path).expect("ui settings should load after accent change");
        assert!(loaded_settings.ascii_accents);

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn motion_setting_prevents_attention_cues_when_disabled() {
        let map_path = temp_map_path("palette-motion.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE))
            .expect("colon should open the command palette");
        for character in "motion".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE))
                .expect("typing should search surface settings");
        }
        assert!(
            !app.ui_settings.motion_enabled,
            "selection should preview motion off"
        );

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should commit motion off");
        assert!(!app.ui_settings.motion_enabled);

        app.cycle_view_mode(true);
        assert!(
            app.motion_cue.is_none(),
            "view changes should stay static when motion is disabled"
        );

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn focus_navigation_sets_a_focus_attention_cue() {
        let map_path = temp_map_path("focus-motion.md");
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
            .expect("down should move focus");

        assert_eq!(
            app.motion_cue.map(|cue| cue.target),
            Some(MotionTarget::Focus)
        );

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn opening_palette_sets_an_input_attention_cue() {
        let map_path = temp_map_path("palette-motion-cue.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE))
            .expect("colon should open the command palette");

        assert_eq!(
            app.motion_cue.map(|cue| cue.target),
            Some(MotionTarget::PaletteInput)
        );

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn view_mode_change_sets_a_scope_attention_cue() {
        let map_path = temp_map_path("scope-motion-cue.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0, 0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE))
            .expect("v should cycle to focus branch");

        assert_eq!(
            app.motion_cue.map(|cue| cue.target),
            Some(MotionTarget::Scope)
        );

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn applying_a_filter_sets_a_filter_result_attention_cue() {
        let map_path = temp_map_path("filter-motion-cue.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.apply_filter("#todo")
            .expect("filter application should succeed");

        assert_eq!(
            app.motion_cue.map(|cue| cue.target),
            Some(MotionTarget::FilterResult)
        );
        assert_eq!(app.editor.focus_path(), &[0, 1]);

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn moving_between_matches_keeps_filter_result_attention() {
        let map_path = temp_map_path("filter-match-motion-cue.md");
        let document = parse_document(
            "- Product [id:product]\n  - Todo A #todo [id:product/a]\n  - Todo B #todo [id:product/b]\n",
        )
        .document;
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.apply_filter("#todo")
            .expect("filter application should succeed");
        app.move_match(1)
            .expect("moving to next match should succeed");

        assert_eq!(
            app.motion_cue.map(|cue| cue.target),
            Some(MotionTarget::FilterResult)
        );
        assert_eq!(app.editor.focus_path(), &[0, 1]);

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn scope_handoff_role_targets_the_surviving_branch() {
        assert_eq!(
            scope_handoff_role(&[0, 0], &[0, 0], ViewMode::SubtreeOnly),
            ScopeHandoffRole::Root
        );
        assert_eq!(
            scope_handoff_role(&[0, 0, 1], &[0, 0], ViewMode::SubtreeOnly),
            ScopeHandoffRole::Branch
        );
        assert_eq!(
            scope_handoff_role(&[0, 1], &[0, 0], ViewMode::SubtreeOnly),
            ScopeHandoffRole::None
        );
        assert_eq!(
            scope_handoff_role(&[0], &[0, 0], ViewMode::FocusBranch),
            ScopeHandoffRole::Branch
        );
        assert_eq!(
            scope_handoff_role(&[0, 0, 2], &[0, 0], ViewMode::FocusBranch),
            ScopeHandoffRole::Branch
        );
        assert_eq!(
            scope_handoff_role(&[0, 1], &[0, 0], ViewMode::FocusBranch),
            ScopeHandoffRole::None
        );
    }

    #[test]
    fn status_model_tracks_overlay_and_filter_context() {
        let map_path = temp_map_path("status-model.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        assert_eq!(app.ui_settings.theme, ThemeId::Workbench);
        assert!(app.ui_settings.motion_enabled);
        assert!(!app.ui_settings.ascii_accents);

        app.open_search_overlay(SearchSection::Query);
        app.search.as_mut().expect("search should open").draft_query = "#todo".to_string();
        app.search.as_mut().expect("search should open").cursor = "#todo".len();
        let draft = app.status_model();
        assert_eq!(draft.scope_label, "Draft '#todo'");

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should apply the query");
        let filtered = app.status_model();
        assert_eq!(filtered.filter_summary.as_deref(), Some("#todo (1)"));
        assert!(filtered.scope_label.contains("Active filter"));

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn status_model_tracks_subtree_scope_and_focus_id() {
        let map_path = temp_map_path("status-subtree.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0, 0],
            None,
            true,
            SavedViewsState::default(),
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE))
            .expect("v should cycle to focus branch");
        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE))
            .expect("v should cycle to subtree only");

        let status = app.status_model();
        assert_eq!(app.view_mode, ViewMode::SubtreeOnly);
        assert!(app.autosave);
        assert_eq!(status.focus_id.as_deref(), Some("product/direction"));
        assert!(status.scope_label.contains("Subtree rooted at 'Direction'"));

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn view_mode_cycle_reprojects_visible_rows() {
        let map_path = temp_map_path("views.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0, 0],
            None,
            false,
            SavedViewsState::default(),
        );

        let full_rows = app.visible_rows();
        assert_eq!(full_rows[0].text, "Product Idea");
        assert_eq!(full_rows[1].text, "Direction");
        assert_eq!(full_rows[2].text, "CLI-first MVP");
        assert_eq!(full_rows[3].text, "Tasks");

        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE))
            .expect("v should cycle to focus branch");
        assert_eq!(app.view_mode, ViewMode::FocusBranch);
        let focus_rows = app.visible_rows();
        assert_eq!(
            focus_rows
                .iter()
                .map(|row| row.text.as_str())
                .collect::<Vec<_>>(),
            vec!["Product Idea", "Direction", "CLI-first MVP", "Tasks"]
        );
        assert!(focus_rows[3].dimmed, "peer branch should be dimmed");

        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE))
            .expect("v should cycle to subtree only");
        assert_eq!(app.view_mode, ViewMode::SubtreeOnly);
        let subtree_rows = app.visible_rows();
        assert_eq!(
            subtree_rows
                .iter()
                .map(|row| (row.text.as_str(), row.depth))
                .collect::<Vec<_>>(),
            vec![("Direction", 0), ("CLI-first MVP", 1)]
        );

        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .expect("escape should return to full map before clearing filters");
        assert_eq!(app.view_mode, ViewMode::FullMap);

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn subtree_only_keeps_a_stable_root_for_navigation() {
        let map_path = temp_map_path("subtree-root.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0, 0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE))
            .expect("v should cycle to focus branch");
        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE))
            .expect("v should cycle to subtree only");
        assert_eq!(app.view_mode, ViewMode::SubtreeOnly);
        assert_eq!(app.subtree_root_path(), Some(vec![0, 0]));

        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
            .expect("down should move inside the subtree");
        assert_eq!(app.editor.focus_path(), &[0, 0, 0]);
        assert_eq!(app.subtree_root_path(), Some(vec![0, 0]));

        app.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE))
            .expect("g should return to the subtree root");
        assert_eq!(app.editor.focus_path(), &[0, 0]);

        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE))
            .expect("left should collapse the subtree root first");
        assert_eq!(app.editor.focus_path(), &[0, 0]);

        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE))
            .expect("left should not escape above the subtree root");
        assert_eq!(app.editor.focus_path(), &[0, 0]);
        assert_eq!(
            app.status.text,
            "Already at the subtree root. Press Esc to leave the isolated branch."
        );

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn mindmap_scene_follows_the_active_view_mode() {
        let map_path = temp_map_path("mindmap-view-mode.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0, 0],
            None,
            false,
            SavedViewsState::default(),
        );

        let full_scene = app.current_mindmap_scene();
        assert!(
            full_scene
                .bubbles
                .iter()
                .any(|bubble| bubble.path == vec![0, 1]),
            "full map should include the sibling branch"
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE))
            .expect("v should cycle to focus branch");
        let focus_scene = app.current_mindmap_scene();
        assert!(
            focus_scene
                .bubbles
                .iter()
                .any(|bubble| bubble.path == vec![0, 1]),
            "focus branch should still include the peer branch"
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE))
            .expect("v should cycle to subtree only");
        let subtree_scene = app.current_mindmap_scene();
        assert!(
            !subtree_scene
                .bubbles
                .iter()
                .any(|bubble| bubble.path == vec![0, 1]),
            "subtree only should hide sibling branches from the visual map"
        );
        assert!(
            subtree_scene
                .bubbles
                .iter()
                .any(|bubble| bubble.path == vec![0, 0]),
            "subtree root should remain visible in the visual map"
        );

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn clearing_filter_reverts_filtered_focus_to_focus_branch() {
        let map_path = temp_map_path("filtered-focus.md");
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

        app.handle_key(KeyEvent::new(KeyCode::Char('V'), KeyModifiers::NONE))
            .expect("V should reverse-cycle to filtered focus");
        assert_eq!(app.view_mode, ViewMode::FilteredFocus);

        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE))
            .expect("c should clear the filter");
        assert!(app.filter.is_none(), "filter should clear");
        assert_eq!(
            app.view_mode,
            ViewMode::FocusBranch,
            "filtered focus should fall back once the filter is gone"
        );

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
    fn undo_and_redo_restore_structural_changes() {
        let map_path = temp_map_path("undo-redo.md");
        let document = sample_document();
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
            "New Branch #todo [id:product/new]".to_string(),
        );
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should submit");
        assert!(serialize_document(app.editor.document()).contains("New Branch"));

        app.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE))
            .expect("u should undo");
        assert!(!serialize_document(app.editor.document()).contains("New Branch"));
        assert_eq!(app.editor.focus_path(), &[0]);

        app.handle_key(KeyEvent::new(KeyCode::Char('U'), KeyModifiers::NONE))
            .expect("U should redo");
        assert!(serialize_document(app.editor.document()).contains("New Branch"));
        assert_eq!(app.editor.focus_path(), &[0, 2]);

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn undo_in_autosave_updates_the_saved_map() {
        let map_path = temp_map_path("undo-autosave.md");
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
        assert!(
            std::fs::read_to_string(&map_path)
                .expect("autosaved map should be readable")
                .contains("New Branch")
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE))
            .expect("u should undo");

        let saved = std::fs::read_to_string(&map_path).expect("saved map should be readable");
        assert!(!saved.contains("New Branch"));

        cleanup_sidecars(&map_path);
        std::fs::remove_file(map_path).ok();
    }

    #[test]
    fn palette_history_items_preview_recent_undo_state() {
        let map_path = temp_map_path("palette-history-preview.md");
        let document = sample_document();
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
            "New Branch #todo [id:product/new]".to_string(),
        );
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should submit");

        let items = app.palette_history_items("undo");
        assert!(
            !items.is_empty(),
            "history items should include undo entries"
        );
        assert_eq!(items[0].kind, PaletteItemKind::History);
        assert!(items[0].title.contains("Undo"));
        assert!(items[0].preview.contains("Restores this recent map state."));
        assert!(items[0].preview.contains("Focus lands on 'Product Idea'."));
        assert!(items[0].preview.contains("Changes from current:"));
        assert!(
            items[0]
                .preview
                .contains("focus 'New Branch' -> 'Product Idea'")
        );

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn palette_group_headers_start_when_result_kind_changes() {
        let items = vec![
            PaletteItem {
                kind: PaletteItemKind::Action,
                title: "Undo".to_string(),
                subtitle: "1 step back".to_string(),
                preview: String::new(),
                score: 10,
                target: PaletteTarget::Action(PaletteAction::Undo),
            },
            PaletteItem {
                kind: PaletteItemKind::Action,
                title: "Redo".to_string(),
                subtitle: "1 step forward".to_string(),
                preview: String::new(),
                score: 9,
                target: PaletteTarget::Action(PaletteAction::Redo),
            },
            PaletteItem {
                kind: PaletteItemKind::History,
                title: "Undo · Added a child node.".to_string(),
                subtitle: "1 step back".to_string(),
                preview: String::new(),
                score: 8,
                target: PaletteTarget::UndoSteps(1),
            },
            PaletteItem {
                kind: PaletteItemKind::Checkpoint,
                title: "Checkpoint · Planning milestone".to_string(),
                subtitle: "manual snapshot".to_string(),
                preview: String::new(),
                score: 7,
                target: PaletteTarget::Checkpoint(0),
            },
            PaletteItem {
                kind: PaletteItemKind::Checkpoint,
                title: "Checkpoint · Design review".to_string(),
                subtitle: "manual snapshot".to_string(),
                preview: String::new(),
                score: 6,
                target: PaletteTarget::Checkpoint(1),
            },
        ];

        assert_eq!(
            palette_group_starts(&items),
            vec![true, false, true, true, false]
        );
    }

    #[test]
    fn palette_group_navigation_jumps_between_group_boundaries() {
        let items = vec![
            PaletteItem {
                kind: PaletteItemKind::Action,
                title: "Undo".to_string(),
                subtitle: "1 step back".to_string(),
                preview: String::new(),
                score: 10,
                target: PaletteTarget::Action(PaletteAction::Undo),
            },
            PaletteItem {
                kind: PaletteItemKind::Action,
                title: "Redo".to_string(),
                subtitle: "1 step forward".to_string(),
                preview: String::new(),
                score: 9,
                target: PaletteTarget::Action(PaletteAction::Redo),
            },
            PaletteItem {
                kind: PaletteItemKind::History,
                title: "Undo · Added a child node.".to_string(),
                subtitle: "1 step back".to_string(),
                preview: String::new(),
                score: 8,
                target: PaletteTarget::UndoSteps(1),
            },
            PaletteItem {
                kind: PaletteItemKind::Checkpoint,
                title: "Checkpoint · Planning milestone".to_string(),
                subtitle: "manual snapshot".to_string(),
                preview: String::new(),
                score: 7,
                target: PaletteTarget::Checkpoint(0),
            },
        ];

        assert_eq!(next_palette_group_index(&items, 0), 2);
        assert_eq!(next_palette_group_index(&items, 2), 3);
        assert_eq!(next_palette_group_index(&items, 3), 3);
        assert_eq!(previous_palette_group_index(&items, 3), 2);
        assert_eq!(previous_palette_group_index(&items, 2), 0);
        assert_eq!(previous_palette_group_index(&items, 0), 0);
    }

    #[test]
    fn palette_group_summary_marks_the_selected_group() {
        let items = vec![
            PaletteItem {
                kind: PaletteItemKind::Action,
                title: "Undo".to_string(),
                subtitle: "1 step back".to_string(),
                preview: String::new(),
                score: 10,
                target: PaletteTarget::Action(PaletteAction::Undo),
            },
            PaletteItem {
                kind: PaletteItemKind::History,
                title: "Undo · Added a child node.".to_string(),
                subtitle: "1 step back".to_string(),
                preview: String::new(),
                score: 8,
                target: PaletteTarget::UndoSteps(1),
            },
            PaletteItem {
                kind: PaletteItemKind::Safety,
                title: "Safety · delete · Tasks".to_string(),
                subtitle: "automatic safety snapshot".to_string(),
                preview: String::new(),
                score: 7,
                target: PaletteTarget::Checkpoint(0),
            },
        ];

        assert_eq!(
            palette_group_summary(&items, 1),
            "Action · [History] · Safety"
        );
    }

    #[test]
    fn palette_distinguishes_manual_checkpoints_from_safety_snapshots() {
        let map_path = temp_map_path("checkpoint-kinds.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0, 1],
            None,
            false,
            SavedViewsState::default(),
        );

        app.save_checkpoint("Planning milestone")
            .expect("manual checkpoint should save");
        app.save_automatic_checkpoint("Safety checkpoint: delete · Tasks")
            .expect("automatic checkpoint should save");

        let checkpoint_items = app.palette_checkpoint_items("checkpoint");
        assert!(
            checkpoint_items
                .iter()
                .any(|item| item.kind == PaletteItemKind::Checkpoint
                    && item.title == "Checkpoint · Planning milestone")
        );
        assert!(
            checkpoint_items
                .iter()
                .any(|item| item.kind == PaletteItemKind::Safety
                    && item.title == "Safety · delete · Tasks")
        );

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn palette_can_jump_back_multiple_steps_through_recent_actions() {
        let map_path = temp_map_path("palette-history-run.md");
        let document = sample_document();
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
            "Branch One #todo [id:product/one]".to_string(),
        );
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should submit");
        app.begin_prompt(
            PromptMode::AddChild,
            "Branch Two #todo [id:product/two]".to_string(),
        );
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should submit");
        assert!(serialize_document(app.editor.document()).contains("Branch Two"));

        app.execute_palette_target(PaletteTarget::UndoSteps(2))
            .expect("palette history restore should succeed");

        let rendered = serialize_document(app.editor.document());
        assert!(!rendered.contains("Branch One"));
        assert!(!rendered.contains("Branch Two"));
        assert_eq!(app.redo_history.len(), 2);
        assert_eq!(
            app.status.text,
            "Undid 2 actions through: Added a child node."
        );

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn delete_creates_an_automatic_checkpoint_that_can_be_restored() {
        let map_path = temp_map_path("checkpoint-delete.md");
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
        app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE))
            .expect("second x should delete");
        assert_eq!(app.checkpoints.checkpoints.len(), 1);
        assert!(
            app.checkpoints.checkpoints[0]
                .name
                .starts_with("Safety checkpoint: delete · ")
        );
        assert!(!serialize_document(app.editor.document()).contains("Tasks #todo"));

        app.restore_checkpoint(0)
            .expect("restoring a checkpoint should succeed");
        assert!(serialize_document(app.editor.document()).contains("Tasks #todo"));
        assert_eq!(
            app.editor.current().expect("focus should exist").text,
            "Tasks"
        );

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn checkpoint_palette_preview_shows_richer_restore_context() {
        let map_path = temp_map_path("checkpoint-preview.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0, 1],
            None,
            false,
            SavedViewsState::default(),
        );

        app.save_checkpoint("Tasks checkpoint")
            .expect("checkpoint should save");

        let items = app.palette_checkpoint_items("tasks checkpoint");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].kind, PaletteItemKind::Checkpoint);
        assert!(items[0].preview.contains("Restore this manual checkpoint."));
        assert!(items[0].preview.contains("Focus lands on 'Tasks'."));
        assert!(items[0].preview.contains("Changes from current:"));
        assert!(items[0].preview.contains("focus stays 'Tasks'"));
        assert!(items[0].preview.contains("view stays Full Map"));
        assert!(
            items[0]
                .preview
                .contains("This snapshot was named intentionally and kept as a manual checkpoint.")
        );
        assert!(
            items[0]
                .preview
                .contains("The current state can still be undone after restore.")
        );

        cleanup_sidecars(&map_path);
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
    fn mindmap_overlay_opens_pans_and_closes() {
        let map_path = temp_map_path("mindmap.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0, 0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE))
            .expect("m should open the mindmap overlay");
        assert!(app.mindmap.is_some(), "mindmap overlay should be active");

        app.handle_key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE))
            .expect("right should pan the overlay");
        assert_eq!(
            app.mindmap.as_ref().map(|overlay| overlay.pan_x),
            Some(6),
            "panning should update the camera offset"
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('0'), KeyModifiers::NONE))
            .expect("0 should recenter the overlay");
        assert_eq!(
            app.mindmap
                .as_ref()
                .map(|overlay| (overlay.pan_x, overlay.pan_y)),
            Some((0, 0))
        );

        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))
            .expect("escape should close the overlay");
        assert!(app.mindmap.is_none(), "mindmap overlay should close");

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
        assert_eq!(
            app.motion_cue.map(|cue| cue.target),
            Some(MotionTarget::FilterResult)
        );
        assert_eq!(
            app.status.text,
            "Opened saved view 'todo' and landed on the first of 1 matches."
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
    fn applying_a_facet_lands_on_the_first_match_with_filter_result_attention() {
        let map_path = temp_map_path("facet-landing.md");
        let document = parse_document(
            "- Product\n  - Todo A #todo [id:product/a]\n  - Todo B #todo [id:product/b]\n  - Backlog\n",
        )
        .document;
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.open_search_overlay(SearchSection::Facets);
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should apply the first available facet");

        assert_eq!(
            app.filter.as_ref().map(|filter| filter.query.raw()),
            Some("#todo")
        );
        assert_eq!(app.editor.focus_path(), &[0, 0]);
        assert_eq!(
            app.motion_cue.map(|cue| cue.target),
            Some(MotionTarget::FilterResult)
        );
        assert_eq!(
            app.status.text,
            "Applied facet #todo and landed on the first of 2 matches."
        );

        cleanup_sidecars(&map_path);
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
