use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::prelude::{Color, Modifier, Style};
use ratatui::widgets::Widget;

use crate::model::{Document, MetadataEntry, Node};

const COLUMN_WIDTH: i32 = 34;
const ROOT_GAP: i32 = 3;
const CHILD_GAP: i32 = 2;
const BUBBLE_MIN_WIDTH: usize = 16;
const BUBBLE_MAX_WIDTH: usize = 30;
const INNER_WRAP_WIDTH: usize = 24;
const CONNECTOR_ELBOW_OFFSET: i32 = 6;
const BUBBLE_COLLISION_GAP: i32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BubbleKind {
    Focus,
    Ancestor,
    Descendant,
    Peer,
    Match,
    Context,
    Collapsed,
}

#[derive(Debug, Clone)]
pub struct Bubble {
    pub path: Vec<usize>,
    pub x: i32,
    pub y: i32,
    pub width: usize,
    pub height: usize,
    pub lines: Vec<String>,
    pub kind: BubbleKind,
    pub matched: bool,
    pub child_count: usize,
    pub hidden_children: usize,
}

/// A routed world-space edge between bubbles.
///
/// Routes are orthogonal segments today because the terminal renderer turns them
/// into box-drawing cells. Keeping construction behind helpers makes it easier
/// to add curved or renderer-specific routes without changing layout callers.
#[derive(Debug, Clone)]
pub struct Connector {
    pub from: (i32, i32),
    pub to: (i32, i32),
    pub elbow_x: Option<i32>,
    pub kind: ConnectorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorKind {
    Tree,
    Relation,
}

impl Connector {
    fn tree_segment(from: (i32, i32), to: (i32, i32)) -> Self {
        Self::segment(from, to, ConnectorKind::Tree)
    }

    fn relation_segment(from: (i32, i32), to: (i32, i32)) -> Self {
        Self::segment(from, to, ConnectorKind::Relation)
    }

