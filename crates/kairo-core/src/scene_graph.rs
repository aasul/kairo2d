//! Validated, ordered scene hierarchy shared by the runtime and editor.
//! Runtime handles include a graph nonce and a generation; file IDs are separate.
use anyhow::{ensure, Context, Result};
use glam::{Mat3, Vec2};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeSet, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_GRAPH: AtomicU64 = AtomicU64::new(1);
const MAX_NODES: usize = 50_000;
const MAX_DEPTH: usize = 256;
const MAX_FILE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId {
    graph: u64,
    slot: u32,
    generation: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Transform2D {
    pub position: [f32; 2],
    pub rotation: f32,
    pub scale: [f32; 2],
    pub pivot: [f32; 2],
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            position: [0.0; 2],
            rotation: 0.0,
            scale: [1.0; 2],
            pivot: [0.0; 2],
        }
    }
}

impl Transform2D {
    pub fn validate(self) -> Result<()> {
        ensure!(
            self.position
                .into_iter()
                .chain(self.scale)
                .chain(self.pivot)
                .chain([self.rotation])
                .all(f32::is_finite),
            "scene transform must contain finite numbers"
        );
        Ok(())
    }

    pub fn matrix(self) -> Mat3 {
        Mat3::from_scale_angle_translation(
            Vec2::from_array(self.scale),
            self.rotation,
            Vec2::from_array(self.position),
        ) * Mat3::from_translation(-Vec2::from_array(self.pivot))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum NodeKind {
    Node,
    Node2D,
    Sprite,
    AnimatedSprite,
    Camera2D,
    PhysicsBody2D,
    StaticBody2D,
    DynamicBody2D,
    CharacterBody2D,
    Collider2D,
    Area2D,
    TileMap,
    ParticleEmitter,
    AudioSource,
    Text,
    CanvasLayer,
    Control,
    ScriptNode,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeFile {
    pub id: u64,
    pub name: String,
    pub kind: NodeKind,
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "yes")]
    pub visible: bool,
    #[serde(default)]
    pub transform: Transform2D,
    #[serde(default)]
    pub tags: BTreeSet<String>,
    #[serde(default)]
    pub metadata: Map<String, Value>,
    #[serde(default)]
    pub properties: Map<String, Value>,
    #[serde(default)]
    pub script: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefab: Option<String>,
    #[serde(default)]
    pub children: Vec<NodeFile>,
}

fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneFile {
    pub version: u32,
    pub name: String,
    pub root: NodeFile,
}

struct Slot {
    generation: u32,
    node: Option<Node>,
    world: Mat3,
    dirty: bool,
}

pub struct Node {
    data: NodeFile,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
}

impl Node {
    pub fn data(&self) -> &NodeFile {
        &self.data
    }
    pub fn parent(&self) -> Option<NodeId> {
        self.parent
    }
    pub fn children(&self) -> &[NodeId] {
        &self.children
    }
}

pub struct SceneGraph {
    nonce: u64,
    name: String,
    slots: Vec<Slot>,
    free: Vec<u32>,
    root: NodeId,
    next_file_id: u64,
    live: usize,
}

impl SceneGraph {
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        valid_name(&name)?;
        let nonce = NEXT_GRAPH
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map_err(|_| anyhow::anyhow!("scene graph identifier space exhausted"))?;
        let root = NodeId {
            graph: nonce,
            slot: 0,
            generation: 0,
        };
        let data = NodeFile {
            id: 1,
            name: name.clone(),
            kind: NodeKind::Node,
            enabled: true,
            visible: true,
            transform: Transform2D::default(),
            tags: BTreeSet::new(),
            metadata: Map::new(),
            properties: Map::new(),
            script: None,
            prefab: None,
            children: Vec::new(),
        };
        Ok(Self {
            nonce,
            name,
            slots: vec![Slot {
                generation: 0,
                node: Some(Node {
                    data,
                    parent: None,
                    children: Vec::new(),
                }),
                world: Mat3::IDENTITY,
                dirty: true,
            }],
            free: Vec::new(),
            root,
            next_file_id: 2,
            live: 1,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn root(&self) -> NodeId {
        self.root
    }
    pub fn len(&self) -> usize {
        self.live
    }
    pub fn is_empty(&self) -> bool {
        false
    }

    fn slot(&self, id: NodeId) -> Result<&Slot> {
        ensure!(id.graph == self.nonce, "node belongs to another scene");
        let slot = self
            .slots
            .get(id.slot as usize)
            .context("stale node handle")?;
        ensure!(
            slot.generation == id.generation && slot.node.is_some(),
            "stale node handle"
        );
        Ok(slot)
    }

    fn slot_mut(&mut self, id: NodeId) -> Result<&mut Slot> {
        self.slot(id)?;
        Ok(&mut self.slots[id.slot as usize])
    }

    pub fn node(&self, id: NodeId) -> Result<&Node> {
        Ok(self.slot(id)?.node.as_ref().expect("validated slot"))
    }
    fn depth(&self, id: NodeId) -> Result<usize> {
        let mut depth = 0;
        let mut parent = self.node(id)?.parent;
        while let Some(id) = parent {
            depth += 1;
            parent = self.node(id)?.parent;
        }
        Ok(depth)
    }

    fn subtree_depth(&self, id: NodeId) -> Result<usize> {
        let mut pending = vec![(id, 0)];
        let mut maximum = 0;
        while let Some((id, depth)) = pending.pop() {
            maximum = maximum.max(depth);
            pending.extend(
                self.node(id)?
                    .children
                    .iter()
                    .map(|child| (*child, depth + 1)),
            );
        }
        Ok(maximum)
    }
    fn node_mut(&mut self, id: NodeId) -> Result<&mut Node> {
        Ok(self.slot_mut(id)?.node.as_mut().expect("validated slot"))
    }

    fn allocate(&mut self, mut data: NodeFile, parent: Option<NodeId>) -> Result<NodeId> {
        ensure!(self.live < MAX_NODES, "scene exceeds {MAX_NODES} nodes");
        data.children.clear();
        let node = Node {
            data,
            parent,
            children: Vec::new(),
        };
        let id = if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.node = Some(node);
            slot.dirty = true;
            NodeId {
                graph: self.nonce,
                slot: index,
                generation: slot.generation,
            }
        } else {
            let index = u32::try_from(self.slots.len()).context("scene slot space exhausted")?;
            self.slots.push(Slot {
                generation: 0,
                node: Some(node),
                world: Mat3::IDENTITY,
                dirty: true,
            });
            NodeId {
                graph: self.nonce,
                slot: index,
                generation: 0,
            }
        };
        self.live += 1;
        Ok(id)
    }

    pub fn create(
        &mut self,
        parent: NodeId,
        kind: NodeKind,
        name: impl Into<String>,
    ) -> Result<NodeId> {
        ensure!(
            self.depth(parent)? < MAX_DEPTH,
            "scene hierarchy exceeds {MAX_DEPTH} levels"
        );
        let name = name.into();
        valid_name(&name)?;
        let id = self.next_file_id;
        self.next_file_id = id.checked_add(1).context("scene file ID space exhausted")?;
        let data = NodeFile {
            id,
            name,
            kind,
            enabled: true,
            visible: true,
            transform: Transform2D::default(),
            tags: BTreeSet::new(),
            metadata: Map::new(),
            properties: Map::new(),
            script: None,
            prefab: None,
            children: Vec::new(),
        };
        let child = self.allocate(data, Some(parent))?;
        self.node_mut(parent)?.children.push(child);
        Ok(child)
    }

    pub fn set_name(&mut self, id: NodeId, name: impl Into<String>) -> Result<()> {
        let name = name.into();
        valid_name(&name)?;
        self.node_mut(id)?.data.name = name;
        Ok(())
    }

    pub fn set_transform(&mut self, id: NodeId, transform: Transform2D) -> Result<()> {
        transform.validate()?;
        self.node_mut(id)?.data.transform = transform;
        self.invalidate(id)
    }

    pub fn set_enabled(&mut self, id: NodeId, enabled: bool) -> Result<()> {
        self.node_mut(id)?.data.enabled = enabled;
        Ok(())
    }

    pub fn set_visible(&mut self, id: NodeId, visible: bool) -> Result<()> {
        self.node_mut(id)?.data.visible = visible;
        Ok(())
    }

    pub fn add_tag(&mut self, id: NodeId, tag: impl Into<String>) -> Result<()> {
        let tag = tag.into();
        valid_name(&tag)?;
        self.node_mut(id)?.data.tags.insert(tag);
        Ok(())
    }

    pub fn remove_tag(&mut self, id: NodeId, tag: &str) -> Result<bool> {
        Ok(self.node_mut(id)?.data.tags.remove(tag))
    }

    pub fn set_script(&mut self, id: NodeId, script: Option<String>) -> Result<()> {
        if let Some(path) = &script {
            valid_asset_path(path, ".lua")?;
        }
        self.node_mut(id)?.data.script = script;
        Ok(())
    }

    pub fn set_prefab(&mut self, id: NodeId, prefab: Option<String>) -> Result<()> {
        if let Some(path) = &prefab {
            valid_asset_path(path, ".prefab")?;
        }
        self.node_mut(id)?.data.prefab = prefab;
        Ok(())
    }

    pub fn set_property(&mut self, id: NodeId, name: &str, value: Value) -> Result<()> {
        valid_name(name)?;
        self.node_mut(id)?
            .data
            .properties
            .insert(name.to_owned(), value);
        Ok(())
    }

    pub fn remove_property(&mut self, id: NodeId, name: &str) -> Result<bool> {
        Ok(self.node_mut(id)?.data.properties.remove(name).is_some())
    }

    pub fn reparent(&mut self, id: NodeId, new_parent: NodeId, index: usize) -> Result<()> {
        ensure!(id != self.root, "cannot reparent the scene root");
        self.node(id)?;
        self.node(new_parent)?;
        ensure!(
            self.depth(new_parent)? + self.subtree_depth(id)? < MAX_DEPTH,
            "scene hierarchy exceeds {MAX_DEPTH} levels"
        );
        let mut cursor = Some(new_parent);
        while let Some(ancestor) = cursor {
            ensure!(ancestor != id, "parent cycle rejected");
            cursor = self.node(ancestor)?.parent;
        }
        let old = self.node(id)?.parent.context("node has no parent")?;
        self.node_mut(old)?.children.retain(|child| *child != id);
        let siblings = &mut self.node_mut(new_parent)?.children;
        siblings.insert(index.min(siblings.len()), id);
        self.node_mut(id)?.parent = Some(new_parent);
        self.invalidate(id)
    }

    pub fn reorder(&mut self, id: NodeId, index: usize) -> Result<()> {
        let parent = self.node(id)?.parent.context("cannot reorder scene root")?;
        let children = &mut self.node_mut(parent)?.children;
        children.retain(|child| *child != id);
        children.insert(index.min(children.len()), id);
        Ok(())
    }

    pub fn destroy(&mut self, id: NodeId) -> Result<()> {
        ensure!(id != self.root, "cannot destroy the scene root");
        let parent = self.node(id)?.parent.context("node has no parent")?;
        let mut pending = vec![id];
        let mut descendants = Vec::new();
        while let Some(current) = pending.pop() {
            let node = self.node(current)?;
            pending.extend_from_slice(&node.children);
            descendants.push(current);
        }
        self.node_mut(parent)?.children.retain(|child| *child != id);
        for current in descendants {
            let slot = &mut self.slots[current.slot as usize];
            slot.node = None;
            slot.dirty = true;
            if let Some(next) = slot.generation.checked_add(1) {
                slot.generation = next;
                self.free.push(current.slot);
            }
            self.live -= 1;
        }
        Ok(())
    }

    fn invalidate(&mut self, id: NodeId) -> Result<()> {
        let mut pending = vec![id];
        while let Some(current) = pending.pop() {
            let children = self.node(current)?.children.clone();
            self.slot_mut(current)?.dirty = true;
            pending.extend(children);
        }
        Ok(())
    }

    pub fn world_matrix(&mut self, id: NodeId) -> Result<Mat3> {
        let mut chain = Vec::new();
        let mut cursor = Some(id);
        while let Some(current) = cursor {
            let slot = self.slot(current)?;
            if !slot.dirty {
                break;
            }
            chain.push(current);
            cursor = slot.node.as_ref().expect("validated slot").parent;
        }
        for current in chain.into_iter().rev() {
            let (parent, local) = {
                let node = self.node(current)?;
                (node.parent, node.data.transform.matrix())
            };
            let matrix = parent
                .map(|id| self.slots[id.slot as usize].world)
                .unwrap_or(Mat3::IDENTITY)
                * local;
            let slot = self.slot_mut(current)?;
            slot.world = matrix;
            slot.dirty = false;
        }
        Ok(self.slot(id)?.world)
    }

    pub fn world_position(&mut self, id: NodeId) -> Result<Vec2> {
        let pivot = Vec2::from_array(self.node(id)?.data.transform.pivot);
        Ok(self.world_matrix(id)?.transform_point2(pivot))
    }

    pub fn find_child(&self, parent: NodeId, name: &str) -> Result<Option<NodeId>> {
        Ok(self
            .node(parent)?
            .children
            .iter()
            .copied()
            .find(|child| self.node(*child).is_ok_and(|node| node.data.name == name)))
    }

    pub fn find_recursive(&self, parent: NodeId, name: &str) -> Result<Option<NodeId>> {
        let mut pending = self
            .node(parent)?
            .children
            .iter()
            .rev()
            .copied()
            .collect::<Vec<_>>();
        while let Some(id) = pending.pop() {
            let node = self.node(id)?;
            if node.data.name == name {
                return Ok(Some(id));
            }
            pending.extend(node.children.iter().rev().copied());
        }
        Ok(None)
    }

    pub fn find_path(&self, path: &str) -> Result<Option<NodeId>> {
        let mut current = self.root;
        for (index, part) in path.split('/').filter(|part| !part.is_empty()).enumerate() {
            if index == 0 && part == self.name {
                continue;
            }
            let Some(child) = self.find_child(current, part)? else {
                return Ok(None);
            };
            current = child;
        }
        Ok(Some(current))
    }

    pub fn path(&self, id: NodeId) -> Result<String> {
        let mut names = Vec::new();
        let mut current = Some(id);
        while let Some(node_id) = current {
            let node = self.node(node_id)?;
            names.push(node.data.name.as_str());
            current = node.parent;
        }
        names.reverse();
        Ok(names.join("/"))
    }

    pub fn find_by_name(&self, name: &str) -> Vec<NodeId> {
        self.find_where(|node| node.data.name == name)
    }
    pub fn find_by_tag(&self, tag: &str) -> Vec<NodeId> {
        self.find_where(|node| node.data.tags.contains(tag))
    }
    pub fn find_by_file_id(&self, id: u64) -> Option<NodeId> {
        self.find_where(|node| node.data.id == id)
            .into_iter()
            .next()
    }

    fn find_where(&self, test: impl Fn(&Node) -> bool) -> Vec<NodeId> {
        self.slots
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| {
                let node = slot.node.as_ref()?;
                test(node).then_some(NodeId {
                    graph: self.nonce,
                    slot: index as u32,
                    generation: slot.generation,
                })
            })
            .collect()
    }

