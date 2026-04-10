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
use crate::locations::{
    FrequentLocation, LocationMemoryAnchor, LocationMemoryState, load_locations_for,
    save_locations_for,
};
use crate::mindmap::{
    MindmapWidget, Scene as MindmapScene, Theme as MindmapTheme, default_export_path, export_png,
};
use crate::model::{Document, LinkEntry, Node};
use crate::parser::parse_node_fragment;
use crate::query::{
    FilterQuery, backlinks_to, find_matches, link_entries, metadata_key_counts_for_filter,
    metadata_value_counts_for_filter, tag_counts, tag_counts_for_filter,
};
use crate::serializer::serialize_document;
use crate::session::{load_session_for, resolve_session_focus, save_session_for};
use crate::ui_settings::{ThemeId, UiSettings, load_ui_settings_for, save_ui_settings_for};
use crate::views::{SavedView, SavedViewsState, load_views_for, save_views_for};

const TICK_RATE: Duration = Duration::from_millis(150);
const HISTORY_LIMIT: usize = 64;
const CHECKPOINT_LIMIT: usize = 12;
const RECENT_LOCATION_LIMIT: usize = 16;
const FREQUENT_LOCATION_LIMIT: usize = 16;
const FREQUENT_LOCATION_MIN_VISITS: usize = 3;

type Palette = MindmapTheme;

thread_local! {
    static ACTIVE_PALETTE: Cell<Palette> = Cell::new(ThemeId::Workbench.theme());
    static ACTIVE_ASCII_ACCENTS: Cell<bool> = const { Cell::new(false) };
    static ACTIVE_MINIMAL_MODE: Cell<bool> = const { Cell::new(false) };
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
enum FocusReveal {
    Preserve,
    Reveal,
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
    EditDetail,
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
            Self::EditDetail => "Edit Details",
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
            Self::EditDetail => {
                "Write longer notes for the selected node. Enter adds lines. Ctrl+S saves."
            }
            _ => "Use full node syntax: Label #tag @key:value [id:path/to/node] [[target]]",
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
            Self::Facets => "Browse",
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
    Ids,
}

impl FacetTab {
    fn title(self) -> &'static str {
        match self {
            Self::Tags => "Tags",
            Self::Keys => "Keys",
            Self::Values => "Values",
            Self::Ids => "Ids",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Tags => Self::Keys,
            Self::Keys => Self::Values,
            Self::Values => Self::Ids,
            Self::Ids => Self::Tags,
        }
    }

    fn previous(self) -> Self {
        match self {
            Self::Tags => Self::Ids,
            Self::Keys => Self::Tags,
            Self::Values => Self::Keys,
            Self::Ids => Self::Values,
        }
    }

    fn empty_message(self) -> &'static str {
        match self {
            Self::Tags => "No tags exist in the current scope.",
            Self::Keys => "No metadata keys exist in the current scope.",
            Self::Values => "No metadata values exist in the current scope.",
            Self::Ids => "No deep-link ids exist in the current scope.",
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
    Recipe,
    Setting,
    Relation,
    Inline,
    Frequent,
    Location,
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
            Self::Recipe => "Recipe",
            Self::Setting => "Setting",
            Self::Relation => "Relation",
            Self::Inline => "Inline",
            Self::Frequent => "Frequent",
            Self::Location => "Location",
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
    StartHere,
    Navigation,
    Editing,
    Details,
    Search,
    Views,
    Palette,
    Safety,
    Themes,
    Mindmap,
    Syntax,
    Ids,
    Relations,
}

impl HelpTopic {
    fn title(self) -> &'static str {
        match self {
            Self::StartHere => "Start Here",
            Self::Navigation => "Navigation",
            Self::Editing => "Editing",
            Self::Details => "Node Details",
            Self::Search => "Search And Browse",
            Self::Views => "View Modes",
            Self::Palette => "Command Palette",
            Self::Safety => "Safety And History",
            Self::Themes => "Themes",
            Self::Mindmap => "Visual Mindmap",
            Self::Syntax => "Tags And Metadata",
            Self::Ids => "Ids And Deep Links",
            Self::Relations => "Relations And Backlinks",
        }
    }

    fn summary(self) -> &'static str {
        match self {
            Self::StartHere => "Learn the core mental model and the first few things worth trying.",
            Self::Navigation => "Move through the tree, jump quickly, and open major overlays.",
            Self::Editing => "Add, rename, delete, and reshape branches without leaving the map.",
            Self::Details => "Keep node titles short while storing longer notes under a branch.",
            Self::Search => {
                "Filter by text, browse tags, metadata, and ids, and reopen saved views."
            }
            Self::Views => "Switch between full-map and focused working modes.",
            Self::Palette => {
                "Jump to actions, places, views, relations, and help from one surface."
            }
            Self::Safety => "Undo, redo, checkpoints, autosave, and recent restore history.",
            Self::Themes => "Change the visual surface without leaving the map.",
            Self::Mindmap => "Inspect the current working set visually and export it as a PNG.",
            Self::Syntax => "Use lightweight inline structure for grouping and structured fields.",
            Self::Ids => {
                "Give branches stable addresses for jumps, exports, and deep-linked opens."
            }
            Self::Relations => {
                "Connect distant branches laterally and follow the resulting backlinks."
            }
        }
    }

    fn hint(self) -> &'static str {
        match self {
            Self::StartHere => {
                "Intro guide for first-time users, first steps, and what matters most."
            }
            Self::Navigation => "User guide plus movement keys and large-map wayfinding tips.",
            Self::Editing => "User guide plus editing keys, undo safety, and restructuring tips.",
            Self::Details => {
                "User guide plus detail-line syntax, the d editor, and note-taking tips."
            }
            Self::Search => "User guide plus query keys, browse controls, ids, and filtering tips.",
            Self::Views => "User guide plus view-mode keys and focused-workflow tips.",
            Self::Palette => "User guide plus jump patterns, previews, and help recipes.",
            Self::Safety => "User guide plus undo, checkpoints, autosave, and restore habits.",
            Self::Themes => "User guide plus theme controls and calmer-surface tips.",
            Self::Mindmap => "User guide plus visual-map keys and export tips.",
            Self::Syntax => "User guide plus tags and metadata reference and authoring tips.",
            Self::Ids => "User guide plus id naming, deep-link jumps, and CLI usage.",
            Self::Relations => {
                "User guide plus cross-link syntax, backlinks, and relation navigation."
            }
        }
    }

    fn keywords(self) -> &'static str {
        match self {
            Self::StartHere => {
                "start begin beginner intro getting started first steps first five minutes overview basics new user"
            }
            Self::Navigation => {
                "navigate movement arrows focus jump root open id palette hotkeys relations backlinks related cross links"
            }
            Self::Editing => {
                "edit add delete reshape move indent outdent sibling child root write undo redo checkpoint history"
            }
            Self::Details => {
                "details notes note detail prose quote rationale description long form longer text multiline d editor"
            }
            Self::Search => {
                "search filter query browse fields saved views tags metadata ids deep links matches key keys value values"
            }
            Self::Views => "views focus branch subtree filtered focus isolate branch presentation",
            Self::Palette => {
                "palette command palette actions recent frequent saved views checkpoints help recipes jump launcher"
            }
            Self::Safety => {
                "safety undo redo checkpoint checkpoints autosave save revert restore history recent actions recovery"
            }
            Self::Themes => {
                "theme themes paper blueprint calm violet monograph terminal neon workbench palette ui settings motion ascii accents minimal purple lavender"
            }
            Self::Mindmap => "mindmap visual bubble canvas png export pan recenter map overlay",
            Self::Syntax => {
                "syntax tags metadata key keys value values fields structured attributes inline node format example"
            }
            Self::Ids => {
                "ids id deep links deep link open jump palette links stable address target export cli mdm view open"
            }
            Self::Relations => {
                "relations backlinks cross links cross-links references wiki links rel target source incoming outgoing related graph lateral"
            }
        }
    }

    fn guide_intro(self) -> &'static str {
        match self {
            Self::StartHere => {
                "If you are new to mdmind, keep the mental model small: one line is one node, focus is the center of the UI, and view or search can calm things down when the tree gets large."
            }
            Self::Navigation => {
                "Navigation in mdmind is tree-first. Stay in the outline while exploring, then use the palette or id jumps when the map gets too large to scroll comfortably."
            }
            Self::Editing => {
                "Editing stays inline and reversible. You reshape the tree in place, keep your cursor on the branch you care about, and lean on undo or checkpoints when a structural change feels risky."
            }
            Self::Details => {
                "Node details are for the moments when one line is not enough. They let you keep the tree scannable while still attaching real prose, quotes, rationale, or research context to a branch."
            }
            Self::Search => {
                "Search is the fastest way to cut noise without losing your place. The beginner path is simple: plain text first, then #tags, then @key:value metadata when the map grows more structured."
            }
            Self::Views => {
                "View modes are not just cosmetic. They change how much of the map stays visible so you can switch between orientation, local focus, and filtered work without manually collapsing half the tree."
            }
            Self::Palette => {
                "The command palette is the fastest way to act when you already know what you want. It pulls actions, jumps, saved views, recipes, relations, history, and help into one place."
            }
            Self::Safety => {
                "Safety should make you bolder, not slower. Undo, redo, checkpoints, and restore history are there so you can reshape real maps without treating every edit like a one-shot risk."
            }
            Self::Themes => {
                "Themes and surface settings support different working styles. Use them to make the interface calmer, denser, brighter, or more terminal-native without changing how the map behaves."
            }
            Self::Mindmap => {
                "The visual mindmap works best as a second lens on the current working set, not as a separate mode with a different truth. It follows your active view and filter so the tree and the map stay aligned."
            }
            Self::Syntax => {
                "Tags and metadata are the easiest structured layer to adopt. They give you fast grouping and reliable filtering without forcing you to decide every branch's long-term address or relationship model up front."
            }
            Self::Ids => {
                "Ids are how a branch becomes reliably addressable. They matter when you want to jump by id, deep-link from the CLI, export one subtree, or create a stable target for cross-links."
            }
            Self::Relations => {
                "Relations are for the moments when the tree alone is not enough. They let one branch point to another without changing ownership or turning the whole map into a graph-first document."
            }
        }
    }

    fn guide_body(self) -> &'static [&'static str] {
        match self {
            Self::StartHere => &[
                "You do not need every feature on day one. Start with the core loop: move, add or edit a node, search when the map gets noisy, and use the palette when you know what you want.",
                "A good first map is small and concrete. One project, one trip, one feature area, one story outline. Add ids and relations later, once the shape starts to matter.",
                "If you do not want a blank file, start from mdm init and one of the built-in templates.",
            ],
            Self::Navigation => &[
                "The current focus is the center of the interface. The outline, focus card, and visual map all follow it, so plain arrow movement is enough for a lot of work.",
                "When you already know the branch or id you want, use the palette instead of drilling manually. It is faster and keeps navigation feeling like one surface instead of several.",
            ],
            Self::Editing => &[
                "Most edits should feel like shaping a branch rather than opening a separate editor. Add, rename, move, indent, and delete all happen from the current focus, so your attention stays on structure instead of mode switching.",
                "The safety layer matters here. Undo and checkpoints mean you can move quickly through structural edits without treating every change as dangerous, especially in larger maps where a reparent or delete can have bigger consequences.",
            ],
            Self::Details => &[
                "Details live under a node instead of inside its main label. That means the visible tree can stay compact even when a branch needs a paragraph, quote, scene note, meeting rationale, or a few lines of research context.",
                "In the raw file, detail lines use | ... directly under the node they belong to. In the TUI, press d to edit them in a larger text area, then save with Ctrl+S.",
            ],
            Self::Search => &[
                "Start with the simplest version: press /, type a normal word or phrase, and press Enter. Once that feels natural, move on to #tags and then @key:value filters.",
                "Use query search when you know what you want. Use browse when you want to inspect tags, metadata, or ids that already exist in the current scope. Use saved views when a filter is worth coming back to.",
                "Applying a query, view, or recipe lands you on the first useful match so search still feels like navigation, not a detached result list.",
            ],
            Self::Views => &[
                "Full Map is for orientation. Focus Branch is for working with context. Subtree Only is for treating one branch as a temporary workspace. Filtered Focus is for mixing search results with enough structure to stay oriented.",
                "The important mental model is that view mode changes the visible projection, not the underlying document. You are not rewriting the map when you isolate a branch, only changing what the interface chooses to show you.",
            ],
            Self::Palette => &[
                "Use the palette when you already know what you want: a branch, an id, an action, a saved view, a recent location, a checkpoint, a help topic, or a workflow recipe. It is less about browsing and more about intention.",
                "The palette also makes advanced features feel casual. You do not have to remember a special relation mode, history mode, or settings panel. Typing a few words is often enough to reach the right thing.",
            ],
            Self::Safety => &[
                "Undo and redo restore more than text. They bring back structure, focus, and working context, so it feels like returning to a known workspace instead of just reversing one tiny edit.",
                "Checkpoints are for the moments when you want a named restore point before a bigger restructure. Recent action history is for quick short-range recovery when you only need to step back through the last few changes.",
            ],
            Self::Themes => &[
                "A good theme should reduce fatigue and make hierarchy easier to read. It should not feel like decoration pasted on top. That is why themes apply across the header, outline, overlays, status surfaces, and mindmap together.",
                "Minimal mode belongs here too. It is the pro layout choice when you want less instructional chrome and more working room. It condenses the shell, widens the main outline, and trims the right-side context lanes down to the essentials.",
            ],
            Self::Mindmap => &[
                "The mindmap is most useful for cluster recognition, branch shape, and presentation. It is less about direct editing and more about seeing the current scope when the outline stops being enough.",
                "Because it follows filters and view modes, it works best after you isolate a branch or apply a focused query. Visible cross-links also draw relation edges there, so lateral structure stays visible without taking over the whole document.",
            ],
            Self::Syntax => &[
                "Use tags for quick grouping and metadata for fields you expect to query repeatedly. A few stable patterns like #todo, @status:active, and @owner:mira are usually more valuable than lots of one-off annotations.",
                "Keep the visible label human first, then layer structure onto it. The line should still read well as plain text even if someone ignores the tags and metadata completely.",
            ],
            Self::Ids => &[
                "Not every node needs an id. Add ids to branches you expect to revisit, deep-link, export, or reference from somewhere else. That usually means major branches, durable work items, named sections, or cross-map anchors.",
                "The payoff is consistency. Palette jumps, mdmind and mdm deep-link opens, export targets, saved references, and cross-links all get more reliable when a branch has a stable id instead of depending on its visible label.",
            ],
            Self::Relations => &[
                "A plain relation like [[target/id]] says two branches are connected. A typed relation like [[rel:blocks->target/id]] says why they are connected. Start with the plain form most of the time.",
                "Backlinks are derived from incoming relations, so you do not maintain them manually. If one branch points at another, the target can surface that incoming reference for you.",
            ],
        }
    }

    fn command_reference(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::StartHere => &[
                ("↑ / ↓", "Move through visible nodes"),
                ("a / e", "Add a child or edit the current node"),
                ("/", "Search by text, tags, or metadata"),
                (": / Ctrl+P", "Open the command palette"),
                ("?", "Open built-in help"),
                ("m", "Open the visual mindmap"),
            ],
            Self::Navigation => &[
                ("↑ / ↓", "Move through visible nodes"),
                (
                    "← / →",
                    "Collapse or expand, or move to parent / first child",
                ),
                ("Enter / Space", "Toggle expanded or collapsed state"),
                ("g", "Jump to the map root, or subtree root in Subtree Only"),
                (": / Ctrl+P", "Open the command palette"),
                (
                    "recipe query",
                    "Run built-in workflows like review todo or work inside branch",
                ),
                (
                    "relation query",
                    "Use the palette to jump across relations and backlinks",
                ),
                ("[", "Follow the next backlink into this node"),
                ("]", "Follow the next outgoing relation"),
                ("o", "Jump straight to a node id"),
                ("m", "Open the visual mindmap"),
            ],
            Self::Editing => &[
                ("a / A / Shift+R", "Add a child, sibling, or root branch"),
                ("e", "Edit the selected node inline"),
                ("x", "Delete after a second confirmation press"),
                ("u / U", "Undo or redo the last structural change"),
                ("Alt+↑ / Alt+↓", "Reorder the node among siblings"),
                (
                    "Alt+← / Alt+→",
                    "Move out one level or indent into the previous sibling",
                ),
            ],
            Self::Details => &[
                ("d", "Open the node detail editor"),
                ("Enter", "Add a new line while editing details"),
                ("Ctrl+S", "Save detail edits"),
                ("Esc", "Cancel detail editing"),
                ("| detail text", "Raw file syntax for attached details"),
            ],
            Self::Search => &[
                ("/", "Open query search"),
                ("text query", "Start with a normal word or phrase"),
                ("#tag query", "Filter by a tag like #todo or #blocked"),
                ("@key:value", "Filter by metadata like @status:active"),
                ("b / w", "Open browse or saved views"),
                ("Tab", "Switch Query, Browse, and Saved Views"),
                ("Enter", "Apply the current query or pick"),
                ("n / N", "Move between matches in the tree"),
                ("c", "Clear the active filter"),
            ],
            Self::Views => &[
                (
                    "v / V",
                    "Cycle Full Map, Focus Branch, Subtree Only, and Filtered Focus",
                ),
                ("Esc", "Exit a focused projection before clearing filters"),
                ("g", "Return to the subtree root inside Subtree Only"),
                ("←", "Stay inside the subtree root boundary in Subtree Only"),
                ("m", "Open a visual map of the current projection"),
            ],
            Self::Palette => &[
                (": / Ctrl+P", "Open the command palette"),
                ("branch or id", "Jump straight to a place in the map"),
                ("recipe query", "Run built-in or contextual workflows"),
                ("checkpoint / undo / redo", "Browse recovery targets"),
                ("theme / minimal", "Preview surface settings"),
                ("help topic", "Open built-in help from the same surface"),
            ],
            Self::Safety => &[
                ("u / U", "Undo or redo the last structural change"),
                (
                    "checkpoint",
                    "Create or restore named checkpoints from the palette",
                ),
                ("undo / redo", "Browse recent action history in the palette"),
                ("s / S", "Save now or toggle autosave"),
                ("r", "Reload from disk"),
            ],
            Self::Themes => &[
                (
                    ": / Ctrl+P",
                    "Open the palette for themes and surface settings",
                ),
                (
                    "theme",
                    "Preview themes like paper, violet, monograph, or blueprint",
                ),
                ("minimal", "Toggle the quieter pro layout"),
                ("ascii", "Toggle terminal-style accents"),
                ("motion", "Toggle attention-guiding motion"),
                ("Enter / Esc", "Keep or cancel a previewed surface change"),
            ],
            Self::Mindmap => &[
                ("m", "Open or close the visual mindmap"),
                ("Arrow keys", "Pan the canvas"),
                ("0", "Recenter the camera"),
                ("p", "Export a PNG from the current visual map"),
                ("Esc", "Return to the main tree surface"),
            ],
            Self::Syntax => &[
                ("#tag", "Add a topic or workflow marker"),
                ("@key:value", "Add structured metadata"),
                (
                    "Label + syntax",
                    "Combine visible text, tags, and metadata on one line",
                ),
                (
                    "/ and :",
                    "Search and palette understand the same inline syntax",
                ),
            ],
            Self::Ids => &[
                ("[id:path/to/node]", "Add a stable id target to a node line"),
                ("o", "Open the jump-to-id prompt"),
                ("product/tasks", "Jump by id through the palette"),
                (
                    "mdmind map.md#id",
                    "Open the TUI straight to a deep-linked branch",
                ),
                ("mdm links map.md", "List ids available in a map"),
                (
                    "mdm view map.md#id",
                    "Open a deep-linked subtree from the CLI",
                ),
            ],
            Self::Relations => &[
                ("[[target/id]]", "Create a plain cross-link to another id"),
                (
                    "[[rel:kind->target/id]]",
                    "Create a typed cross-link with meaning",
                ),
                ("]", "Follow an outgoing relation"),
                ("[", "Follow a backlink into the current node"),
                ("backlink", "Search relation jumps from the palette"),
                (
                    "mdm relations map.md#id",
                    "Inspect outgoing and incoming relations in the CLI",
                ),
            ],
        }
    }

    fn tips(self) -> &'static [&'static str] {
        match self {
            Self::StartHere => &[
                "If you are just learning, focus on movement, add or edit, search, and the palette first.",
                "You can adopt ids and relations later. They are power features, not prerequisites.",
            ],
            Self::Navigation => &[
                "If the tree starts feeling noisy, change view mode before you keep scrolling.",
                "Use recent locations in the palette when you are bouncing between two branches repeatedly.",
                "If the current branch has cross-links, type a target id, relation kind, or 'backlink' in the palette to jump across them.",
                "Use ] to follow outgoing relations and [ to follow backlinks when you want quick keyboard navigation without opening the palette.",
            ],
            Self::Editing => &[
                "Take a manual checkpoint before a large reparent or delete if you expect to compare two structures.",
                "Treat node labels as concise map lines and structural anchors, not mini paragraphs.",
            ],
            Self::Details => &[
                "Use details for content that belongs to one branch but would make the main tree harder to scan.",
                "Quotes, rationale, meeting notes, scene notes, and research excerpts all fit here well.",
            ],
            Self::Search => &[
                "Start broad with text, then tighten with #tags or @metadata once you see the pattern you need.",
                "If you are learning a new map, use browse before you try to guess every available metadata value or id path.",
                "Saved views are best for recurring workflows, not one-off ad hoc filters.",
                "Use palette recipes when you know the workflow you want but do not want to remember the exact filter.",
                "If your map uses @owner or several @status values, try typing 'owner' or 'status' in the palette to see contextual review recipes.",
            ],
            Self::Views => &[
                "Use Focus Branch when you still need orientation. Use Subtree Only when you want the rest of the map to disappear.",
                "If a branch feels slippery, remember that Subtree Only keeps a stable root until you leave the mode.",
            ],
            Self::Palette => &[
                "If you already know the target, use the palette instead of scrolling there manually.",
                "Think of the palette as the universal jump surface, not just an action launcher.",
                "Typing a workflow like 'review todo' is often faster than remembering the exact filter.",
            ],
            Self::Safety => &[
                "Use undo for recent edits, checkpoints for larger experiments, and recent history when you want to compare several nearby states.",
                "If autosave is on, undo still writes the restored state back to disk so the file and the UI stay aligned.",
                "Before a major restructure, a named checkpoint is usually better than cautious micro-edits.",
            ],
            Self::Themes => &[
                "Use Monograph with minimal mode for the calmest current surface.",
                "Keep motion on if you want attention guidance, but use minimal mode when you want less explanatory chrome and a roomier main tree.",
            ],
            Self::Mindmap => &[
                "Open the mindmap after isolating a branch or applying a filter, not before.",
                "If the canvas feels busy, tighten the working set in the tree first and reopen the map.",
                "Relation edges only appear when both linked nodes are visible in the current projection, so view mode still controls visual noise.",
            ],
            Self::Syntax => &[
                "Prefer consistent metadata keys like @status or @owner across the whole map instead of inventing near-duplicates.",
                "A few shared tags and metadata fields are usually enough to unlock search, browse, and saved views.",
            ],
            Self::Ids => &[
                "Prefer short, stable id paths like product/api-design over visible labels with spaces.",
                "Add ids to branches you expect to deep-link, export, or revisit often from the palette.",
                "A deep link is not only for the CLI. You can launch mdmind itself straight into map.md#some/id.",
                "If a deep link breaks, run mdm links to see what ids the file actually exposes.",
            ],
            Self::Relations => &[
                "Use plain [[target]] for lightweight references and [[rel:kind->target]] when the link meaning matters.",
                "Do not use relations as a substitute for basic tree structure. If everything links everywhere, the map gets harder to read.",
                "If a node has several outgoing or incoming links, [ and ] now open a small picker instead of guessing.",
            ],
        }
    }

    fn example(self) -> Option<&'static str> {
        match self {
            Self::Search => Some("#todo @status:active"),
            Self::Palette => Some("review todo"),
            Self::Safety => Some("checkpoint"),
            Self::Details => Some("| This branch still depends on partner auth."),
            Self::Syntax => Some("API Design #backend @status:todo @owner:mira"),
            Self::Ids => Some("API Design #backend [id:product/api-design]"),
            Self::Relations => Some("Launch Readiness [[rel:blocked-by->product/api-design]]"),
            _ => None,
        }
    }

    fn order_rank(self) -> usize {
        match self {
            Self::StartHere => 0,
            Self::Navigation => 1,
            Self::Editing => 2,
            Self::Details => 3,
            Self::Search => 4,
            Self::Views => 5,
            Self::Palette => 6,
            Self::Safety => 7,
            Self::Syntax => 8,
            Self::Ids => 9,
            Self::Relations => 10,
            Self::Themes => 11,
            Self::Mindmap => 12,
        }
    }

    fn track_label(self) -> &'static str {
        match self {
            Self::StartHere => "Basics",
            Self::Navigation
            | Self::Editing
            | Self::Details
            | Self::Search
            | Self::Views
            | Self::Palette => "Workflow",
            Self::Safety => "Safety",
            Self::Syntax | Self::Ids | Self::Relations => "Structure",
            Self::Themes | Self::Mindmap => "Surfaces",
        }
    }

    fn search_text(self) -> String {
        let reference_text = self
            .command_reference()
            .iter()
            .map(|(command, description)| format!("{command} {description}"))
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "{} {} {} {} {} {} {} {} {}",
            self.title(),
            self.track_label(),
            self.summary(),
            self.hint(),
            self.keywords(),
            self.guide_intro(),
            self.guide_body().join(" "),
            reference_text,
            self.tips().join(" ")
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
    EditDetails,
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
enum PaletteRecipe {
    ReviewTodo,
    ReviewActive,
    ReviewBlocked,
    WorkInsideBranch,
    BrowseFacets,
    SaveWorkingSet,
    VisualizeCurrentView,
}

impl PaletteRecipe {
    fn title(self) -> &'static str {
        match self {
            Self::ReviewTodo => "Review Todo Work",
            Self::ReviewActive => "Review Active Work",
            Self::ReviewBlocked => "Review Blocked Work",
            Self::WorkInsideBranch => "Work Inside This Branch",
            Self::BrowseFacets => "Browse Tags And Metadata",
            Self::SaveWorkingSet => "Save Current Working Set",
            Self::VisualizeCurrentView => "Visualize Current View",
        }
    }

    fn subtitle(self) -> &'static str {
        match self {
            Self::ReviewTodo => "Filter to #todo items and land on the first match",
            Self::ReviewActive => "Filter to @status:active work and focus the working set",
            Self::ReviewBlocked => "Filter to @status:blocked work and review what is stuck",
            Self::WorkInsideBranch => "Isolate the current branch as a rooted workspace",
            Self::BrowseFacets => "Browse tags, metadata, and deep-link ids",
            Self::SaveWorkingSet => "Name the current filter as a reusable saved view",
            Self::VisualizeCurrentView => "Open the mindmap on the current visible working set",
        }
    }

    fn keywords(self) -> &'static str {
        match self {
            Self::ReviewTodo => "recipe workflow review todo tasks triage open work filter #todo",
            Self::ReviewActive => {
                "recipe workflow review active status current work filter @status:active"
            }
            Self::ReviewBlocked => {
                "recipe workflow review blocked status stuck work filter @status:blocked"
            }
            Self::WorkInsideBranch => {
                "recipe workflow subtree branch isolate focus local workspace"
            }
            Self::BrowseFacets => "recipe workflow browse tags metadata values keys ids deep links",
            Self::SaveWorkingSet => {
                "recipe workflow save current filter working set saved view reuse"
            }
            Self::VisualizeCurrentView => {
                "recipe workflow mindmap visualize current visible view map"
            }
        }
    }

    fn preview(self) -> &'static str {
        match self {
            Self::ReviewTodo => {
                "Apply `#todo`, switch to Filtered Focus, and land on the first matching branch.\nUse this when you want to review open work without manually building a query."
            }
            Self::ReviewActive => {
                "Apply `@status:active`, switch to Filtered Focus, and land on the first matching branch.\nThis is useful for a quick pass over work already in motion."
            }
            Self::ReviewBlocked => {
                "Apply `@status:blocked`, switch to Filtered Focus, and land on the first matching branch.\nUse this as a recurring unblock review when your map tracks status."
            }
            Self::WorkInsideBranch => {
                "Enter Subtree Only on the current node and keep that node as the rooted workspace.\nThis is best for local edits, cleanup, and presenting one branch."
            }
            Self::BrowseFacets => {
                "Open the Find surface directly on Facets so you can browse tags, metadata keys, and values.\nThis is the fastest guided way to discover a map's vocabulary."
            }
            Self::SaveWorkingSet => {
                "Open the saved-view prompt for the current active filter.\nUse this after you have a working query you expect to revisit."
            }
            Self::VisualizeCurrentView => {
                "Open the visual mindmap for the current view and filter scope.\nGood for a quick spatial read or for exporting a polished PNG."
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SurfaceSetting {
    Motion(bool),
    AsciiAccents(bool),
    MinimalMode(bool),
}

impl SurfaceSetting {
    fn label(self) -> &'static str {
        match self {
            Self::Motion(true) => "Motion: On",
            Self::Motion(false) => "Motion: Off",
            Self::AsciiAccents(true) => "ASCII Accents: On",
            Self::AsciiAccents(false) => "ASCII Accents: Off",
            Self::MinimalMode(true) => "Minimal Mode: On",
            Self::MinimalMode(false) => "Minimal Mode: Off",
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
            Self::MinimalMode(true) => {
                "Reduce secondary copy and chrome in overlays for a quieter expert-focused surface"
            }
            Self::MinimalMode(false) => "Restore richer helper text and fuller overlay chrome",
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
            Self::MinimalMode(true) => {
                "Preview a quieter overlay style with less helper copy. Enter commits it; Esc restores the previous surface."
            }
            Self::MinimalMode(false) => {
                "Preview the fuller guided surface with richer helper copy. Enter commits it; Esc restores the previous surface."
            }
        }
    }

    fn keywords(self) -> &'static str {
        match self {
            Self::Motion(true) => "motion on animate guidance focus filter scope input",
            Self::Motion(false) => "motion off static reduce disable animation guidance",
            Self::AsciiAccents(true) => "ascii accents on terminal art separators chrome",
            Self::AsciiAccents(false) => "ascii accents off terminal art separators chrome default",
            Self::MinimalMode(true) => {
                "minimal mode on reduced chrome fewer hints quieter overlays"
            }
            Self::MinimalMode(false) => {
                "minimal mode off full hints guided overlays descriptive chrome"
            }
        }
    }
}