    fn segment(from: (i32, i32), to: (i32, i32), kind: ConnectorKind) -> Self {
        Self {
            from,
            to,
            elbow_x: None,
            kind,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryEdge {
    Top,
    Bottom,
}

#[derive(Debug, Clone)]
pub struct SpatialLayout {
    /// Positioned nodes in stable document order.
    pub bubbles: Vec<Bubble>,
    /// Routed edges in world coordinates, independent of the terminal viewport.
    pub connectors: Vec<Connector>,
}

#[derive(Debug, Clone)]
pub struct Scene {
    pub bubbles: Vec<Bubble>,
    pub connectors: Vec<Connector>,
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
    pub focus_center: (i32, i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Camera {
    pub origin_x: i32,
    pub origin_y: i32,
    pub width: u16,
    pub height: u16,
    pub zoom_percent: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub background: Color,
    pub surface: Color,
    pub surface_alt: Color,
    pub border: Color,
    pub accent: Color,
    pub sky: Color,
    pub warn: Color,
    pub danger: Color,
    pub tag: Color,
    pub metadata: Color,
    pub id: Color,
    pub query: Color,
    pub attention: Color,
    pub relation: Color,
    pub count: Color,
    pub selection: Color,
    pub selection_text: Color,
    pub text: Color,
    pub muted: Color,
}

#[derive(Debug, Clone, Copy)]
struct BubbleStyle {
    border: Color,
    fill: Color,
    text: Color,
}

#[derive(Debug, Clone)]
struct VisibleNode {
    path: Vec<usize>,
    text: String,
    tags: Vec<String>,
    metadata: Vec<MetadataEntry>,
    id: Option<String>,
    children: Vec<VisibleNode>,
    child_count: usize,
    hidden_children: usize,
    matched: bool,
    kind: BubbleKind,
}

#[derive(Debug, Clone, Copy)]
struct SceneProjection<'a> {
    expanded: &'a HashSet<Vec<usize>>,
    filter_matches: Option<&'a HashSet<Vec<usize>>>,
    visible_paths: Option<&'a HashSet<Vec<usize>>>,
    focus_path: &'a [usize],
    focus_parent: Option<&'a [usize]>,
    focus_ancestors: &'a HashSet<Vec<usize>>,
}

#[derive(Debug, Clone)]
struct MeasuredNode {
    node: VisibleNode,
    width: usize,
    height: usize,
    lines: Vec<String>,
    subtree_height: i32,
    children: Vec<MeasuredNode>,
}

#[derive(Debug, Clone, Copy)]
struct Size {
    width: u16,
    height: u16,
}

#[derive(Debug, Clone, Copy)]
struct ScreenBubbleBounds {
    x0: i32,
    x1: i32,
    y0: i32,
    y1: i32,
}

#[derive(Debug, Clone)]
struct CellSurface {
    width: u16,
    height: u16,
    cells: Vec<Cell>,
}

#[derive(Debug, Clone, Copy)]
struct Cell {
    symbol: char,
    fg: Color,
    bg: Color,
    bold: bool,
}

#[derive(Debug, Clone)]
pub struct MindmapWidget<'a> {
    scene: &'a Scene,
    camera: Camera,
    theme: Theme,
    highlight_path: Option<&'a [usize]>,
    boundary_cue: Option<(&'a [usize], BoundaryEdge)>,
}

impl Scene {
    pub fn build(
        document: &Document,
        focus_path: &[usize],
        expanded: &HashSet<Vec<usize>>,
        filter_matches: Option<&[Vec<usize>]>,
        visible_paths: Option<&HashSet<Vec<usize>>>,
    ) -> Self {
        let focus_parent = if focus_path.is_empty() {
            None
        } else {
            Some(focus_path[..focus_path.len().saturating_sub(1)].to_vec())
        };
        let focus_ancestors = ancestor_set(focus_path);
        let match_set =
            filter_matches.map(|matches| matches.iter().cloned().collect::<HashSet<_>>());
        let projection = SceneProjection {
            expanded,
            filter_matches: match_set.as_ref(),
            visible_paths,
            focus_path,
            focus_parent: focus_parent.as_deref(),
            focus_ancestors: &focus_ancestors,
        };

        let visible = build_visible_nodes(&document.nodes, projection, Vec::new()).0;

        let measured = visible.into_iter().map(measure_node).collect::<Vec<_>>();
        if measured.is_empty() {
            return Self {
                bubbles: Vec::new(),
                connectors: Vec::new(),
                min_x: 0,
                min_y: 0,
                max_x: 0,
                max_y: 0,
                focus_center: (0, 0),
            };
        }

        let total_height = measured.iter().map(|node| node.subtree_height).sum::<i32>()
            + ROOT_GAP * (measured.len().saturating_sub(1) as i32);
        let start_y = 2.max((0 - total_height) / 2);
        let mut cursor_y = start_y;

        let mut bubbles = Vec::new();
        let mut connectors = Vec::new();
        for node in &measured {
            place_node(node, 4, cursor_y, &mut bubbles, &mut connectors);
            cursor_y += node.subtree_height + ROOT_GAP;
        }
        connectors.extend(route_relation_connectors(document, &bubbles));

        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        let mut focus_center = (0, 0);
        for bubble in &bubbles {
            min_x = min_x.min(bubble.x);
            min_y = min_y.min(bubble.y);
            max_x = max_x.max(bubble.x + bubble.width as i32 - 1);
            max_y = max_y.max(bubble.y + bubble.height as i32 - 1);
            if bubble.path == focus_path {
                focus_center = (
                    bubble.x + bubble.width as i32 / 2,
                    bubble.y + bubble.height as i32 / 2,
                );
            }
        }

        if focus_path.is_empty() {
            focus_center = (min_x, min_y);
        }

        Self {
            bubbles,
            connectors,
            min_x,
            min_y,
            max_x,
            max_y,
            focus_center,
        }
    }

    pub fn build_spatial(
        document: &Document,
        focus_path: &[usize],
        expanded: &HashSet<Vec<usize>>,
        filter_matches: Option<&[Vec<usize>]>,
        visible_paths: Option<&HashSet<Vec<usize>>>,
    ) -> Self {
        Self::build_spatial_with_layout_focus(
            document,
            focus_path,
            focus_path,
            expanded,
            filter_matches,
            visible_paths,
        )
    }

    pub fn build_spatial_with_layout_focus(
        document: &Document,
        focus_path: &[usize],
        layout_focus_path: &[usize],
        expanded: &HashSet<Vec<usize>>,
        filter_matches: Option<&[Vec<usize>]>,
        visible_paths: Option<&HashSet<Vec<usize>>>,
    ) -> Self {
        let tree_scene = Self::build(
            document,
            focus_path,
            expanded,
            filter_matches,
            visible_paths,
        );
        SpatialLayout::build(document, tree_scene, focus_path, layout_focus_path)
            .into_scene(focus_path)
    }

    fn from_parts(bubbles: Vec<Bubble>, connectors: Vec<Connector>, focus_path: &[usize]) -> Self {
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        let mut focus_center = (0, 0);
        for bubble in &bubbles {
            min_x = min_x.min(bubble.x);
            min_y = min_y.min(bubble.y);
            max_x = max_x.max(bubble.x + bubble.width as i32 - 1);
            max_y = max_y.max(bubble.y + bubble.height as i32 - 1);
            if bubble.path == focus_path {
                focus_center = (
                    bubble.x + bubble.width as i32 / 2,
                    bubble.y + bubble.height as i32 / 2,
                );
            }
        }

        if bubbles.is_empty() {
            min_x = 0;
            min_y = 0;
            max_x = 0;
            max_y = 0;
        } else if focus_path.is_empty() {
            focus_center = (min_x, min_y);
        }

        Self {
            bubbles,
            connectors,
            min_x,
            min_y,
            max_x,
            max_y,
            focus_center,
        }
    }

    pub fn width(&self) -> i32 {
        (self.max_x - self.min_x + 1).max(0)
    }

    pub fn height(&self) -> i32 {
        (self.max_y - self.min_y + 1).max(0)
    }

    pub fn camera(
        &self,
        viewport_width: u16,
        viewport_height: u16,
        pan_x: i32,
        pan_y: i32,
    ) -> Camera {
        self.camera_with_zoom(viewport_width, viewport_height, pan_x, pan_y, 100)
    }

    pub fn camera_with_zoom(
        &self,
        viewport_width: u16,
        viewport_height: u16,
        pan_x: i32,
        pan_y: i32,
        zoom_percent: u16,
    ) -> Camera {
        let viewport_width = viewport_width.max(1);
        let viewport_height = viewport_height.max(1);
        let zoom_percent = zoom_percent.clamp(50, 200);
        let view_world_width = scaled_viewport(viewport_width, zoom_percent);
        let view_world_height = scaled_viewport(viewport_height, zoom_percent);
        let fit_x = self.width() <= view_world_width;
        let fit_y = self.height() <= view_world_height;

        let origin_x = if fit_x {
            self.min_x - ((view_world_width - self.width()) / 2)
        } else {
            self.focus_center.0 - view_world_width / 2 + pan_x
        };
        let origin_y = if fit_y {
            self.min_y - ((view_world_height - self.height()) / 2)
        } else {
            self.focus_center.1 - view_world_height / 2 + pan_y
        };

        Camera {
            origin_x,
            origin_y,
            width: viewport_width,
            height: viewport_height,
            zoom_percent,
        }
    }

    pub fn focus_camera_with_zoom(
        &self,
        viewport_width: u16,
        viewport_height: u16,
        pan_x: i32,
        pan_y: i32,
        zoom_percent: u16,
    ) -> Camera {
        let viewport_width = viewport_width.max(1);
        let viewport_height = viewport_height.max(1);
        let zoom_percent = zoom_percent.clamp(50, 200);
        let view_world_width = scaled_viewport(viewport_width, zoom_percent);
        let view_world_height = scaled_viewport(viewport_height, zoom_percent);

        Camera {
            origin_x: self.focus_center.0 - view_world_width / 2 + pan_x,
            origin_y: self.focus_center.1 - view_world_height / 2 + pan_y,
            width: viewport_width,
            height: viewport_height,
            zoom_percent,
        }
    }

    pub fn framed_focus_camera_with_zoom(
        &self,
        viewport_width: u16,
        viewport_height: u16,
        pan_x: i32,
        pan_y: i32,
        zoom_percent: u16,
    ) -> Camera {
        let viewport_width = viewport_width.max(1);
        let viewport_height = viewport_height.max(1);
        let zoom_percent = zoom_percent.clamp(50, 200);
        let view_world_width = scaled_viewport(viewport_width, zoom_percent);
        let view_world_height = scaled_viewport(viewport_height, zoom_percent);
        let fit_x = self.width() <= view_world_width;
        let fit_y = self.height() <= view_world_height;

        let origin_x = if fit_x {
            self.min_x - ((view_world_width - self.width()) / 2) + pan_x
        } else {
            self.focus_center.0 - view_world_width / 2 + pan_x
        };
        let origin_y = if fit_y {
            self.min_y - ((view_world_height - self.height()) / 2) + pan_y
        } else {
            self.focus_center.1 - view_world_height / 2 + pan_y
        };

        Camera {
            origin_x,
            origin_y,
            width: viewport_width,
            height: viewport_height,
            zoom_percent,
        }
    }

    pub fn describe(&self) -> String {
        let tree_connectors = self
            .connectors
            .iter()
            .filter(|connector| connector.kind == ConnectorKind::Tree)
            .count();
        let relation_connectors = self
            .connectors
            .iter()
            .filter(|connector| connector.kind == ConnectorKind::Relation)
            .count();
        format!(
            "{} bubbles, {} tree connectors, {} relation edges, canvas {}x{}",
            self.bubbles.len(),
            tree_connectors,
            relation_connectors,
            self.width(),
            self.height()
        )
    }

    pub fn contains_path(&self, path: &[usize]) -> bool {
        self.bubbles.iter().any(|bubble| bubble.path == path)
    }

    pub fn next_path_after(&self, current: &[usize], reverse: bool) -> Option<Vec<usize>> {
        if self.bubbles.is_empty() {
            return None;
        }

        let current_index = self
            .bubbles
            .iter()
            .position(|bubble| bubble.path == current)
            .unwrap_or(0);
        let next_index = if reverse {
            current_index
                .checked_sub(1)
                .unwrap_or_else(|| self.bubbles.len() - 1)
        } else {
            (current_index + 1) % self.bubbles.len()
        };

        self.bubbles
            .get(next_index)
            .map(|bubble| bubble.path.clone())
    }
}

impl SpatialLayout {
    fn build(
        document: &Document,
        tree_scene: Scene,
        focus_path: &[usize],
        layout_focus_path: &[usize],
    ) -> Self {
        if tree_scene.bubbles.is_empty() {
            return Self {
                bubbles: Vec::new(),
                connectors: Vec::new(),
            };
        }

        let mut layout = Self {
            bubbles: place_spatial_bubbles(&tree_scene, layout_focus_path),
            connectors: Vec::new(),
        };
        layout.normalize(focus_path);
        layout.route_connectors(document);
        layout
    }

    fn normalize(&mut self, focus_path: &[usize]) {
        normalize_spatial_collisions(&mut self.bubbles, focus_path);
    }

    fn route_connectors(&mut self, document: &Document) {
        self.connectors = route_tree_connectors(&self.bubbles)
            .into_iter()
            .chain(route_relation_connectors(document, &self.bubbles))
            .collect();
    }

    fn into_scene(self, focus_path: &[usize]) -> Scene {
        Scene::from_parts(self.bubbles, self.connectors, focus_path)
    }
}

impl Theme {
    fn bubble_style(self, kind: BubbleKind, matched: bool) -> BubbleStyle {
        match kind {
            BubbleKind::Focus => BubbleStyle {
                border: self.attention,
                fill: self.selection,
                text: self.selection_text,
            },
            BubbleKind::Ancestor => BubbleStyle {
                border: self.muted,
                fill: self.surface,
                text: self.text,
            },
            BubbleKind::Descendant => BubbleStyle {
                border: self.border,
                fill: self.surface,
                text: self.text,
            },
            BubbleKind::Peer => BubbleStyle {
                border: self.border,
                fill: self.surface,
                text: self.text,
            },
            BubbleKind::Match => BubbleStyle {
                border: self.warn,
                fill: self.surface_alt,
                text: self.text,
            },
            BubbleKind::Collapsed => BubbleStyle {
                border: self.muted,
                fill: self.surface,
                text: self.text,
            },
            BubbleKind::Context => BubbleStyle {
                border: if matched { self.warn } else { self.border },
                fill: self.surface,
                text: self.text,
            },
        }
    }
}

fn scaled_viewport(viewport: u16, zoom_percent: u16) -> i32 {
    ((viewport as i32 * 100) / zoom_percent.max(1) as i32).max(1)
}

fn world_to_screen(value: i32, origin: i32, zoom_percent: u16) -> i32 {
    (((value - origin) as f64) * (zoom_percent as f64 / 100.0)).round() as i32
}

impl<'a> MindmapWidget<'a> {
    pub fn new(scene: &'a Scene, camera: Camera, theme: Theme) -> Self {
        Self {
            scene,
            camera,
            theme,
            highlight_path: None,
            boundary_cue: None,
        }
    }

    pub fn highlight_path(mut self, path: &'a [usize]) -> Self {
        self.highlight_path = Some(path);
        self
    }

    pub fn boundary_cue(mut self, path: &'a [usize], edge: BoundaryEdge) -> Self {
        self.boundary_cue = Some((path, edge));
        self
    }
}

impl Widget for MindmapWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let camera = Camera {
            width: area.width,
            height: area.height,
            ..self.camera
        };
        let surface = render_surface(
            self.scene,
            camera,
            self.theme,
            self.highlight_path,
            self.boundary_cue,
        );

        for y in 0..surface.height {
            for x in 0..surface.width {
                let cell = surface.cells[(y as usize * surface.width as usize) + x as usize];
                if let Some(target) = buf.cell_mut((area.x + x, area.y + y)) {
                    let mut style = Style::default().fg(cell.fg).bg(cell.bg);
                    if cell.bold {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    target.set_symbol(&cell.symbol.to_string());
                    target.set_style(style);
                }
            }
        }
    }
}

pub fn export_png(
    scene: &Scene,
    camera: Camera,
    theme: Theme,
    path: &Path,
) -> Result<PathBuf, String> {
    let surface = render_surface(scene, camera, theme, None, None);
    let pixel_width = surface.width as usize * 10;
    let pixel_height = surface.height as usize * 18;
    let mut pixels = vec![0_u8; pixel_width * pixel_height * 4];

    for y in 0..surface.height as usize {
        for x in 0..surface.width as usize {
            let cell = surface.cells[y * surface.width as usize + x];
            let bg = color_to_rgba(cell.bg);
            fill_rect(&mut pixels, pixel_width, x * 10, y * 18, 10, 18, bg);
            if cell.symbol != ' ' {
                let fg = color_to_rgba(cell.fg);
                draw_glyph(
                    &mut pixels,
                    pixel_width,
                    x * 10 + 2,
                    y * 18 + 5,
                    cell.symbol,
                    fg,
                );
            }
        }
    }

    let png = encode_png(pixel_width as u32, pixel_height as u32, &pixels);
    fs::write(path, png)
        .map_err(|error| format!("Could not write '{}': {error}", path.display()))?;
    Ok(path.to_path_buf())
}

pub fn default_export_path(map_path: &Path) -> PathBuf {
    let stem = map_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "map".to_string());
    map_path.with_file_name(format!("{stem}.mindmap.png"))
}

fn build_visible_nodes(
    nodes: &[Node],
    projection: SceneProjection<'_>,
    prefix: Vec<usize>,
) -> (Vec<VisibleNode>, bool) {
    let mut visible = Vec::new();
    let mut subtree_has_match = false;

    for (index, node) in nodes.iter().enumerate() {
        let mut path = prefix.clone();
        path.push(index);
        let matched = projection
            .filter_matches
            .is_some_and(|matches| matches.contains(&path));
        let show_children =
            projection.filter_matches.is_some() || projection.expanded.contains(&path);
        let (children, child_has_match) =
            build_visible_nodes(&node.children, projection, path.clone());
        let include = match projection.visible_paths {
            Some(paths) => paths.contains(&path),
            None => match projection.filter_matches {
                Some(_) => matched || child_has_match,
                None => true,
            },
        };

        if !include {
            if child_has_match {
                visible.extend(children);
                subtree_has_match = true;
            }
            continue;
        }

        let kind = if path == projection.focus_path {
            BubbleKind::Focus
        } else if projection.focus_ancestors.contains(&path) {
            BubbleKind::Ancestor
        } else if path.starts_with(projection.focus_path) {
            BubbleKind::Descendant
        } else if projection
            .focus_parent
            .is_some_and(|parent| path[..path.len().saturating_sub(1)] == *parent)
        {
            BubbleKind::Peer
        } else if matched {
            BubbleKind::Match
        } else {
            BubbleKind::Context
        };

        let hidden_children = if show_children {
            0
        } else {
            node.children.len()
        };
        let kind = if hidden_children > 0 && kind == BubbleKind::Context {
            BubbleKind::Collapsed
        } else {
            kind
        };

        visible.push(VisibleNode {
            path,
            text: node.text.clone(),
            tags: node.tags.clone(),
            metadata: node.metadata.clone(),
            id: node.id.clone(),
            children: if show_children { children } else { Vec::new() },
            child_count: node.children.len(),
            hidden_children,
            matched,
            kind,
        });
        subtree_has_match = true;
    }

    (visible, subtree_has_match)
}

fn ancestor_set(path: &[usize]) -> HashSet<Vec<usize>> {
    let mut set = HashSet::new();
    if path.len() <= 1 {
        return set;
    }
    for index in 0..path.len() - 1 {
        set.insert(path[..=index].to_vec());
    }
    set
}

fn measure_node(node: VisibleNode) -> MeasuredNode {
    let lines = build_bubble_lines(&node);
    let VisibleNode {
        path,
        text,
        tags,
        metadata,
        id,
        children,
        child_count,
        hidden_children,
        matched,
        kind,
    } = node;
    let width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0)
        .saturating_add(4)
        .clamp(BUBBLE_MIN_WIDTH, BUBBLE_MAX_WIDTH);
    let height = lines.len().max(1) + 2;

    let children = children.into_iter().map(measure_node).collect::<Vec<_>>();
    let child_span = if children.is_empty() {
        0
    } else {
        children
            .iter()
            .map(|child| child.subtree_height)
            .sum::<i32>()
            + CHILD_GAP * (children.len().saturating_sub(1) as i32)
    };
    let subtree_height = (height as i32).max(child_span);

    MeasuredNode {
        node: VisibleNode {
            path,
            text,
            tags,
            metadata,
            id,
            children: Vec::new(),
            child_count,
            hidden_children,
            matched,
            kind,
        },
        width,
        height,
        lines,
        subtree_height,
        children,
    }
}

fn place_node(
    node: &MeasuredNode,
    x: i32,
    subtree_top: i32,
    bubbles: &mut Vec<Bubble>,
    connectors: &mut Vec<Connector>,
) {
    let y = subtree_top + (node.subtree_height - node.height as i32) / 2;
    bubbles.push(Bubble {
        path: node.node.path.clone(),
        x,
        y,
        width: node.width,
        height: node.height,
        lines: node.lines.clone(),
        kind: node.node.kind,
        matched: node.node.matched,
        child_count: node.node.child_count,
        hidden_children: node.node.hidden_children,
    });

    if node.children.is_empty() {
        return;
    }

    let children_total = node
        .children
        .iter()
        .map(|child| child.subtree_height)
        .sum::<i32>()
        + CHILD_GAP * (node.children.len().saturating_sub(1) as i32);
    let mut cursor_y = subtree_top + (node.subtree_height - children_total) / 2;
    let parent_mid = y + node.height as i32 / 2;

    for child in &node.children {
        let child_y = cursor_y + (child.subtree_height - child.height as i32) / 2;
        let child_mid = child_y + child.height as i32 / 2;
        let child_x = x + COLUMN_WIDTH;
        connectors.push(Connector::tree_segment(
            (x + node.width as i32, parent_mid),
            (child_x - 1, child_mid),
        ));
        place_node(child, child_x, cursor_y, bubbles, connectors);
        cursor_y += child.subtree_height + CHILD_GAP;
    }
}

fn route_relation_connectors(document: &Document, bubbles: &[Bubble]) -> Vec<Connector> {
    let mut id_to_anchor = HashMap::new();
    let mut visible_paths = HashSet::new();
    for bubble in bubbles {
        visible_paths.insert(bubble.path.clone());
        if let Some(node) = get_node_by_path(&document.nodes, &bubble.path)
            && let Some(id) = &node.id
        {
            id_to_anchor.insert(id.clone(), bubble_relation_anchor(bubble));
        }
    }

    let mut connectors = Vec::new();
    collect_relation_connectors_from(
        &document.nodes,
        &visible_paths,
        &id_to_anchor,
        &mut connectors,
        Vec::new(),
    );
    connectors
}

fn collect_relation_connectors_from(
    nodes: &[Node],
    visible_paths: &HashSet<Vec<usize>>,
    id_to_anchor: &HashMap<String, (i32, i32)>,
    connectors: &mut Vec<Connector>,
    prefix: Vec<usize>,
) {
    for (index, node) in nodes.iter().enumerate() {
        let mut path = prefix.clone();
        path.push(index);

        if visible_paths.contains(&path)
            && let Some(node_id) = &node.id
            && let Some(from) = id_to_anchor.get(node_id).copied()
        {
            for relation in &node.relations {
                if let Some(to) = id_to_anchor.get(&relation.target).copied()
                    && from != to
                {
                    connectors.push(Connector::relation_segment(from, to));
                }
            }
        }

        collect_relation_connectors_from(
            &node.children,
            visible_paths,
            id_to_anchor,
            connectors,
            path,
        );
    }
}

fn bubble_relation_anchor(bubble: &Bubble) -> (i32, i32) {
    (
        bubble.x + bubble.width as i32 / 2,
        bubble.y + bubble.height as i32 / 2,
    )
}

fn place_spatial_bubbles(tree_scene: &Scene, layout_focus_path: &[usize]) -> Vec<Bubble> {
    let layout_focus_center = tree_scene
        .bubbles
        .iter()
        .find(|bubble| bubble.path == layout_focus_path)
        .map(bubble_relation_anchor)
        .unwrap_or(tree_scene.focus_center);
    let tree_centers = tree_scene
        .bubbles
        .iter()
        .map(|bubble| (bubble.path.clone(), bubble_relation_anchor(bubble)))
        .collect::<HashMap<_, _>>();

    tree_scene
        .bubbles
        .iter()
        .map(|bubble| {
            let mut bubble = bubble.clone();
            place_spatial_bubble(
                &mut bubble,
                layout_focus_path,
                layout_focus_center,
                &tree_centers,
            );
            bubble
        })
        .collect()
}

fn place_spatial_bubble(
    bubble: &mut Bubble,
    focus_path: &[usize],
    tree_focus_center: (i32, i32),
    tree_centers: &HashMap<Vec<usize>, (i32, i32)>,
) {
    let old_center = (
        bubble.x + bubble.width as i32 / 2,
        bubble.y + bubble.height as i32 / 2,
    );
    let relative_y = old_center.1 - tree_focus_center.1;
    let depth_delta = bubble.path.len() as i32 - focus_path.len() as i32;
    let focus_sibling_y = stable_sibling_y(focus_path, tree_focus_center, tree_centers);
    let (center_x, center_y) = if bubble.path == focus_path {
        (0, focus_sibling_y)
    } else if focus_path.starts_with(&bubble.path) {
        let distance = (focus_path.len() - bubble.path.len()) as i32;
        (-COLUMN_WIDTH * distance, -(distance * 2))
    } else if bubble.path.starts_with(focus_path) {
        (
            COLUMN_WIDTH * depth_delta.max(1),
            focus_sibling_y + relative_y,
        )
    } else if is_peer_path(&bubble.path, focus_path) {
        (
            0,
            stable_sibling_y(&bubble.path, tree_focus_center, tree_centers),
        )
    } else {
        (
            (old_center.0 - tree_focus_center.0).clamp(-COLUMN_WIDTH * 2, COLUMN_WIDTH * 2),
            relative_y,
        )
    };

    bubble.x = center_x - bubble.width as i32 / 2;
    bubble.y = center_y - bubble.height as i32 / 2;
}

fn stable_sibling_y(
    path: &[usize],
    fallback_center: (i32, i32),
    tree_centers: &HashMap<Vec<usize>, (i32, i32)>,
) -> i32 {
    if path.is_empty() {
        return 0;
    }

    let parent_path = path[..path.len() - 1].to_vec();
    let parent_center = tree_centers
        .get(&parent_path)
        .copied()
        .unwrap_or(fallback_center);
    let current_center = tree_centers.get(path).copied().unwrap_or(fallback_center);
    current_center.1 - parent_center.1
}

fn normalize_spatial_collisions(bubbles: &mut [Bubble], _focus_path: &[usize]) {
    normalize_spatial_branch_overlaps(bubbles);

    let mut columns: HashMap<i32, Vec<usize>> = HashMap::new();
    for (index, bubble) in bubbles.iter().enumerate() {
        columns
            .entry(spatial_column(bubble))
            .or_default()
            .push(index);
    }

    for indices in columns.values_mut() {
        indices.sort_by(|left, right| {
            bubbles[*left]
                .y
                .cmp(&bubbles[*right].y)
                .then_with(|| bubbles[*left].path.cmp(&bubbles[*right].path))
        });

        let mut min_y = i32::MIN / 2;
        for index in indices {
            let bubble = &mut bubbles[*index];
            if bubble.y <= min_y {
                bubble.y = min_y + CHILD_GAP + 3;
            }
            min_y = bubble.y + bubble.height as i32;
        }
    }

    normalize_spatial_rectangle_overlaps(bubbles);
    normalize_spatial_branch_overlaps(bubbles);
    normalize_spatial_rectangle_overlaps(bubbles);
}

fn normalize_spatial_branch_overlaps(bubbles: &mut [Bubble]) {
    if bubbles.len() < 2 {
        return;
    }

    for _ in 0..bubbles.len() {
        let groups = spatial_sibling_groups(bubbles);
        let mut moved = false;

        for sibling_groups in groups.values() {
            let mut previous_bounds: Option<GroupBounds> = None;
            for group in sibling_groups {
                let mut bounds = group.bounds;
                if let Some(previous) = previous_bounds {
                    if bounds_overlap_horizontally(previous, bounds)
                        && bounds.min_y <= previous.max_y + BUBBLE_COLLISION_GAP
                    {
                        let dy = previous.max_y + BUBBLE_COLLISION_GAP + 1 - bounds.min_y;
                        shift_bubble_group(bubbles, &group.root_path, dy);
                        bounds = bounds.shifted(dy);
                        moved = true;
                    }
                }
                previous_bounds = Some(match previous_bounds {
                    Some(previous) if previous.max_y > bounds.max_y => previous,
                    _ => bounds,
                });
            }
        }

        if !moved {
            break;
        }
    }
}

#[derive(Debug, Clone)]
struct BranchGroup {
    root_path: Vec<usize>,
    bounds: GroupBounds,
}

#[derive(Debug, Clone, Copy)]
struct GroupBounds {
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
}

impl GroupBounds {
    fn shifted(self, dy: i32) -> Self {
        Self {
            min_y: self.min_y + dy,
            max_y: self.max_y + dy,
            ..self
        }
    }
}

fn spatial_sibling_groups(bubbles: &[Bubble]) -> HashMap<Vec<usize>, Vec<BranchGroup>> {
    let visible_paths = bubbles
        .iter()
        .map(|bubble| bubble.path.clone())
        .collect::<HashSet<_>>();
    let mut groups: HashMap<Vec<usize>, Vec<BranchGroup>> = HashMap::new();

    for bubble in bubbles {
        if bubble.path.is_empty() {
            continue;
        }
        let parent_path = bubble.path[..bubble.path.len() - 1].to_vec();
        if !parent_path.is_empty() && !visible_paths.contains(&parent_path) {
            continue;
        }
        groups.entry(parent_path).or_default().push(BranchGroup {
            root_path: bubble.path.clone(),
            bounds: subtree_branch_bounds(bubbles, &bubble.path),
        });
    }

    for sibling_groups in groups.values_mut() {
        sibling_groups.sort_by(|left, right| left.root_path.cmp(&right.root_path));
    }
    groups
}

fn subtree_branch_bounds(bubbles: &[Bubble], root_path: &[usize]) -> GroupBounds {
    subtree_bounds(bubbles, root_path, true).expect("branch root should be present")
}

fn subtree_bounds(
    bubbles: &[Bubble],
    root_path: &[usize],
    include_root: bool,
) -> Option<GroupBounds> {
    let mut bounds: Option<GroupBounds> = None;
    for bubble in bubbles.iter().filter(|bubble| {
        bubble.path.starts_with(root_path) && (include_root || bubble.path != root_path)
    }) {
        let bubble_bounds = GroupBounds {
            min_x: bubble.x,
            max_x: bubble.x + bubble.width as i32 - 1,
            min_y: bubble.y,
            max_y: bubble_bottom(bubble),
        };
        bounds = Some(match bounds {
            Some(bounds) => GroupBounds {
                min_x: bounds.min_x.min(bubble_bounds.min_x),
                max_x: bounds.max_x.max(bubble_bounds.max_x),
                min_y: bounds.min_y.min(bubble_bounds.min_y),
                max_y: bounds.max_y.max(bubble_bounds.max_y),
            },
            None => bubble_bounds,
        });
    }
    bounds
}

fn bounds_overlap_horizontally(left: GroupBounds, right: GroupBounds) -> bool {
    left.min_x <= right.max_x && right.min_x <= left.max_x
}

fn shift_bubble_group(bubbles: &mut [Bubble], root_path: &[usize], dy: i32) {
    for bubble in bubbles
        .iter_mut()
        .filter(|bubble| bubble.path.starts_with(root_path))
    {
        bubble.y += dy;
    }
}

fn normalize_spatial_rectangle_overlaps(bubbles: &mut [Bubble]) {
    if bubbles.len() < 2 {
        return;
    }

    for _ in 0..bubbles.len() {
        let mut moved = false;
        let mut indices = (0..bubbles.len()).collect::<Vec<_>>();
        indices.sort_by(|left, right| {
            bubbles[*left]
                .y
                .cmp(&bubbles[*right].y)
                .then_with(|| bubbles[*left].path.cmp(&bubbles[*right].path))
        });

        for current_position in 0..indices.len() {
            let upper_index = indices[current_position];
            for lower_index in indices.iter().skip(current_position + 1).copied() {
                if !bubbles_overlap_horizontally(&bubbles[upper_index], &bubbles[lower_index]) {
                    continue;
                }

                let minimum_y = bubble_bottom(&bubbles[upper_index]) + BUBBLE_COLLISION_GAP + 1;
                if bubbles[lower_index].y < minimum_y {
                    bubbles[lower_index].y = minimum_y;
                    moved = true;
                }
            }
        }

        if !moved {
            break;
        }
    }
}

fn bubbles_overlap_horizontally(left: &Bubble, right: &Bubble) -> bool {
    let left_end = left.x + left.width as i32 - 1;
    let right_end = right.x + right.width as i32 - 1;
    left.x <= right_end && right.x <= left_end
}

fn bubble_bottom(bubble: &Bubble) -> i32 {
    bubble.y + bubble.height as i32 - 1
}

fn spatial_column(bubble: &Bubble) -> i32 {
    (bubble.x + bubble.width as i32 / 2).div_euclid(COLUMN_WIDTH)
}

fn is_peer_path(path: &[usize], focus_path: &[usize]) -> bool {
    !path.is_empty()
        && !focus_path.is_empty()
        && path != focus_path
        && path.len() == focus_path.len()
        && path[..path.len() - 1] == focus_path[..focus_path.len() - 1]
}

fn route_tree_connectors(bubbles: &[Bubble]) -> Vec<Connector> {
    let mut path_to_bubble = HashMap::new();
    for bubble in bubbles {
        path_to_bubble.insert(bubble.path.clone(), bubble);
    }

    let mut child_groups: HashMap<(Vec<usize>, i32), Vec<&Bubble>> = HashMap::new();
    for bubble in bubbles {
        if bubble.path.is_empty() {
            continue;
        }
        let mut parent_path = bubble.path[..bubble.path.len() - 1].to_vec();
        while !parent_path.is_empty() {
            if let Some(parent) = path_to_bubble.get(&parent_path) {
                let parent_center = bubble_relation_anchor(parent);
                let child_center = bubble_relation_anchor(bubble);
                let direction = (child_center.0 - parent_center.0).signum();
                child_groups
                    .entry((
                        parent.path.clone(),
                        if direction == 0 { 1 } else { direction },
                    ))
                    .or_default()
                    .push(bubble);
                break;
            }
            parent_path.pop();
        }
    }

    let mut connectors = Vec::new();
    for ((parent_path, direction), children) in child_groups {
        let Some(parent) = path_to_bubble.get(&parent_path) else {
            continue;
        };
        connectors.extend(tree_group_connectors(parent, direction, &children));
    }
    connectors
}

fn tree_group_connectors(parent: &Bubble, direction: i32, children: &[&Bubble]) -> Vec<Connector> {
    if children.is_empty() {
        return Vec::new();
    }

    let parent_center = bubble_relation_anchor(parent);
    let parent_anchor = if direction >= 0 {
        (parent.x + parent.width as i32, parent_center.1)
    } else {
        (parent.x - 1, parent_center.1)
    };
    let child_anchors = children
        .iter()
        .map(|child| {
            let child_center = bubble_relation_anchor(child);
            if direction >= 0 {
                (child.x - 1, child_center.1)
            } else {
                (child.x + child.width as i32, child_center.1)
            }
        })
        .collect::<Vec<_>>();
    let nearest_child_x = if direction >= 0 {
        child_anchors
            .iter()
            .map(|anchor| anchor.0)
            .min()
            .unwrap_or(parent_anchor.0)
    } else {
        child_anchors
            .iter()
            .map(|anchor| anchor.0)
            .max()
            .unwrap_or(parent_anchor.0)
    };
    let spine_x = tree_connector_elbow_x(parent_anchor.0, nearest_child_x, &parent.path);
    let min_y = child_anchors
        .iter()
        .map(|anchor| anchor.1)
        .chain(std::iter::once(parent_anchor.1))
        .min()
        .unwrap_or(parent_anchor.1);
    let max_y = child_anchors
        .iter()
        .map(|anchor| anchor.1)
        .chain(std::iter::once(parent_anchor.1))
        .max()
        .unwrap_or(parent_anchor.1);

    let mut connectors = vec![
        Connector::tree_segment(parent_anchor, (spine_x, parent_anchor.1)),
        Connector::tree_segment((spine_x, min_y), (spine_x, max_y)),
    ];
    connectors.extend(
        child_anchors
            .into_iter()
            .map(|child_anchor| Connector::tree_segment((spine_x, child_anchor.1), child_anchor)),
    );
    connectors
}

fn tree_connector_elbow_x(from_x: i32, to_x: i32, parent_path: &[usize]) -> i32 {
    let distance = (to_x - from_x).abs();
    if distance == 0 {
        return from_x;
    }

    let direction = (to_x - from_x).signum();
    let lane = parent_path.last().copied().unwrap_or(0) as i32 % 4;
    let offset = (CONNECTOR_ELBOW_OFFSET + lane * 3).min(distance).max(1);
    from_x + direction * offset
}

fn get_node_by_path<'a>(nodes: &'a [Node], path: &[usize]) -> Option<&'a Node> {
    let mut current = nodes;
    let mut node = None;
    for index in path {
        node = current.get(*index);
        current = &node?.children;
    }
    node
}