    fn export_node(&self, id: NodeId) -> Result<NodeFile> {
        let node = self.node(id)?;
        let mut data = node.data.clone();
        data.children = node
            .children
            .iter()
            .map(|child| self.export_node(*child))
            .collect::<Result<_>>()?;
        Ok(data)
    }

    pub fn to_file(&self) -> Result<SceneFile> {
        Ok(SceneFile {
            version: 1,
            name: self.name.clone(),
            root: self.export_node(self.root)?,
        })
    }

    pub fn to_json(&self) -> Result<String> {
        let source = serde_json::to_string_pretty(&self.to_file()?)?;
        ensure!(source.len() <= MAX_FILE_BYTES, "scene file exceeds 16 MiB");
        Ok(source)
    }

    pub fn from_json(source: &str) -> Result<Self> {
        ensure!(source.len() <= MAX_FILE_BYTES, "scene file exceeds 16 MiB");
        let file: SceneFile = serde_json::from_str(source).context("invalid scene JSON")?;
        Self::from_file(file)
    }

    pub fn from_file(file: SceneFile) -> Result<Self> {
        ensure!(
            file.version == 1,
            "unsupported scene file version {}",
            file.version
        );
        ensure!(
            file.root.name == file.name,
            "scene root name must match scene name"
        );
        let mut graph = Self::new(&file.name)?;
        let mut ids = HashSet::new();
        let mut pending = vec![(None, file.root, 0_usize)];
        let mut root_seen = false;
        while let Some((parent, mut data, depth)) = pending.pop() {
            ensure!(
                depth <= MAX_DEPTH,
                "scene hierarchy exceeds {MAX_DEPTH} levels"
            );
            ensure!(ids.insert(data.id), "duplicate scene node ID {}", data.id);
            valid_name(&data.name)?;
            data.transform.validate()?;
            if let Some(path) = &data.script {
                valid_asset_path(path, ".lua")?;
            }
            if let Some(path) = &data.prefab {
                valid_asset_path(path, ".prefab")?;
            }
            for tag in &data.tags {
                valid_name(tag)?;
            }
            let children = std::mem::take(&mut data.children);
            let id = if let Some(parent) = parent {
                let id = graph.allocate(data, Some(parent))?;
                graph.node_mut(parent)?.children.push(id);
                id
            } else {
                ensure!(!root_seen, "scene has more than one root");
                root_seen = true;
                graph.node_mut(graph.root)?.data = data;
                graph.root
            };
            let max = graph.node(id)?.data.id;
            graph.next_file_id = graph.next_file_id.max(
                max.checked_add(1)
                    .context("scene node ID space exhausted")?,
            );
            for child in children.into_iter().rev() {
                pending.push((Some(id), child, depth + 1));
            }
        }
        ensure!(root_seen, "scene has no root");
        Ok(graph)
    }