#[derive(Debug, Clone)]
enum PaletteTarget {
    Action(PaletteAction),
    Recipe(PaletteRecipe),
    QueryRecipe {
        title: String,
        query: String,
        view_mode: ViewMode,
    },
    Setting(SurfaceSetting),
    RelationPath {
        path: Vec<usize>,
        message: String,
    },
    InlineFilter(String),
    InlineId(String),
    RecentLocation(Vec<usize>),
    NodePath(Vec<usize>),
    SavedView {
        name: String,
        query: String,
    },
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
            Self::Setting(SurfaceSetting::MinimalMode(enabled)) => {
                preview.minimal_mode = *enabled;
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
    preview_scroll: u16,
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
            preview_scroll: 0,
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

    fn reset_preview_scroll(&mut self) {
        self.preview_scroll = 0;
    }
}

#[derive(Debug, Clone)]
struct PromptState {
    mode: PromptMode,
    value: String,
    cursor: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptAssistTone {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PromptAssist {
    tone: PromptAssistTone,
    lines: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RelationPickerKind {
    Outgoing,
    Backlink,
}

impl RelationPickerKind {
    fn title(self) -> &'static str {
        match self {
            Self::Outgoing => "Outgoing Relations",
            Self::Backlink => "Backlinks",
        }
    }

    fn open_message(self) -> &'static str {
        match self {
            Self::Outgoing => "Choose an outgoing relation to follow.",
            Self::Backlink => "Choose a backlink source to follow.",
        }
    }
}

#[derive(Debug, Clone)]
struct RelationPickerItem {
    path: Vec<usize>,
    title: String,
    subtitle: String,
    status_message: String,
}

#[derive(Debug, Clone)]
struct RelationPickerState {
    kind: RelationPickerKind,
    selected: usize,
    items: Vec<RelationPickerItem>,
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

    fn move_line_start(&mut self) {
        self.cursor = line_start(&self.value, self.cursor);
    }

    fn move_line_end(&mut self) {
        self.cursor = line_end(&self.value, self.cursor);
    }

    fn move_up(&mut self) {
        let current_start = line_start(&self.value, self.cursor);
        if current_start == 0 {
            self.cursor = 0;
            return;
        }

        let current_column = line_column(&self.value, current_start, self.cursor);
        let previous_end = current_start.saturating_sub(1);
        let previous_start = line_start(&self.value, previous_end);
        self.cursor =
            line_index_for_column(&self.value, previous_start, previous_end, current_column);
    }

