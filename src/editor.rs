use crate::app::AppError;
use crate::model::{Diagnostic, Document, Node, has_errors};
use crate::parser::{parse_document, parse_node_fragment};
use crate::serializer::serialize_document;
use crate::session::SessionState;
use crate::validate::validate_document;

#[derive(Debug, Clone)]
pub struct Editor {
    document: Document,
    focus_path: Vec<usize>,
    dirty: bool,
}

#[derive(Debug, Clone)]
pub struct EditorState {
    pub document: Document,
    pub focus_path: Vec<usize>,
    pub dirty: bool,
}

impl Editor {
    pub fn new(document: Document, focus_path: Vec<usize>) -> Self {
        Self {
            document,
            focus_path,
            dirty: false,
        }
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn focus_path(&self) -> &[usize] {
        &self.focus_path
    }

    pub fn set_focus_path(&mut self, path: Vec<usize>) -> Result<(), AppError> {
        if path.is_empty() && self.document.nodes.is_empty() {
            self.focus_path = path;
            return Ok(());
        }

        if get_node(&self.document.nodes, &path).is_none() {
            return Err(AppError::new("The requested focus path does not exist."));
        }

        self.focus_path = path;
        Ok(())
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn session_state(&self) -> SessionState {
        SessionState {
            focus_path: self.focus_path.clone(),
            focus_id: self.current().and_then(|node| node.id.clone()),
        }
    }

    pub fn current(&self) -> Option<&Node> {
        get_node(&self.document.nodes, &self.focus_path)
    }

    pub fn breadcrumb(&self) -> Vec<String> {
        let mut breadcrumb = Vec::new();
        let mut nodes = &self.document.nodes;
        for index in &self.focus_path {
            let Some(node) = nodes.get(*index) else {
                break;
            };

            breadcrumb.push(if node.text.is_empty() {
                "(empty)".to_string()
            } else {
                node.text.clone()
            });
            nodes = &node.children;
        }
        breadcrumb
    }

    pub fn move_root(&mut self) -> Result<(), AppError> {
        if self.document.nodes.is_empty() {
            return Err(AppError::new("The document has no nodes yet."));
        }
        self.focus_path = vec![0];
        Ok(())
    }

    pub fn move_parent(&mut self) -> Result<(), AppError> {
        if self.focus_path.len() <= 1 {
            return Err(AppError::new("You are already at the top node."));
        }
        self.focus_path.pop();
        Ok(())
    }

    pub fn move_next_sibling(&mut self) -> Result<(), AppError> {
        let siblings_len = self.current_siblings().len();
        let Some(current) = self.focus_path.last_mut() else {
            return Err(AppError::new("The document has no focused node."));
        };

        if *current + 1 >= siblings_len {
            return Err(AppError::new("There is no next sibling."));
        }

        *current += 1;
        Ok(())
    }

    pub fn move_previous_sibling(&mut self) -> Result<(), AppError> {
        let Some(current) = self.focus_path.last_mut() else {
            return Err(AppError::new("The document has no focused node."));
        };

        if *current == 0 {
            return Err(AppError::new("There is no previous sibling."));
        }

        *current -= 1;
        Ok(())
    }

    pub fn move_child(&mut self, child_index: usize) -> Result<(), AppError> {
        let current = self
            .current()
            .ok_or_else(|| AppError::new("The document has no focused node."))?;
        if child_index == 0 || child_index > current.children.len() {
            return Err(AppError::new(format!(
                "Child {child_index} does not exist."
            )));
        }

        self.focus_path.push(child_index - 1);
        Ok(())
    }

    pub fn open_id(&mut self, id: &str) -> Result<(), AppError> {
        let Some(path) = find_path_by_id(&self.document.nodes, id) else {
            return Err(AppError::new(format!("No node id matches '{id}'.")));
        };
        self.focus_path = path;
        Ok(())
    }

    pub fn add_root(&mut self, fragment: &str) -> Result<(), AppError> {
        let node = parse_fragment(fragment)?;
        let next_index = self.document.nodes.len();
        self.apply_change(move |document| {
            document.nodes.push(node);
            vec![next_index]
        })
    }

    pub fn add_child(&mut self, fragment: &str) -> Result<(), AppError> {
        if self.focus_path.is_empty() && self.document.nodes.is_empty() {
            return self.add_root(fragment);
        }

        let node = parse_fragment(fragment)?;
        let focus_path = self.focus_path.clone();
        self.apply_change(move |document| {
            let parent = get_node_mut(&mut document.nodes, &focus_path)
                .expect("focus path should be valid before mutation");
            parent.children.push(node);
            let mut next_focus = focus_path.clone();
            next_focus.push(parent.children.len() - 1);
            next_focus
        })
    }

    pub fn add_sibling(&mut self, fragment: &str) -> Result<(), AppError> {
        if self.focus_path.is_empty() {
            return self.add_root(fragment);
        }

        let node = parse_fragment(fragment)?;
        let focus_path = self.focus_path.clone();
        self.apply_change(move |document| {
            let insert_index = *focus_path.last().expect("checked");
            let parent_path = &focus_path[..focus_path.len() - 1];

            if parent_path.is_empty() {
                document.nodes.insert(insert_index + 1, node);
                vec![insert_index + 1]
            } else {
                let parent = get_node_mut(&mut document.nodes, parent_path)
                    .expect("parent path should be valid before mutation");
                parent.children.insert(insert_index + 1, node);
                let mut next_focus = parent_path.to_vec();
                next_focus.push(insert_index + 1);
                next_focus
            }
        })
    }

    pub fn edit_current(&mut self, fragment: &str) -> Result<(), AppError> {
        let Some(current) = self.current().cloned() else {
            return Err(AppError::new("The document has no focused node."));
        };

        let replacement = parse_fragment(fragment)?;
        let focus_path = self.focus_path.clone();
        self.apply_change(move |document| {
            let node = get_node_mut(&mut document.nodes, &focus_path)
                .expect("focus path should be valid before mutation");
            node.text = replacement.text;
            node.detail = current.detail;
            node.tags = replacement.tags;
            node.metadata = replacement.metadata;
            node.id = replacement.id;
            node.relations = replacement.relations;
            node.children = current.children;
            focus_path
        })
    }

    pub fn edit_current_detail(&mut self, detail: &str) -> Result<(), AppError> {
        if self.current().is_none() {
            return Err(AppError::new("The document has no focused node."));
        }

        let focus_path = self.focus_path.clone();
        let detail_lines = normalize_detail(detail);
        self.apply_change(move |document| {
            let node = get_node_mut(&mut document.nodes, &focus_path)
                .expect("focus path should be valid before mutation");
            node.detail = detail_lines;
            focus_path
        })
    }

    pub fn move_node_up(&mut self) -> Result<(), AppError> {
        if self.focus_path.is_empty() {
            return Err(AppError::new("The document has no focused node."));
        }

        let focus_path = self.focus_path.clone();
        let current_index = *focus_path.last().expect("checked");
        if current_index == 0 {
            return Err(AppError::new(
                "This node is already the first item at its level.",
            ));
        }

        self.apply_change(move |document| {
            let parent_path = &focus_path[..focus_path.len() - 1];
            let node = take_node_at(&mut document.nodes, &focus_path)
                .expect("focus path should be valid before mutation");
            insert_node_at(&mut document.nodes, parent_path, current_index - 1, node)
                .expect("target insertion path should be valid");
            let mut next_focus = parent_path.to_vec();
            next_focus.push(current_index - 1);
            next_focus
        })
    }

    pub fn move_node_down(&mut self) -> Result<(), AppError> {
        if self.focus_path.is_empty() {
            return Err(AppError::new("The document has no focused node."));
        }

        let focus_path = self.focus_path.clone();
        let current_index = *focus_path.last().expect("checked");
        let siblings_len = self.current_siblings().len();
        if current_index + 1 >= siblings_len {
            return Err(AppError::new(
                "This node is already the last item at its level.",
            ));
        }

        self.apply_change(move |document| {
            let parent_path = &focus_path[..focus_path.len() - 1];
            let node = take_node_at(&mut document.nodes, &focus_path)
                .expect("focus path should be valid before mutation");
            insert_node_at(&mut document.nodes, parent_path, current_index + 1, node)
                .expect("target insertion path should be valid");
            let mut next_focus = parent_path.to_vec();
            next_focus.push(current_index + 1);
            next_focus
        })
    }

    pub fn outdent_node(&mut self) -> Result<(), AppError> {
        if self.focus_path.len() <= 1 {
            return Err(AppError::new("Root-level nodes cannot move further left."));
        }

        let focus_path = self.focus_path.clone();
        let parent_path = focus_path[..focus_path.len() - 1].to_vec();
        let grandparent_path = parent_path[..parent_path.len() - 1].to_vec();
        let parent_index = *parent_path.last().expect("checked");

        self.apply_change(move |document| {
            let node = take_node_at(&mut document.nodes, &focus_path)
                .expect("focus path should be valid before mutation");
            insert_node_at(
                &mut document.nodes,
                &grandparent_path,
                parent_index + 1,
                node,
            )
            .expect("target insertion path should be valid");
            let mut next_focus = grandparent_path.clone();
            next_focus.push(parent_index + 1);
            next_focus
        })
    }

    pub fn indent_node(&mut self) -> Result<(), AppError> {
        if self.focus_path.is_empty() {
            return Err(AppError::new("The document has no focused node."));
        }

        let focus_path = self.focus_path.clone();
        let current_index = *focus_path.last().expect("checked");
        if current_index == 0 {
            return Err(AppError::new(
                "This node has no previous sibling to indent into.",
            ));
        }

        self.apply_change(move |document| {
            let parent_path = &focus_path[..focus_path.len() - 1];
            let previous_sibling_path = {
                let mut path = parent_path.to_vec();
                path.push(current_index - 1);
                path
            };

            let node = take_node_at(&mut document.nodes, &focus_path)
                .expect("focus path should be valid before mutation");
            let child_count = get_node_mut_vec(&mut document.nodes, &previous_sibling_path)
                .expect("previous sibling should exist")
                .children
                .len();
            insert_node_at(
                &mut document.nodes,
                &previous_sibling_path,
                child_count,
                node,
            )
            .expect("target insertion path should be valid");
            let mut next_focus = previous_sibling_path.clone();
            next_focus.push(child_count);
            next_focus
        })
    }

    pub fn delete_current(&mut self) -> Result<(), AppError> {
        if self.focus_path.is_empty() {
            return Err(AppError::new("The document has no focused node."));
        }

        let focus_path = self.focus_path.clone();
        self.apply_change(move |document| {
            let parent_path = &focus_path[..focus_path.len() - 1];
            let current_index = *focus_path.last().expect("checked");
            let sibling_count = sibling_count_at(&document.nodes, parent_path);

            let _removed = take_node_at(&mut document.nodes, &focus_path)
                .expect("focus path should be valid before mutation");

            if sibling_count > current_index + 1 {
                let mut next_focus = parent_path.to_vec();
                next_focus.push(current_index);
                next_focus
            } else if current_index > 0 {
                let mut next_focus = parent_path.to_vec();
                next_focus.push(current_index - 1);
                next_focus
            } else if !parent_path.is_empty() {
                parent_path.to_vec()
            } else {
                default_focus_path(document)
            }
        })
    }

    pub fn save(&mut self) -> Result<(), AppError> {
        let (document, _) = reparse_and_validate(self.document.clone(), self.focus_path.clone())?;
        self.document = document;
        Ok(())
    }

    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    pub fn state(&self) -> EditorState {
        EditorState {
            document: self.document.clone(),
            focus_path: self.focus_path.clone(),
            dirty: self.dirty,
        }
    }

    pub fn restore_state(&mut self, state: EditorState) -> Result<(), AppError> {
        let (document, focus_path) = reparse_and_validate(state.document, state.focus_path)?;
        self.document = document;
        self.focus_path = focus_path;
        self.dirty = state.dirty;
        Ok(())
    }

    fn apply_change<F>(&mut self, mutator: F) -> Result<(), AppError>
    where
        F: FnOnce(&mut Document) -> Vec<usize>,
    {
        let mut candidate = self.document.clone();
        let next_focus = mutator(&mut candidate);
        let (document, focus_path) = reparse_and_validate(candidate, next_focus)?;
        self.document = document;
        self.focus_path = focus_path;
        self.dirty = true;
        Ok(())
    }

    fn current_siblings(&self) -> &[Node] {
        if self.focus_path.is_empty() {
            return &self.document.nodes;
        }

        let parent_path = &self.focus_path[..self.focus_path.len() - 1];
        if parent_path.is_empty() {
            &self.document.nodes
        } else {
            &get_node(&self.document.nodes, parent_path)
                .expect("parent path should be valid")
                .children
        }
    }
}

pub fn default_focus_path(document: &Document) -> Vec<usize> {
    if document.nodes.is_empty() {
        Vec::new()
    } else {
        vec![0]
    }
}

pub fn find_path_by_id(nodes: &[Node], id: &str) -> Option<Vec<usize>> {
    find_path_by_id_with_prefix(nodes, id, Vec::new())
}

fn find_path_by_id_with_prefix(nodes: &[Node], id: &str, prefix: Vec<usize>) -> Option<Vec<usize>> {
    for (index, node) in nodes.iter().enumerate() {
        let mut path = prefix.clone();
        path.push(index);
        if node.id.as_deref() == Some(id) {
            return Some(path);
        }
        if let Some(found) = find_path_by_id_with_prefix(&node.children, id, path.clone()) {
            return Some(found);
        }
    }
    None
}

pub fn get_node<'a>(nodes: &'a [Node], path: &[usize]) -> Option<&'a Node> {
    let (first, rest) = path.split_first()?;
    let node = nodes.get(*first)?;
    if rest.is_empty() {
        Some(node)
    } else {
        get_node(&node.children, rest)
    }
}