fn build_bubble_lines(node: &VisibleNode) -> Vec<String> {
    let mut lines = wrap_text(
        if node.text.is_empty() {
            "(empty)"
        } else {
            &node.text
        },
        INNER_WRAP_WIDTH,
    );
    if lines.len() > 2 {
        lines.truncate(2);
    }

    if !node.tags.is_empty() {
        lines.push(truncate(&node.tags.join(" "), INNER_WRAP_WIDTH));
    } else if !node.metadata.is_empty() {
        lines.push(truncate(&format_metadata(&node.metadata), INNER_WRAP_WIDTH));
    }

    if let Some(id) = &node.id {
        lines.push(truncate(id, INNER_WRAP_WIDTH));
    }

    lines.truncate(3);
    lines
}

fn format_metadata(entries: &[MetadataEntry]) -> String {
    entries
        .iter()
        .take(2)
        .map(|entry| format!("@{}:{}", entry.key, entry.value))
        .collect::<Vec<_>>()
        .join(" ")
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let needs_space = !current.is_empty();
        let proposed = current.chars().count() + word.chars().count() + usize::from(needs_space);
        if proposed > width && !current.is_empty() {
            lines.push(current);
            current = word.to_string();
        } else {
            if needs_space {
                current.push(' ');
            }
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        vec![text.to_string()]
    } else {
        lines
    }
}

