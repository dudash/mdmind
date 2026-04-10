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
}

#[derive(Debug, Clone)]
pub struct Connector {
    pub from: (i32, i32),
    pub to: (i32, i32),
    pub kind: ConnectorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorKind {
    Tree,
    Relation,
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
        connectors.extend(build_relation_connectors(document, &bubbles));

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
        let viewport_width = viewport_width.max(1);
        let viewport_height = viewport_height.max(1);
        let fit_x = self.width() <= viewport_width as i32;
        let fit_y = self.height() <= viewport_height as i32;

        let origin_x = if fit_x {
            self.min_x - ((viewport_width as i32 - self.width()) / 2)
        } else {
            self.focus_center.0 - viewport_width as i32 / 2 + pan_x
        };
        let origin_y = if fit_y {
            self.min_y - ((viewport_height as i32 - self.height()) / 2)
        } else {
            self.focus_center.1 - viewport_height as i32 / 2 + pan_y
        };

        Camera {
            origin_x,
            origin_y,
            width: viewport_width,
            height: viewport_height,
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
}

impl Theme {
    fn bubble_style(self, kind: BubbleKind, matched: bool) -> BubbleStyle {
        match kind {
            BubbleKind::Focus => BubbleStyle {
                border: self.accent,
                fill: self.surface_alt,
                text: self.text,
            },
            BubbleKind::Ancestor => BubbleStyle {
                border: self.warn,
                fill: self.surface,
                text: self.text,
            },
            BubbleKind::Descendant => BubbleStyle {
                border: self.sky,
                fill: self.surface,
                text: self.text,
            },
            BubbleKind::Peer => BubbleStyle {
                border: self.accent,
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

impl<'a> MindmapWidget<'a> {
    pub fn new(scene: &'a Scene, camera: Camera, theme: Theme) -> Self {
        Self {
            scene,
            camera,
            theme,
        }
    }
}

impl Widget for MindmapWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let camera = Camera {
            width: area.width,
            height: area.height,
            ..self.camera
        };
        let surface = render_surface(self.scene, camera, self.theme);

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
    let surface = render_surface(scene, camera, theme);
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
        connectors.push(Connector {
            from: (x + node.width as i32, parent_mid),
            to: (child_x - 1, child_mid),
            kind: ConnectorKind::Tree,
        });
        place_node(child, child_x, cursor_y, bubbles, connectors);
        cursor_y += child.subtree_height + CHILD_GAP;
    }
}

fn build_relation_connectors(document: &Document, bubbles: &[Bubble]) -> Vec<Connector> {
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
                    connectors.push(Connector {
                        from,
                        to,
                        kind: ConnectorKind::Relation,
                    });
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

    if node.hidden_children > 0 {
        lines.push(format!(
            "folded {} child{}",
            node.hidden_children,
            if node.hidden_children == 1 { "" } else { "ren" }
        ));
    } else if let Some(id) = &node.id {
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

fn render_surface(scene: &Scene, camera: Camera, theme: Theme) -> CellSurface {
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
                cell.fg = theme.accent;
                cell.bg = theme.background;
                cell.bold = true;
            }
        }
    }

    for bubble in &scene.bubbles {
        draw_shadow(&mut surface, bubble, camera, theme);
    }
    for bubble in &scene.bubbles {
        draw_bubble(&mut surface, bubble, camera, theme);
    }

    surface
}

fn draw_shadow(surface: &mut CellSurface, bubble: &Bubble, camera: Camera, theme: Theme) {
    let shadow_x = bubble.x + 1 - camera.origin_x;
    let shadow_y = bubble.y + 1 - camera.origin_y;
    let shadow = theme.background;
    for y in 0..bubble.height as i32 {
        for x in 0..bubble.width as i32 {
            if let Some(cell) = surface.get_mut_checked(shadow_x + x, shadow_y + y) {
                cell.bg = shadow;
            }
        }
    }
}

fn draw_bubble(surface: &mut CellSurface, bubble: &Bubble, camera: Camera, theme: Theme) {
    let style = theme.bubble_style(bubble.kind, bubble.matched);
    let x0 = bubble.x - camera.origin_x;
    let y0 = bubble.y - camera.origin_y;
    let x1 = x0 + bubble.width as i32 - 1;
    let y1 = y0 + bubble.height as i32 - 1;

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
            cell.bold = bubble.kind == BubbleKind::Focus && border;
        }
    }

    for (index, line) in bubble.lines.iter().enumerate() {
        let y = y0 + 1 + index as i32;
        let available = bubble.width.saturating_sub(2);
        let text = truncate(line, available);
        for (offset, character) in text.chars().enumerate() {
            let Some(cell) = surface.get_mut_checked(x0 + 1 + offset as i32, y) else {
                continue;
            };
            cell.symbol = character;
            cell.fg = if bubble.kind == BubbleKind::Focus && index == 0 {
                theme.accent
            } else if index == 0 {
                style.text
            } else {
                theme.muted
            };
            cell.bg = style.fill;
            cell.bold = index == 0;
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
}

fn draw_connector(masks: &mut [u8], size: Size, camera: Camera, connector: &Connector) {
    let x1 = connector.from.0 - camera.origin_x;
    let y1 = connector.from.1 - camera.origin_y;
    let x2 = connector.to.0 - camera.origin_x;
    let y2 = connector.to.1 - camera.origin_y;
    let elbow_x = (x1 + x2) / 2;

    add_horizontal(masks, size, x1, elbow_x, y1);
    add_vertical(masks, size, elbow_x, y1, y2);
    add_horizontal(masks, size, elbow_x, x2, y2);
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
            text: Color::Rgb(233, 241, 248),
            muted: Color::Rgb(129, 153, 178),
        }
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
        assert!(direction.lines.iter().any(|line| line.contains("folded 1")));
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