fn get_node_mut<'a>(nodes: &'a mut [Node], path: &[usize]) -> Option<&'a mut Node> {
    let (first, rest) = path.split_first()?;
    let node = nodes.get_mut(*first)?;
    if rest.is_empty() {
        Some(node)
    } else {
        get_node_mut(&mut node.children, rest)
    }
}

fn get_node_mut_vec<'a>(nodes: &'a mut Vec<Node>, path: &[usize]) -> Option<&'a mut Node> {
    get_node_mut(nodes.as_mut_slice(), path)
}

fn take_node_at(nodes: &mut Vec<Node>, path: &[usize]) -> Option<Node> {
    let (first, rest) = path.split_first()?;
    if rest.is_empty() {
        if *first < nodes.len() {
            Some(nodes.remove(*first))
        } else {
            None
        }
    } else {
        let parent = nodes.get_mut(*first)?;
        take_node_at(&mut parent.children, rest)
    }
}

fn sibling_count_at(nodes: &[Node], parent_path: &[usize]) -> usize {
    if parent_path.is_empty() {
        nodes.len()
    } else {
        get_node(nodes, parent_path)
            .map(|node| node.children.len())
            .unwrap_or(0)
    }
}

fn insert_node_at(
    nodes: &mut Vec<Node>,
    parent_path: &[usize],
    index: usize,
    node: Node,
) -> Option<()> {
    if parent_path.is_empty() {
        if index <= nodes.len() {
            nodes.insert(index, node);
            Some(())
        } else {
            None
        }
    } else {
        let parent = get_node_mut_vec(nodes, parent_path)?;
        if index <= parent.children.len() {
            parent.children.insert(index, node);
            Some(())
        } else {
            None
        }
    }
}