fn truncate(text: &str, width: usize) -> String {
    let count = text.chars().count();
    if count <= width {
        return text.to_string();
    }
    text.chars()
        .take(width.saturating_sub(1))
        .collect::<String>()
        + "…"
}

fn render_surface(
    scene: &Scene,
    camera: Camera,
    theme: Theme,
    highlight_path: Option<&[usize]>,
    boundary_cue: Option<(&[usize], BoundaryEdge)>,
) -> CellSurface {
    let size = Size {
        width: camera.width.max(1),
        height: camera.height.max(1),
    };
    let mut surface = CellSurface::new(size, theme.background);
    let mut tree_masks = vec![0_u8; size.width as usize * size.height as usize];
    let mut relation_masks = vec![0_u8; size.width as usize * size.height as usize];

    for connector in &scene.connectors {
        match connector.kind {
            ConnectorKind::Tree => draw_connector(&mut tree_masks, size, camera, connector),
            ConnectorKind::Relation => draw_connector(&mut relation_masks, size, camera, connector),
        }
    }
    for y in 0..size.height as usize {
        for x in 0..size.width as usize {
            let mask = tree_masks[y * size.width as usize + x];
            if mask != 0 {
                let symbol = line_symbol(mask);
                let cell = surface.get_mut(x as i32, y as i32);
                cell.symbol = symbol;
                cell.fg = theme.muted;
                cell.bg = theme.background;
            }
        }
    }
    for y in 0..size.height as usize {
        for x in 0..size.width as usize {
            let mask = relation_masks[y * size.width as usize + x];
            if mask != 0 {
                let symbol = line_symbol(mask);
                let cell = surface.get_mut(x as i32, y as i32);
                cell.symbol = symbol;
                cell.fg = theme.relation;
                cell.bg = theme.background;
                cell.bold = true;
            }
        }
    }

    for bubble in &scene.bubbles {
        draw_shadow(&mut surface, bubble, camera, theme);
    }
    for bubble in &scene.bubbles {
        let highlighted = highlight_path.is_some_and(|path| path == bubble.path.as_slice());
        let boundary_edge = boundary_cue.and_then(|(path, edge)| {
            if path == bubble.path.as_slice() {
                Some(edge)
            } else {
                None
            }
        });
        draw_bubble(
            &mut surface,
            bubble,
            camera,
            theme,
            highlighted,
            boundary_edge,
        );
    }

    surface
}

fn draw_shadow(surface: &mut CellSurface, bubble: &Bubble, camera: Camera, theme: Theme) {
    let shadow_x = world_to_screen(bubble.x + 1, camera.origin_x, camera.zoom_percent);
    let shadow_y = world_to_screen(bubble.y + 1, camera.origin_y, camera.zoom_percent);
    let shadow_x1 = world_to_screen(
        bubble.x + bubble.width as i32,
        camera.origin_x,
        camera.zoom_percent,
    )
    .max(shadow_x);
    let shadow_y1 = world_to_screen(
        bubble.y + bubble.height as i32,
        camera.origin_y,
        camera.zoom_percent,
    )
    .max(shadow_y);
    let shadow = theme.background;
    for y in shadow_y..=shadow_y1 {
        for x in shadow_x..=shadow_x1 {
            if let Some(cell) = surface.get_mut_checked(x, y) {
                cell.bg = shadow;
            }
        }
    }
}