    pub fn duplicate(&mut self, id: NodeId, parent: NodeId) -> Result<NodeId> {
        let source = self.export_node(id)?;
        self.instantiate_subtree(parent, &source)
    }

    /// Copy a prefab hierarchy into this scene with fresh file and runtime IDs.
    /// The template is validated completely before the scene is modified.
    pub fn instantiate_subtree(&mut self, parent: NodeId, template: &NodeFile) -> Result<NodeId> {
        let parent_depth = self.depth(parent)?;
        let mut count = 0_usize;
        let mut pending = vec![(template, parent_depth + 1)];
        while let Some((node, depth)) = pending.pop() {
            count += 1;
            ensure!(
                count <= MAX_NODES && self.live + count <= MAX_NODES,
                "scene exceeds {MAX_NODES} nodes"
            );
            ensure!(
                depth <= MAX_DEPTH,
                "scene hierarchy exceeds {MAX_DEPTH} levels"
            );
            valid_name(&node.name)?;
            node.transform.validate()?;
            if let Some(path) = &node.script {
                valid_asset_path(path, ".lua")?;
            }
            if let Some(path) = &node.prefab {
                valid_asset_path(path, ".prefab")?;
            }
            for tag in &node.tags {
                valid_name(tag)?;
            }
            pending.extend(node.children.iter().map(|child| (child, depth + 1)));
        }
        self.next_file_id
            .checked_add(count as u64)
            .context("scene file ID space exhausted")?;
        let mut pending = vec![(parent, template.clone())];
        let mut first = None;
        while let Some((destination, mut source)) = pending.pop() {
            let children = std::mem::take(&mut source.children);
            source.id = self.next_file_id;
            self.next_file_id += 1;
            let id = self.allocate(source, Some(destination))?;
            self.node_mut(destination)?.children.push(id);
            if first.is_none() {
                first = Some(id);
            }
            for child in children.into_iter().rev() {
                pending.push((id, child));
            }
        }
        first.context("prefab contains no nodes")
    }
}

fn valid_name(name: &str) -> Result<()> {
    ensure!(
        !name.is_empty() && name.len() <= 128 && !name.contains(['/', '\\', '\0']),
        "node name or tag must contain 1..128 bytes without path separators"
    );
    Ok(())
}

fn valid_asset_path(path: &str, extension: &str) -> Result<()> {
    ensure!(
        path.len() <= 256
            && path.ends_with(extension)
            && !path.contains('\\')
            && path
                .split('/')
                .all(|part| !part.is_empty() && part != "." && part != ".." && !part.contains(':')),
        "invalid project-relative asset path: {path}"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hierarchy_rejects_cycles_and_stale_handles() {
        let mut graph = SceneGraph::new("Game").unwrap();
        let a = graph.create(graph.root(), NodeKind::Node2D, "A").unwrap();
        let b = graph.create(a, NodeKind::Sprite, "B").unwrap();
        assert!(graph.reparent(a, b, 0).is_err());
        assert_eq!(graph.path(b).unwrap(), "Game/A/B");
        graph.destroy(a).unwrap();
        assert!(graph.node(a).is_err());
        assert!(graph.node(b).is_err());
        let c = graph.create(graph.root(), NodeKind::Node2D, "C").unwrap();
        assert_ne!(a, c);
        assert!(graph.node(a).is_err());
    }

    #[test]
    fn order_reparent_search_and_dirty_transforms() {
        let mut graph = SceneGraph::new("Game").unwrap();
        let a = graph.create(graph.root(), NodeKind::Node2D, "A").unwrap();
        let b = graph.create(graph.root(), NodeKind::Node2D, "B").unwrap();
        let child = graph.create(a, NodeKind::Sprite, "Sprite").unwrap();
        graph.add_tag(child, "hero").unwrap();
        graph
            .set_transform(
                a,
                Transform2D {
                    position: [10.0, 0.0],
                    ..Default::default()
                },
            )
            .unwrap();
        graph
            .set_transform(
                child,
                Transform2D {
                    position: [2.0, 3.0],
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(graph.world_position(child).unwrap().to_array(), [12.0, 3.0]);
        graph
            .set_transform(
                a,
                Transform2D {
                    position: [20.0, 0.0],
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(graph.world_position(child).unwrap().to_array(), [22.0, 3.0]);
        graph.reparent(child, b, 0).unwrap();
        assert_eq!(graph.world_position(child).unwrap().to_array(), [2.0, 3.0]);
        graph
            .set_transform(
                child,
                Transform2D {
                    position: [2.0, 3.0],
                    pivot: [4.0, 5.0],
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(graph.world_position(child).unwrap().to_array(), [2.0, 3.0]);
        assert_eq!(
            graph
                .world_matrix(child)
                .unwrap()
                .transform_point2(Vec2::ZERO)
                .to_array(),
            [-2.0, -2.0]
        );
        graph.reorder(b, 0).unwrap();
        assert_eq!(graph.node(graph.root()).unwrap().children(), &[b, a]);
        assert_eq!(graph.find_path("Game/B/Sprite").unwrap(), Some(child));
        assert_eq!(graph.find_by_tag("hero"), vec![child]);
    }

    #[test]
    fn scene_roundtrip_and_validation() {
        let mut graph = SceneGraph::new("Level").unwrap();
        let parent = graph
            .create(graph.root(), NodeKind::Node2D, "World")
            .unwrap();
        let child = graph.create(parent, NodeKind::Sprite, "Hero").unwrap();
        graph
            .set_script(child, Some("scripts/player.lua".into()))
            .unwrap();
        graph
            .set_property(child, "texture", Value::String("assets/player.png".into()))
            .unwrap();
        let source = graph.to_json().unwrap();
        let restored = SceneGraph::from_json(&source).unwrap();
        let restored_child = restored.find_path("Level/World/Hero").unwrap().unwrap();
        assert_eq!(
            restored
                .node(restored_child)
                .unwrap()
                .data()
                .script
                .as_deref(),
            Some("scripts/player.lua")
        );
        assert_eq!(restored.to_json().unwrap(), source);
        let mut invalid = restored.to_file().unwrap();
        invalid.root.children[0].id = invalid.root.id;
        assert!(SceneGraph::from_file(invalid).is_err());
        assert!(SceneGraph::from_json("{bad json").is_err());
    }

    #[test]
    fn deep_hierarchy_is_iterative_for_mutation_and_cache() {
        let mut graph = SceneGraph::new("Deep").unwrap();
        let mut parent = graph.root();
        for _ in 0..200 {
            parent = graph.create(parent, NodeKind::Node2D, "Branch").unwrap();
        }
        graph
            .set_transform(
                graph.root(),
                Transform2D {
                    position: [4.0, 5.0],
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(graph.world_position(parent).unwrap().to_array(), [4.0, 5.0]);
        graph
            .destroy(graph.node(graph.root()).unwrap().children()[0])
            .unwrap();
        assert_eq!(graph.len(), 1);
        assert!(graph.node(parent).is_err());
    }

    #[test]
    fn five_thousand_nodes_keep_cached_transforms_and_unique_ids() {
        let started = std::time::Instant::now();
        let mut graph = SceneGraph::new("Scale").unwrap();
        let mut ids = Vec::new();
        for index in 0..5_000 {
            ids.push(
                graph
                    .create(graph.root(), NodeKind::Sprite, format!("Sprite{index}"))
                    .unwrap(),
            );
        }
        graph
            .set_transform(
                graph.root(),
                Transform2D {
                    position: [8.0, 13.0],
                    ..Default::default()
                },
            )
            .unwrap();
        for id in &ids {
            assert_eq!(graph.world_position(*id).unwrap().to_array(), [8.0, 13.0]);
        }
        assert_eq!(graph.len(), 5_001);
        assert_eq!(
            ids.iter()
                .map(|id| graph.node(*id).unwrap().data().id)
                .collect::<HashSet<_>>()
                .len(),
            ids.len()
        );
        eprintln!(
            "5,000-node creation and transform validation: {:?}",
            started.elapsed()
        );
    }
}