fn parse_fragment(fragment: &str) -> Result<Node, AppError> {
    parse_node_fragment(fragment)
        .map_err(|diagnostics| AppError::new(join_diagnostics(&diagnostics)))
}

fn normalize_detail(detail: &str) -> Vec<String> {
    let normalized = detail.replace("\r\n", "\n");
    let trimmed = normalized.trim_matches('\n');
    if trimmed.trim().is_empty() {
        Vec::new()
    } else {
        trimmed.split('\n').map(str::to_string).collect()
    }
}

fn reparse_and_validate(
    candidate: Document,
    desired_focus_path: Vec<usize>,
) -> Result<(Document, Vec<usize>), AppError> {
    let source = serialize_document(&candidate);
    let parsed = parse_document(&source);
    if has_errors(&parsed.diagnostics) {
        return Err(AppError::new(join_diagnostics(&parsed.diagnostics)));
    }

    let validation = validate_document(&parsed.document);
    if validation
        .iter()
        .any(|diagnostic| diagnostic.severity == crate::model::Severity::Error)
    {
        return Err(AppError::new(join_diagnostics(&validation)));
    }

    let focus_path = if desired_focus_path.is_empty() && parsed.document.nodes.is_empty() {
        Vec::new()
    } else if get_node(&parsed.document.nodes, &desired_focus_path).is_some() {
        desired_focus_path
    } else {
        default_focus_path(&parsed.document)
    };

    Ok((parsed.document, focus_path))
}

fn join_diagnostics(diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| format!("line {}: {}", diagnostic.line, diagnostic.message))
        .collect::<Vec<_>>()
        .join("\n")
}
