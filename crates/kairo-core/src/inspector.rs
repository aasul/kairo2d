//! Data sent between the runtime and inspector. It contains no evaluator, addresses, or Lua handles.
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::hash::{Hash, Hasher};
pub const MAX_INSPECTION_BYTES: usize = 24 * 1024;
pub const RUNTIME_PAGE_SIZE: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeNodeKey {
    pub graph: u64,
    pub id: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeNodeInfo {
    pub key: RuntimeNodeKey,
    pub parent_id: Option<u64>,
    pub depth: usize,
    pub name: String,
    pub kind: crate::scene_graph::NodeKind,
    pub enabled: bool,
    pub visible: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeNodeDetails {
    pub node: RuntimeNodeInfo,
    pub path: String,
    pub transform: crate::scene_graph::Transform2D,
    pub tags: Vec<String>,
    pub script: Option<String>,
    pub prefab: Option<String>,
    pub properties: serde_json::Map<String, Value>,
    pub properties_truncated: bool,
    #[serde(default)]
    pub exposed: Vec<InspectNode>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeTreePage {
    pub scene: String,
    #[serde(default)]
    pub source: Option<String>,
    pub graph: u64,
    pub offset: usize,
    pub total: usize,
    pub nodes: Vec<RuntimeNodeInfo>,
    pub selected: Option<RuntimeNodeDetails>,
    pub selected_missing: bool,
}

fn short_path(path: &str, limit: usize) -> String {
    if path.len() <= limit {
        return path.to_owned();
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    let mut end = limit - 17;
    while !path.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}~{:016x}", &path[..end], hasher.finish())
}

impl RuntimeTreePage {
    pub fn from_graph(
        graph: &crate::scene_graph::SceneGraph,
        offset: usize,
        selected: Option<RuntimeNodeKey>,
    ) -> Result<Self> {
        let total = graph.len();
        let offset = offset.min((total.saturating_sub(1) / RUNTIME_PAGE_SIZE) * RUNTIME_PAGE_SIZE);
        let mut nodes = Vec::new();
        for (id, depth) in graph.preorder_page(offset, RUNTIME_PAGE_SIZE)? {
            let node = graph.node(id)?;
            nodes.push(RuntimeNodeInfo {
                key: RuntimeNodeKey {
                    graph: graph.graph_id(),
                    id: node.data().id,
                },
                parent_id: node
                    .parent()
                    .map(|parent| graph.node(parent).map(|node| node.data().id))
                    .transpose()?,
                depth,
                name: node.data().name.clone(),
                kind: node.data().kind,
                enabled: node.data().enabled,
                visible: node.data().visible,
            });
        }
        let selected_id = selected
            .filter(|key| key.graph == graph.graph_id())
            .and_then(|key| graph.find_by_file_id(key.id));
        let selected_details = selected_id
            .map(|id| -> Result<RuntimeNodeDetails> {
                let node = graph.node(id)?;
                let data = node.data();
                let mut properties = serde_json::Map::new();
                let mut properties_truncated = data.properties.len() > 32;
                let mut bytes = 0;
                for (key, value) in data.properties.iter().take(32) {
                    let encoded = serde_json::to_vec(&(key, value))?;
                    if encoded.len() > 256 || bytes + encoded.len() > 2048 {
                        properties_truncated = true;
                        continue;
                    }
                    bytes += encoded.len();
                    properties.insert(key.clone(), value.clone());
                }
                let info = RuntimeNodeInfo {
                    key: RuntimeNodeKey {
                        graph: graph.graph_id(),
                        id: data.id,
                    },
                    parent_id: node
                        .parent()
                        .map(|parent| graph.node(parent).map(|node| node.data().id))
                        .transpose()?,
                    depth: 0,
                    name: data.name.clone(),
                    kind: data.kind,
                    enabled: data.enabled,
                    visible: data.visible,
                };
                Ok(RuntimeNodeDetails {
                    node: info,
                    path: short_path(&graph.path(id)?, 256),
                    transform: data.transform,
                    tags: data.tags.iter().take(16).cloned().collect(),
                    script: data.script.as_ref().map(|path| short_path(path, 256)),
                    prefab: data.prefab.as_ref().map(|path| short_path(path, 256)),
                    properties,
                    properties_truncated,
                    exposed: runtime_exposed(data)?,
                })
            })
            .transpose()?;
        let page = Self {
            scene: graph.name().to_owned(),
            source: None,
            graph: graph.graph_id(),
            offset,
            total,
            nodes,
            selected_missing: selected.is_some() && selected_details.is_none(),
            selected: selected_details,
        };
        page.validate()?;
        Ok(page)
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.graph > 0
                && self.scene.len() <= 128
                && self
                    .source
                    .as_ref()
                    .is_none_or(|path| path.len() <= 256 && path.ends_with(".scene"))
                && self.total <= 50_000
                && self.nodes.len() <= RUNTIME_PAGE_SIZE
                && self.offset < self.total,
            "invalid runtime scene page"
        );
        for node in &self.nodes {
            ensure!(
                node.key.graph == self.graph
                    && node.key.id > 0
                    && node.name.len() <= 128
                    && node.depth <= 256,
                "invalid runtime node"
            );
        }
        if let Some(selected) = &self.selected {
            ensure!(
                selected.node.key.graph == self.graph
                    && selected.path.len() <= 1024
                    && selected.tags.len() <= 16
                    && selected.properties.len() <= 32
                    && selected.exposed.len() <= 16,
                "invalid selected runtime node"
            );
            selected.transform.validate()?;
            for field in &selected.exposed {
                field.validate()?;
            }
        }
        Ok(())
    }
}
/// A fresh positive Lua-integer-compatible ID for each VM, including after process restart.
pub fn next_session() -> Result<u64> {
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes)
        .map_err(|error| anyhow::anyhow!("cannot generate inspector session ID: {error}"))?;
    Ok((u64::from_le_bytes(bytes) & i64::MAX as u64).max(1))
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectKind {
    Number,
    Boolean,
    String,
    Vector,
    Color,
    Table,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectMetadata {
    pub kind: InspectKind,
    #[serde(default)]
    pub writable: bool,
    #[serde(default)]
    pub persist: bool,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub choices: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectNode {
    pub path: String,
    pub value: Value,
    pub metadata: InspectMetadata,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneStatus {
    pub current: String,
    pub registered: Vec<String>,
    pub stack: Vec<String>,
    #[serde(default)]
    pub active_nodes: usize,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectSnapshot {
    pub session: u64,
    pub nodes: Vec<InspectNode>,
    pub scenes: SceneStatus,
    #[serde(default)]
    pub runtime: Option<RuntimeTreePage>,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectUpdate {
    pub session: u64,
    pub path: String,
    pub expected: Value,
    pub value: Value,
}

/// A small, explicit set of scene-node fields that scripts may opt into live editing.
pub fn runtime_field_value(data: &crate::scene_graph::NodeFile, path: &str) -> Option<Value> {
    if let Some(name) = path.strip_prefix("property.") {
        return data.properties.get(name).cloned();
    }
    Some(match path {
        "position" => serde_json::json!(data.transform.position),
        "rotation" => serde_json::json!(data.transform.rotation),
        "scale" => serde_json::json!(data.transform.scale),
        "pivot" => serde_json::json!(data.transform.pivot),
        "enabled" => Value::Bool(data.enabled),
        "visible" => Value::Bool(data.visible),
        _ => return None,
    })
}

fn runtime_exposed(data: &crate::scene_graph::NodeFile) -> Result<Vec<InspectNode>> {
    let Some(value) = data.metadata.get("runtime_inspector") else {
        return Ok(Vec::new());
    };
    let fields = value
        .as_object()
        .context("runtime inspector metadata must be an object")?;
    ensure!(fields.len() <= 16, "too many exposed runtime fields");
    fields
        .iter()
        .map(|(path, metadata)| {
            let field = InspectNode {
                path: path.clone(),
                value: runtime_field_value(data, path)
                    .with_context(|| format!("runtime field '{path}' is unavailable"))?,
                metadata: serde_json::from_value(metadata.clone())?,
            };
            field.validate()?;
            Ok(field)
        })
        .collect()
}

pub fn apply_runtime_edit(
    graph: &mut crate::scene_graph::SceneGraph,
    key: RuntimeNodeKey,
    update: &InspectUpdate,
    session: u64,
) -> Result<()> {
    ensure!(
        key.graph == graph.graph_id(),
        "runtime node belongs to another scene"
    );
    let id = graph
        .find_by_file_id(key.id)
        .context("runtime node is stale or destroyed")?;
    let current = runtime_exposed(graph.node(id)?.data())?
        .into_iter()
        .find(|field| field.path == update.path)
        .context("runtime field is not explicitly exposed")?;
    current.validate_update(update, session)?;
    write_runtime_field(graph, id, &update.path, &update.value)
}

/// Apply a persistent tuning value only after the editor has identified a
/// matching authored node and checked the exact on-disk scene value.
pub fn apply_scene_field(
    graph: &mut crate::scene_graph::SceneGraph,
    id: crate::scene_graph::NodeId,
    path: &str,
    expected: Value,
    value: Value,
    metadata: InspectMetadata,
) -> Result<()> {
    ensure!(
        metadata.writable && metadata.persist,
        "field is not marked persistable"
    );
    let current = InspectNode {
        path: path.to_owned(),
        value: runtime_field_value(graph.node(id)?.data(), path)
            .context("scene field does not exist")?,
        metadata,
    };
    ensure!(
        current.value == expected,
        "scene value changed; refresh before applying"
    );
    current.validate_update(
        &InspectUpdate {
            session: 1,
            path: path.to_owned(),
            expected,
            value: value.clone(),
        },
        1,
    )?;
    write_runtime_field(graph, id, path, &value)
}

fn write_runtime_field(
    graph: &mut crate::scene_graph::SceneGraph,
    id: crate::scene_graph::NodeId,
    path: &str,
    value: &Value,
) -> Result<()> {
    match path {
        "enabled" => graph.set_enabled(id, value.as_bool().context("expected boolean")?),
        "visible" => graph.set_visible(id, value.as_bool().context("expected boolean")?),
        "position" | "scale" | "pivot" => {
            let numbers = value.as_array().context("expected Vec2")?;
            ensure!(numbers.len() == 2, "expected Vec2");
            let pair = [
                numbers[0].as_f64().context("expected number")? as f32,
                numbers[1].as_f64().context("expected number")? as f32,
            ];
            let mut transform = graph.node(id)?.data().transform;
            match path {
                "position" => transform.position = pair,
                "scale" => transform.scale = pair,
                _ => transform.pivot = pair,
            }
            graph.set_transform(id, transform)
        }
        "rotation" => {
            let mut transform = graph.node(id)?.data().transform;
            transform.rotation = value.as_f64().context("expected number")? as f32;
            graph.set_transform(id, transform)
        }
        path => {
            let name = path
                .strip_prefix("property.")
                .context("unsupported runtime field")?;
            graph.set_property(id, name, value.clone())
        }
    }
}
fn bounded_value(value: &Value, depth: usize, count: &mut usize) -> Result<()> {
    *count += 1;
    ensure!(
        *count <= 64 && depth <= 4,
        "inspector value exceeds 64 nodes or four levels"
    );
    match value {
        Value::Number(n) => {
            ensure!(
                n.as_f64().is_some_and(f64::is_finite),
                "inspector number must be finite"
            );
        }
        Value::Bool(_) => {}
        Value::String(s) => ensure!(s.len() <= 512, "inspector string exceeds 512 bytes"),
        Value::Array(values) => {
            for item in values {
                bounded_value(item, depth + 1, count)?;
            }
        }
        Value::Object(values) => {
            for (key, item) in values {
                ensure!(
                    !key.is_empty() && key.len() <= 64,
                    "invalid inspector table key"
                );
                bounded_value(item, depth + 1, count)?;
            }
        }
        Value::Null => anyhow::bail!("nil/null cannot be inspected"),
    }
    Ok(())
}
fn same_shape(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(_), Value::Number(_))
        | (Value::Bool(_), Value::Bool(_))
        | (Value::String(_), Value::String(_)) => true,
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same_shape(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, v)| b.get(key).is_some_and(|b| same_shape(v, b)))
        }
        _ => false,
    }
}
impl InspectNode {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.path.is_empty()
                && self.path.len() <= 128
                && self.path.split('.').all(|p| !p.is_empty()
                    && p.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))),
            "invalid inspector path"
        );
        bounded_value(&self.value, 0, &mut 0)?;
        let m = &self.metadata;
        ensure!(
            m.min.is_none_or(f64::is_finite) && m.max.is_none_or(f64::is_finite),
            "inspector bounds must be finite"
        );
        ensure!(
            m.min.zip(m.max).is_none_or(|(min, max)| min <= max),
            "inspector minimum exceeds maximum"
        );
        ensure!(
            m.choices.len() <= 32 && m.choices.iter().all(|s| s.len() <= 128),
            "inspector choice limit exceeded"
        );
        let check_number = |value: &Value| -> bool {
            value
                .as_f64()
                .is_some_and(|v| m.min.is_none_or(|n| v >= n) && m.max.is_none_or(|n| v <= n))
        };
        let valid = match m.kind {
            InspectKind::Number => check_number(&self.value),
            InspectKind::Boolean => self.value.is_boolean(),
            InspectKind::String => self
                .value
                .as_str()
                .is_some_and(|v| m.choices.is_empty() || m.choices.iter().any(|s| s == v)),
            InspectKind::Vector => self
                .value
                .as_array()
                .is_some_and(|v| (2..=4).contains(&v.len()) && v.iter().all(check_number)),
            InspectKind::Color => self.value.as_array().is_some_and(|v| {
                v.len() == 4
                    && v.iter()
                        .all(|v| v.as_f64().is_some_and(|n| (0.0..=1.0).contains(&n)))
            }),
            InspectKind::Table => self.value.is_array() || self.value.is_object(),
        };
        ensure!(
            valid,
            "value for '{}' does not match its type, range, or choices",
            self.path
        );
        ensure!(
            serde_json::to_vec(self)?.len() <= 2048,
            "inspector field exceeds 2 KiB"
        );
        Ok(())
    }
    pub fn validate_update(&self, update: &InspectUpdate, session: u64) -> Result<()> {
        ensure!(
            update.session == session && self.path == update.path,
            "stale inspector session/path; refresh after reload"
        );
        ensure!(
            self.metadata.writable,
            "inspector field is read-only; expose it with writable=true"
        );
        ensure!(
            self.value == update.expected,
            "inspector value changed since reading; pause or refresh before editing"
        );
        ensure!(
            same_shape(&self.value, &update.value),
            "inspector edit cannot change the field's shape or type"
        );
        let mut candidate = self.clone();
        candidate.value = update.value.clone();
        candidate.validate()
    }
}
impl InspectSnapshot {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.nodes.len() <= 64
                && self.scenes.registered.len() <= 64
                && self.scenes.stack.len() <= 16
                && self.scenes.active_nodes <= 800_000,
            "inspection limit exceeded"
        );
        for node in &self.nodes {
            node.validate()?;
        }
        if let Some(runtime) = &self.runtime {
            runtime.validate()?;
        }
        for name in self
            .scenes
            .registered
            .iter()
            .chain(&self.scenes.stack)
            .chain(std::iter::once(&self.scenes.current))
        {
            ensure!(name.len() <= 64, "invalid scene metadata");
        }
        ensure!(
            self.error.as_ref().is_none_or(|s| s.len() <= 1024),
            "inspection error exceeds limit"
        );
        ensure!(
            serde_json::to_vec(self)?.len() <= MAX_INSPECTION_BYTES,
            "inspection exceeds 24 KiB"
        );
        Ok(())
    }
    pub fn node(&self, path: &str) -> Result<&InspectNode> {
        self.nodes
            .iter()
            .find(|node| node.path == path)
            .context("inspector field is not exposed")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn node() -> InspectNode {
        InspectNode {
            path: "player.speed".into(),
            value: 200.into(),
            metadata: InspectMetadata {
                kind: InspectKind::Number,
                writable: true,
                persist: false,
                min: Some(0.0),
                max: Some(500.0),
                choices: Vec::new(),
            },
        }
    }
    #[test]
    fn edits_require_explicit_write_permission_and_current_session_value() {
        let mut node = node();
        let mut update = InspectUpdate {
            session: 4,
            path: node.path.clone(),
            expected: node.value.clone(),
            value: 300.into(),
        };
        node.validate_update(&update, 4).unwrap();
        assert!(node.validate_update(&update, 5).is_err());
        update.value = 900.into();
        assert!(node.validate_update(&update, 4).is_err());
        update.value = 300.into();
        node.metadata.writable = false;
        assert!(node.validate_update(&update, 4).is_err());
        node.metadata.writable = true;
        node.value = 201.into();
        assert!(node.validate_update(&update, 4).is_err());
    }
    #[test]
    fn malformed_paths_nested_types_and_oversized_values_are_rejected() {
        let mut n = node();
        n.path = "../../secret".into();
        assert!(n.validate().is_err());
        n.path = "player.speed".into();
        n.value = "fast".into();
        assert!(n.validate().is_err());
        n.metadata.kind = InspectKind::Table;
        n.value = serde_json::json!({"bad": null});
        assert!(n.validate().is_err());
        assert!(!same_shape(
            &serde_json::json!({"x":1}),
            &serde_json::json!({"y":1})
        ));
    }

    #[test]
    fn runtime_pages_preserve_hierarchy_and_reject_destroyed_selection() {
        use crate::scene_graph::{NodeKind, SceneGraph};
        let mut graph = SceneGraph::new("Arena").unwrap();
        let root = graph.root();
        let mut ids = Vec::new();
        for index in 0..40 {
            ids.push(
                graph
                    .create(root, NodeKind::Node2D, format!("Enemy{index}"))
                    .unwrap(),
            );
        }
        let key = RuntimeNodeKey {
            graph: graph.graph_id(),
            id: graph.node(ids[35]).unwrap().data().id,
        };
        let page = RuntimeTreePage::from_graph(&graph, 32, Some(key)).unwrap();
        assert_eq!(page.total, 41);
        assert_eq!(page.nodes[0].name, "Enemy31");
        assert_eq!(page.nodes[0].parent_id, Some(1));
        assert_eq!(page.selected.as_ref().unwrap().path, "Arena/Enemy35");
        graph.destroy(ids[35]).unwrap();
        let page = RuntimeTreePage::from_graph(&graph, 32, Some(key)).unwrap();
        assert!(page.selected.is_none() && page.selected_missing);
        let replacement = graph.create(root, NodeKind::Node2D, "Replacement").unwrap();
        assert_ne!(graph.node(replacement).unwrap().data().id, key.id);
    }
}