fn draw_bubble(
    surface: &mut CellSurface,
    bubble: &Bubble,
    camera: Camera,
    theme: Theme,
    highlighted: bool,
    boundary_edge: Option<BoundaryEdge>,
) {
    let mut style = theme.bubble_style(bubble.kind, bubble.matched);
    if highlighted && bubble.kind != BubbleKind::Focus {
        style = BubbleStyle {
            border: theme.attention,
            fill: theme.selection,
            text: theme.selection_text,
        };
    }
    let x0 = world_to_screen(bubble.x, camera.origin_x, camera.zoom_percent);
    let y0 = world_to_screen(bubble.y, camera.origin_y, camera.zoom_percent);
    let x1 = world_to_screen(
        bubble.x + bubble.width as i32 - 1,
        camera.origin_x,
        camera.zoom_percent,
    )
    .max(x0 + 5);
    let y1 = world_to_screen(
        bubble.y + bubble.height as i32 - 1,
        camera.origin_y,
        camera.zoom_percent,
    )
    .max(y0 + 2);
    let rendered_width = (x1 - x0 + 1).max(1) as usize;
    let rendered_height = (y1 - y0 + 1).max(1) as usize;

    for y in y0..=y1 {
        for x in x0..=x1 {
            let Some(cell) = surface.get_mut_checked(x, y) else {
                continue;
            };
            let border = x == x0 || x == x1 || y == y0 || y == y1;
            cell.bg = style.fill;
            cell.fg = if border { style.border } else { style.text };
            cell.symbol = if x == x0 && y == y0 {
                '╭'
            } else if x == x1 && y == y0 {
                '╮'
            } else if x == x0 && y == y1 {
                '╰'
            } else if x == x1 && y == y1 {
                '╯'
            } else if y == y0 || y == y1 {
                '─'
            } else if x == x0 || x == x1 {
                '│'
            } else {
                ' '
            };
            cell.bold = (bubble.kind == BubbleKind::Focus || highlighted) && border;
        }
    }

    for (index, line) in bubble.lines.iter().enumerate() {
        if index >= rendered_height.saturating_sub(2) {
            break;
        }
        let y = y0 + 1 + index as i32;
        let available = rendered_width.saturating_sub(2);
        let text = truncate(line, available);
        let title_line = is_title_line(&bubble.lines, index);
        for (offset, character) in text.chars().enumerate() {
            let Some(cell) = surface.get_mut_checked(x0 + 1 + offset as i32, y) else {
                continue;
            };
            cell.symbol = character;
            cell.fg = if bubble.kind == BubbleKind::Focus {
                if title_line {
                    theme.selection_text
                } else {
                    theme.muted
                }
            } else if highlighted {
                theme.selection_text
            } else if title_line {
                style.text
            } else {
                theme.muted
            };
            cell.bg = style.fill;
            cell.bold = title_line;
        }
    }

    if bubble.matched && bubble.kind != BubbleKind::Focus {
        if let Some(cell) = surface.get_mut_checked(x1 - 1, y0 + 1) {
            cell.symbol = '●';
            cell.fg = theme.warn;
            cell.bg = style.fill;
            cell.bold = true;
        }
    }

    if highlighted && bubble.kind != BubbleKind::Focus {
        if let Some(cell) = surface.get_mut_checked(x0 + 1, y0) {
            cell.symbol = '◆';
            cell.fg = theme.attention;
            cell.bg = style.fill;
            cell.bold = true;
        }
    }

    if bubble.child_count > 0 {
        draw_child_marker(
            surface,
            x1,
            y0,
            y1,
            style.fill,
            theme,
            bubble.hidden_children > 0,
        );
    }

    if let Some(edge) = boundary_edge {
        draw_boundary_edge(
            surface,
            ScreenBubbleBounds { x0, x1, y0, y1 },
            edge,
            style.fill,
            theme,
        );
    }
}

fn draw_child_marker(
    surface: &mut CellSurface,
    x1: i32,
    y0: i32,
    y1: i32,
    fill: Color,
    theme: Theme,
    collapsed: bool,
) {
    let y = y0 + ((y1 - y0) / 2).max(1);
    if let Some(cell) = surface.get_mut_checked(x1, y) {
        cell.symbol = if collapsed { '›' } else { '•' };
        cell.fg = theme.count;
        cell.bg = fill;
        cell.bold = true;
    }
}

fn draw_boundary_edge(
    surface: &mut CellSurface,
    bounds: ScreenBubbleBounds,
    edge: BoundaryEdge,
    fill: Color,
    theme: Theme,
) {
    let y = match edge {
        BoundaryEdge::Top => bounds.y0,
        BoundaryEdge::Bottom => bounds.y1,
    };
    for x in bounds.x0..=bounds.x1 {
        let Some(cell) = surface.get_mut_checked(x, y) else {
            continue;
        };
        cell.fg = theme.warn;
        cell.bg = fill;
        cell.bold = true;
    }
}

fn is_title_line(lines: &[String], index: usize) -> bool {
    if index == 0 {
        return true;
    }
    if index > 1 {
        return false;
    }

    let Some(line) = lines.get(index) else {
        return false;
    };
    !line.starts_with('#')
        && !line.starts_with('@')
        && !line.starts_with("folded ")
        && !line.contains('/')
}

fn draw_connector(masks: &mut [u8], size: Size, camera: Camera, connector: &Connector) {
    let x1 = world_to_screen(connector.from.0, camera.origin_x, camera.zoom_percent);
    let y1 = world_to_screen(connector.from.1, camera.origin_y, camera.zoom_percent);
    let x2 = world_to_screen(connector.to.0, camera.origin_x, camera.zoom_percent);
    let y2 = world_to_screen(connector.to.1, camera.origin_y, camera.zoom_percent);
    let elbow_x = connector
        .elbow_x
        .map(|world_x| world_to_screen(world_x, camera.origin_x, camera.zoom_percent))
        .unwrap_or_else(|| connector_elbow_x(x1, x2));

    if !connector_intersects_surface(size, x1, y1, elbow_x, x2, y2) {
        return;
    }

    add_horizontal(masks, size, x1, elbow_x, y1);
    add_vertical(masks, size, elbow_x, y1, y2);
    add_horizontal(masks, size, elbow_x, x2, y2);
}

fn connector_elbow_x(from_x: i32, to_x: i32) -> i32 {
    let distance = (to_x - from_x).abs();
    if distance == 0 {
        return from_x;
    }

    let direction = (to_x - from_x).signum();
    let offset = distance.clamp(1, CONNECTOR_ELBOW_OFFSET);
    from_x + direction * offset
}

fn point_on_surface(size: Size, x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && x < size.width as i32 && y < size.height as i32
}

fn connector_intersects_surface(
    size: Size,
    x1: i32,
    y1: i32,
    elbow_x: i32,
    x2: i32,
    y2: i32,
) -> bool {
    point_on_surface(size, x1, y1)
        || point_on_surface(size, x2, y2)
        || horizontal_segment_intersects_surface(size, x1, elbow_x, y1)
        || vertical_segment_intersects_surface(size, elbow_x, y1, y2)
        || horizontal_segment_intersects_surface(size, elbow_x, x2, y2)
}

fn horizontal_segment_intersects_surface(size: Size, x1: i32, x2: i32, y: i32) -> bool {
    if y < 0 || y >= size.height as i32 {
        return false;
    }
    let start = x1.min(x2);
    let end = x1.max(x2);
    end >= 0 && start < size.width as i32
}

fn vertical_segment_intersects_surface(size: Size, x: i32, y1: i32, y2: i32) -> bool {
    if x < 0 || x >= size.width as i32 {
        return false;
    }
    let start = y1.min(y2);
    let end = y1.max(y2);
    end >= 0 && start < size.height as i32
}

fn add_horizontal(masks: &mut [u8], size: Size, x1: i32, x2: i32, y: i32) {
    let start = x1.min(x2);
    let end = x1.max(x2);
    for x in start..=end {
        if x > start {
            add_mask(masks, size, x, y, WEST);
        }
        if x < end {
            add_mask(masks, size, x, y, EAST);
        }
    }
}

fn add_vertical(masks: &mut [u8], size: Size, x: i32, y1: i32, y2: i32) {
    let start = y1.min(y2);
    let end = y1.max(y2);
    for y in start..=end {
        if y > start {
            add_mask(masks, size, x, y, NORTH);
        }
        if y < end {
            add_mask(masks, size, x, y, SOUTH);
        }
    }
}

fn add_mask(masks: &mut [u8], size: Size, x: i32, y: i32, mask: u8) {
    if x < 0 || y < 0 || x >= size.width as i32 || y >= size.height as i32 {
        return;
    }
    let index = y as usize * size.width as usize + x as usize;
    masks[index] |= mask;
}

const NORTH: u8 = 0b0001;
const EAST: u8 = 0b0010;
const SOUTH: u8 = 0b0100;
const WEST: u8 = 0b1000;

fn line_symbol(mask: u8) -> char {
    const CAP_DOWN: u8 = SOUTH;
    const CAP_RIGHT: u8 = EAST;
    const CAP_UP: u8 = NORTH;
    const CAP_LEFT: u8 = WEST;
    const HORIZONTAL: u8 = EAST | WEST;
    const VERTICAL: u8 = NORTH | SOUTH;
    const DOWN_RIGHT: u8 = SOUTH | EAST;
    const DOWN_LEFT: u8 = SOUTH | WEST;
    const UP_RIGHT: u8 = NORTH | EAST;
    const UP_LEFT: u8 = NORTH | WEST;
    const T_RIGHT: u8 = NORTH | EAST | SOUTH;
    const T_LEFT: u8 = NORTH | SOUTH | WEST;
    const T_DOWN: u8 = EAST | SOUTH | WEST;
    const T_UP: u8 = NORTH | EAST | WEST;
    const CROSS: u8 = NORTH | EAST | SOUTH | WEST;
    match mask {
        CAP_DOWN => '╷',
        CAP_RIGHT => '╶',
        CAP_UP => '╵',
        CAP_LEFT => '╴',
        HORIZONTAL => '─',
        VERTICAL => '│',
        DOWN_RIGHT => '╭',
        DOWN_LEFT => '╮',
        UP_RIGHT => '╰',
        UP_LEFT => '╯',
        T_RIGHT => '├',
        T_LEFT => '┤',
        T_DOWN => '┬',
        T_UP => '┴',
        CROSS => '┼',
        _ => '·',
    }
}

impl CellSurface {
    fn new(size: Size, background: Color) -> Self {
        Self {
            width: size.width,
            height: size.height,
            cells: vec![
                Cell {
                    symbol: ' ',
                    fg: Color::Reset,
                    bg: background,
                    bold: false,
                };
                size.width as usize * size.height as usize
            ],
        }
    }

    fn get_mut(&mut self, x: i32, y: i32) -> &mut Cell {
        let index = y as usize * self.width as usize + x as usize;
        &mut self.cells[index]
    }