    fn move_down(&mut self) {
        let current_start = line_start(&self.value, self.cursor);
        let current_end = line_end(&self.value, self.cursor);
        if current_end >= self.value.len() {
            self.cursor = self.value.len();
            return;
        }

        let current_column = line_column(&self.value, current_start, self.cursor);
        let next_start = current_end + 1;
        let next_end = line_end(&self.value, next_start);
        self.cursor = line_index_for_column(&self.value, next_start, next_end, current_column);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VisibleRow {
    path: Vec<usize>,
    depth: usize,
    text: String,
    tags: Vec<String>,
    metadata: Vec<String>,
    relations: Vec<String>,
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
    location_memory: LocationMemoryState,
    recent_locations: Vec<PathAnchor>,
    checkpoints: CheckpointsState,
    relation_picker: Option<RelationPickerState>,
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

        let mut app = Self {
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
            location_memory: LocationMemoryState::default(),
            recent_locations: Vec::new(),
            checkpoints: CheckpointsState::default(),
            relation_picker: None,
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
        };
        app.remember_current_location();
        app
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

        if self.relation_picker.is_some() {
            self.quit_armed = false;
            self.delete_armed = false;
            return self.handle_relation_picker_key(key);
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
            KeyCode::Char('b') => {
                self.delete_armed = false;
                self.open_search_overlay(SearchSection::Facets);
            }
            KeyCode::Char('w') => {
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
            KeyCode::Char('d') => {
                self.delete_armed = false;
                let initial = self
                    .editor
                    .current()
                    .map(Node::detail_text)
                    .unwrap_or_default();
                self.begin_prompt(PromptMode::EditDetail, initial);
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
                        self.finalize_focus_change(MotionTarget::Focus)?;
                        self.delete_armed = false;
                        self.set_status(StatusTone::Info, "Returned to the subtree root.");
                    } else {
                        self.set_status(StatusTone::Warning, "No subtree root is available.");
                    }
                } else {
                    self.editor.move_root()?;
                    self.finalize_focus_change(MotionTarget::Focus)?;
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
            KeyCode::Char(']') => {
                self.delete_armed = false;
                self.follow_outgoing_relation()?;
            }
            KeyCode::Char('[') => {
                self.delete_armed = false;
                self.follow_backlink()?;
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
                palette.selected = previous_palette_group_index_with_mode(
                    &items,
                    palette.selected,
                    palette.query.trim().is_empty(),
                );
            }
            KeyCode::Tab => {
                let items = self.palette_items(&palette.query);
                palette.selected = next_palette_group_index_with_mode(
                    &items,
                    palette.selected,
                    palette.query.trim().is_empty(),
                );
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

    fn handle_relation_picker_key(&mut self, key: KeyEvent) -> Result<bool, AppError> {
        let Some(mut picker) = self.relation_picker.take() else {
            return Ok(true);
        };

        match key.code {
            KeyCode::Esc => {
                self.set_status(StatusTone::Info, "Closed relation picker.");
                return Ok(true);
            }
            KeyCode::Up | KeyCode::Char('k') => {
                picker.selected = picker.selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !picker.items.is_empty() {
                    picker.selected = (picker.selected + 1).min(picker.items.len() - 1);
                }
            }
            KeyCode::Enter => {
                if let Some(item) = picker.items.get(picker.selected).cloned() {
                    self.editor.set_focus_path(item.path)?;
                    self.finalize_focus_change(MotionTarget::Focus)?;
                    self.set_status(StatusTone::Success, item.status_message);
                    return Ok(true);
                }
                self.set_status(StatusTone::Warning, "No relation is selected.");
                return Ok(true);
            }
            _ => {}
        }

        self.relation_picker = Some(picker);
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
                help.reset_preview_scroll();
            }
            KeyCode::Down => {
                let len = self.help_topics(&help.query).len();
                if len > 0 {
                    help.selected = (help.selected + 1).min(len - 1);
                }
                help.reset_preview_scroll();
            }
            KeyCode::PageUp => {
                help.preview_scroll = help.preview_scroll.saturating_sub(8);
            }
            KeyCode::PageDown => {
                help.preview_scroll = help.preview_scroll.saturating_add(8);
            }
            KeyCode::Home if key.modifiers.contains(KeyModifiers::CONTROL) => help.cursor = 0,
            KeyCode::End if key.modifiers.contains(KeyModifiers::CONTROL) => {
                help.cursor = help.query.len()
            }
            KeyCode::Home => help.preview_scroll = 0,
            KeyCode::End => help.preview_scroll = u16::MAX,
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                help.preview_scroll = help.preview_scroll.saturating_sub(4);
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                help.preview_scroll = help.preview_scroll.saturating_add(4);
            }
            KeyCode::Backspace => {
                help.backspace();
                help.reset_preview_scroll();
            }
            KeyCode::Delete => {
                help.delete();
                help.reset_preview_scroll();
            }
            KeyCode::Left => help.move_left(),
            KeyCode::Right => help.move_right(),
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
                help.reset_preview_scroll();
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
                    if search.facet_tab == FacetTab::Ids {
                        self.editor.open_id(&item.label)?;
                        self.finalize_focus_change(MotionTarget::Focus)?;
                        self.set_status(
                            StatusTone::Success,
                            format!("Jumped to deep link '{}'.", item.label),
                        );
                    } else {
                        search.draft_query =
                            compose_query_with_token(&search.draft_query, &item.token);
                        search.cursor = search.draft_query.len();
                        self.apply_search_facet(&item.label, &search.draft_query)?;
                    }
                } else {
                    self.set_status(StatusTone::Warning, search.facet_tab.empty_message());
                    self.search = Some(search.clone());
                }
                return Ok(true);
            }
            KeyCode::Char('c') if key.modifiers.is_empty() => {
                self.clear_filter();
                search.draft_query.clear();
                search.cursor = 0;
                search.facet_selected = 0;
                search.view_selected = 0;
                self.set_status(StatusTone::Info, "Cleared the active filter.");
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
            KeyCode::Char('c') if key.modifiers.is_empty() => {
                self.clear_filter();
                search.draft_query.clear();
                search.cursor = 0;
                search.facet_selected = 0;
                search.view_selected = 0;
                self.set_status(StatusTone::Info, "Cleared the active filter.");
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
        let multiline = prompt.mode == PromptMode::EditDetail;
        match key.code {
            KeyCode::Esc => {
                self.set_status(StatusTone::Info, "Cancelled input.");
            }
            KeyCode::Enter => {
                if multiline {
                    prompt.insert('\n');
                } else {
                    submit = Some((prompt.mode, prompt.value.trim().to_string()));
                }
            }
            KeyCode::Backspace => prompt.backspace(),
            KeyCode::Delete => prompt.delete(),
            KeyCode::Left => prompt.move_left(),
            KeyCode::Right => prompt.move_right(),
            KeyCode::Up if multiline => prompt.move_up(),
            KeyCode::Down if multiline => prompt.move_down(),
            KeyCode::Home if multiline => prompt.move_line_start(),
            KeyCode::End if multiline => prompt.move_line_end(),
            KeyCode::Home => prompt.cursor = 0,
            KeyCode::End => prompt.cursor = prompt.value.len(),
            KeyCode::Char(character)
                if !key.modifiers.contains(KeyModifiers::CONTROL)
                    && !key.modifiers.contains(KeyModifiers::ALT) =>
            {
                prompt.insert(character);
            }
            KeyCode::Tab if multiline => {
                prompt.insert(' ');
                prompt.insert(' ');
            }
            KeyCode::Char('s') if multiline && key.modifiers.contains(KeyModifiers::CONTROL) => {
                submit = Some((prompt.mode, prompt.value.clone()));
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
        if mode == PromptMode::EditDetail {
            let had_detail = self
                .editor
                .current()
                .is_some_and(|node| !node.detail.is_empty());
            self.apply_edit(
                |editor| editor.edit_current_detail(value),
                if value.trim().is_empty() && had_detail {
                    "Cleared the selected node details."
                } else {
                    "Updated the selected node details."
                },
                None,
            )?;
            return Ok(());
        }

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
            PromptMode::EditDetail => unreachable!("handled above"),
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
                self.finalize_focus_change(MotionTarget::Focus)?;
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

    fn current_anchor(&self) -> Option<PathAnchor> {
        self.editor.current().map(|node| PathAnchor {
            path: self.editor.focus_path().to_vec(),
            id: node.id.clone(),
        })
    }

    fn memory_anchor(anchor: &PathAnchor) -> LocationMemoryAnchor {
        LocationMemoryAnchor {
            path: anchor.path.clone(),
            id: anchor.id.clone(),
        }
    }

    fn anchors_match(
        existing_id: &Option<String>,
        existing_path: &[usize],
        anchor: &PathAnchor,
    ) -> bool {
        existing_id
            .as_ref()
            .zip(anchor.id.as_ref())
            .is_some_and(|(left, right)| left == right)
            || existing_path == anchor.path
    }

    fn resolve_anchor_path(&self, anchor: &PathAnchor) -> Option<Vec<usize>> {
        if let Some(id) = &anchor.id
            && let Some(path) = find_path_by_id(&self.editor.document().nodes, id)
        {
            return Some(path);
        }
        get_node(&self.editor.document().nodes, &anchor.path).map(|_| anchor.path.clone())
    }

    fn remember_current_location(&mut self) {
        let Some(anchor) = self.current_anchor() else {
            return;
        };
        if self
            .recent_locations
            .first()
            .is_some_and(|existing| Self::anchors_match(&existing.id, &existing.path, &anchor))
        {
            return;
        }

        self.recent_locations
            .retain(|existing| !Self::anchors_match(&existing.id, &existing.path, &anchor));
        self.recent_locations.insert(0, anchor.clone());
        if self.recent_locations.len() > RECENT_LOCATION_LIMIT {
            self.recent_locations.truncate(RECENT_LOCATION_LIMIT);
        }

        let next_seen = self
            .location_memory
            .frequent
            .iter()
            .map(|entry| entry.last_seen)
            .max()
            .unwrap_or(0)
            + 1;
        if let Some(entry) = self
            .location_memory
            .frequent
            .iter_mut()
            .find(|entry| Self::anchors_match(&entry.anchor.id, &entry.anchor.path, &anchor))
        {
            entry.visits += 1;
            entry.last_seen = next_seen;
            entry.anchor = Self::memory_anchor(&anchor);
        } else {
            self.location_memory.frequent.push(FrequentLocation {
                anchor: Self::memory_anchor(&anchor),
                visits: 1,
                last_seen: next_seen,
            });
        }
        self.location_memory.frequent.sort_by(|left, right| {
            right
                .visits
                .cmp(&left.visits)
                .then_with(|| right.last_seen.cmp(&left.last_seen))
        });
        if self.location_memory.frequent.len() > FREQUENT_LOCATION_LIMIT {
            self.location_memory
                .frequent
                .truncate(FREQUENT_LOCATION_LIMIT);
        }
    }

    fn finalize_focus_change_with_reveal(
        &mut self,
        motion: MotionTarget,
        reveal: FocusReveal,
    ) -> Result<(), AppError> {
        if reveal == FocusReveal::Reveal {
            self.expand_focus_chain();
        }
        self.remember_current_location();
        self.persist_session()?;
        self.persist_location_memory()?;
        self.trigger_motion(motion);
        Ok(())
    }

    fn finalize_focus_change(&mut self, motion: MotionTarget) -> Result<(), AppError> {
        self.finalize_focus_change_with_reveal(motion, FocusReveal::Reveal)
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
        self.finalize_focus_change_with_reveal(MotionTarget::Focus, FocusReveal::Preserve)?;
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
        self.finalize_focus_change_with_reveal(MotionTarget::Focus, FocusReveal::Preserve)?;
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
        self.finalize_focus_change_with_reveal(MotionTarget::Focus, FocusReveal::Preserve)?;
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
            "Palette open. Start with recent places, views, and related jumps up top, or arrow through actions, recipes, setup, recovery, and help below. Type any time to filter.",
        );
    }

    fn open_help(&mut self, topic: Option<HelpTopic>) {
        self.quit_armed = false;
        self.delete_armed = false;
        self.help = Some(HelpOverlayState::new(topic));
        self.trigger_motion(MotionTarget::HelpInput);
        let message = match topic {
            Some(topic) => format!(
                "Help open on {}. Type to refine guides, references, and tips.",
                topic.title()
            ),
            None => "Help open. Type to search guides, references, and tips.".to_string(),
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

    fn open_relation_picker(&mut self, kind: RelationPickerKind, items: Vec<RelationPickerItem>) {
        self.quit_armed = false;
        self.delete_armed = false;
        self.relation_picker = Some(RelationPickerState {
            kind,
            selected: 0,
            items,
        });
        self.set_status(StatusTone::Info, kind.open_message());
    }

    fn follow_outgoing_relation(&mut self) -> Result<(), AppError> {
        let Some(_) = self.editor.current() else {
            self.set_status(StatusTone::Warning, "No focused node is available.");
            return Ok(());
        };

        let items = collect_outgoing_relation_picker_items(
            self.editor.document(),
            self.editor.focus_path(),
        );
        if items.is_empty() {
            self.set_status(
                StatusTone::Info,
                "This node has no outgoing relations. Use [id:...] and [[target]] to add one.",
            );
            return Ok(());
        }

        if items.len() == 1 {
            let item = items.into_iter().next().expect("single item should exist");
            self.editor.set_focus_path(item.path)?;
            self.finalize_focus_change(MotionTarget::Focus)?;
            self.set_status(StatusTone::Success, item.status_message);
        } else {
            self.open_relation_picker(RelationPickerKind::Outgoing, items);
        }
        Ok(())
    }

    fn follow_backlink(&mut self) -> Result<(), AppError> {
        let Some(node) = self.editor.current() else {
            self.set_status(StatusTone::Warning, "No focused node is available.");
            return Ok(());
        };

        let Some(target_id) = node.id.as_deref() else {
            self.set_status(
                StatusTone::Info,
                "This node has no [id:...], so backlinks cannot target it yet.",
            );
            return Ok(());
        };

        let items = collect_backlink_picker_items(self.editor.document(), target_id);
        if items.is_empty() {
            self.set_status(StatusTone::Info, "No backlinks point at this node yet.");
            return Ok(());
        }

        if items.len() == 1 {
            let item = items.into_iter().next().expect("single item should exist");
            self.editor.set_focus_path(item.path)?;
            self.finalize_focus_change(MotionTarget::Focus)?;
            self.set_status(StatusTone::Success, item.status_message);
        } else {
            self.open_relation_picker(RelationPickerKind::Backlink, items);
        }
        Ok(())
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
                PaletteAction::EditDetails => {
                    let initial = self
                        .editor
                        .current()
                        .map(Node::detail_text)
                        .unwrap_or_default();
                    self.begin_prompt(PromptMode::EditDetail, initial);
                }
                PaletteAction::JumpToId => self.begin_prompt(PromptMode::OpenId, String::new()),
                PaletteAction::JumpToRoot => {
                    if self.view_mode == ViewMode::SubtreeOnly {
                        if let Some(path) = self.subtree_root_path() {
                            self.editor.set_focus_path(path)?;
                            self.finalize_focus_change(MotionTarget::Focus)?;
                            self.set_status(StatusTone::Info, "Returned to the subtree root.");
                        }
                    } else {
                        self.editor.move_root()?;
                        self.finalize_focus_change(MotionTarget::Focus)?;
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
            PaletteTarget::Recipe(recipe) => match recipe {
                PaletteRecipe::ReviewTodo => {
                    self.apply_filter("#todo")?;
                    if self.active_filter_match_count() > 0 {
                        self.set_view_mode(ViewMode::FilteredFocus);
                        self.set_status(
                            StatusTone::Success,
                            "Recipe applied: reviewing #todo work in Filtered Focus.",
                        );
                    }
                }
                PaletteRecipe::ReviewActive => {
                    self.apply_filter("@status:active")?;
                    if self.active_filter_match_count() > 0 {
                        self.set_view_mode(ViewMode::FilteredFocus);
                        self.set_status(
                            StatusTone::Success,
                            "Recipe applied: reviewing @status:active work in Filtered Focus.",
                        );
                    }
                }
                PaletteRecipe::ReviewBlocked => {
                    self.apply_filter("@status:blocked")?;
                    if self.active_filter_match_count() > 0 {
                        self.set_view_mode(ViewMode::FilteredFocus);
                        self.set_status(
                            StatusTone::Success,
                            "Recipe applied: reviewing @status:blocked work in Filtered Focus.",
                        );
                    }
                }
                PaletteRecipe::WorkInsideBranch => {
                    self.set_view_mode(ViewMode::SubtreeOnly);
                    if let Some(node) = self.editor.current() {
                        self.set_status(
                            StatusTone::Success,
                            format!("Recipe applied: working inside '{}'.", node.text),
                        );
                    }
                }
                PaletteRecipe::BrowseFacets => {
                    self.open_search_overlay(SearchSection::Facets);
                }
                PaletteRecipe::SaveWorkingSet => {
                    if self.filter.is_some() {
                        self.begin_prompt(PromptMode::SaveView, String::new());
                        self.set_status(
                            StatusTone::Info,
                            "Recipe applied: name the current filter to save it as a view.",
                        );
                    } else {
                        self.set_status(
                            StatusTone::Warning,
                            "Save Current Working Set needs an active filter first.",
                        );
                    }
                }
                PaletteRecipe::VisualizeCurrentView => {
                    self.mindmap = Some(MindmapOverlayState::default());
                    let scene = self.current_mindmap_scene();
                    self.set_status(
                        StatusTone::Info,
                        format!(
                            "Recipe applied: visual mindmap open. {}. Arrow keys pan, 0 recenters, p exports PNG.",
                            scene.describe()
                        ),
                    );
                }
            },
            PaletteTarget::QueryRecipe {
                title,
                query,
                view_mode,
            } => {
                self.apply_filter(&query)?;
                if self.active_filter_match_count() > 0 {
                    self.set_view_mode(view_mode);
                    self.set_status(StatusTone::Success, format!("Recipe applied: {title}."));
                }
            }
            PaletteTarget::Setting(setting) => {
                self.apply_surface_setting(setting)?;
            }
            PaletteTarget::RelationPath { path, message } => {
                self.editor.set_focus_path(path)?;
                self.finalize_focus_change(MotionTarget::Focus)?;
                self.set_status(StatusTone::Success, message);
            }
            PaletteTarget::InlineFilter(query) => {
                self.apply_inline_palette_filter(&query)?;
            }
            PaletteTarget::InlineId(id) => {
                self.editor.open_id(&id)?;
                self.finalize_focus_change(MotionTarget::Focus)?;
                self.set_status(StatusTone::Success, format!("Jumped to inline id '{id}'."));
            }
            PaletteTarget::RecentLocation(path) => {
                self.editor.set_focus_path(path)?;
                self.finalize_focus_change(MotionTarget::Focus)?;
                if let Some(node) = self.editor.current() {
                    self.set_status(
                        StatusTone::Success,
                        format!("Returned to recent location '{}'.", node.text),
                    );
                }
            }
            PaletteTarget::NodePath(path) => {
                self.editor.set_focus_path(path)?;
                self.finalize_focus_change(MotionTarget::Focus)?;
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
            FacetTab::Ids => self
                .search_id_entries(scope.map(FilterQuery::raw))
                .into_iter()
                .map(|entry| FacetItem {
                    label: entry.id,
                    token: entry.breadcrumb,
                    count: entry.line,
                    detail: entry.text,
                })
                .collect(),
        }
    }

    fn search_id_entries(&self, raw: Option<&str>) -> Vec<LinkEntry> {
        let mut entries = link_entries(self.editor.document());
        if let Some(raw) = raw {
            let query = raw.trim().to_lowercase();
            if !query.is_empty() {
                entries.retain(|entry| {
                    entry.id.to_lowercase().contains(&query)
                        || entry.text.to_lowercase().contains(&query)
                        || entry.breadcrumb.to_lowercase().contains(&query)
                });
            }
        }
        entries
    }

    fn palette_items(&self, raw: &str) -> Vec<PaletteItem> {
        let query = raw.trim().to_lowercase();
        if query.is_empty() {
            return self.palette_home_items();
        }

        let mut items = Vec::new();
        items.extend(self.palette_action_items(&query));
        items.extend(self.palette_recipe_items(&query));
        items.extend(self.palette_contextual_recipe_items(&query));
        items.extend(self.palette_theme_items(&query));
        items.extend(self.palette_setting_items(&query));
        items.extend(self.palette_relation_items(&query));
        items.extend(self.palette_inline_items(&query));
        items.extend(self.palette_frequent_location_items(&query));
        items.extend(self.palette_recent_location_items(&query));
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

    fn palette_home_items(&self) -> Vec<PaletteItem> {
        let mut items = Vec::new();
        items.extend(self.palette_frequent_location_items("").into_iter().take(3));
        items.extend(self.palette_recent_location_items("").into_iter().take(4));
        items.extend(self.palette_saved_view_items("").into_iter().take(4));
        items.extend(self.palette_relation_items("").into_iter().take(4));
        items.extend(self.palette_recipe_items(""));
        items.extend(self.palette_action_items(""));
        items.extend(self.palette_history_items("").into_iter().take(4));
        items.extend(self.palette_checkpoint_items("").into_iter().take(4));
        items.extend(self.palette_theme_items(""));
        items.extend(self.palette_setting_items(""));
        items.extend(self.palette_home_help_items());
        items
    }

    fn palette_home_help_items(&self) -> Vec<PaletteItem> {
        self.help_topics("")
            .into_iter()
            .filter(|topic| {
                matches!(
                    topic,
                    HelpTopic::StartHere
                        | HelpTopic::Details
                        | HelpTopic::Search
                        | HelpTopic::Palette
                        | HelpTopic::Ids
                )
            })
            .map(|topic| PaletteItem {
                kind: PaletteItemKind::Help,
                title: topic.title().to_string(),
                subtitle: topic.summary().to_string(),
                preview: format!("Help · {}", topic.hint()),
                score: 0,
                target: PaletteTarget::HelpTopic(topic),
            })
            .collect()
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
                "Edit Details",
                "Write longer notes for the selected node",
                "details notes prose description quote rationale edit",
                PaletteAction::EditDetails,
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
                "Browse Map Fields",
                "Open browse for tags, metadata, and ids",
                "facets browse tags metadata ids deep links",
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

    fn palette_recipe_items(&self, query: &str) -> Vec<PaletteItem> {
        let mut recipes = vec![
            PaletteRecipe::ReviewTodo,
            PaletteRecipe::ReviewActive,
            PaletteRecipe::ReviewBlocked,
            PaletteRecipe::WorkInsideBranch,
            PaletteRecipe::BrowseFacets,
            PaletteRecipe::VisualizeCurrentView,
        ];

        if self.filter.is_some() {
            recipes.push(PaletteRecipe::SaveWorkingSet);
        }

        recipes
            .into_iter()
            .filter_map(|recipe| {
                let title = recipe.title();
                let subtitle = recipe.subtitle();
                let preview = recipe.preview();
                let haystack = format!("{title} {subtitle} {preview} {}", recipe.keywords());
                palette_match_score(query, title, &haystack).map(|score| PaletteItem {
                    kind: PaletteItemKind::Recipe,
                    title: title.to_string(),
                    subtitle: subtitle.to_string(),
                    preview: preview.to_string(),
                    score: score + 325,
                    target: PaletteTarget::Recipe(recipe),
                })
            })
            .collect()
    }

    fn palette_contextual_recipe_items(&self, query: &str) -> Vec<PaletteItem> {
        if query.is_empty() {
            return Vec::new();
        }

        let owner_items = metadata_value_counts_for_filter(self.editor.document(), None)
            .into_iter()
            .filter(|entry| entry.key == "owner")
            .take(4)
            .filter_map(|entry| {
                let title = format!("Review Owner · {}", entry.value);
                let subtitle = format!(
                    "Review {} node{} owned by {}",
                    entry.count,
                    if entry.count == 1 { "" } else { "s" },
                    entry.value
                );
                let preview = format!(
                    "Apply `@owner:{}` in Filtered Focus and land on the first matching branch.\nUse this when you want a quick owner-specific review without typing the full filter every time.",
                    entry.value
                );
                let haystack = format!(
                    "{title} {subtitle} {preview} recipe workflow owner assignee assigned {} @owner:{}",
                    entry.value, entry.value
                );
                palette_match_score(query, &title, &haystack).map(|score| PaletteItem {
                    kind: PaletteItemKind::Recipe,
                    title: title.clone(),
                    subtitle,
                    preview,
                    score: score + 320 + entry.count.min(20) as i64,
                    target: PaletteTarget::QueryRecipe {
                        title: format!("reviewing @owner:{} work in Filtered Focus", entry.value),
                        query: format!("@owner:{}", entry.value),
                        view_mode: ViewMode::FilteredFocus,
                    },
                })
            });

        let status_items = metadata_value_counts_for_filter(self.editor.document(), None)
            .into_iter()
            .filter(|entry| entry.key == "status")
            .filter(|entry| entry.value != "active" && entry.value != "blocked")
            .take(4)
            .filter_map(|entry| {
                let title = format!("Review Status · {}", entry.value);
                let subtitle = format!(
                    "Review {} node{} with status {}",
                    entry.count,
                    if entry.count == 1 { "" } else { "s" },
                    entry.value
                );
                let preview = format!(
                    "Apply `@status:{}` in Filtered Focus and land on the first matching branch.\nUseful when this map tracks a custom workflow state beyond the built-in active and blocked recipes.",
                    entry.value
                );
                let haystack = format!(
                    "{title} {subtitle} {preview} recipe workflow status state review {} @status:{}",
                    entry.value, entry.value
                );
                palette_match_score(query, &title, &haystack).map(|score| PaletteItem {
                    kind: PaletteItemKind::Recipe,
                    title: title.clone(),
                    subtitle,
                    preview,
                    score: score + 318 + entry.count.min(20) as i64,
                    target: PaletteTarget::QueryRecipe {
                        title: format!("reviewing @status:{} work in Filtered Focus", entry.value),
                        query: format!("@status:{}", entry.value),
                        view_mode: ViewMode::FilteredFocus,
                    },
                })
            });

        owner_items.chain(status_items).collect()
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
            SurfaceSetting::MinimalMode(true),
            SurfaceSetting::MinimalMode(false),
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

    fn palette_relation_items(&self, query: &str) -> Vec<PaletteItem> {
        collect_relation_palette_entries(self.editor.document(), self.editor.focus_path())
            .into_iter()
            .filter_map(|entry| {
                palette_match_score(query, &entry.title, &entry.haystack).map(|score| PaletteItem {
                    kind: PaletteItemKind::Relation,
                    title: entry.title,
                    subtitle: entry.subtitle,
                    preview: entry.preview,
                    score: score + 390,
                    target: PaletteTarget::RelationPath {
                        path: entry.path,
                        message: entry.message,
                    },
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

    fn palette_inline_items(&self, query: &str) -> Vec<PaletteItem> {
        if query.is_empty() {
            return Vec::new();
        }

        let mut items = Vec::new();

        items.extend(
            link_entries(self.editor.document())
                .into_iter()
                .filter_map(|entry| {
                    let haystack = format!(
                        "[id:{}] {} {} inline id deep link target jump",
                        entry.id, entry.text, entry.breadcrumb
                    );
                    let score_boost = if query.contains('/') || query.starts_with("[id:") {
                        540
                    } else {
                        430
                    };
                    palette_match_score(query, &entry.id, &haystack).map(|score| PaletteItem {
                        kind: PaletteItemKind::Inline,
                        title: format!("[id:{}]", entry.id),
                        subtitle: entry.breadcrumb,
                        preview: format!(
                            "Jump directly to this inline id target.\n{}\nCurrent label '{}'.",
                            entry.id, entry.text
                        ),
                        score: score + score_boost,
                        target: PaletteTarget::InlineId(entry.id),
                    })
                }),
        );

        items.extend(
            tag_counts(self.editor.document())
                .into_iter()
                .filter_map(|entry| {
                    let token = entry.tag;
                    let haystack =
                        format!("{token} tag inline filter {} matching nodes", entry.count);
                    let score_boost = if query.starts_with('#') { 520 } else { 400 };
                    palette_match_score(query, &token, &haystack).map(|score| PaletteItem {
                        kind: PaletteItemKind::Inline,
                        title: token.clone(),
                        subtitle: format!(
                            "tag · {} match{}",
                            entry.count,
                            if entry.count == 1 { "" } else { "es" }
                        ),
                        preview: format!(
                            "Filter to nodes tagged {token}.\n{} matching node{} in this map.",
                            entry.count,
                            if entry.count == 1 { "" } else { "s" }
                        ),
                        score: score + score_boost,
                        target: PaletteTarget::InlineFilter(token),
                    })
                }),
        );

        items.extend(
            metadata_key_counts_for_filter(self.editor.document(), None)
                .into_iter()
                .filter_map(|entry| {
                    let token = format!("@{}", entry.key);
                    let haystack =
                        format!("{token} metadata key inline filter {} matching nodes", entry.count);
                    let score_boost = if query.starts_with('@') { 500 } else { 390 };
                    palette_match_score(query, &token, &haystack).map(|score| PaletteItem {
                        kind: PaletteItemKind::Inline,
                        title: token.clone(),
                        subtitle: format!(
                            "metadata key · {} match{}",
                            entry.count,
                            if entry.count == 1 { "" } else { "es" }
                        ),
                        preview: format!(
                            "Filter to nodes with metadata key {token}.\n{} matching node{} in this map.",
                            entry.count,
                            if entry.count == 1 { "" } else { "s" }
                        ),
                        score: score + score_boost,
                        target: PaletteTarget::InlineFilter(token),
                    })
                }),
        );

        items.extend(
            metadata_value_counts_for_filter(self.editor.document(), None)
                .into_iter()
                .filter_map(|entry| {
                    let token = format!("@{}:{}", entry.key, entry.value);
                    let haystack =
                        format!("{token} metadata value inline filter {} matching nodes", entry.count);
                    let score_boost = if query.starts_with('@') && query.contains(':') {
                        530
                    } else {
                        410
                    };
                    palette_match_score(query, &token, &haystack).map(|score| PaletteItem {
                        kind: PaletteItemKind::Inline,
                        title: token.clone(),
                        subtitle: format!(
                            "metadata value · {} match{}",
                            entry.count,
                            if entry.count == 1 { "" } else { "es" }
                        ),
                        preview: format!(
                            "Filter to nodes with metadata value {token}.\n{} matching node{} in this map.",
                            entry.count,
                            if entry.count == 1 { "" } else { "s" }
                        ),
                        score: score + score_boost,
                        target: PaletteTarget::InlineFilter(token),
                    })
                }),
        );

        items
    }

    fn palette_recent_location_items(&self, query: &str) -> Vec<PaletteItem> {
        self.recent_locations
            .iter()
            .filter_map(|anchor| {
                let path = self.resolve_anchor_path(anchor)?;
                let node = get_node(&self.editor.document().nodes, &path)?;
                let title = if node.text.is_empty() {
                    "(empty)".to_string()
                } else {
                    node.text.clone()
                };
                let breadcrumb = breadcrumb_for_path(self.editor.document(), &path);
                let subtitle = node
                    .id
                    .clone()
                    .unwrap_or_else(|| breadcrumb.clone());
                let preview = format!(
                    "Jump back to this recent location.\n{}\nThe current view and filter stay in place.",
                    breadcrumb
                );
                let haystack = format!("{title} {subtitle} {breadcrumb} recent location jump");
                palette_match_score(query, &title, &haystack).map(|score| PaletteItem {
                    kind: PaletteItemKind::Location,
                    title,
                    subtitle,
                    preview,
                    score: score + 360,
                    target: PaletteTarget::RecentLocation(path),
                })
            })
            .collect()
    }

    fn palette_frequent_location_items(&self, query: &str) -> Vec<PaletteItem> {
        self.location_memory
            .frequent
            .iter()
            .filter(|entry| entry.visits >= FREQUENT_LOCATION_MIN_VISITS)
            .filter_map(|entry| {
                let anchor = PathAnchor {
                    path: entry.anchor.path.clone(),
                    id: entry.anchor.id.clone(),
                };
                let path = self.resolve_anchor_path(&anchor)?;
                let node = get_node(&self.editor.document().nodes, &path)?;
                let title = if node.text.is_empty() {
                    "(empty)".to_string()
                } else {
                    node.text.clone()
                };
                let breadcrumb = breadcrumb_for_path(self.editor.document(), &path);
                let subtitle = format!(
                    "{} visits{}",
                    entry.visits,
                    node.id
                        .as_ref()
                        .map(|id| format!(" · {id}"))
                        .unwrap_or_default()
                );
                let preview = format!(
                    "Jump to a frequently revisited location.\n{}\nVisited {} times in this map.",
                    breadcrumb, entry.visits
                );
                let haystack = format!(
                    "{title} {subtitle} {breadcrumb} frequent location place revisit visits"
                );
                palette_match_score(query, &title, &haystack).map(|score| PaletteItem {
                    kind: PaletteItemKind::Frequent,
                    title,
                    subtitle,
                    preview,
                    score: score + 358 + entry.visits.min(20) as i64,
                    target: PaletteTarget::RecentLocation(path),
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
                        preview: format!("Help · {}", topic.hint()),
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
            HelpTopic::StartHere,
            HelpTopic::Navigation,
            HelpTopic::Editing,
            HelpTopic::Details,
            HelpTopic::Search,
            HelpTopic::Views,
            HelpTopic::Palette,
            HelpTopic::Safety,
            HelpTopic::Themes,
            HelpTopic::Mindmap,
            HelpTopic::Syntax,
            HelpTopic::Ids,
            HelpTopic::Relations,
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
                .then_with(|| left.0.order_rank().cmp(&right.0.order_rank()))
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

    fn apply_inline_palette_filter(&mut self, query: &str) -> Result<(), AppError> {
        self.apply_filter(query)?;
        let count = self.active_filter_match_count();
        if count == 0 {
            self.set_status(
                StatusTone::Warning,
                format!("Applied inline filter '{query}', but no nodes matched."),
            );
        } else {
            self.set_status(
                StatusTone::Success,
                format!(
                    "Applied inline filter '{query}' and landed on the first of {count} matches."
                ),
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
                format!("Applied browse item {label}, but no nodes matched."),
            );
        } else {
            self.set_status(
                StatusTone::Success,
                format!("Applied browse item {label} and landed on the first of {count} matches."),
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
        self.remember_current_location();
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
        self.finalize_focus_change(MotionTarget::FilterResult)?;
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
            self.finalize_focus_change(MotionTarget::Focus)?;
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

    fn persist_location_memory(&self) -> Result<(), AppError> {
        save_locations_for(&self.map_path, &self.location_memory)
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
            SurfaceSetting::MinimalMode(enabled) => baseline.minimal_mode == enabled,
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
            SurfaceSetting::MinimalMode(enabled) => {
                let changed = self.ui_settings.minimal_mode != enabled;
                self.ui_settings.minimal_mode = enabled;
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
            SurfaceSetting::MinimalMode(true) if changed => (
                StatusTone::Success,
                "Minimal mode enabled. Overlays now use a quieter, lower-noise layout.",
            ),
            SurfaceSetting::MinimalMode(true) => {
                (StatusTone::Info, "Minimal mode is already enabled.")
            }
            SurfaceSetting::MinimalMode(false) if changed => (
                StatusTone::Success,
                "Minimal mode disabled. Richer helper copy and fuller overlay chrome are back.",
            ),
            SurfaceSetting::MinimalMode(false) => {
                (StatusTone::Info, "Minimal mode is already disabled.")
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
                    SurfaceSetting::MinimalMode(true) if changed => (
                        StatusTone::Success,
                        "Minimal mode enabled. Overlays now use a quieter, lower-noise layout.",
                    ),
                    SurfaceSetting::MinimalMode(true) => {
                        (StatusTone::Info, "Minimal mode is already enabled.")
                    }
                    SurfaceSetting::MinimalMode(false) if changed => (
                        StatusTone::Success,
                        "Minimal mode disabled. Richer helper copy and fuller overlay chrome are back.",
                    ),
                    SurfaceSetting::MinimalMode(false) => {
                        (StatusTone::Info, "Minimal mode is already disabled.")
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
    let location_memory = load_locations_for(&loaded.target.path)?;
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
    app.location_memory = location_memory;
    app.recent_locations.clear();
    app.remember_current_location();
    app.checkpoints = checkpoints;
    app.persist_session()?;
    app.persist_location_memory()?;

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
    ACTIVE_MINIMAL_MODE.with(|enabled| enabled.set(app.ui_settings.minimal_mode));
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
            Constraint::Length(if app.ui_settings.minimal_mode { 3 } else { 4 }),
            Constraint::Min(16),
            Constraint::Length(if app.ui_settings.minimal_mode { 3 } else { 4 }),
            Constraint::Length(if app.ui_settings.minimal_mode { 0 } else { 1 }),
        ])
        .split(area);

    render_header(frame, outer[0], app);
    render_body(frame, outer[1], app);
    render_status(frame, outer[2], app);
    if !app.ui_settings.minimal_mode {
        render_keybar(frame, outer[3], app);
    }

    if let Some(help) = &app.help {
        render_help_overlay(frame, centered_rect(78, 80, area), app, help);
    }

    if let Some(picker) = &app.relation_picker {
        render_relation_picker_overlay(frame, centered_rect(66, 52, area), app, picker);
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
        render_prompt_overlay(
            frame,
            prompt_overlay_rect(prompt.mode, app.ui_settings.minimal_mode, area),
            prompt,
            app,
        );
    }
}

#[allow(non_snake_case)]
fn render_header(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let PALETTE = app.theme_colors();
    let minimal = app.ui_settings.minimal_mode;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(PALETTE.border))
        .style(Style::default().bg(PALETTE.surface))
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let breadcrumb = app.editor.breadcrumb();
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
    let (view_bg, view_fg) = view_badge_style(app.view_mode, PALETTE);
    let filter_badge = app
        .filter
        .as_ref()
        .map(|filter| format!(" FILTER {} ", filter.matches.len()));
    if minimal {
        let map_name = app
            .map_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(ToString::to_string)
            .unwrap_or_else(|| app.map_path.to_string_lossy().into_owned());
        let mut line = vec![
            Span::styled(
                "mdmind",
                Style::default()
                    .fg(PALETTE.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(map_name, Style::default().fg(PALETTE.text)),
            Span::raw("  "),
            status_chip("STATE", badge, badge_color, PALETTE.background),
            Span::raw(" "),
            status_chip(
                "SAVE",
                if app.autosave { "auto" } else { "manual" },
                PALETTE.surface_alt,
                PALETTE.text,
            ),
            Span::raw(" "),
            status_chip("VIEW", app.view_mode.label(), view_bg, view_fg),
        ];
        if let Some(filter) = &app.filter {
            line.push(Span::raw(" "));
            line.push(status_chip(
                "FILTER",
                &filter.matches.len().to_string(),
                PALETTE.surface_alt,
                PALETTE.text,
            ));
        }
        frame.render_widget(
            Paragraph::new(Line::from(line)).wrap(Wrap { trim: false }),
            inner,
        );
        return;
    }
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
                .fg(view_fg)
                .bg(view_bg)
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
        header_breadcrumb_line(&breadcrumb, PALETTE, inner.width as usize),
    ];

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn header_breadcrumb_line(
    breadcrumb: &[String],
    palette: Palette,
    max_width: usize,
) -> Line<'static> {
    if breadcrumb.is_empty() {
        return Line::from(vec![
            breadcrumb_chip("PATH", palette.surface_alt, palette.text),
            breadcrumb_separator(palette.surface_alt, palette.surface),
            breadcrumb_chip("(no focus)", palette.surface, palette.muted),
        ]);
    }

    let visible_segments = truncate_breadcrumb_segments(breadcrumb, max_width);
    let mut spans = Vec::new();
    let label_bg = palette.surface_alt;
    spans.push(breadcrumb_chip("PATH", label_bg, palette.text));

    let segment_count = visible_segments.len();
    for (index, segment) in visible_segments.iter().enumerate() {
        let (bg, fg) = breadcrumb_segment_style(index, segment_count, palette);
        spans.push(breadcrumb_separator(
            label_bg_for_segment(index, segment_count, palette),
            bg,
        ));
        spans.push(breadcrumb_chip(segment.clone(), bg, fg));
    }

    Line::from(spans)
}

fn truncate_breadcrumb_segments(breadcrumb: &[String], max_width: usize) -> Vec<String> {
    if breadcrumb.is_empty() {
        return Vec::new();
    }

    if breadcrumb_render_width(breadcrumb) + breadcrumb_chip_width("PATH") <= max_width {
        return breadcrumb.to_vec();
    }

    if breadcrumb.len() == 1 {
        let available = max_width
            .saturating_sub(breadcrumb_chip_width("PATH"))
            .saturating_sub(breadcrumb_separator_width());
        return vec![truncate_breadcrumb_segment(&breadcrumb[0], available)];
    }

    let first = breadcrumb
        .first()
        .cloned()
        .expect("breadcrumb should have a first segment");
    let last = breadcrumb
        .last()
        .cloned()
        .expect("breadcrumb should have a last segment");

    let compact = vec![first.clone(), "…".to_string(), last.clone()];
    if breadcrumb_render_width(&compact) + breadcrumb_chip_width("PATH") <= max_width {
        return compact;
    }

    let prefix = [first.clone(), "…".to_string()];
    let prefix_width = breadcrumb_render_width(&prefix) + breadcrumb_chip_width("PATH");
    if prefix_width < max_width {
        let remaining = max_width.saturating_sub(prefix_width + breadcrumb_separator_width());
        let truncated_last = truncate_breadcrumb_segment(&last, remaining);
        if !truncated_last.is_empty() {
            return vec![first, "…".to_string(), truncated_last];
        }
    }

    let tail_prefix = ["…".to_string()];
    let tail_prefix_width = breadcrumb_render_width(&tail_prefix) + breadcrumb_chip_width("PATH");
    if tail_prefix_width < max_width {
        let remaining = max_width.saturating_sub(tail_prefix_width + breadcrumb_separator_width());
        let truncated_last = truncate_breadcrumb_segment(&last, remaining);
        if !truncated_last.is_empty() {
            return vec!["…".to_string(), truncated_last];
        }
    }

    let available = max_width
        .saturating_sub(breadcrumb_chip_width("PATH"))
        .saturating_sub(breadcrumb_separator_width());
    vec![truncate_breadcrumb_segment(&last, available)]
}

fn breadcrumb_render_width(segments: &[String]) -> usize {
    segments.iter().fold(0, |width, segment| {
        width + breadcrumb_chip_width(segment) + breadcrumb_separator_width()
    })
}

fn breadcrumb_chip_width(content: &str) -> usize {
    content.chars().count() + 2
}

fn breadcrumb_separator_width() -> usize {
    if ascii_accents_enabled() { 3 } else { 1 }
}

fn truncate_breadcrumb_segment(segment: &str, available_width: usize) -> String {
    let max_chars = available_width.saturating_sub(2);
    if segment.chars().count() <= max_chars {
        return segment.to_string();
    }
    if max_chars == 0 {
        return String::new();
    }
    if max_chars == 1 {
        return "…".to_string();
    }

    let mut truncated = segment.chars().take(max_chars - 1).collect::<String>();
    truncated.push('…');
    truncated
}

fn label_bg_for_segment(index: usize, segment_count: usize, palette: Palette) -> Color {
    if index == 0 {
        palette.surface_alt
    } else {
        breadcrumb_segment_style(index - 1, segment_count, palette).0
    }
}

fn breadcrumb_segment_style(
    index: usize,
    segment_count: usize,
    palette: Palette,
) -> (Color, Color) {
    if index + 1 == segment_count {
        (palette.sky, palette.background)
    } else if index == 0 {
        (palette.accent, palette.background)
    } else if index % 2 == 1 {
        (palette.surface_alt, palette.text)
    } else {
        (palette.border, palette.text)
    }
}

fn breadcrumb_chip(content: impl Into<String>, bg: Color, fg: Color) -> Span<'static> {
    Span::styled(
        format!(" {} ", content.into()),
        Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
    )
}

fn breadcrumb_separator(left_bg: Color, right_bg: Color) -> Span<'static> {
    if ascii_accents_enabled() {
        Span::styled(" > ", Style::default().fg(right_bg))
    } else {
        Span::styled("", Style::default().fg(left_bg).bg(right_bg))
    }
}

fn view_badge_style(view_mode: ViewMode, palette: Palette) -> (Color, Color) {
    match view_mode {
        ViewMode::FullMap => (palette.sky, palette.background),
        ViewMode::FocusBranch => (palette.accent, palette.background),
        ViewMode::SubtreeOnly => (palette.border, palette.text),
        ViewMode::FilteredFocus => (palette.warn, palette.background),
    }
}

fn render_body(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(if app.ui_settings.minimal_mode {
            [Constraint::Percentage(62), Constraint::Percentage(38)]
        } else {
            [Constraint::Percentage(40), Constraint::Percentage(60)]
        })
        .split(area);

    render_outline(frame, columns[0], app);
    render_focus_cluster(frame, columns[1], app);
}

#[allow(non_snake_case)]
fn render_outline(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let PALETTE = app.theme_colors();
    let minimal = app.ui_settings.minimal_mode;
    let rows = app.visible_rows();
    let selected_index = app.selected_index(&rows);
    let scope_root = app.projection_focus_path();
    let focus_path = app.editor.focus_path().to_vec();
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
            let is_selected = row.path == focus_path;
            let show_detail = !minimal || is_selected || row.matched;
            let show_tags = !row.tags.is_empty();
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
            if show_tags {
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
            if show_detail && !row.metadata.is_empty() {
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
            if show_detail && !row.relations.is_empty() {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    row.relations.join(" "),
                    Style::default().fg(if row.dimmed {
                        PALETTE.border
                    } else {
                        PALETTE.sky
                    }),
                ));
            }
            if show_detail && let Some(id) = &row.id {
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
            if !minimal && row.has_children {
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
    let focus_height = focus_card_height(app);
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(focus_height), Constraint::Min(8)])
        .split(area);

    render_focus_card(frame, sections[0], app);

    if app.ui_settings.minimal_mode {
        render_children_lane(frame, sections[1], app);
        return;
    }

    let lanes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(28),
            Constraint::Percentage(32),
            Constraint::Percentage(40),
        ])
        .split(sections[1]);

    render_parent_lane(frame, lanes[0], app);
    render_backlinks_lane(frame, lanes[1], app);
    render_children_lane(frame, lanes[2], app);
}

fn focus_card_height(app: &TuiApp) -> u16 {
    let detail_lines = app
        .editor
        .current()
        .map(|node| node.detail.len().min(3) as u16)
        .unwrap_or(0);

    if !app.ui_settings.minimal_mode {
        let mut height = if app.filter.is_some() { 9 } else { 8 };
        if detail_lines > 0 {
            height += detail_lines + 1;
        }
        return height.min(13);
    }

    let Some(node) = app.editor.current() else {
        return 6;
    };

    let mut height = 6_u16;
    if !node.tags.is_empty() || !node.metadata.is_empty() {
        height += 1;
    }
    if !node.relations.is_empty() {
        height += 1;
    }
    if detail_lines > 0 {
        height += detail_lines + 1;
    }
    if app.filter.is_some() {
        height += 1;
    }
    if app.view_mode == ViewMode::SubtreeOnly && app.subtree_root_node().is_some() {
        height += 1;
    }

    height.min(12)
}

#[allow(non_snake_case)]
fn render_focus_card(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let PALETTE = app.theme_colors();
    let minimal = app.ui_settings.minimal_mode;
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
            if !node.relations.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("outgoing ", Style::default().fg(PALETTE.muted)),
                    Span::styled(
                        node.relations
                            .iter()
                            .map(|relation| relation.display_token())
                            .collect::<Vec<_>>()
                            .join(" "),
                        Style::default().fg(PALETTE.sky),
                    ),
                ]));
            }
            if !node.detail.is_empty() {
                let visible_lines = if minimal { 2 } else { 3 };
                let mut detail_iter = node.detail.iter().take(visible_lines).enumerate();
                for (index, detail_line) in detail_iter.by_ref() {
                    let label = if index == 0 { "details " } else { "        " };
                    let text = if detail_line.is_empty() {
                        " ".to_string()
                    } else {
                        detail_line.clone()
                    };
                    lines.push(Line::from(vec![
                        Span::styled(label, Style::default().fg(PALETTE.muted)),
                        Span::styled(text, Style::default().fg(PALETTE.text)),
                    ]));
                }
                if node.detail.len() > visible_lines {
                    lines.push(Line::from(vec![
                        Span::styled("        ", Style::default().fg(PALETTE.muted)),
                        Span::styled(
                            format!("… {} more line(s)", node.detail.len() - visible_lines),
                            Style::default().fg(PALETTE.sky),
                        ),
                    ]));
                }
            }
            if !minimal {
                lines.push(Line::from(vec![
                    Span::styled("relations ", Style::default().fg(PALETTE.muted)),
                    Span::styled(
                        summarize_relationships(app),
                        Style::default().fg(PALETTE.text),
                    ),
                ]));
            }
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
            if !minimal {
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
            }
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
fn render_backlinks_lane(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let PALETTE = app.theme_colors();
    let title = styled_title("Backlinks", PALETTE.accent);
    render_simple_lane(
        frame,
        area,
        title,
        backlink_lines(app),
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
        .style(if minimal_mode_enabled() {
            style.bg(PALETTE.background)
        } else {
            style
        })
        .padding(Padding::horizontal(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

#[allow(non_snake_case)]
fn render_status(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let PALETTE = app.theme_colors();
    let status = app.status_model();
    if app.ui_settings.minimal_mode {
        let mut line = vec![
            Span::styled(
                match status.tone {
                    StatusTone::Info => "info",
                    StatusTone::Success => "saved",
                    StatusTone::Warning => "warn",
                    StatusTone::Error => "error",
                },
                Style::default()
                    .fg(match status.tone {
                        StatusTone::Info => PALETTE.sky,
                        StatusTone::Success => PALETTE.accent,
                        StatusTone::Warning => PALETTE.warn,
                        StatusTone::Error => PALETTE.danger,
                    })
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(status.message, Style::default().fg(PALETTE.text)),
            Span::raw("  "),
            Span::styled(
                format!("focus {}", status.focus_label),
                Style::default().fg(if motion_level(MotionTarget::Focus) > 0 {
                    PALETTE.sky
                } else {
                    PALETTE.muted
                }),
            ),
            Span::raw("  "),
            Span::styled(
                compact_scope_label(&status.scope_label),
                Style::default().fg(if motion_level(MotionTarget::Scope) > 0 {
                    PALETTE.warn
                } else {
                    PALETTE.muted
                }),
            ),
        ];
        if let Some(filter) = &status.filter_summary {
            line.push(Span::raw("  "));
            line.push(Span::styled(
                format!("filter {filter}"),
                Style::default().fg(PALETTE.accent),
            ));
        }
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(PALETTE.border))
            .style(Style::default().bg(PALETTE.background));
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(Line::from(line)).wrap(Wrap { trim: false }),
            inner,
        );
        return;
    }
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
fn keybar_spans(app: &TuiApp) -> Vec<Span<'static>> {
    let mut spans = vec![
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
        key_hint("d", "details"),
        separator_span(),
        key_hint("x", "delete"),
        separator_span(),
        key_hint("u/U", "undo"),
        separator_span(),
        key_hint(":", "palette"),
        separator_span(),
        key_hint("v/V", "mode"),
        separator_span(),
        key_hint("/", "find"),
        separator_span(),
        key_hint("s", "save"),
        separator_span(),
        key_hint("r", "revert"),
        separator_span(),
        key_hint("?", "help"),
    ];

    if app.filter.is_some() {
        spans.push(separator_span());
        spans.push(key_hint("n/N", "match"));
    }

    let has_outgoing_relations = app
        .editor
        .current()
        .is_some_and(|node| !node.relations.is_empty());
    let has_backlinks = app
        .editor
        .current()
        .and_then(|node| node.id.as_deref())
        .is_some_and(|id| !backlinks_to(app.editor.document(), id).is_empty());
    if has_outgoing_relations || has_backlinks {
        spans.push(separator_span());
        spans.push(key_hint("[ ]", "related"));
    }

    spans
}

#[cfg(test)]
fn keybar_text(app: &TuiApp) -> String {
    keybar_spans(app)
        .into_iter()
        .map(|span| span.content.into_owned())
        .collect::<String>()
}

#[allow(non_snake_case)]
fn render_keybar(frame: &mut Frame, area: Rect, app: &TuiApp) {
    let PALETTE = active_palette();
    let line = Line::from(keybar_spans(app));
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
    let minimal = app.ui_settings.minimal_mode;
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
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(vec![Line::from(vec![
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
                Style::default()
                    .fg(PALETTE.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                if minimal {
                    "Guides and quick answers."
                } else {
                    "Search guides and quick answers in place."
                },
                Style::default().fg(PALETTE.muted),
            ),
            Span::raw("  "),
            Span::styled(
                format!("View: {}", app.view_mode.label()),
                Style::default().fg(PALETTE.sky),
            ),
        ])]),
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
        .constraints([Constraint::Percentage(34), Constraint::Percentage(66)])
        .split(sections[2]);

    let topics = app.help_topics(&help.query);
    if topics.is_empty() {
        frame.render_widget(
            Paragraph::new(if minimal {
                "No help matches."
            } else {
                "No help articles match yet. Try 'start', 'palette', 'safety', 'relations', or 'deep link'."
            })
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
        let selected_index = help.selected.min(topics.len() - 1);
        let items = topics
            .iter()
            .enumerate()
            .map(|topic| {
                let (index, topic) = topic;
                let selected = index == selected_index;
                let mut lines = Vec::new();
                let current_track = topic.track_label();
                let previous_track = index
                    .checked_sub(1)
                    .and_then(|prev| topics.get(prev))
                    .map(|prev| prev.track_label());

                if index > 0 && previous_track != Some(current_track) {
                    lines.push(Line::from(""));
                }
                if previous_track != Some(current_track) {
                    lines.push(help_track_heading(current_track, PALETTE));
                }

                lines.push(Line::from(vec![
                    Span::styled(
                        if selected { "▎ " } else { "  " },
                        Style::default().fg(if selected {
                            PALETTE.accent
                        } else {
                            PALETTE.border
                        }),
                    ),
                    Span::styled(
                        topic.title(),
                        Style::default()
                            .fg(if selected { PALETTE.text } else { PALETTE.sky })
                            .add_modifier(Modifier::BOLD),
                    ),
                ]));

                if selected {
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(topic.summary(), Style::default().fg(PALETTE.text)),
                    ]));
                }

                ListItem::new(lines)
            })
            .collect::<Vec<_>>();
        let mut state = ListState::default();
        state.select(Some(selected_index));
        frame.render_stateful_widget(
            List::new(items)
                .block(
                    Block::default()
                        .title(styled_title("Topics", PALETTE.sky))
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(PALETTE.sky))
                        .style(Style::default().bg(PALETTE.surface))
                        .padding(Padding::horizontal(1)),
                )
                .highlight_symbol("")
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
        help_preview_lines(app, topic)
    } else {
        vec![Line::from(Span::styled(
            "Type to search built-in help by topic, workflow, recipe, or command.",
            Style::default().fg(PALETTE.muted),
        ))]
    };
    let preview_title = if let Some(topic) = topics.get(help.selected).copied() {
        Line::from(vec![
            Span::styled(
                format!(" {} ", topic.title()),
                Style::default()
                    .fg(PALETTE.background)
                    .bg(PALETTE.warn)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            help_track_chip(topic.track_label(), PALETTE, false),
        ])
    } else {
        styled_title("Article", PALETTE.warn)
    };
    let preview_block = Block::default()
        .title(preview_title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(PALETTE.warn))
        .style(Style::default().bg(PALETTE.surface))
        .padding(Padding::horizontal(1));
    let preview_inner = preview_block.inner(columns[1]);
    frame.render_widget(preview_block, columns[1]);
    let (preview_content, preview_scrollbar) = if preview_inner.width > 2 {
        let split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(preview_inner);
        (split[0], Some(split[1]))
    } else {
        (preview_inner, None)
    };
    let preview_visible_height = preview_content.height as usize;
    let preview_total_height =
        wrapped_preview_height(&preview_lines, preview_content.width).min(u16::MAX as usize);
    let max_scroll = preview_total_height
        .saturating_sub(preview_visible_height)
        .min(u16::MAX as usize) as u16;
    let scroll = help.preview_scroll.min(max_scroll);
    frame.render_widget(
        Paragraph::new(preview_lines)
            .scroll((scroll, 0))
            .wrap(Wrap { trim: false }),
        preview_content,
    );
    if let Some(scrollbar_area) = preview_scrollbar {
        render_preview_scrollbar(
            frame,
            scrollbar_area,
            scroll,
            preview_total_height,
            preview_visible_height,
            PALETTE,
        );
    }

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            key_hint("type", "filter"),
            separator_span(),
            key_hint("↑↓", "select"),
            separator_span(),
            key_hint("PgUp/Dn", "scroll"),
            separator_span(),
            key_hint("^U/^D", "scroll"),
            separator_span(),
            key_hint("Home/End", "top/bot"),
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
    let minimal = app.ui_settings.minimal_mode;
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
                    if minimal {
                        "Ids, filters, history, views, help."
                    } else {
                        "Jump to ids, apply #tag or @metadata filters, revisit frequent places and recent locations, browse recent actions, restore checkpoints, preview themes and settings, open saved views, or find the right help topic."
                    },
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
    let home_mode = palette.query.trim().is_empty();
    if items.is_empty() {
        frame.render_widget(
            Paragraph::new(if minimal {
                "No matches yet."
            } else {
                "No matches yet. Try 'review todo', 'save', 'paper', '#todo', '@status:active', 'product/tasks', or a node label like 'tasks'."
            })
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
        let group_starts = palette_group_starts_with_mode(&items, home_mode);
        let list_items = items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let selected = index == palette.selected.min(items.len() - 1);
                let group_color = palette_group_color(item.kind, PALETTE, home_mode);
                let mut lines = Vec::new();
                if index > 0 && group_starts[index] {
                    lines.push(Line::from(""));
                }
                if group_starts[index] {
                    lines.push(palette_group_header_line(item.kind, PALETTE, home_mode));
                }
                let row = if home_mode {
                    Line::from(vec![
                        Span::styled(
                            if selected { "› " } else { "  " },
                            Style::default().fg(if selected {
                                group_color
                            } else {
                                PALETTE.border
                            }),
                        ),
                        Span::styled(
                            item.title.clone(),
                            Style::default()
                                .fg(if selected { group_color } else { PALETTE.text })
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw("  "),
                        Span::styled(
                            item.subtitle.clone(),
                            Style::default().fg(if selected {
                                PALETTE.text
                            } else {
                                PALETTE.muted
                            }),
                        ),
                    ])
                } else {
                    Line::from(vec![
                        Span::styled(
                            if selected { "› " } else { "  " },
                            Style::default().fg(if selected {
                                group_color
                            } else {
                                PALETTE.border
                            }),
                        ),
                        Span::styled(
                            item.kind.label(),
                            Style::default().fg(group_color).add_modifier(Modifier::DIM),
                        ),
                        Span::raw(" "),
                        Span::styled(
                            item.title.clone(),
                            Style::default()
                                .fg(if selected { group_color } else { PALETTE.text })
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw("  "),
                        Span::styled(
                            item.subtitle.clone(),
                            Style::default().fg(if selected {
                                PALETTE.text
                            } else {
                                PALETTE.muted
                            }),
                        ),
                    ])
                };
                lines.push(row);
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
                        .bg(PALETTE.background)
                        .fg(PALETTE.text)
                        .add_modifier(Modifier::BOLD),
                ),
            body[0],
            &mut state,
        );
    }

    let preview_lines = if let Some(item) = items.get(palette.selected) {
        let group_label = palette_group_label(item.kind, home_mode);
        let group_color = palette_group_color(item.kind, PALETTE, home_mode);
        vec![
            Line::from(vec![
                Span::styled(
                    format!(" {} ", group_label),
                    Style::default()
                        .fg(PALETTE.background)
                        .bg(group_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(item.title.clone(), Style::default().fg(group_color)),
            ]),
            Line::from(Span::styled(
                item.subtitle.clone(),
                Style::default().fg(PALETTE.sky),
            )),
            Line::from(""),
            Line::from(Span::styled(
                item.preview.clone(),
                Style::default().fg(PALETTE.text),
            )),
        ]
    } else {
        vec![Line::from(Span::styled(
            "The empty palette starts with recent places, views, and related jumps up top, then a browseable catalog of actions, recipes, setup, recovery, and help below. Type to narrow by ids, #tags, @metadata, nodes, or relations.",
            Style::default().fg(PALETTE.muted),
        ))]
    };
    frame.render_widget(
        Paragraph::new(preview_lines)
            .block(
                Block::default()
                    .title(styled_title("Details", PALETTE.warn))
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
fn render_relation_picker_overlay(
    frame: &mut Frame,
    area: Rect,
    app: &TuiApp,
    picker: &RelationPickerState,
) {
    let PALETTE = app.theme_colors();
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(styled_title(picker.kind.title(), PALETTE.accent))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(PALETTE.accent))
        .style(Style::default().bg(PALETTE.surface_alt))
        .padding(Padding::uniform(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(6),
            Constraint::Length(1),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(match picker.kind {
            RelationPickerKind::Outgoing => {
                "The current node has multiple outgoing relations. Pick one target to follow."
            }
            RelationPickerKind::Backlink => {
                "The current node has multiple backlinks. Pick one source branch to follow."
            }
        })
        .style(Style::default().fg(PALETTE.muted))
        .wrap(Wrap { trim: false }),
        sections[0],
    );

    let items = picker
        .items
        .iter()
        .map(|item| {
            ListItem::new(vec![Line::from(vec![
                Span::styled(
                    item.title.clone(),
                    Style::default()
                        .fg(PALETTE.text)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(item.subtitle.clone(), Style::default().fg(PALETTE.sky)),
            ])])
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    if !picker.items.is_empty() {
        state.select(Some(picker.selected.min(picker.items.len() - 1)));
    }
    frame.render_stateful_widget(
        List::new(items)
            .block(
                Block::default()
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
        sections[1],
        &mut state,
    );

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            key_hint("↑↓", "choose"),
            separator_span(),
            key_hint("Enter", "follow"),
            separator_span(),
            key_hint("Esc", "close"),
        ]))
        .alignment(Alignment::Center)
        .style(Style::default().fg(PALETTE.muted)),
        sections[2],
    );
}

#[allow(non_snake_case)]
fn render_search_overlay(frame: &mut Frame, area: Rect, app: &TuiApp, search: &SearchOverlayState) {
    let PALETTE = app.theme_colors();
    let minimal = app.ui_settings.minimal_mode;
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
                    if minimal {
                        "Query, browse, saved views."
                    } else {
                        "Query, browse tags, metadata, and ids, then reopen saved working sets without leaving the map."
                    },
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
        Paragraph::new(Line::from(search_footer_hints(search.section)))
            .alignment(Alignment::Center)
            .style(Style::default().fg(PALETTE.muted)),
        sections[3],
    );
}

fn search_footer_hints(section: SearchSection) -> Vec<Span<'static>> {
    match section {
        SearchSection::Query => vec![
            key_hint("Tab", "sections"),
            separator_span(),
            key_hint("Enter", "apply"),
            separator_span(),
            key_hint("type", "query"),
            separator_span(),
            key_hint("Esc", "close"),
        ],
        SearchSection::Facets => vec![
            key_hint("Tab", "sections"),
            separator_span(),
            key_hint("←→", "browse"),
            separator_span(),
            key_hint("↑↓", "select"),
            separator_span(),
            key_hint("Enter", "run"),
            separator_span(),
            key_hint("c", "clear"),
            separator_span(),
            key_hint("Esc", "close"),
        ],
        SearchSection::Views => vec![
            key_hint("Tab", "sections"),
            separator_span(),
            key_hint("↑↓", "select"),
            separator_span(),
            key_hint("Enter", "open"),
            separator_span(),
            key_hint("a", "save"),
            separator_span(),
            key_hint("x", "delete"),
            separator_span(),
            key_hint("c", "clear"),
            separator_span(),
            key_hint("Esc", "close"),
        ],
    }
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
        if app.ui_settings.minimal_mode {
            "Type text, #tag, or @key:value.".to_string()
        } else {
            "Type text, #tag, or @key:value. Enter applies the query. Empty input clears the filter."
                .to_string()
        }
    } else if app.ui_settings.minimal_mode {
        format!("{} match(es).", preview.len())
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

    let tabs = [
        FacetTab::Tags,
        FacetTab::Keys,
        FacetTab::Values,
        FacetTab::Ids,
    ]
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
        if tab != FacetTab::Ids {
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
                let detail = if search.facet_tab == FacetTab::Ids {
                    format!("line {}", item.count)
                } else {
                    format!("{} nodes", item.count)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        item.label.clone(),
                        Style::default()
                            .fg(PALETTE.text)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(detail, Style::default().fg(PALETTE.warn)),
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
            if search.facet_tab == FacetTab::Ids {
                Line::from(vec![
                    Span::styled("path ", Style::default().fg(PALETTE.muted)),
                    Span::styled(item.token.clone(), Style::default().fg(PALETTE.sky)),
                ])
            } else {
                Line::from(vec![
                    Span::styled("apply ", Style::default().fg(PALETTE.muted)),
                    Span::styled(
                        compose_query_with_token(&search.draft_query, &item.token),
                        Style::default().fg(PALETTE.accent),
                    ),
                ])
            },
            Line::from(Span::styled(
                if search.facet_tab == FacetTab::Ids {
                    if app.ui_settings.minimal_mode {
                        "Enter jumps to this id."
                    } else {
                        "Enter jumps to this deep-linked branch. Left and right switch Tags / Keys / Values / Ids."
                    }
                } else if app.ui_settings.minimal_mode {
                    "Enter applies this browse item."
                } else {
                    "Enter applies this browse item. Left and right switch Tags / Keys / Values / Ids."
                },
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
            Paragraph::new(if app.ui_settings.minimal_mode {
                "No saved views yet."
            } else {
                "No saved views yet. Type or apply a query, then press a to save it."
            })
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
                if app.ui_settings.minimal_mode {
                    "Enter opens. a saves. x deletes."
                } else {
                    "Enter opens this view. a saves the current query. x deletes the selected view."
                },
                Style::default().fg(PALETTE.muted),
            )),
        ]
    } else {
        let views_path = crate::views::views_path_for(&app.map_path)
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| "(unavailable)".to_string());
        vec![
            Line::from(Span::styled(
                if app.ui_settings.minimal_mode {
                    "Saved views sidecar."
                } else {
                    "Saved views live in a local sidecar next to the map."
                },
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
        HelpTopic::StartHere => {
            if count_nodes(&app.editor.document().nodes) <= 8 {
                "this map is still small enough to learn by browsing and editing directly"
                    .to_string()
            } else {
                "if the map feels dense, start with movement, search, and the palette before deeper features"
                    .to_string()
            }
        }
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
        HelpTopic::Details => app
            .editor
            .current()
            .map(|node| {
                if node.detail.is_empty() {
                    "current node has no attached details yet".to_string()
                } else {
                    format!("current node has {} detail line(s)", node.detail.len())
                }
            })
            .unwrap_or_else(|| {
                "focus a branch first, then add details if it needs more than one line".to_string()
            }),
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
        HelpTopic::Palette => {
            "use the palette for intentional jumps: actions, ids, workflows, relations, history, and help"
                .to_string()
        }
        HelpTopic::Safety => {
            if app.autosave {
                format!(
                    "autosave is on with {} undo step(s) and {} checkpoint(s) available",
                    app.undo_history.len(),
                    app.checkpoints.checkpoints.len()
                )
            } else {
                format!(
                    "manual save mode with {} undo step(s) and {} checkpoint(s) available",
                    app.undo_history.len(),
                    app.checkpoints.checkpoints.len()
                )
            }
        }
        HelpTopic::Themes => format!(
            "current theme {}, motion {}, accents {}, minimal {} stored next to the map",
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
            },
            if app.ui_settings.minimal_mode {
                "on"
            } else {
                "off"
            },
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
            .unwrap_or_else(|| {
                "create a node first, then add inline tags, ids, or references".to_string()
            }),
        HelpTopic::Ids => app
            .editor
            .current()
            .map(|node| match &node.id {
                Some(id) => format!("current node already exposes [id:{id}]"),
                None => "current node has no [id:...] yet, so it is not deep-linkable".to_string(),
            })
            .unwrap_or_else(|| {
                "focus a branch first, then add an id if it should be a stable target".to_string()
            }),
        HelpTopic::Relations => app
            .editor
            .current()
            .map(|node| {
                if node.relations.is_empty() {
                    "current node has no outgoing relations yet".to_string()
                } else {
                    format!(
                        "current node has {} outgoing relation(s)",
                        node.relations.len()
                    )
                }
            })
            .unwrap_or_else(|| {
                "focus a node with an id if you want backlinks to have a stable target".to_string()
            }),
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

#[derive(Debug, Clone)]
struct PaletteRelationEntry {
    path: Vec<usize>,
    title: String,
    subtitle: String,
    preview: String,
    haystack: String,
    message: String,
}

fn palette_kind_rank(kind: PaletteItemKind) -> u8 {
    match kind {
        PaletteItemKind::Action => 0,
        PaletteItemKind::Recipe => 1,
        PaletteItemKind::Theme => 2,
        PaletteItemKind::Setting => 3,
        PaletteItemKind::Relation => 4,
        PaletteItemKind::Inline => 5,
        PaletteItemKind::Frequent => 6,
        PaletteItemKind::Location => 7,
        PaletteItemKind::History => 8,
        PaletteItemKind::Checkpoint => 9,
        PaletteItemKind::Safety => 10,
        PaletteItemKind::Node => 11,
        PaletteItemKind::SavedView => 12,
        PaletteItemKind::Help => 13,
    }
}

fn palette_home_group_label(kind: PaletteItemKind) -> &'static str {
    match kind {
        PaletteItemKind::Location | PaletteItemKind::Frequent => "Recent",
        PaletteItemKind::SavedView => "Views",
        PaletteItemKind::Relation => "Related",
        PaletteItemKind::Recipe => "Recipes",
        PaletteItemKind::Action | PaletteItemKind::Inline | PaletteItemKind::Node => "Actions",
        PaletteItemKind::History | PaletteItemKind::Checkpoint | PaletteItemKind::Safety => {
            "Recovery"
        }
        PaletteItemKind::Theme | PaletteItemKind::Setting => "Setup",
        PaletteItemKind::Help => "Help",
    }
}

fn palette_group_label(kind: PaletteItemKind, home_mode: bool) -> &'static str {
    if home_mode {
        palette_home_group_label(kind)
    } else {
        kind.label()
    }
}

fn palette_group_color(kind: PaletteItemKind, palette: Palette, home_mode: bool) -> Color {
    if home_mode {
        match palette_home_group_label(kind) {
            "Recent" => palette.sky,
            "Views" => palette.accent,
            "Related" => palette.warn,
            "Recipes" => palette.accent,
            "Actions" => palette.sky,
            "Recovery" => palette.warn,
            "Setup" => palette.border,
            "Help" => palette.accent,
            _ => palette.sky,
        }
    } else {
        match kind {
            PaletteItemKind::Action => palette.warn,
            PaletteItemKind::Recipe => palette.accent,
            PaletteItemKind::Theme | PaletteItemKind::Setting => palette.border,
            PaletteItemKind::Relation => palette.warn,
            PaletteItemKind::Inline => palette.accent,
            PaletteItemKind::Frequent | PaletteItemKind::Location | PaletteItemKind::SavedView => {
                palette.sky
            }
            PaletteItemKind::History | PaletteItemKind::Checkpoint | PaletteItemKind::Safety => {
                palette.warn
            }
            PaletteItemKind::Node => palette.text,
            PaletteItemKind::Help => palette.accent,
        }
    }
}

#[cfg(test)]
fn palette_group_starts(items: &[PaletteItem]) -> Vec<bool> {
    palette_group_starts_with_mode(items, false)
}

fn palette_group_starts_with_mode(items: &[PaletteItem], home_mode: bool) -> Vec<bool> {
    let mut starts = Vec::with_capacity(items.len());
    let mut previous_label = None;
    for item in items {
        let current_label = palette_group_label(item.kind, home_mode);
        let starts_group = previous_label != Some(current_label);
        starts.push(starts_group);
        previous_label = Some(current_label);
    }
    starts
}

#[cfg(test)]
fn next_palette_group_index(items: &[PaletteItem], selected: usize) -> usize {
    next_palette_group_index_with_mode(items, selected, false)
}

fn next_palette_group_index_with_mode(
    items: &[PaletteItem],
    selected: usize,
    home_mode: bool,
) -> usize {
    if items.is_empty() {
        return 0;
    }
    let group_starts = palette_group_starts_with_mode(items, home_mode);
    let selected = selected.min(items.len() - 1);
    for (index, starts_group) in group_starts.iter().enumerate().skip(selected + 1) {
        if *starts_group {
            return index;
        }
    }
    selected
}

#[cfg(test)]
fn previous_palette_group_index(items: &[PaletteItem], selected: usize) -> usize {
    previous_palette_group_index_with_mode(items, selected, false)
}

fn previous_palette_group_index_with_mode(
    items: &[PaletteItem],
    selected: usize,
    home_mode: bool,
) -> usize {
    if items.is_empty() {
        return 0;
    }
    let group_starts = palette_group_starts_with_mode(items, home_mode);
    let selected = selected.min(items.len() - 1);
    for index in (0..selected).rev() {
        if group_starts[index] {
            return index;
        }
    }
    0
}

#[cfg(test)]
fn palette_group_summary(items: &[PaletteItem], selected: usize) -> String {
    palette_group_summary_with_mode(items, selected, false)
}

#[cfg(test)]
fn palette_group_summary_with_mode(
    items: &[PaletteItem],
    selected: usize,
    home_mode: bool,
) -> String {
    if items.is_empty() {
        return "No groups".to_string();
    }

    let selected = selected.min(items.len() - 1);
    let mut groups: Vec<&str> = Vec::new();
    for item in items {
        let label = palette_group_label(item.kind, home_mode);
        if groups.last().copied() != Some(label) {
            groups.push(label);
        }
    }

    let current_label = palette_group_label(items[selected].kind, home_mode);
    groups
        .into_iter()
        .map(|label| {
            if label == current_label {
                format!("[{label}]")
            } else {
                label.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

fn palette_group_header_line(
    kind: PaletteItemKind,
    palette: Palette,
    home_mode: bool,
) -> Line<'static> {
    let group_label = palette_group_label(kind, home_mode);
    let group_color = palette_group_color(kind, palette, home_mode);
    Line::from(vec![Span::styled(
        format!(" {} ", group_label.to_uppercase()),
        Style::default()
            .fg(palette.background)
            .bg(group_color)
            .add_modifier(Modifier::BOLD),
    )])
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

fn collect_relation_palette_entries(
    document: &Document,
    focus_path: &[usize],
) -> Vec<PaletteRelationEntry> {
    let Some(current) = get_node(&document.nodes, focus_path) else {
        return Vec::new();
    };

    let current_label = node_label_for_document(document, focus_path);
    let mut entries = Vec::new();

    for relation in &current.relations {
        let Some(path) = find_path_by_id(&document.nodes, &relation.target) else {
            continue;
        };
        let target_label = node_label_for_document(document, &path);
        let target_breadcrumb = breadcrumb_for_path(document, &path);
        let token = relation.display_token();
        let relation_label = relation.label();
        let title = format!("{relation_label} → {target_label}");
        let subtitle = format!("outgoing · {target_breadcrumb}");
        let preview = format!(
            "Follow an outgoing cross-link from '{current_label}'.\n{token}\nTarget: {target_breadcrumb}"
        );
        let haystack = format!(
            "{title} {subtitle} {preview} relation related outgoing cross-link {} {}",
            relation.target, relation_label
        );
        let message = format!("Followed relation {token} to '{target_label}'.");
        entries.push(PaletteRelationEntry {
            path,
            title,
            subtitle,
            preview,
            haystack,
            message,
        });
    }

    if let Some(current_id) = current.id.as_deref() {
        collect_backlink_palette_entries_from(
            &document.nodes,
            current_id,
            &mut entries,
            Vec::new(),
            &mut Vec::new(),
        );
    }

    entries.sort_by(|left, right| {
        left.title
            .to_lowercase()
            .cmp(&right.title.to_lowercase())
            .then_with(|| {
                left.subtitle
                    .to_lowercase()
                    .cmp(&right.subtitle.to_lowercase())
            })
    });
    entries
}

fn collect_outgoing_relation_picker_items(
    document: &Document,
    focus_path: &[usize],
) -> Vec<RelationPickerItem> {
    let Some(current) = get_node(&document.nodes, focus_path) else {
        return Vec::new();
    };

    current
        .relations
        .iter()
        .filter_map(|relation| {
            let path = find_path_by_id(&document.nodes, &relation.target)?;
            let target_label = node_label_for_document(document, &path);
            let breadcrumb = breadcrumb_for_path(document, &path);
            Some(RelationPickerItem {
                path,
                title: target_label.clone(),
                subtitle: format!("{} · {}", relation.label(), breadcrumb),
                status_message: format!(
                    "Followed relation {} to '{}'.",
                    relation.label(),
                    target_label
                ),
            })
        })
        .collect()
}

fn collect_backlink_picker_items(document: &Document, target_id: &str) -> Vec<RelationPickerItem> {
    let mut items = Vec::new();
    collect_backlink_picker_items_from(
        &document.nodes,
        target_id,
        &mut items,
        Vec::new(),
        &mut Vec::new(),
    );
    items
}

fn collect_backlink_picker_items_from(
    nodes: &[Node],
    target_id: &str,
    items: &mut Vec<RelationPickerItem>,
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

        for relation in &node.relations {
            if relation.target == target_id {
                items.push(RelationPickerItem {
                    path: path.clone(),
                    title: label.clone(),
                    subtitle: format!("{} · {}", relation.label(), breadcrumb_text),
                    status_message: format!(
                        "Followed backlink {} from '{}'.",
                        relation.label(),
                        label
                    ),
                });
            }
        }

        collect_backlink_picker_items_from(&node.children, target_id, items, path, breadcrumb);
        breadcrumb.pop();
    }
}

fn collect_backlink_palette_entries_from(
    nodes: &[Node],
    target_id: &str,
    entries: &mut Vec<PaletteRelationEntry>,
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

        for relation in &node.relations {
            if relation.target == target_id {
                let token = relation.display_token();
                let relation_label = relation.label();
                let title = format!("{relation_label} ← {label}");
                let subtitle = format!("backlink · {breadcrumb_text}");
                let preview = format!(
                    "Jump to a node that points at the current branch.\n{token}\nSource: {breadcrumb_text}"
                );
                let haystack = format!(
                    "{title} {subtitle} {preview} relation related incoming backlink source {} {}",
                    relation.target, relation_label
                );
                let message = format!("Jumped to backlink source '{label}' via {token}.");
                entries.push(PaletteRelationEntry {
                    path: path.clone(),
                    title,
                    subtitle,
                    preview,
                    haystack,
                    message,
                });
            }
        }

        collect_backlink_palette_entries_from(&node.children, target_id, entries, path, breadcrumb);
        breadcrumb.pop();
    }
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
        let detail_preview = node.detail_preview().unwrap_or_default();
        let id = node.id.clone().unwrap_or_default();
        let secondary = if id.is_empty() {
            breadcrumb_text.clone()
        } else {
            id.clone()
        };
        let haystack = format!(
            "{} {} {} {} {} {} {}",
            label,
            breadcrumb_text,
            id,
            node.tags.join(" "),
            metadata,
            detail_preview,
            node.relations
                .iter()
                .map(|relation| relation.display_token())
                .collect::<Vec<_>>()
                .join(" ")
        );
        entries.push(PaletteNodeEntry {
            path: path.clone(),
            primary: label,
            secondary,
            preview: if detail_preview.is_empty() {
                breadcrumb_text
            } else {
                format!("{breadcrumb_text}\n{detail_preview}")
            },
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

fn breadcrumb_for_path(document: &Document, path: &[usize]) -> String {
    if path.is_empty() {
        return "(no focus)".to_string();
    }

    let mut labels = Vec::new();
    let mut nodes = &document.nodes;
    for index in path {
        let Some(node) = nodes.get(*index) else {
            break;
        };
        labels.push(if node.text.is_empty() {
            "(empty)".to_string()
        } else {
            node.text.clone()
        });
        nodes = &node.children;
    }

    if labels.is_empty() {
        "(missing focus)".to_string()
    } else {
        labels.join(" / ")
    }
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
            let detail = entry
                .detail_snippet
                .or(entry.id)
                .unwrap_or(entry.breadcrumb);
            (entry.text, detail)
        })
        .collect()
}

#[allow(non_snake_case)]
fn render_prompt_overlay(frame: &mut Frame, area: Rect, prompt: &PromptState, app: &TuiApp) {
    let PALETTE = app.theme_colors();
    let minimal = app.ui_settings.minimal_mode;
    frame.render_widget(Clear, area);
    let block = Block::default()
        .title(styled_title(prompt.mode.title(), PALETTE.warn))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(PALETTE.warn))
        .style(Style::default().bg(PALETTE.surface_alt))
        .padding(Padding::uniform(1));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if prompt.mode == PromptMode::EditDetail {
        render_detail_prompt_overlay(frame, inner, prompt, app);
        return;
    }

    if minimal {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(1)])
            .split(inner);
        render_prompt_input(
            frame,
            chunks[0],
            prompt,
            PALETTE,
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(PALETTE.accent))
                .style(Style::default().bg(PALETTE.surface)),
        );
        frame.render_widget(
            Paragraph::new("Enter saves. Esc cancels.").style(Style::default().fg(PALETTE.muted)),
            chunks[1],
        );
        return;
    }

    let assist = prompt_assist(app, prompt);
    let assist_color = match assist.tone {
        PromptAssistTone::Info => PALETTE.sky,
        PromptAssistTone::Success => PALETTE.accent,
        PromptAssistTone::Warning => PALETTE.warn,
        PromptAssistTone::Error => PALETTE.danger,
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(inner);

    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(28)])
        .split(chunks[0]);
    frame.render_widget(
        Paragraph::new(prompt.mode.hint())
            .style(Style::default().fg(PALETTE.muted))
            .wrap(Wrap { trim: false }),
        header_chunks[0],
    );
    frame.render_widget(
        Paragraph::new("Enter saves  ·  Esc cancels")
            .alignment(Alignment::Right)
            .style(Style::default().fg(PALETTE.sky)),
        header_chunks[1],
    );

    render_prompt_input(
        frame,
        chunks[1],
        prompt,
        PALETTE,
        Block::default()
            .title(prompt_input_title(prompt.mode))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(PALETTE.accent))
            .style(Style::default().bg(PALETTE.surface)),
    );

    render_prompt_feedback(frame, chunks[2], &assist, assist_color, PALETTE);

    if let Some(footer) = prompt_footer_text(prompt.mode) {
        frame.render_widget(
            Paragraph::new(footer)
                .style(Style::default().fg(PALETTE.muted))
                .wrap(Wrap { trim: false }),
            chunks[3],
        );
    }
}

fn render_detail_prompt_overlay(frame: &mut Frame, area: Rect, prompt: &PromptState, app: &TuiApp) {
    let palette = app.theme_colors();
    let minimal = app.ui_settings.minimal_mode;
    let node_label = app
        .editor
        .current()
        .map(|node| {
            if node.text.is_empty() {
                "(empty)".to_string()
            } else {
                node.text.clone()
            }
        })
        .unwrap_or_else(|| "(missing focus)".to_string());
    let detail_summary = detail_prompt_summary(&prompt.value);

    if minimal {
        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(6),
                Constraint::Length(1),
            ])
            .split(area);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("node ", Style::default().fg(palette.muted)),
                Span::styled(node_label, Style::default().fg(palette.sky)),
                Span::raw("  "),
                Span::styled(detail_summary.clone(), Style::default().fg(palette.warn)),
            ])),
            sections[0],
        );
        render_prompt_input(
            frame,
            sections[1],
            prompt,
            palette,
            Block::default()
                .title(" Details ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(palette.accent))
                .style(Style::default().bg(palette.surface)),
        );
        frame.render_widget(
            Paragraph::new("Enter: new line  ·  ^S: save  ·  Esc: cancel")
                .style(Style::default().fg(palette.muted)),
            sections[2],
        );
        return;
    }

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(8),
            Constraint::Length(2),
            Constraint::Length(2),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("node ", Style::default().fg(palette.muted)),
                Span::styled(node_label, Style::default().fg(palette.sky)),
                Span::raw("  "),
                Span::styled(detail_summary, Style::default().fg(palette.warn)),
            ]),
            Line::from(Span::styled(
                "Use details for longer notes, quotes, rationale, and context without bloating the one-line node label.",
                Style::default().fg(palette.muted),
            )),
        ])
        .wrap(Wrap { trim: false }),
        sections[0],
    );

    render_prompt_input(
        frame,
        sections[1],
        prompt,
        palette,
        Block::default()
            .title(" Details ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(palette.accent))
            .style(Style::default().bg(palette.surface)),
    );

    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "Detail lines are stored under the node as | ... in the raw file.",
                Style::default().fg(palette.sky),
            )),
            Line::from(Span::styled(
                "Leave the editor empty if you want to clear the current details.",
                Style::default().fg(palette.muted),
            )),
        ])
        .wrap(Wrap { trim: false }),
        sections[2],
    );
    frame.render_widget(
        Paragraph::new("Enter: new line  ·  ^S: save  ·  Esc: cancel")
            .style(Style::default().fg(palette.muted))
            .wrap(Wrap { trim: false }),
        sections[3],
    );
}

fn render_prompt_input(
    frame: &mut Frame,
    area: Rect,
    prompt: &PromptState,
    palette: Palette,
    input_block: Block<'_>,
) {
    let input_inner = input_block.inner(area);
    frame.render_widget(input_block, area);

    if prompt.mode == PromptMode::EditDetail {
        let view = multiline_view(
            &prompt.value,
            prompt.cursor,
            input_inner.width as usize,
            input_inner.height as usize,
        );
        let lines = if prompt.value.is_empty() {
            let mut placeholder = vec![Line::from(Span::styled(
                "Start typing attached detail lines for this node.",
                Style::default().fg(palette.muted),
            ))];
            if input_inner.height > 1 {
                placeholder.push(Line::from(Span::styled(
                    "Good fits: rationale, quotes, research notes, meeting context.",
                    Style::default().fg(palette.border),
                )));
            }
            placeholder
        } else {
            styled_detail_lines(&view, palette)
        };
        frame.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: false }),
            input_inner,
        );
        if input_inner.width > 0 && input_inner.height > 0 {
            frame.set_cursor_position((
                input_inner.x + view.cursor_x,
                input_inner.y + view.cursor_y,
            ));
        }
        return;
    }

    let (visible_value, cursor_offset) =
        single_line_view(&prompt.value, prompt.cursor, input_inner.width as usize);
    frame.render_widget(
        Paragraph::new(highlight_prompt_input(&visible_value, prompt.mode, palette))
            .wrap(Wrap { trim: false }),
        input_inner,
    );
    if input_inner.width > 0 && input_inner.height > 0 {
        frame.set_cursor_position((input_inner.x + cursor_offset as u16, input_inner.y));
    }
}

fn highlight_prompt_input(value: &str, mode: PromptMode, palette: Palette) -> Line<'static> {
    if mode == PromptMode::EditDetail {
        return Line::from(Span::styled(
            value.to_string(),
            Style::default().fg(palette.text),
        ));
    }

    let mut spans = Vec::new();
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch.is_whitespace() {
            let mut whitespace = String::from(ch);
            while let Some(next) = chars.peek() {
                if next.is_whitespace() {
                    whitespace.push(chars.next().expect("peeked whitespace should exist"));
                } else {
                    break;
                }
            }
            spans.push(Span::styled(whitespace, Style::default().fg(palette.muted)));
            continue;
        }

        let mut token = String::from(ch);
        while let Some(next) = chars.peek() {
            if next.is_whitespace() {
                break;
            }
            token.push(chars.next().expect("peeked token char should exist"));
        }

        let style = match prompt_token_kind(&token, mode) {
            PromptTokenKind::Text => Style::default().fg(palette.text),
            PromptTokenKind::Tag => Style::default().fg(palette.accent),
            PromptTokenKind::Metadata => Style::default().fg(palette.warn),
            PromptTokenKind::Id => Style::default().fg(palette.sky),
        };
        spans.push(Span::styled(token, style));
    }

    Line::from(spans)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptTokenKind {
    Text,
    Tag,
    Metadata,
    Id,
}

fn prompt_token_kind(token: &str, mode: PromptMode) -> PromptTokenKind {
    match mode {
        PromptMode::AddChild | PromptMode::AddSibling | PromptMode::AddRoot | PromptMode::Edit => {
            if token.starts_with("#") {
                PromptTokenKind::Tag
            } else if token.starts_with("@") {
                PromptTokenKind::Metadata
            } else if token.starts_with("[id:") || token.starts_with("id:") {
                PromptTokenKind::Id
            } else {
                PromptTokenKind::Text
            }
        }
        PromptMode::EditDetail => PromptTokenKind::Text,
        PromptMode::OpenId => PromptTokenKind::Id,
        PromptMode::SaveView | PromptMode::SaveCheckpoint => PromptTokenKind::Text,
    }
}

fn render_prompt_feedback(
    frame: &mut Frame,
    area: Rect,
    assist: &PromptAssist,
    color: Color,
    palette: Palette,
) {
    let mut lines = Vec::new();
    if let Some(first) = assist.lines.first() {
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {} ", prompt_feedback_label(assist.tone)),
                Style::default()
                    .fg(palette.background)
                    .bg(color)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(first.clone(), Style::default().fg(palette.text)),
        ]));
    }
    for line in assist.lines.iter().skip(1) {
        lines.push(Line::from(vec![
            Span::styled("• ", Style::default().fg(color)),
            Span::styled(line.clone(), Style::default().fg(palette.muted)),
        ]));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::TOP)
                    .border_style(Style::default().fg(color))
                    .style(Style::default().bg(palette.surface_alt)),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn prompt_input_title(mode: PromptMode) -> &'static str {
    match mode {
        PromptMode::AddChild | PromptMode::AddSibling | PromptMode::AddRoot | PromptMode::Edit => {
            " Node line "
        }
        PromptMode::EditDetail => " Details ",
        PromptMode::OpenId => " Node id ",
        PromptMode::SaveView => " View name ",
        PromptMode::SaveCheckpoint => " Checkpoint name ",
    }
}

fn prompt_feedback_label(tone: PromptAssistTone) -> &'static str {
    match tone {
        PromptAssistTone::Info => "INFO",
        PromptAssistTone::Success => "READY",
        PromptAssistTone::Warning => "CHECK",
        PromptAssistTone::Error => "ERROR",
    }
}

fn prompt_footer_text(mode: PromptMode) -> Option<&'static str> {
    match mode {
        PromptMode::AddChild | PromptMode::AddSibling | PromptMode::AddRoot | PromptMode::Edit => {
            Some("Single-line node syntax: Label #tag @key:value [id:path] [[target]].")
        }
        PromptMode::EditDetail => Some(
            "Details are stored below the node as | ... lines and stay separate from the one-line tree label.",
        ),
        PromptMode::OpenId => Some("Ids come from [id:...] tokens on node lines."),
        PromptMode::SaveView => Some("Saves the current active filter under a reusable name."),
        PromptMode::SaveCheckpoint => {
            Some("Stores a local snapshot of the current workspace state.")
        }
    }
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

fn prompt_assist(app: &TuiApp, prompt: &PromptState) -> PromptAssist {
    match prompt.mode {
        PromptMode::AddChild | PromptMode::AddSibling | PromptMode::AddRoot | PromptMode::Edit => {
            prompt_fragment_assist(app, prompt)
        }
        PromptMode::EditDetail => prompt_detail_assist(app, prompt.value.as_str()),
        PromptMode::OpenId => prompt_open_id_assist(app, prompt.value.trim()),
        PromptMode::SaveView => {
            if let Some(query) = app.current_search_query_for_save() {
                PromptAssist {
                    tone: PromptAssistTone::Info,
                    lines: vec![
                        "This saves the current active filter under a short name.".to_string(),
                        format!("Current query: {query}"),
                    ],
                }
            } else {
                PromptAssist {
                    tone: PromptAssistTone::Warning,
                    lines: vec![
                        "No active filter is ready to save yet.".to_string(),
                        "Apply a query first, then name the saved view here.".to_string(),
                    ],
                }
            }
        }
        PromptMode::SaveCheckpoint => PromptAssist {
            tone: PromptAssistTone::Info,
            lines: vec![
                "This captures the current document, focus, view mode, and filter.".to_string(),
                current_scope_label(app, None),
            ],
        },
    }
}

fn prompt_detail_assist(app: &TuiApp, value: &str) -> PromptAssist {
    let node_label = app
        .editor
        .current()
        .map(|node| {
            if node.text.is_empty() {
                "(empty)".to_string()
            } else {
                node.text.clone()
            }
        })
        .unwrap_or_else(|| "(missing focus)".to_string());
    let normalized = value.replace("\r\n", "\n");
    let trimmed = normalized.trim_matches('\n');
    if trimmed.trim().is_empty() {
        return PromptAssist {
            tone: PromptAssistTone::Info,
            lines: vec![
                format!("Details for '{node_label}' are currently empty."),
                "Use this space for longer notes, quotes, rationale, and context that would feel noisy in the main tree."
                    .to_string(),
            ],
        };
    }

    let line_count = trimmed.lines().count();
    PromptAssist {
        tone: PromptAssistTone::Success,
        lines: vec![
            format!("Editing details for '{node_label}'."),
            format!("{line_count} line(s) will be stored under this node."),
        ],
    }
}

fn prompt_fragment_assist(app: &TuiApp, prompt: &PromptState) -> PromptAssist {
    let value = prompt.value.trim();
    if value.is_empty() {
        return PromptAssist {
            tone: PromptAssistTone::Info,
            lines: vec![
                "Type a node label, then optionally add #tags, @key:value, [id:path], and [[target]]."
                    .to_string(),
                "Example: API Design #backend @status:todo [id:product/api-design]".to_string(),
            ],
        };
    }

    let parsed = match parse_node_fragment(value) {
        Ok(node) => node,
        Err(diagnostics) => {
            let mut lines = diagnostics
                .into_iter()
                .take(3)
                .map(|diagnostic| diagnostic.message)
                .collect::<Vec<_>>();
            if lines.is_empty() {
                lines.push("The inline syntax could not be parsed.".to_string());
            }
            return PromptAssist {
                tone: PromptAssistTone::Error,
                lines,
            };
        }
    };

    let mut lines = Vec::new();
    let mut tone = PromptAssistTone::Success;
    if parsed.text.is_empty() {
        tone = PromptAssistTone::Error;
        lines.push("Node labels cannot be empty; add visible text before saving.".to_string());
    } else {
        lines.push(format!("Label: {}", parsed.text));
    }

    if parsed.tags.is_empty() {
        lines.push("Tags: none".to_string());
    } else {
        lines.push(format!("Tags: {}", parsed.tags.join(" ")));
    }

    if parsed.metadata.is_empty() {
        lines.push("Metadata: none".to_string());
    } else {
        lines.push(format!(
            "Metadata: {}",
            parsed
                .metadata
                .iter()
                .map(|entry| format!("@{}:{}", entry.key, entry.value))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }

    if let Some(id) = &parsed.id {
        if let Some(existing_path) = find_path_by_id(&app.editor.document().nodes, id) {
            let is_same_node =
                prompt.mode == PromptMode::Edit && existing_path == app.editor.focus_path();
            if is_same_node {
                if tone != PromptAssistTone::Error {
                    tone = PromptAssistTone::Info;
                }
                lines.push(format!("Id: {id} (unchanged on this node)"));
            } else {
                tone = PromptAssistTone::Error;
                lines.push(format!(
                    "Id: {id} is already used at {}",
                    breadcrumb_for_path(app.editor.document(), &existing_path)
                ));
            }
        } else {
            lines.push(format!("Id: {id} is available"));
        }
    } else {
        if tone == PromptAssistTone::Success {
            tone = PromptAssistTone::Info;
        }
        lines.push("Id: none".to_string());
    }

    PromptAssist { tone, lines }
}

fn prompt_open_id_assist(app: &TuiApp, raw: &str) -> PromptAssist {
    if raw.is_empty() {
        return PromptAssist {
            tone: PromptAssistTone::Info,
            lines: vec![
                "Type an inline id like product/tasks or product/api-design.".to_string(),
                "Ids come from [id:...] tokens on node lines.".to_string(),
            ],
        };
    }

    if let Some(path) = find_path_by_id(&app.editor.document().nodes, raw) {
        let breadcrumb = breadcrumb_for_path(app.editor.document(), &path);
        PromptAssist {
            tone: PromptAssistTone::Success,
            lines: vec![
                format!("Will jump to {breadcrumb}"),
                format!("Target id: {raw}"),
            ],
        }
    } else {
        PromptAssist {
            tone: PromptAssistTone::Warning,
            lines: vec![
                format!("No node id matches '{raw}' yet."),
                "Check the [id:...] token or search the palette for nearby ids.".to_string(),
            ],
        }
    }
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
                relations: node
                    .relations
                    .iter()
                    .map(|relation| relation.display_token())
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
                ViewMode::FullMap => projection.filter.is_some() || include_children_by_expansion,
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
        if minimal_mode_enabled() {
            format!(" {title} ")
        } else if ascii_accents_enabled() {
            format!(" // {title} // ")
        } else {
            format!(" {title} ")
        },
        Style::default()
            .fg(if minimal_mode_enabled() {
                color
            } else {
                palette.background
            })
            .bg(if minimal_mode_enabled() {
                palette.surface
            } else {
                color
            })
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

fn detail_prompt_summary(value: &str) -> String {
    let normalized = value.replace("\r\n", "\n");
    let trimmed = normalized.trim_matches('\n');
    let line_count = if trimmed.is_empty() {
        0
    } else {
        trimmed.lines().count()
    };
    let char_count = normalized.chars().count();
    format!("{line_count} line(s) · {char_count} char(s)")
}

fn styled_detail_lines(view: &TextAreaView, palette: Palette) -> Vec<Line<'static>> {
    view.lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            if index as u16 != view.cursor_y {
                return Line::from(Span::styled(
                    line.clone(),
                    Style::default().fg(palette.text),
                ));
            }

            let cursor_col = view.cursor_x as usize;
            let total_chars = line.chars().count();
            if cursor_col >= total_chars {
                return Line::from(vec![
                    Span::styled(line.clone(), Style::default().fg(palette.text)),
                    Span::styled(
                        " ",
                        Style::default()
                            .fg(palette.background)
                            .bg(palette.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]);
            }

            let cursor_byte = nth_char_boundary(line, cursor_col);
            let next_byte = nth_char_boundary(line, cursor_col + 1);
            let before = line[..cursor_byte].to_string();
            let current = line[cursor_byte..next_byte].to_string();
            let after = line[next_byte..].to_string();

            Line::from(vec![
                Span::styled(before, Style::default().fg(palette.text)),
                Span::styled(
                    current,
                    Style::default()
                        .fg(palette.background)
                        .bg(palette.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(after, Style::default().fg(palette.text)),
            ])
        })
        .collect()
}

fn nth_char_boundary(value: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }
    value
        .char_indices()
        .nth(char_index)
        .map(|(offset, _)| offset)
        .unwrap_or(value.len())
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

fn minimal_mode_enabled() -> bool {
    ACTIVE_MINIMAL_MODE.with(|enabled| enabled.get())
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
    Span::raw(if minimal_mode_enabled() {
        "  "
    } else if ascii_accents_enabled() {
        " | "
    } else {
        " · "
    })
}

fn compact_scope_label(scope_label: &str) -> String {
    scope_label
        .replace("Active filter: ", "")
        .replace("Draft query: ", "draft ")
        .replace("Subtree rooted at ", "subtree ")
        .replace("Whole map", "map")
        .replace(" matching nodes", " matches")
        .replace(" · view ", "  ")
}

fn help_section_heading(title: &'static str, color: Color) -> Line<'static> {
    Line::from(Span::styled(
        format!(" {title} "),
        Style::default()
            .fg(color)
            .bg(active_palette().surface_alt)
            .add_modifier(Modifier::BOLD),
    ))
}

fn help_track_heading(label: &str, palette: Palette) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!(" {} ", label.to_uppercase()),
            Style::default()
                .fg(palette.background)
                .bg(palette.border)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("────────────────", Style::default().fg(palette.border)),
    ])
}

fn help_track_chip(label: &str, palette: Palette, selected: bool) -> Span<'static> {
    Span::styled(
        format!(" {label} "),
        Style::default()
            .fg(if selected {
                palette.background
            } else {
                palette.text
            })
            .bg(if selected {
                palette.accent
            } else {
                palette.surface_alt
            })
            .add_modifier(Modifier::BOLD),
    )
}

fn help_preview_lines(app: &TuiApp, topic: HelpTopic) -> Vec<Line<'static>> {
    let palette = app.theme_colors();
    let mut lines = vec![Line::from(Span::styled(
        topic.summary(),
        Style::default()
            .fg(palette.muted)
            .add_modifier(Modifier::ITALIC),
    ))];
    lines.push(Line::from(""));
    lines.push(help_section_heading("What This Covers", palette.accent));
    lines.push(Line::from(Span::styled(
        topic.guide_intro(),
        Style::default()
            .fg(palette.text)
            .add_modifier(Modifier::ITALIC),
    )));
    lines.push(Line::from(""));
    for paragraph in topic.guide_body() {
        lines.push(Line::from(Span::styled(
            *paragraph,
            Style::default().fg(palette.text),
        )));
        lines.push(Line::from(""));
    }
    lines.push(help_section_heading("Useful Keys", palette.sky));
    for (command, description) in topic.command_reference() {
        lines.push(Line::from(vec![
            Span::styled(
                format!("{command:<18}"),
                Style::default()
                    .fg(palette.sky)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(*description, Style::default().fg(palette.text)),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(help_section_heading("Tips", palette.warn));
    for tip in topic.tips() {
        lines.push(Line::from(vec![
            Span::styled("• ", Style::default().fg(palette.warn)),
            Span::styled(*tip, Style::default().fg(palette.text)),
        ]));
    }
    if let Some(example) = topic.example() {
        lines.push(Line::from(""));
        lines.push(help_section_heading("Try It", palette.border));
        lines.push(Line::from(Span::styled(
            example,
            Style::default()
                .fg(palette.accent)
                .bg(palette.surface_alt)
                .add_modifier(Modifier::BOLD),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Right now ", Style::default().fg(palette.muted)),
        Span::styled(
            help_context_line(app, topic),
            Style::default().fg(palette.warn),
        ),
    ]));
    lines
}

fn wrapped_preview_height(lines: &[Line<'_>], width: u16) -> usize {
    let width = usize::from(width.max(1));
    lines
        .iter()
        .map(|line| wrapped_line_height(line, width))
        .sum()
}

fn wrapped_line_height(line: &Line<'_>, width: usize) -> usize {
    line.to_string()
        .split('\n')
        .map(|segment| segment.chars().count().max(1).div_ceil(width))
        .sum::<usize>()
        .max(1)
}

fn render_preview_scrollbar(
    frame: &mut Frame,
    area: Rect,
    scroll: u16,
    total_height: usize,
    visible_height: usize,
    palette: Palette,
) {
    if area.height == 0 || total_height <= visible_height {
        return;
    }

    let track_height = usize::from(area.height);
    let thumb_height = ((visible_height * track_height) / total_height)
        .max(1)
        .min(track_height);
    let max_thumb_offset = track_height.saturating_sub(thumb_height);
    let max_scroll = total_height.saturating_sub(visible_height).max(1);
    let thumb_offset =
        ((usize::from(scroll) * max_thumb_offset) / max_scroll).min(max_thumb_offset);

    for offset in 0..track_height {
        let glyph = if (thumb_offset..thumb_offset + thumb_height).contains(&offset) {
            "█"
        } else {
            "│"
        };
        let color = if glyph == "█" {
            palette.warn
        } else {
            palette.border
        };
        frame.render_widget(
            Paragraph::new(glyph).style(Style::default().fg(color).bg(palette.surface)),
            Rect::new(area.x, area.y + offset as u16, 1, 1),
        );
    }
}

fn summarize_relationships(app: &TuiApp) -> String {
    let outgoing = app
        .editor
        .current()
        .map(|node| node.relations.len())
        .unwrap_or(0);
    let backlinks = app
        .editor
        .current()
        .and_then(|node| node.id.as_deref())
        .map(|id| backlinks_to(app.editor.document(), id).len())
        .unwrap_or(0);
    let children = app
        .editor
        .current()
        .map(|node| node.children.len())
        .unwrap_or(0);

    format!("{outgoing} outgoing · {backlinks} backlinks · {children} children")
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
fn backlink_lines(app: &TuiApp) -> Vec<Line<'static>> {
    let PALETTE = app.theme_colors();
    let Some(target_id) = app.editor.current().and_then(|node| node.id.as_deref()) else {
        return vec![Line::from(Span::styled(
            "Add an [id:...] token to this node to collect backlinks.",
            Style::default().fg(PALETTE.muted),
        ))];
    };

    let backlinks = backlinks_to(app.editor.document(), target_id);
    if backlinks.is_empty() {
        return vec![Line::from(Span::styled(
            "No backlinks point at this node yet.",
            Style::default().fg(PALETTE.muted),
        ))];
    }

    backlinks
        .into_iter()
        .map(|entry| {
            let relation = if entry.relation == "ref" {
                "ref".to_string()
            } else {
                entry.relation
            };
            Line::from(vec![
                Span::styled("• ", Style::default().fg(PALETTE.accent)),
                Span::styled(entry.breadcrumb, Style::default().fg(PALETTE.text)),
                Span::raw(" "),
                Span::styled(
                    format!("[[{relation}->{}]]", entry.target),
                    Style::default().fg(PALETTE.sky),
                ),
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

fn centered_rect_with_min(
    horizontal_percent: u16,
    vertical_percent: u16,
    min_width: u16,
    min_height: u16,
    area: Rect,
) -> Rect {
    let mut rect = centered_rect(horizontal_percent, vertical_percent, area);
    rect.width = rect.width.max(min_width.min(area.width));
    rect.height = rect.height.max(min_height.min(area.height));
    rect.x = area.x + area.width.saturating_sub(rect.width) / 2;
    rect.y = area.y + area.height.saturating_sub(rect.height) / 2;
    rect
}

fn prompt_overlay_rect(mode: PromptMode, minimal: bool, area: Rect) -> Rect {
    if mode == PromptMode::EditDetail {
        return if minimal {
            centered_rect_with_min(70, 48, 54, 12, area)
        } else {
            centered_rect_with_min(76, 68, 64, 20, area)
        };
    }

    if minimal {
        centered_rect_with_min(60, 18, 48, 8, area)
    } else {
        centered_rect_with_min(68, 40, 56, 17, area)
    }
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

fn line_start(value: &str, index: usize) -> usize {
    value[..index]
        .rfind('\n')
        .map(|position| position + 1)
        .unwrap_or(0)
}

fn line_end(value: &str, index: usize) -> usize {
    value[index..]
        .find('\n')
        .map(|offset| index + offset)
        .unwrap_or(value.len())
}

fn line_column(value: &str, start: usize, index: usize) -> usize {
    value[start..index].chars().count()
}

fn line_index_for_column(value: &str, start: usize, end: usize, column: usize) -> usize {
    for (count, (offset, _)) in value[start..end].char_indices().enumerate() {
        if count == column {
            return start + offset;
        }
    }
    end
}

fn single_line_view(value: &str, cursor: usize, width: usize) -> (String, usize) {
    if width == 0 {
        return (String::new(), 0);
    }
    if value.is_empty() {
        return (String::new(), 0);
    }
    if value.len() <= width {
        return (value.to_string(), cursor.min(value.len()));
    }

    let clamped_cursor = cursor.min(value.len());
    let mut start = clamped_cursor.saturating_sub(width.saturating_sub(1));
    while start > 0 && !value.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = start;
    while end < value.len() {
        let next = next_boundary(value, end);
        if next - start > width {
            break;
        }
        end = next;
    }
    if end == start {
        end = next_boundary(value, start);
    }

    (
        value[start..end].to_string(),
        clamped_cursor.saturating_sub(start).min(width),
    )
}

#[derive(Debug, Clone)]
struct TextAreaView {
    lines: Vec<String>,
    cursor_x: u16,
    cursor_y: u16,
}

fn multiline_view(value: &str, cursor: usize, width: usize, height: usize) -> TextAreaView {
    if width == 0 || height == 0 {
        return TextAreaView {
            lines: vec![String::new()],
            cursor_x: 0,
            cursor_y: 0,
        };
    }

    let mut lines = vec![String::new()];
    let mut visual_line = 0usize;
    let mut visual_col = 0usize;
    let mut cursor_line = 0usize;
    let mut cursor_col = 0usize;
    let mut cursor_recorded = cursor == 0;

    for (index, ch) in value.char_indices() {
        if !cursor_recorded && index == cursor {
            cursor_line = visual_line;
            cursor_col = visual_col;
            cursor_recorded = true;
        }

        if ch == '\n' {
            lines.push(String::new());
            visual_line += 1;
            visual_col = 0;
            continue;
        }

        if visual_col >= width {
            lines.push(String::new());
            visual_line += 1;
            visual_col = 0;
        }

        lines
            .last_mut()
            .expect("multiline view should always have a line")
            .push(ch);
        visual_col += 1;
    }

    if !cursor_recorded {
        cursor_line = visual_line;
        cursor_col = visual_col;
    }

    let scroll = cursor_line.saturating_sub(height.saturating_sub(1));
    let visible_lines = (0..height)
        .map(|offset| lines.get(scroll + offset).cloned().unwrap_or_default())
        .collect::<Vec<_>>();

    TextAreaView {
        lines: visible_lines,
        cursor_x: cursor_col as u16,
        cursor_y: cursor_line.saturating_sub(scroll) as u16,
    }
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

    fn sample_document_with_relations() -> Document {
        parse_document(
            "- Product Idea [id:product]\n  - MVP Scope [id:product/mvp] [[prompts/library]] [[rel:supports->product/requirements]]\n  - Requirements [id:product/requirements]\n  - Prompt Library [id:prompts/library] [[rel:informs->product/mvp]]\n",
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
        let locations_path = crate::locations::locations_path_for(map_path)
            .expect("locations path should be derivable");
        if locations_path.exists() {
            std::fs::remove_file(locations_path).ok();
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
    fn empty_query_palette_defaults_to_a_mixed_home_surface() {
        let map_path = temp_map_path("palette-home.md");
        let document = sample_document();
        let app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        let items = app.palette_items("");
        assert!(!items.is_empty(), "home palette should not be empty");
        assert_ne!(items[0].kind, PaletteItemKind::Action);
        assert!(
            items
                .iter()
                .any(|item| item.kind == PaletteItemKind::Action),
            "home palette should still include a few direct actions"
        );
        assert!(
            items.iter().any(|item| item.title == "Show Help"),
            "home palette should expose the broader action catalog"
        );
        assert!(
            items.iter().any(|item| item.title == "Theme: Monograph"),
            "home palette should expose setup browsing too"
        );
        assert!(
            items.iter().any(|item| {
                matches!(
                    item.kind,
                    PaletteItemKind::Location
                        | PaletteItemKind::Frequent
                        | PaletteItemKind::Recipe
                        | PaletteItemKind::SavedView
                        | PaletteItemKind::History
                        | PaletteItemKind::Checkpoint
                        | PaletteItemKind::Safety
                )
            }),
            "home palette should include contextual, non-action results"
        );
        assert!(
            items.iter().any(|item| item.kind == PaletteItemKind::Help),
            "home palette should include help entries"
        );
        assert!(
            items.iter().all(|item| item.kind != PaletteItemKind::Node),
            "empty query should not immediately dump node search results"
        );

        cleanup_sidecars(&map_path);
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
        for character in "start".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE))
                .expect("typing should update the palette query");
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should open help for the matching topic");

        assert!(app.help.is_some(), "help should open from the palette");
        let topics = app.help_topics(&app.help.as_ref().expect("help should be open").query);
        assert_eq!(topics.first().copied(), Some(HelpTopic::StartHere));
        assert!(app.status.text.contains("Start Here"));

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn command_palette_surfaces_recipe_results() {
        let map_path = temp_map_path("palette-recipes.md");
        let document = sample_document();
        let app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        let items = app.palette_items("review todo");
        let recipe = items
            .iter()
            .find(|item| item.kind == PaletteItemKind::Recipe)
            .expect("recipe result should appear for review todo");
        assert_eq!(recipe.title, "Review Todo Work");

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn command_palette_can_run_a_recipe() {
        let map_path = temp_map_path("palette-recipe-run.md");
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
        for character in "review todo".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE))
                .expect("typing should update the palette query");
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should run the selected recipe");

        assert_eq!(app.view_mode, ViewMode::FilteredFocus);
        assert_eq!(
            app.filter.as_ref().map(|filter| filter.query.raw()),
            Some("#todo")
        );
        assert_eq!(app.editor.focus_path(), &[0, 1]);
        assert_eq!(
            app.status.text,
            "Recipe applied: reviewing #todo work in Filtered Focus."
        );

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn command_palette_surfaces_contextual_owner_recipes() {
        let map_path = temp_map_path("palette-owner-recipe.md");
        let document = parse_document(
            "- Studio [id:studio]\n  - Product @owner:mira @status:planned\n  - Launch @owner:mira @status:blocked\n  - Ops @owner:theo @status:active\n",
        )
        .document;
        let app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        let items = app.palette_items("owner mira");
        let recipe = items
            .iter()
            .find(|item| {
                item.kind == PaletteItemKind::Recipe && item.title == "Review Owner · mira"
            })
            .expect("owner recipe should appear when owner metadata exists");
        assert!(recipe.subtitle.contains("2 nodes"));

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn command_palette_can_run_a_contextual_status_recipe() {
        let map_path = temp_map_path("palette-status-recipe-run.md");
        let document = parse_document(
            "- Studio [id:studio]\n  - Product @owner:mira @status:planned\n  - Launch @owner:mira @status:blocked\n  - Ops @owner:theo @status:active\n",
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

        app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE))
            .expect("colon should open the command palette");
        for character in "status planned".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE))
                .expect("typing should update the palette query");
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should run the contextual recipe");

        assert_eq!(app.view_mode, ViewMode::FilteredFocus);
        assert_eq!(
            app.filter.as_ref().map(|filter| filter.query.raw()),
            Some("@status:planned")
        );
        assert_eq!(app.editor.focus_path(), &[0, 0]);
        assert_eq!(
            app.status.text,
            "Recipe applied: reviewing @status:planned work in Filtered Focus."
        );

        cleanup_sidecars(&map_path);
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
    fn searchable_help_supports_article_scroll_without_changing_topic_selection() {
        let map_path = temp_map_path("help-scroll.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.open_help(Some(HelpTopic::Themes));
        let initial_selected = app.help.as_ref().expect("help should open").selected;
        assert_eq!(
            app.help.as_ref().expect("help should open").preview_scroll,
            0
        );

        app.handle_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE))
            .expect("page down should scroll the article");
        assert_eq!(
            app.help.as_ref().expect("help should stay open").selected,
            initial_selected
        );
        assert!(
            app.help
                .as_ref()
                .expect("help should stay open")
                .preview_scroll
                > 0,
            "page down should move the article preview"
        );

        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
            .expect("down should move to the next topic");
        assert_eq!(
            app.help
                .as_ref()
                .expect("help should stay open")
                .preview_scroll,
            0,
            "changing topics should reset article scroll"
        );

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn help_preview_height_grows_when_the_pane_gets_narrower() {
        let map_path = temp_map_path("help-wrap-height.md");
        let document = sample_document();
        let app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        let lines = help_preview_lines(&app, HelpTopic::Themes);
        let wide_height = wrapped_preview_height(&lines, 60);
        let narrow_height = wrapped_preview_height(&lines, 20);
        assert!(
            narrow_height > wide_height,
            "narrower preview panes should require more wrapped rows"
        );

        cleanup_sidecars(&map_path);
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

        let deep_link_topics = app.help_topics("deep link");
        assert!(
            deep_link_topics.contains(&HelpTopic::Ids),
            "searching for 'deep link' should find the ids topic"
        );

        let backlink_topics = app.help_topics("backlink");
        assert!(
            backlink_topics.contains(&HelpTopic::Relations),
            "searching for 'backlink' should find the relations topic"
        );

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn help_topics_default_to_a_guided_order_instead_of_alphabetical() {
        let map_path = temp_map_path("help-order.md");
        let document = sample_document();
        let app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        let topics = app.help_topics("");
        assert_eq!(topics.first().copied(), Some(HelpTopic::StartHere));
        assert_eq!(topics.get(1).copied(), Some(HelpTopic::Navigation));
        assert_eq!(topics.get(2).copied(), Some(HelpTopic::Editing));
        assert!(
            topics.iter().position(|topic| *topic == HelpTopic::Palette)
                < topics.iter().position(|topic| *topic == HelpTopic::Themes),
            "workflow topics should appear before surface-polish topics"
        );

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn searchable_help_indexes_guide_and_tip_text() {
        let map_path = temp_map_path("help-guide-tip.md");
        let document = sample_document();
        let app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        let noisy_topics = app.help_topics("noisy");
        assert!(
            noisy_topics.contains(&HelpTopic::Navigation),
            "searching for a tip-only term should still find the navigation help article"
        );

        let beginner_topics = app.help_topics("first five minutes");
        assert!(
            beginner_topics.contains(&HelpTopic::StartHere),
            "searching for intro-level guide prose should find the Start Here article"
        );

        let rooted_topics = app.help_topics("temporary workspace");
        assert!(
            rooted_topics.contains(&HelpTopic::Views),
            "searching for guide prose should find the views help article"
        );

        let first_query_topics = app.help_topics("plain text");
        assert!(
            first_query_topics.contains(&HelpTopic::Search),
            "searching for beginner search phrasing should find the search help article"
        );

        let palette_topics = app.help_topics("workflow recipe");
        assert!(
            palette_topics.contains(&HelpTopic::Palette),
            "searching for palette workflow language should find the palette article"
        );

        let safety_topics = app.help_topics("autosave checkpoint");
        assert!(
            safety_topics.contains(&HelpTopic::Safety),
            "searching for recovery language should find the safety article"
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
    fn command_palette_can_apply_and_persist_the_violet_theme() {
        let map_path = temp_map_path("palette-violet-theme.md");
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
        for character in "violet".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE))
                .expect("typing should search violet theme");
        }
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should apply the selected violet theme");

        assert_eq!(app.ui_settings.theme, ThemeId::Violet);

        let loaded_settings =
            load_ui_settings_for(&map_path).expect("ui settings should load after theme apply");
        assert_eq!(loaded_settings.theme, ThemeId::Violet);
        assert_eq!(
            app.theme_colors().background,
            ThemeId::Violet.theme().background
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
    fn command_palette_can_toggle_minimal_mode_and_persist_it() {
        let map_path = temp_map_path("palette-minimal.md");
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
        for character in "minimal".chars() {
            app.handle_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE))
                .expect("typing should search minimal mode");
        }
        assert!(
            app.ui_settings.minimal_mode,
            "selection should preview minimal mode"
        );

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should commit minimal mode");
        assert!(app.ui_settings.minimal_mode);

        let loaded_settings =
            load_ui_settings_for(&map_path).expect("ui settings should load after minimal mode");
        assert!(loaded_settings.minimal_mode);

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
    fn collapsing_a_branch_in_full_map_hides_children_immediately() {
        let map_path = temp_map_path("full-collapse.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        let initial_rows = app.visible_rows();
        assert!(initial_rows.iter().any(|row| row.text == "Direction"));
        assert!(initial_rows.iter().any(|row| row.text == "Tasks"));

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should collapse the focused branch");

        let collapsed_rows = app.visible_rows();
        assert_eq!(
            collapsed_rows
                .iter()
                .map(|row| row.text.as_str())
                .collect::<Vec<_>>(),
            vec!["Product Idea"]
        );
        assert_eq!(app.editor.focus_path(), &[0]);

        let session_path =
            crate::session::session_path_for(&map_path).expect("session path should be derivable");
        if session_path.exists() {
            std::fs::remove_file(session_path).ok();
        }
    }

    #[test]
    fn moving_focus_in_full_map_does_not_auto_expand_collapsed_branches() {
        let map_path = temp_map_path("full-preserve-collapse.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should collapse the focused branch");
        assert_eq!(
            app.visible_rows()
                .iter()
                .map(|row| row.text.as_str())
                .collect::<Vec<_>>(),
            vec!["Product Idea"]
        );

        app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE))
            .expect("moving on a collapsed full-map outline should stay calm");
        assert_eq!(
            app.visible_rows()
                .iter()
                .map(|row| row.text.as_str())
                .collect::<Vec<_>>(),
            vec!["Product Idea"]
        );
        assert_eq!(app.editor.focus_path(), &[0]);

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
    fn detail_prompt_inserts_new_lines_and_saves_with_ctrl_s() {
        let map_path = temp_map_path("detail-prompt.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0, 0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE))
            .expect("d should open the detail prompt");
        assert_eq!(
            app.prompt.as_ref().map(|prompt| prompt.mode),
            Some(PromptMode::EditDetail)
        );

        app.prompt.as_mut().expect("prompt should exist").value = "Line one".to_string();
        app.prompt.as_mut().expect("prompt should exist").cursor = "Line one".len();
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should insert a newline in detail mode");
        app.handle_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE))
            .expect("typing should continue in detail mode");
        app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL))
            .expect("ctrl+s should save the detail prompt");

        assert!(app.prompt.is_none(), "prompt should close after saving");
        assert_eq!(
            app.editor.current().expect("focus should exist").detail,
            vec!["Line one".to_string(), "x".to_string()]
        );

        cleanup_sidecars(&map_path);
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
    fn recent_locations_collect_focus_changes_for_palette_results() {
        let map_path = temp_map_path("recent-locations.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.editor
            .set_focus_path(vec![0, 1])
            .expect("tasks path should exist");
        app.finalize_focus_change(MotionTarget::Focus)
            .expect("focus change should record tasks");

        app.editor
            .set_focus_path(vec![0])
            .expect("root path should exist");
        app.finalize_focus_change(MotionTarget::Focus)
            .expect("focus change should record the root");

        let items = app.palette_recent_location_items("recent");
        assert!(
            items
                .iter()
                .any(|item| item.kind == PaletteItemKind::Location && item.title == "Product Idea")
        );
        assert!(
            items
                .iter()
                .any(|item| item.kind == PaletteItemKind::Location && item.title == "Tasks")
        );

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn palette_can_jump_to_a_recent_location() {
        let map_path = temp_map_path("recent-location-jump.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.editor
            .set_focus_path(vec![0, 1])
            .expect("tasks path should exist");
        app.finalize_focus_change(MotionTarget::Focus)
            .expect("focus change should record tasks");

        app.editor
            .set_focus_path(vec![0])
            .expect("root path should exist");
        app.finalize_focus_change(MotionTarget::Focus)
            .expect("focus change should record the root");

        let task_path = app
            .palette_recent_location_items("tasks")
            .into_iter()
            .find_map(|item| match item.target {
                PaletteTarget::RecentLocation(path) if item.title == "Tasks" => Some(path),
                _ => None,
            })
            .expect("tasks should appear as a recent location");

        app.execute_palette_target(PaletteTarget::RecentLocation(task_path))
            .expect("recent location jump should succeed");

        assert_eq!(
            app.editor.current().expect("focus should exist").text,
            "Tasks"
        );
        assert_eq!(app.status.text, "Returned to recent location 'Tasks'.");

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn palette_surfaces_inline_tag_filters() {
        let map_path = temp_map_path("palette-inline-tag.md");
        let document = sample_document();
        let app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        let item = app
            .palette_items("#todo")
            .into_iter()
            .find(|item| item.kind == PaletteItemKind::Inline && item.title == "#todo")
            .expect("inline tag filter should appear in the palette");

        assert!(matches!(item.target, PaletteTarget::InlineFilter(ref query) if query == "#todo"));
        assert!(item.preview.contains("Filter to nodes tagged #todo."));

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn palette_can_apply_an_inline_metadata_filter() {
        let map_path = temp_map_path("palette-inline-metadata.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.execute_palette_target(PaletteTarget::InlineFilter("@status:active".to_string()))
            .expect("inline metadata filter should apply");

        assert_eq!(
            app.filter.as_ref().map(|filter| filter.query.raw()),
            Some("@status:active")
        );
        assert_eq!(app.editor.focus_path(), &[0, 1]);
        assert_eq!(
            app.status.text,
            "Applied inline filter '@status:active' and landed on the first of 1 matches."
        );

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn palette_can_jump_to_an_inline_id_target() {
        let map_path = temp_map_path("palette-inline-id.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        let item = app
            .palette_items("product/tasks")
            .into_iter()
            .find(|item| item.kind == PaletteItemKind::Inline && item.title == "[id:product/tasks]")
            .expect("inline id target should appear in the palette");

        let PaletteTarget::InlineId(id) = item.target else {
            panic!("inline id result should target the inline id jump");
        };
        app.execute_palette_target(PaletteTarget::InlineId(id))
            .expect("inline id jump should succeed");

        assert_eq!(app.editor.focus_path(), &[0, 1]);
        assert_eq!(app.status.text, "Jumped to inline id 'product/tasks'.");

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn palette_surfaces_outgoing_relation_jumps() {
        let map_path = temp_map_path("palette-relations.md");
        let document = sample_document_with_relations();
        let app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0, 0],
            None,
            false,
            SavedViewsState::default(),
        );

        let item = app
            .palette_items("supports")
            .into_iter()
            .find(|item| {
                item.kind == PaletteItemKind::Relation && item.title == "supports → Requirements"
            })
            .expect("typed outgoing relation should appear in the palette");

        assert!(matches!(item.target, PaletteTarget::RelationPath { .. }));
        assert!(
            item.preview
                .contains("[[rel:supports->product/requirements]]")
        );

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn palette_can_follow_a_relation_target() {
        let map_path = temp_map_path("palette-relation-jump.md");
        let document = sample_document_with_relations();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0, 0],
            None,
            false,
            SavedViewsState::default(),
        );

        let relation_target = app
            .palette_items("requirements")
            .into_iter()
            .find_map(|item| match item.target {
                PaletteTarget::RelationPath { path, message }
                    if item.kind == PaletteItemKind::Relation
                        && item.title == "supports → Requirements" =>
                {
                    Some((path, message))
                }
                _ => None,
            })
            .expect("relation jump should appear in the palette");

        app.execute_palette_target(PaletteTarget::RelationPath {
            path: relation_target.0,
            message: relation_target.1,
        })
        .expect("relation jump should succeed");

        assert_eq!(
            app.editor.current().expect("focus should exist").text,
            "Requirements"
        );
        assert_eq!(
            app.status.text,
            "Followed relation [[rel:supports->product/requirements]] to 'Requirements'."
        );

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn palette_surfaces_backlink_jumps_for_current_node() {
        let map_path = temp_map_path("palette-backlinks.md");
        let document = sample_document_with_relations();
        let app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0, 0],
            None,
            false,
            SavedViewsState::default(),
        );

        let item = app
            .palette_items("backlink")
            .into_iter()
            .find(|item| {
                item.kind == PaletteItemKind::Relation && item.title == "informs ← Prompt Library"
            })
            .expect("incoming backlink should appear in the palette");

        assert!(item.preview.contains("[[rel:informs->product/mvp]]"));

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn relation_keys_follow_outgoing_relations() {
        let map_path = temp_map_path("relation-follow-keys.md");
        let document = sample_document_with_relations();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0, 2],
            None,
            false,
            SavedViewsState::default(),
        );

        app.handle_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE))
            .expect("] should follow the single outgoing relation");
        assert_eq!(
            app.editor.current().expect("focus should exist").text,
            "MVP Scope"
        );
        assert_eq!(app.status.text, "Followed relation informs to 'MVP Scope'.");

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn relation_key_can_follow_backlinks() {
        let map_path = temp_map_path("relation-follow-backlink-key.md");
        let document = sample_document_with_relations();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0, 0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.handle_key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE))
            .expect("[ should follow the first backlink");
        assert_eq!(
            app.editor.current().expect("focus should exist").text,
            "Prompt Library"
        );
        assert_eq!(
            app.status.text,
            "Followed backlink informs from 'Prompt Library'."
        );

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn relation_key_opens_a_picker_when_multiple_outgoing_relations_exist() {
        let map_path = temp_map_path("relation-follow-picker.md");
        let document = sample_document_with_relations();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0, 0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.handle_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE))
            .expect("] should open the relation picker");

        let picker = app
            .relation_picker
            .as_ref()
            .expect("multiple outgoing relations should open a picker");
        assert_eq!(picker.kind, RelationPickerKind::Outgoing);
        assert_eq!(picker.items.len(), 2);
        assert_eq!(
            app.editor.current().expect("focus should stay put").text,
            "MVP Scope"
        );
        assert_eq!(app.status.text, "Choose an outgoing relation to follow.");

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn prompt_assist_flags_duplicate_ids_before_submit() {
        let map_path = temp_map_path("prompt-duplicate-id.md");
        let document = sample_document();
        let app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );
        let prompt = PromptState::new(
            PromptMode::AddChild,
            "New Branch [id:product/tasks]".to_string(),
        );

        let assist = prompt_assist(&app, &prompt);
        assert_eq!(assist.tone, PromptAssistTone::Error);
        assert!(
            assist
                .lines
                .iter()
                .any(|line| line.contains("Id: product/tasks is already used"))
        );

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn prompt_assist_allows_unchanged_id_when_editing_current_node() {
        let map_path = temp_map_path("prompt-same-id.md");
        let document = sample_document();
        let app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0, 1],
            None,
            false,
            SavedViewsState::default(),
        );
        let prompt = PromptState::new(
            PromptMode::Edit,
            "Tasks #todo @status:active [id:product/tasks]".to_string(),
        );

        let assist = prompt_assist(&app, &prompt);
        assert_ne!(assist.tone, PromptAssistTone::Error);
        assert!(
            assist
                .lines
                .iter()
                .any(|line| line.contains("Id: product/tasks (unchanged on this node)"))
        );

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn prompt_assist_previews_jump_to_id_targets() {
        let map_path = temp_map_path("prompt-open-id.md");
        let document = sample_document();
        let app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        let assist = prompt_open_id_assist(&app, "product/tasks");
        assert_eq!(assist.tone, PromptAssistTone::Success);
        assert!(
            assist
                .lines
                .iter()
                .any(|line| line.contains("Will jump to Product Idea / Tasks"))
        );

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn single_line_view_keeps_the_cursor_visible_near_the_end() {
        let (visible, cursor) =
            single_line_view("Tasks #todo @status:active [id:product/tasks]", 43, 16);

        assert!(visible.contains("product/tasks"));
        assert!(cursor <= 16);
        assert!(!visible.is_empty());
    }

    #[test]
    fn prompt_token_kind_detects_inline_node_syntax() {
        assert_eq!(
            prompt_token_kind("#todo", PromptMode::Edit),
            PromptTokenKind::Tag
        );
        assert_eq!(
            prompt_token_kind("@status:active", PromptMode::Edit),
            PromptTokenKind::Metadata
        );
        assert_eq!(
            prompt_token_kind("[id:product/tasks]", PromptMode::Edit),
            PromptTokenKind::Id
        );
        assert_eq!(
            prompt_token_kind("Tasks", PromptMode::Edit),
            PromptTokenKind::Text
        );
    }

    #[test]
    fn prompt_open_id_highlights_the_entire_value_as_an_id() {
        assert_eq!(
            prompt_token_kind("product/tasks", PromptMode::OpenId),
            PromptTokenKind::Id
        );
    }

    #[test]
    fn prompt_overlay_rect_uses_a_safe_minimum_height() {
        let frame_area = Rect::new(0, 0, 100, 24);

        let full = prompt_overlay_rect(PromptMode::AddChild, false, frame_area);
        let minimal = prompt_overlay_rect(PromptMode::AddChild, true, frame_area);
        let detail = prompt_overlay_rect(PromptMode::EditDetail, false, frame_area);

        assert!(full.height >= 17);
        assert!(minimal.height >= 8);
        assert!(full.height > minimal.height);
        assert!(detail.height > full.height);
    }

    #[test]
    fn centered_rect_with_min_clamps_to_available_space() {
        let frame_area = Rect::new(0, 0, 40, 10);
        let rect = centered_rect_with_min(10, 10, 56, 17, frame_area);

        assert_eq!(rect.width, 40);
        assert_eq!(rect.height, 10);
        assert_eq!(rect.x, 0);
        assert_eq!(rect.y, 0);
    }

    #[test]
    fn frequent_locations_collect_revisited_nodes_for_palette_results() {
        let map_path = temp_map_path("frequent-locations.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        for path in [vec![0, 1], vec![0], vec![0, 1], vec![0], vec![0, 1]] {
            app.editor
                .set_focus_path(path)
                .expect("path should exist for frequency test");
            app.finalize_focus_change(MotionTarget::Focus)
                .expect("focus change should update frequent locations");
        }

        let items = app.palette_frequent_location_items("tasks");
        assert!(
            items
                .iter()
                .any(|item| item.kind == PaletteItemKind::Frequent
                    && item.title == "Tasks"
                    && item.subtitle.contains("3 visits"))
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
    fn palette_home_group_summary_uses_shelf_labels() {
        let items = vec![
            PaletteItem {
                kind: PaletteItemKind::Location,
                title: "Tasks".to_string(),
                subtitle: "product/tasks".to_string(),
                preview: String::new(),
                score: 10,
                target: PaletteTarget::RecentLocation(vec![0, 1]),
            },
            PaletteItem {
                kind: PaletteItemKind::Recipe,
                title: "Review Todo Work".to_string(),
                subtitle: "Apply #todo".to_string(),
                preview: String::new(),
                score: 9,
                target: PaletteTarget::Recipe(PaletteRecipe::ReviewTodo),
            },
            PaletteItem {
                kind: PaletteItemKind::Checkpoint,
                title: "Checkpoint · Planning milestone".to_string(),
                subtitle: "manual snapshot".to_string(),
                preview: String::new(),
                score: 8,
                target: PaletteTarget::Checkpoint(0),
            },
            PaletteItem {
                kind: PaletteItemKind::Help,
                title: "Start Here".to_string(),
                subtitle: "Learn the core mental model".to_string(),
                preview: String::new(),
                score: 7,
                target: PaletteTarget::HelpTopic(HelpTopic::StartHere),
            },
        ];

        assert_eq!(
            palette_group_summary_with_mode(&items, 1, true),
            "Recent · [Recipes] · Recovery · Help"
        );
        assert_eq!(
            palette_group_starts_with_mode(&items, true),
            vec![true, true, true, true]
        );
    }

    #[test]
    fn keybar_hides_match_and_relation_hints_when_not_relevant() {
        let map_path = temp_map_path("keybar-base.md");
        let document = sample_document();
        let app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        let rendered = keybar_text(&app);
        assert!(!rendered.contains("n/N"));
        assert!(!rendered.contains("[ ]"));
        assert!(!rendered.contains("m:mindmap"));

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn breadcrumb_truncation_keeps_full_path_when_it_fits() {
        let breadcrumb = vec![
            "Glass Archive".to_string(),
            "Core Cast".to_string(),
            "Character tensions".to_string(),
        ];

        assert_eq!(truncate_breadcrumb_segments(&breadcrumb, 120), breadcrumb);
    }

    #[test]
    fn breadcrumb_truncation_collapses_middle_when_space_is_tight() {
        let breadcrumb = vec![
            "The Glass Archive Novel Research And Writing Map".to_string(),
            "Core Cast".to_string(),
            "Character tensions".to_string(),
        ];

        let truncated = truncate_breadcrumb_segments(&breadcrumb, 48);
        assert!(truncated.len() <= 3);
        assert!(truncated.iter().any(|segment| segment == "…"));
        assert!(
            truncated
                .last()
                .is_some_and(|segment| segment.starts_with("Character"))
        );
    }

    #[test]
    fn keybar_shows_match_hint_only_when_filter_is_active() {
        let map_path = temp_map_path("keybar-filter.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.apply_filter("#todo").expect("filter should apply");
        let rendered = keybar_text(&app);
        assert!(rendered.contains("n/N"));
        assert!(!rendered.contains("[ ]"));

        cleanup_sidecars(&map_path);
    }

    #[test]
    fn keybar_shows_relation_hint_only_when_current_node_has_related_links() {
        let map_path = temp_map_path("keybar-relations.md");
        let document = sample_document_with_relations();
        let app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0, 0],
            None,
            false,
            SavedViewsState::default(),
        );

        let rendered = keybar_text(&app);
        assert!(rendered.contains("[ ]"));
        assert!(!rendered.contains("m:mindmap"));

        cleanup_sidecars(&map_path);
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
    fn search_query_mode_allows_typing_c_without_clearing() {
        let map_path = temp_map_path("search-type-c.md");
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
        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE))
            .expect("typing c should update the query");

        let search = app.search.as_ref().expect("search should stay open");
        assert_eq!(search.draft_query, "c");
        assert_eq!(search.cursor, 1);
        assert_eq!(
            app.status.text,
            "Search open. Tab switches sections. Enter applies the current selection."
        );

        cleanup_sidecars(&map_path);
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

        app.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE))
            .expect("b should open browse");
        assert!(
            app.search
                .as_ref()
                .is_some_and(|search| search.section == SearchSection::Facets),
            "search should open on browse"
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

        app.handle_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE))
            .expect("b should open browse");
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
    fn browse_ids_tab_lists_deep_links_and_enters_them() {
        let map_path = temp_map_path("browse-ids.md");
        let document = sample_document();
        let mut app = TuiApp::new(
            map_path.clone(),
            document,
            vec![0],
            None,
            false,
            SavedViewsState::default(),
        );

        app.open_search_overlay(SearchSection::Facets);
        app.handle_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE))
            .expect("left should wrap browse tabs to ids");
        assert_eq!(
            app.search.as_ref().map(|search| search.facet_tab),
            Some(FacetTab::Ids)
        );

        let items = app.facet_items_for_query(FacetTab::Ids, None);
        assert!(items.iter().any(|item| item.label == "product"));
        let selected = items
            .iter()
            .position(|item| item.label == "product/tasks")
            .expect("product/tasks id should exist");

        app.search
            .as_mut()
            .expect("search should exist")
            .facet_selected = selected;
        app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
            .expect("enter should jump to the selected id");

        assert_eq!(app.editor.focus_path(), &[0, 1]);

        cleanup_sidecars(&map_path);
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

        app.handle_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE))
            .expect("w should open saved views");
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

        app.handle_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE))
            .expect("w should reopen saved views");
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
            "Applied browse item #todo and landed on the first of 2 matches."
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