    fn get_mut_checked(&mut self, x: i32, y: i32) -> Option<&mut Cell> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            None
        } else {
            Some(self.get_mut(x, y))
        }
    }
}

fn color_to_rgba(color: Color) -> [u8; 4] {
    match color {
        Color::Rgb(r, g, b) => [r, g, b, 255],
        Color::Black => [0, 0, 0, 255],
        Color::White => [255, 255, 255, 255],
        Color::Gray => [128, 128, 128, 255],
        Color::DarkGray => [64, 64, 64, 255],
        Color::Red => [220, 38, 38, 255],
        Color::Green => [22, 163, 74, 255],
        Color::Blue => [37, 99, 235, 255],
        Color::Yellow => [234, 179, 8, 255],
        Color::Magenta => [192, 38, 211, 255],
        Color::Cyan => [6, 182, 212, 255],
        _ => [18, 24, 35, 255],
    }
}

fn fill_rect(
    pixels: &mut [u8],
    image_width: usize,
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: [u8; 4],
) {
    let image_height = pixels.len() / (image_width * 4);
    for row in y..(y + height).min(image_height) {
        for column in x..(x + width).min(image_width) {
            let index = (row * image_width + column) * 4;
            pixels[index..index + 4].copy_from_slice(&color);
        }
    }
}

fn draw_glyph(
    pixels: &mut [u8],
    image_width: usize,
    x: usize,
    y: usize,
    character: char,
    color: [u8; 4],
) {
    let glyph = glyph_rows(character);
    for (row_index, row) in glyph.iter().enumerate() {
        for bit in 0..5 {
            if row & (1 << (4 - bit)) != 0 {
                fill_rect(pixels, image_width, x + bit, y + row_index, 1, 1, color);
            }
        }
    }
}

fn glyph_rows(character: char) -> [u8; 7] {
    let character = if character.is_ascii_lowercase() {
        character.to_ascii_uppercase()
    } else {
        character
    };

    match character {
        'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        'C' => [0x0E, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0E],
        'D' => [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E],
        'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        'G' => [0x0E, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0F],
        'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'I' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x1F],
        'J' => [0x01, 0x01, 0x01, 0x01, 0x11, 0x11, 0x0E],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x11, 0x19, 0x15, 0x13, 0x11, 0x11],
        'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        'S' => [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
        'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0A],
        'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x14, 0x04, 0x04, 0x04, 0x1F],
        '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        '3' => [0x1F, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x1E, 0x01, 0x01, 0x11, 0x0E],
        '6' => [0x06, 0x08, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x02, 0x0C],
        '-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        '_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1F],
        '/' => [0x01, 0x02, 0x02, 0x04, 0x08, 0x08, 0x10],
        ':' => [0x00, 0x04, 0x04, 0x00, 0x04, 0x04, 0x00],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x06],
        ',' => [0x00, 0x00, 0x00, 0x00, 0x06, 0x06, 0x04],
        '[' => [0x0E, 0x08, 0x08, 0x08, 0x08, 0x08, 0x0E],
        ']' => [0x0E, 0x02, 0x02, 0x02, 0x02, 0x02, 0x0E],
        '(' => [0x02, 0x04, 0x08, 0x08, 0x08, 0x04, 0x02],
        ')' => [0x08, 0x04, 0x02, 0x02, 0x02, 0x04, 0x08],
        '#' => [0x0A, 0x0A, 0x1F, 0x0A, 0x1F, 0x0A, 0x0A],
        '@' => [0x0E, 0x11, 0x17, 0x15, 0x17, 0x10, 0x0E],
        '+' => [0x00, 0x04, 0x04, 0x1F, 0x04, 0x04, 0x00],
        '!' => [0x04, 0x04, 0x04, 0x04, 0x04, 0x00, 0x04],
        '?' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04],
        ' ' => [0x00; 7],
        '●' => [0x00, 0x04, 0x0E, 0x0E, 0x0E, 0x04, 0x00],
        '…' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x15, 0x00],
        _ => [0x1F, 0x11, 0x01, 0x02, 0x04, 0x00, 0x04],
    }
}

fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity((width as usize * height as usize * 4) + height as usize);
    let stride = width as usize * 4;
    for row in 0..height as usize {
        raw.push(0);
        let start = row * stride;
        raw.extend_from_slice(&rgba[start..start + stride]);
    }

    let compressed = zlib_store(&raw);
    let mut png = Vec::new();
    png.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);
    write_chunk(&mut png, b"IHDR", &ihdr(width, height));
    write_chunk(&mut png, b"IDAT", &compressed);
    write_chunk(&mut png, b"IEND", &[]);
    png
}

fn ihdr(width: u32, height: u32) -> Vec<u8> {
    let mut data = Vec::with_capacity(13);
    data.extend_from_slice(&width.to_be_bytes());
    data.extend_from_slice(&height.to_be_bytes());
    data.push(8);
    data.push(6);
    data.push(0);
    data.push(0);
    data.push(0);
    data
}

fn write_chunk(png: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    png.extend_from_slice(&(data.len() as u32).to_be_bytes());
    png.extend_from_slice(chunk_type);
    png.extend_from_slice(data);

    let mut crc_data = Vec::with_capacity(4 + data.len());
    crc_data.extend_from_slice(chunk_type);
    crc_data.extend_from_slice(data);
    png.extend_from_slice(&crc32(&crc_data).to_be_bytes());
}

fn zlib_store(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let mut index = 0;
    while index < data.len() {
        let remaining = data.len() - index;
        let block_len = remaining.min(65_535);
        let final_block = index + block_len >= data.len();
        out.push(if final_block { 0x01 } else { 0x00 });
        out.extend_from_slice(&(block_len as u16).to_le_bytes());
        out.extend_from_slice((!(block_len as u16)).to_le_bytes().as_slice());
        out.extend_from_slice(&data[index..index + block_len]);
        index += block_len;
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65_521;
    let mut a = 1_u32;
    let mut b = 0_u32;
    for byte in data {
        a = (a + *byte as u32) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for byte in data {
        crc ^= *byte as u32;
        for _ in 0..8 {
            if crc & 1 == 1 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_document;

    fn theme() -> Theme {
        Theme {
            background: Color::Rgb(8, 15, 24),
            surface: Color::Rgb(15, 25, 38),
            surface_alt: Color::Rgb(24, 39, 58),
            border: Color::Rgb(41, 65, 91),
            accent: Color::Rgb(67, 201, 176),
            sky: Color::Rgb(94, 191, 255),
            warn: Color::Rgb(248, 189, 94),
            danger: Color::Rgb(244, 114, 93),
            tag: Color::Rgb(67, 201, 176),
            metadata: Color::Rgb(248, 189, 94),
            id: Color::Rgb(94, 191, 255),
            query: Color::Rgb(158, 206, 255),
            attention: Color::Rgb(120, 224, 205),
            relation: Color::Rgb(67, 201, 176),
            count: Color::Rgb(94, 191, 255),
            selection: Color::Rgb(24, 39, 58),
            selection_text: Color::Rgb(233, 241, 248),
            text: Color::Rgb(233, 241, 248),
            muted: Color::Rgb(129, 153, 178),
        }
    }

    fn assert_no_bubble_overlaps(scene: &Scene) {
        for (index, upper) in scene.bubbles.iter().enumerate() {
            for lower in scene.bubbles.iter().skip(index + 1) {
                let overlap_x = upper.x < lower.x + lower.width as i32
                    && lower.x < upper.x + upper.width as i32;
                let overlap_y = upper.y <= bubble_bottom(lower) && lower.y <= bubble_bottom(upper);
                assert!(
                    !(overlap_x && overlap_y),
                    "bubbles overlap: {:?} at ({}, {}) {}x{} and {:?} at ({}, {}) {}x{}",
                    upper.path,
                    upper.x,
                    upper.y,
                    upper.width,
                    upper.height,
                    lower.path,
                    lower.x,
                    lower.y,
                    lower.width,
                    lower.height
                );
            }
        }
    }

    #[test]
    fn focus_style_is_distinct_from_context_bubbles() {
        let palette = theme();
        let focus = palette.bubble_style(BubbleKind::Focus, false);
        let ancestor = palette.bubble_style(BubbleKind::Ancestor, false);
        let descendant = palette.bubble_style(BubbleKind::Descendant, false);
        let peer = palette.bubble_style(BubbleKind::Peer, false);

        assert_eq!(focus.border, palette.attention);
        assert_eq!(focus.fill, palette.selection);
        assert_ne!(focus.border, ancestor.border);
        assert_ne!(focus.border, descendant.border);
        assert_ne!(focus.border, peer.border);
        assert_ne!(ancestor.border, palette.warn);
        assert_ne!(descendant.border, palette.sky);
        assert_ne!(peer.border, palette.accent);
    }

    #[test]
    fn focus_bubble_does_not_use_corner_selection_marker() {
        let scene = Scene {
            bubbles: vec![Bubble {
                path: vec![0],
                x: 1,
                y: 1,
                width: 18,
                height: 4,
                lines: vec!["Current".to_string()],
                kind: BubbleKind::Focus,
                matched: false,
                child_count: 0,
                hidden_children: 0,
            }],
            connectors: Vec::new(),
            min_x: 1,
            min_y: 1,
            max_x: 18,
            max_y: 4,
            focus_center: (9, 2),
        };
        let camera = Camera {
            origin_x: 0,
            origin_y: 0,
            width: 24,
            height: 8,
            zoom_percent: 100,
        };
        let surface = render_surface(&scene, camera, theme(), Some(&[0]), None);

        assert!(
            surface.cells.iter().all(|cell| cell.symbol != '◆'),
            "current focus should be indicated by the whole bubble, not a subtle corner marker"
        );
    }

    #[test]
    fn boundary_cue_tints_only_the_requested_focus_edge() {
        let palette = theme();
        let scene = Scene {
            bubbles: vec![Bubble {
                path: vec![0],
                x: 1,
                y: 1,
                width: 18,
                height: 4,
                lines: vec!["Current".to_string()],
                kind: BubbleKind::Focus,
                matched: false,
                child_count: 0,
                hidden_children: 0,
            }],
            connectors: Vec::new(),
            min_x: 1,
            min_y: 1,
            max_x: 18,
            max_y: 4,
            focus_center: (9, 2),
        };
        let camera = Camera {
            origin_x: 0,
            origin_y: 0,
            width: 24,
            height: 8,
            zoom_percent: 100,
        };

        let top_surface = render_surface(
            &scene,
            camera,
            palette,
            Some(&[0]),
            Some((&[0], BoundaryEdge::Top)),
        );
        let bottom_surface = render_surface(
            &scene,
            camera,
            palette,
            Some(&[0]),
            Some((&[0], BoundaryEdge::Bottom)),
        );

        assert_eq!(top_surface.cells[24 + 2].fg, palette.warn);
        assert_ne!(top_surface.cells[4_usize * 24 + 2].fg, palette.warn);
        assert_eq!(bottom_surface.cells[4_usize * 24 + 2].fg, palette.warn);
        assert_ne!(bottom_surface.cells[24 + 2].fg, palette.warn);
    }

    #[test]
    fn connector_elbows_stay_near_the_parent_anchor() {
        assert_eq!(connector_elbow_x(10, 40), 16);
        assert_eq!(connector_elbow_x(40, 10), 34);
        assert_eq!(connector_elbow_x(10, 13), 13);
    }

    #[test]
    fn spatial_tree_connector_lanes_vary_by_parent_sibling() {
        let first_parent_lane = tree_connector_elbow_x(10, 40, &[0, 0]);
        let second_parent_lane = tree_connector_elbow_x(10, 40, &[0, 1]);

        assert_ne!(
            first_parent_lane, second_parent_lane,
            "expanded sibling branches should not share one vertical connector lane"
        );
        assert!(first_parent_lane > 10 && first_parent_lane < 40);
        assert!(second_parent_lane > 10 && second_parent_lane < 40);
    }

    #[test]
    fn tree_connectors_use_one_bracket_per_visible_child_group() {
        let bubbles = vec![
            Bubble {
                path: vec![0],
                x: 0,
                y: 10,
                width: 10,
                height: 3,
                lines: vec!["Parent".to_string()],
                kind: BubbleKind::Context,
                matched: false,
                child_count: 0,
                hidden_children: 0,
            },
            Bubble {
                path: vec![0, 0],
                x: 30,
                y: 0,
                width: 10,
                height: 3,
                lines: vec!["One".to_string()],
                kind: BubbleKind::Context,
                matched: false,
                child_count: 0,
                hidden_children: 0,
            },
            Bubble {
                path: vec![0, 1],
                x: 30,
                y: 10,
                width: 10,
                height: 3,
                lines: vec!["Two".to_string()],
                kind: BubbleKind::Context,
                matched: false,
                child_count: 0,
                hidden_children: 0,
            },
            Bubble {
                path: vec![0, 2],
                x: 30,
                y: 20,
                width: 10,
                height: 3,
                lines: vec!["Three".to_string()],
                kind: BubbleKind::Context,
                matched: false,
                child_count: 0,
                hidden_children: 0,
            },
        ];

        let connectors = route_tree_connectors(&bubbles);
        let vertical_spines = connectors
            .iter()
            .filter(|connector| {
                connector.from.0 == connector.to.0 && connector.from.1 != connector.to.1
            })
            .count();
        let horizontal_stubs = connectors
            .iter()
            .filter(|connector| {
                connector.from.1 == connector.to.1 && connector.from.0 != connector.to.0
            })
            .count();

        assert_eq!(
            vertical_spines, 1,
            "a visible sibling group should share one measured connector spine"
        );
        assert_eq!(
            horizontal_stubs, 4,
            "the bracket should contain one parent stub plus one stub per child"
        );
    }

    #[test]
    fn connector_end_caps_show_direction_instead_of_dots() {
        assert_eq!(line_symbol(EAST), '╶');
        assert_eq!(line_symbol(WEST), '╴');
        assert_eq!(line_symbol(NORTH), '╵');
        assert_eq!(line_symbol(SOUTH), '╷');
    }

    #[test]
    fn spatial_collision_pass_resolves_cross_column_bubble_overlap() {
        let mut bubbles = vec![
            Bubble {
                path: vec![0],
                x: 0,
                y: 0,
                width: 30,
                height: 4,
                lines: vec!["Parent".to_string()],
                kind: BubbleKind::Context,
                matched: false,
                child_count: 0,
                hidden_children: 0,
            },
            Bubble {
                path: vec![0, 0],
                x: 25,
                y: 1,
                width: 18,
                height: 4,
                lines: vec!["Child".to_string()],
                kind: BubbleKind::Descendant,
                matched: false,
                child_count: 0,
                hidden_children: 0,
            },
        ];

        assert_ne!(spatial_column(&bubbles[0]), spatial_column(&bubbles[1]));
        normalize_spatial_collisions(&mut bubbles, &[0]);

        assert!(
            bubbles[1].y > bubble_bottom(&bubbles[0]),
            "rectangle-level normalization should catch overlaps across neighboring columns"
        );
    }

    #[test]
    fn spatial_branch_collision_shifts_whole_visible_sibling_subtrees() {
        let mut bubbles = vec![
            Bubble {
                path: vec![0],
                x: 0,
                y: 10,
                width: 12,
                height: 3,
                lines: vec!["Parent".to_string()],
                kind: BubbleKind::Context,
                matched: false,
                child_count: 0,
                hidden_children: 0,
            },
            Bubble {
                path: vec![0, 0],
                x: 30,
                y: 0,
                width: 12,
                height: 3,
                lines: vec!["Earlier".to_string()],
                kind: BubbleKind::Context,
                matched: false,
                child_count: 0,
                hidden_children: 0,
            },
            Bubble {
                path: vec![0, 0, 0],
                x: 60,
                y: 28,
                width: 12,
                height: 3,
                lines: vec!["Earlier child".to_string()],
                kind: BubbleKind::Context,
                matched: false,
                child_count: 0,
                hidden_children: 0,
            },
            Bubble {
                path: vec![0, 1],
                x: 30,
                y: 12,
                width: 12,
                height: 3,
                lines: vec!["Opened".to_string()],
                kind: BubbleKind::Focus,
                matched: false,
                child_count: 0,
                hidden_children: 0,
            },
            Bubble {
                path: vec![0, 1, 0],
                x: 60,
                y: 30,
                width: 12,
                height: 3,
                lines: vec!["Opened child".to_string()],
                kind: BubbleKind::Descendant,
                matched: false,
                child_count: 0,
                hidden_children: 0,
            },
        ];
        let opened_y = bubbles[3].y;
        let opened_child_y = bubbles[4].y;

        normalize_spatial_branch_overlaps(&mut bubbles);

        let earlier_bounds = subtree_branch_bounds(&bubbles, &[0, 0]);
        let opened_bounds = subtree_branch_bounds(&bubbles, &[0, 1]);

        assert!(
            opened_bounds.min_y > earlier_bounds.max_y,
            "expanded sibling subtrees should not occupy the same visual band"
        );
        assert_eq!(
            bubbles[3].y - opened_y,
            bubbles[4].y - opened_child_y,
            "the opened sibling and its children should move together as one branch"
        );
    }

    #[test]
    fn spatial_branch_collision_reserves_previous_expanded_sibling_branch() {
        let mut bubbles = vec![
            Bubble {
                path: vec![0],
                x: 0,
                y: 10,
                width: 12,
                height: 3,
                lines: vec!["Parent".to_string()],
                kind: BubbleKind::Context,
                matched: false,
                child_count: 0,
                hidden_children: 0,
            },
            Bubble {
                path: vec![0, 0],
                x: 30,
                y: 0,
                width: 12,
                height: 3,
                lines: vec!["Nora".to_string()],
                kind: BubbleKind::Context,
                matched: false,
                child_count: 0,
                hidden_children: 0,
            },
            Bubble {
                path: vec![0, 0, 0],
                x: 60,
                y: 40,
                width: 12,
                height: 3,
                lines: vec!["Nora child".to_string()],
                kind: BubbleKind::Context,
                matched: false,
                child_count: 0,
                hidden_children: 0,
            },
            Bubble {
                path: vec![0, 1],
                x: 30,
                y: 8,
                width: 12,
                height: 3,
                lines: vec!["Ivo".to_string()],
                kind: BubbleKind::Focus,
                matched: false,
                child_count: 0,
                hidden_children: 0,
            },
        ];
        let ivo_y = bubbles[3].y;

        normalize_spatial_branch_overlaps(&mut bubbles);

        assert!(
            bubbles[3].y > ivo_y,
            "a previous sibling's visible branch should reserve space before the next sibling card"
        );
        assert!(
            bubbles[3].y > bubble_bottom(&bubbles[2]),
            "the next sibling should move below the previous sibling's visible descendants"
        );
    }

    #[test]
    fn scene_respects_collapsed_branches() {
        let document = parse_document(
            "- Product [id:product]\n  - Direction [id:product/direction]\n    - Vision\n  - Tasks [id:product/tasks]\n    - Ship\n",
        )
        .document;
        let expanded = HashSet::from([vec![0]]);
        let scene = Scene::build(&document, &[0], &expanded, None, None);

        assert!(scene.bubbles.iter().any(|bubble| bubble.path == vec![0, 0]));
        let direction = scene
            .bubbles
            .iter()
            .find(|bubble| bubble.path == vec![0, 0])
            .expect("direction bubble should exist");
        assert_eq!(direction.child_count, 1);
        assert_eq!(direction.hidden_children, 1);
        assert!(direction.lines.iter().all(|line| !line.contains("folded")));
    }

    #[test]
    fn collapsed_bubbles_render_a_right_edge_marker() {
        let scene = Scene {
            bubbles: vec![Bubble {
                path: vec![0],
                x: 1,
                y: 1,
                width: 18,
                height: 4,
                lines: vec!["Collapsed".to_string()],
                kind: BubbleKind::Collapsed,
                matched: false,
                child_count: 3,
                hidden_children: 3,
            }],
            connectors: Vec::new(),
            min_x: 1,
            min_y: 1,
            max_x: 18,
            max_y: 4,
            focus_center: (9, 2),
        };
        let camera = Camera {
            origin_x: 0,
            origin_y: 0,
            width: 24,
            height: 8,
            zoom_percent: 100,
        };
        let surface = render_surface(&scene, camera, theme(), None, None);

        assert_eq!(surface.cells[2_usize * 24 + 18].symbol, '›');
    }

    #[test]
    fn expanded_bubbles_render_a_right_edge_marker() {
        let scene = Scene {
            bubbles: vec![Bubble {
                path: vec![0],
                x: 1,
                y: 1,
                width: 18,
                height: 4,
                lines: vec!["Expanded".to_string()],
                kind: BubbleKind::Context,
                matched: false,
                child_count: 3,
                hidden_children: 0,
            }],
            connectors: Vec::new(),
            min_x: 1,
            min_y: 1,
            max_x: 18,
            max_y: 4,
            focus_center: (9, 2),
        };
        let camera = Camera {
            origin_x: 0,
            origin_y: 0,
            width: 24,
            height: 8,
            zoom_percent: 100,
        };
        let surface = render_surface(&scene, camera, theme(), None, None);

        assert_eq!(surface.cells[2_usize * 24 + 18].symbol, '•');
    }

    #[test]
    fn scene_keeps_focus_center() {
        let document = parse_document(
            "- Product [id:product]\n  - Direction [id:product/direction]\n    - Vision\n",
        )
        .document;
        let expanded = HashSet::from([vec![0], vec![0, 0]]);
        let scene = Scene::build(&document, &[0, 0], &expanded, None, None);
        assert!(scene.focus_center.0 > 0);
        assert!(scene.focus_center.1 > 0);
    }

    #[test]
    fn spatial_scene_centers_focus_and_clusters_context() {
        let document = parse_document(
            "- Product [id:product]\n  - Direction [id:product/direction]\n    - Vision\n  - Tasks [id:product/tasks]\n    - Ship\n",
        )
        .document;
        let expanded = HashSet::from([vec![0], vec![0, 0], vec![0, 1]]);
        let scene = Scene::build_spatial(&document, &[0, 0], &expanded, None, None);

        let focus = scene
            .bubbles
            .iter()
            .find(|bubble| bubble.path == vec![0, 0])
            .expect("focus bubble should exist");
        let ancestor = scene
            .bubbles
            .iter()
            .find(|bubble| bubble.path == vec![0])
            .expect("ancestor bubble should exist");
        let child = scene
            .bubbles
            .iter()
            .find(|bubble| bubble.path == vec![0, 0, 0])
            .expect("child bubble should exist");
        let peer = scene
            .bubbles
            .iter()
            .find(|bubble| bubble.path == vec![0, 1])
            .expect("peer bubble should exist");

        assert!(
            ancestor.x < focus.x,
            "ancestor should sit left of the focused bubble"
        );
        assert!(
            child.x > focus.x,
            "descendant should fan to the right of the focused bubble"
        );
        assert_eq!(
            spatial_column(peer),
            spatial_column(focus),
            "peer should stay in the focused sibling column instead of moving to a new role column"
        );
        assert!(
            scene.focus_center.0.abs() <= focus.width as i32,
            "spatial focus should stay near the scene center"
        );
    }

    #[test]
    fn spatial_scene_preserves_sibling_order_when_focus_moves() {
        let document = parse_document(
            "- Moonwake [id:moonwake]\n  - Start Here [id:moonwake/start]\n  - Pitch [id:moonwake/pitch]\n  - World [id:moonwake/world]\n",
        )
        .document;
        let expanded = HashSet::from([vec![0]]);
        let scene = Scene::build_spatial(&document, &[0, 1], &expanded, None, None);

        let start = scene
            .bubbles
            .iter()
            .find(|bubble| bubble.path == vec![0, 0])
            .expect("start sibling should exist");
        let pitch = scene
            .bubbles
            .iter()
            .find(|bubble| bubble.path == vec![0, 1])
            .expect("focused sibling should exist");
        let world = scene
            .bubbles
            .iter()
            .find(|bubble| bubble.path == vec![0, 2])
            .expect("world sibling should exist");

        assert!(
            start.y < pitch.y && pitch.y < world.y,
            "spatial canvas should keep siblings in document order as focus moves"
        );
        assert_eq!(pitch.kind, BubbleKind::Focus);
        assert_eq!(
            spatial_column(start),
            spatial_column(pitch),
            "focused sibling should be highlighted in-place, not promoted to another column"
        );
        assert_eq!(
            spatial_column(pitch),
            spatial_column(world),
            "siblings should stay in the same ordered column"
        );
    }

    #[test]
    fn spatial_scene_keeps_sibling_positions_stable_when_focus_changes() {
        let document = parse_document(
            "- Moonwake [id:moonwake]\n  - Start Here [id:moonwake/start]\n  - Pitch [id:moonwake/pitch]\n  - World [id:moonwake/world]\n",
        )
        .document;
        let expanded = HashSet::from([vec![0]]);
        let pitch_scene = Scene::build_spatial(&document, &[0, 1], &expanded, None, None);
        let world_scene = Scene::build_spatial(&document, &[0, 2], &expanded, None, None);

        for path in [vec![0, 0], vec![0, 1], vec![0, 2]] {
            let pitch_focus_bubble = pitch_scene
                .bubbles
                .iter()
                .find(|bubble| bubble.path == path)
                .expect("bubble should exist in pitch-focused scene");
            let world_focus_bubble = world_scene
                .bubbles
                .iter()
                .find(|bubble| bubble.path == path)
                .expect("bubble should exist in world-focused scene");
            assert_eq!(
                pitch_focus_bubble.y, world_focus_bubble.y,
                "sibling position should stay fixed while focus changes"
            );
        }
    }

    #[test]
    fn spatial_scene_has_no_bubble_overlaps_after_arrow_navigation_layout() {
        let document = parse_document(include_str!("../examples/game-world-moonwake.md")).document;
        let expanded = HashSet::from([
            vec![0],
            vec![0, 1],
            vec![0, 2],
            vec![0, 3],
            vec![0, 4],
            vec![0, 5],
            vec![0, 6],
            vec![0, 7],
        ]);
        let scene = Scene::build_spatial(&document, &[0, 2], &expanded, None, None);

        assert_no_bubble_overlaps(&scene);
    }

    #[test]
    fn spatial_scene_reserves_expanded_character_branch_before_quest_sibling() {
        let document = parse_document(include_str!("../examples/game-world-moonwake.md")).document;
        let expanded = HashSet::from([vec![0], vec![0, 2], vec![0, 4], vec![0, 5]]);
        let scene = Scene::build_spatial(&document, &[0, 4], &expanded, None, None);

        assert_no_bubble_overlaps(&scene);

        let character_branch = subtree_branch_bounds(&scene.bubbles, &[0, 4]);
        let quest = scene
            .bubbles
            .iter()
            .find(|bubble| bubble.path == vec![0, 5])
            .expect("quest architecture sibling should be visible");

        assert!(
            quest.y > character_branch.max_y + BUBBLE_COLLISION_GAP,
            "Quest Architecture should sit below the expanded Characters branch instead of threading through its children"
        );
    }

    #[test]
    fn wrapped_title_continuation_lines_stay_bold() {
        let title_lines = vec![
            "mirror reeds ring when".to_string(),
            "the tide changes".to_string(),
            "#region".to_string(),
        ];
        let id_lines = vec![
            "Glass Marsh".to_string(),
            "moonwake/world/glass".to_string(),
        ];

        assert!(is_title_line(&title_lines, 0));
        assert!(is_title_line(&title_lines, 1));
        assert!(!is_title_line(&title_lines, 2));
        assert!(!is_title_line(&id_lines, 1));
    }

    #[test]
    fn spatial_framed_camera_keeps_focus_neighborhood_visible_when_it_fits() {
        let document = parse_document(
            "- Root [id:root]\n  - Parent [id:root/parent]\n    - Active [id:root/parent/active]\n    - Sibling [id:root/parent/sibling]\n",
        )
        .document;
        let expanded = HashSet::from([vec![0], vec![0, 0]]);
        let scene = Scene::build_spatial(&document, &[0, 0, 0], &expanded, None, None);

        let camera = scene.framed_focus_camera_with_zoom(160, 50, 0, 0, 100);
        let min_x = world_to_screen(scene.min_x, camera.origin_x, camera.zoom_percent);
        let min_y = world_to_screen(scene.min_y, camera.origin_y, camera.zoom_percent);
        let max_x = world_to_screen(scene.max_x, camera.origin_x, camera.zoom_percent);
        let max_y = world_to_screen(scene.max_y, camera.origin_y, camera.zoom_percent);

        assert!(
            min_x >= 0 && max_x < camera.width as i32,
            "fitted focus neighborhood should not clip horizontally"
        );
        assert!(
            min_y >= 0 && max_y < camera.height as i32,
            "fitted focus neighborhood should not clip vertically"
        );
    }

    #[test]
    fn spatial_framed_camera_centers_focus_when_neighborhood_does_not_fit() {
        let document = parse_document(
            "- Root [id:root]\n  - Parent [id:root/parent]\n    - Active [id:root/parent/active]\n    - Sibling [id:root/parent/sibling]\n",
        )
        .document;
        let expanded = HashSet::from([vec![0], vec![0, 0]]);
        let scene = Scene::build_spatial(&document, &[0, 0, 0], &expanded, None, None);

        let camera = scene.framed_focus_camera_with_zoom(30, 8, 0, 0, 100);
        let focus_x = world_to_screen(scene.focus_center.0, camera.origin_x, camera.zoom_percent);
        let focus_y = world_to_screen(scene.focus_center.1, camera.origin_y, camera.zoom_percent);

        assert!(
            (focus_x - 15).abs() <= 1,
            "oversized focus neighborhood should center the active focus horizontally"
        );
        assert!(
            (focus_y - 4).abs() <= 1,
            "oversized focus neighborhood should center the active focus vertically"
        );
    }

    #[test]
    fn renderer_clips_connector_to_offscreen_child() {
        let scene = Scene {
            bubbles: Vec::new(),
            connectors: vec![Connector::tree_segment((8, 4), (8, 40))],
            min_x: 0,
            min_y: 0,
            max_x: 8,
            max_y: 40,
            focus_center: (8, 4),
        };
        let camera = Camera {
            origin_x: 0,
            origin_y: 0,
            width: 20,
            height: 10,
            zoom_percent: 100,
        };
        let surface = render_surface(&scene, camera, theme(), None, None);

        assert!(
            surface.cells.iter().any(|cell| cell.symbol != ' '),
            "outgoing connector should remain visible when its child is just off-screen"
        );
    }

    #[test]
    fn renderer_clips_connector_from_offscreen_parent_to_visible_child() {
        let scene = Scene {
            bubbles: Vec::new(),
            connectors: vec![Connector::tree_segment((8, -20), (8, 4))],
            min_x: 0,
            min_y: -20,
            max_x: 8,
            max_y: 4,
            focus_center: (8, 4),
        };
        let camera = Camera {
            origin_x: 0,
            origin_y: 0,
            width: 20,
            height: 10,
            zoom_percent: 100,
        };
        let surface = render_surface(&scene, camera, theme(), None, None);

        assert!(
            surface.cells.iter().any(|cell| cell.symbol != ' '),
            "incoming connector should remain visible when its parent is just off-screen"
        );
    }

    #[test]
    fn scene_builds_relation_edges_for_visible_cross_links() {
        let document = parse_document(
            "- Product [id:product]\n  - MVP [id:product/mvp] [[prompts/library]]\n  - Prompt Library [id:prompts/library]\n",
        )
        .document;
        let expanded = HashSet::from([vec![0]]);
        let scene = Scene::build(&document, &[0, 0], &expanded, None, None);

        assert!(
            scene
                .connectors
                .iter()
                .any(|connector| connector.kind == ConnectorKind::Relation)
        );
    }

    #[test]
    fn scene_omits_relation_edges_when_target_is_not_visible() {
        let document = parse_document(
            "- Product [id:product]\n  - MVP [id:product/mvp] [[prompts/library]]\n  - Prompt Library [id:prompts/library]\n",
        )
        .document;
        let expanded = HashSet::new();
        let visible_paths = HashSet::from([vec![0], vec![0, 0]]);
        let scene = Scene::build(&document, &[0, 0], &expanded, None, Some(&visible_paths));

        assert!(
            scene
                .connectors
                .iter()
                .all(|connector| connector.kind != ConnectorKind::Relation)
        );
    }

    #[test]
    fn png_export_writes_a_png_signature() {
        let document = parse_document("- Product [id:product]\n").document;
        let expanded = HashSet::new();
        let scene = Scene::build(&document, &[0], &expanded, None, None);
        let camera = scene.camera(40, 18, 0, 0);
        let path = std::env::temp_dir().join("mdmind-mindmap-test.png");
        export_png(&scene, camera, theme(), &path).expect("png export should succeed");
        let bytes = fs::read(&path).expect("png should be readable");
        assert_eq!(&bytes[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        fs::remove_file(path).ok();
    }
}
