use crate::tabs::Tab;
use anyhow::{ensure, Context, Result};
use eframe::egui;
use kairo_core::inspector::{
    apply_scene_field, runtime_field_value, InspectNode, RuntimeNodeDetails,
};
use kairo_core::scene_graph::SceneGraph;
use kairo_core::Config;
use kairo_project::{ProjectFiles, ProjectNode};
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub struct Workspace {
    pub files: ProjectFiles,
    pub config: Config,
    pub nodes: Vec<ProjectNode>,
    pub tabs: Vec<Tab>,
    pub active: usize,
    pub selected: Option<PathBuf>,
    pub filter: String,
    pub recent_files: Vec<PathBuf>,
    collapsed: HashSet<PathBuf>,
}

pub enum BrowserAction {
    Open(PathBuf),
    NewFile,
    NewFolder,
    Rename(PathBuf),
    Duplicate(PathBuf),
    Delete(PathBuf),
    Reveal,
    Refresh,
}

impl Workspace {
    fn authored_runtime_node(
        &self,
        source: &str,
        details: &RuntimeNodeDetails,
    ) -> Result<SceneGraph> {
        let path = Path::new(source);
        ensure!(
            path.extension().is_some_and(|ext| ext == "scene"),
            "runtime source is not a scene file"
        );
        let bytes = self.files.read(path)?;
        let graph = SceneGraph::from_json(std::str::from_utf8(&bytes)?)?;
        let id = graph
            .find_by_file_id(details.node.key.id)
            .context("runtime node is not present in the scene file")?;
        ensure!(
            graph.path(id)? == details.path,
            "scene node path changed; refresh before applying"
        );
        ensure!(
            graph.node(id)?.data().kind == details.node.kind,
            "scene node type changed"
        );
        Ok(graph)
    }

    pub fn runtime_scene_values(
        &self,
        source: &str,
        details: &RuntimeNodeDetails,
    ) -> Result<BTreeMap<String, serde_json::Value>> {
        let graph = self.authored_runtime_node(source, details)?;
        let id = graph
            .find_by_file_id(details.node.key.id)
            .context("scene node is missing")?;
        details
            .exposed
            .iter()
            .map(|field| {
                Ok((
                    field.path.clone(),
                    runtime_field_value(graph.node(id)?.data(), &field.path).with_context(
                        || format!("'{}' is not stored in the scene file", field.path),
                    )?,
                ))
            })
            .collect()
    }

    pub fn apply_runtime_to_scene(
        &mut self,
        source: &str,
        details: &RuntimeNodeDetails,
        field: &InspectNode,
        expected_scene: serde_json::Value,
    ) -> Result<()> {
        let path = Path::new(source);
        ensure!(
            !self
                .tabs
                .iter()
                .any(|tab| tab.path() == path && tab.dirty()),
            "save or close the modified scene tab before applying a runtime value"
        );
        let mut graph = self.authored_runtime_node(source, details)?;
        let id = graph
            .find_by_file_id(details.node.key.id)
            .context("scene node is missing")?;
        apply_scene_field(
            &mut graph,
            id,
            &field.path,
            expected_scene,
            field.value.clone(),
            field.metadata.clone(),
        )?;
        self.files.write(path, graph.to_json()?.as_bytes())?;
        for tab in &mut self.tabs {
            if tab.path() == path {
                *tab = Tab::open(&self.files, path.to_path_buf())?;
            }
        }
        self.refresh()
    }

    pub fn open(root: &Path) -> Result<Self> {
        let files = ProjectFiles::open(root)?;
        ensure!(
            files.filesystem().exists("main.lua")?,
            "this directory has no main.lua"
        );
        let config = Config::load(files.filesystem())?;
        let nodes = files.list()?;
        let mut workspace = Self {
            files,
            config,
            nodes,
            tabs: Vec::new(),
            active: 0,
            selected: None,
            filter: String::new(),
            recent_files: Vec::new(),
            collapsed: HashSet::new(),
        };
        workspace.open_file("main.lua".into())?;
        Ok(workspace)
    }

    pub fn open_file(&mut self, path: PathBuf) -> Result<()> {
        self.files.resolve(&path)?;
        if let Some(index) = self.tabs.iter().position(|tab| tab.path() == path) {
            self.active = index;
        } else {
            ensure!(
                self.tabs.len() < 32,
                "close some tabs before opening more (limit 32)"
            );
            let tab = Tab::open(&self.files, path.clone())?;
            self.tabs.push(tab);
            self.active = self.tabs.len() - 1;
        }
        self.recent_files.retain(|v| *v != path);
        self.recent_files.insert(0, path.clone());
        self.recent_files.truncate(64);
        self.selected = Some(path);
        Ok(())
    }

    pub fn dirty(&self) -> bool {
        self.tabs.iter().any(Tab::dirty)
    }

    pub fn save_all(&mut self) -> Result<()> {
        for tab in &mut self.tabs {
            if tab.dirty() {
                tab.save(&self.files)?;
            }
        }
        self.refresh()
    }

    pub fn refresh(&mut self) -> Result<()> {
        self.nodes = self.files.list()?;
        self.config = Config::load(self.files.filesystem())?;
        Ok(())
    }

    pub fn save_tool_file(
        &mut self,
        path: &Path,
        previous: Option<&[u8]>,
        bytes: &[u8],
    ) -> Result<()> {
        ensure!(
            !self
                .tabs
                .iter()
                .any(|tab| tab.path() == path && tab.dirty()),
            "save or close the modified code tab before saving from this tool"
        );
        let actual = if self.files.resolve(path)?.try_exists()? {
            Some(self.files.read(path)?)
        } else {
            None
        };
        ensure!(
            actual.as_deref() == previous,
            "file changed on disk; reload it before saving"
        );
        if previous.is_some() {
            self.files.write(path, bytes)?;
        } else {
            self.files.create_file(path, bytes)?;
        }
        for tab in &mut self.tabs {
            if let Tab::Code(document) = tab {
                if document.path == path {
                    document.reload(&self.files)?;
                }
            }
        }
        self.refresh()
    }

    pub fn save_settings(&mut self, settings: &mut kairo_project::SettingsDocument) -> Result<()> {
        let path = Path::new("kairo.toml");
        ensure!(
            !self
                .tabs
                .iter()
                .any(|tab| tab.path() == path && tab.dirty()),
            "save or close the modified kairo.toml code tab before applying project settings"
        );
        settings.save(&self.files)?;
        for tab in &mut self.tabs {
            if let Tab::Code(document) = tab {
                if document.path == path {
                    document.reload(&self.files)?;
                }
            }
        }
        self.refresh()
    }

    pub fn close_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.tabs.remove(index);
            if self.active > index {
                self.active -= 1;
            }
            self.active = self.active.min(self.tabs.len().saturating_sub(1));
        }
    }

    pub fn selected_parent(&self) -> PathBuf {
        match &self.selected {
            Some(path)
                if self
                    .nodes
                    .iter()
                    .any(|node| node.relative == *path && node.directory) =>
            {
                path.clone()
            }
            Some(path) => path.parent().unwrap_or(Path::new("")).to_owned(),
            None => PathBuf::new(),
        }
    }

    pub fn browser(&mut self, ui: &mut egui::Ui) -> Option<BrowserAction> {
        let mut action = None;
        ui.strong("PROJECT");
        ui.add(
            egui::TextEdit::singleline(&mut self.filter)
                .hint_text("Filter files")
                .desired_width(f32::INFINITY),
        );
        ui.horizontal(|ui| {
            if ui.button("+ Lua").clicked() {
                action = Some(BrowserAction::NewFile);
            }
            if ui.button("+ Folder").clicked() {
                action = Some(BrowserAction::NewFolder);
            }
            if ui.button("Refresh").clicked() {
                action = Some(BrowserAction::Refresh);
            }
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .id_source("project-tree")
            .show(ui, |ui| {
                for node in &self.nodes {
                    if self.filter.is_empty() {
                        if node
                            .relative
                            .ancestors()
                            .skip(1)
                            .any(|parent| self.collapsed.contains(parent))
                        {
                            continue;
                        }
                    } else if !node
                        .relative
                        .to_string_lossy()
                        .to_lowercase()
                        .contains(&self.filter.to_lowercase())
                    {
                        continue;
                    }
                    let depth = node.relative.components().count().saturating_sub(1);
                    let name = node
                        .relative
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    let label = if node.directory {
                        format!(
                            "{} {name}",
                            if self.collapsed.contains(&node.relative) {
                                ">"
                            } else {
                                "v"
                            }
                        )
                    } else {
                        name.to_string()
                    };
                    ui.horizontal(|ui| {
                        ui.add_space(depth as f32 * 12.0);
                        let response = ui.selectable_label(
                            self.selected.as_ref() == Some(&node.relative),
                            label,
                        );
                        if response.clicked() {
                            self.selected = Some(node.relative.clone());
                            if node.directory && !self.collapsed.remove(&node.relative) {
                                self.collapsed.insert(node.relative.clone());
                            }
                        }
                        if response.double_clicked() && !node.directory {
                            action = Some(BrowserAction::Open(node.relative.clone()));
                        }
                        response.context_menu(|ui| {
                            if !node.directory && ui.button("Open").clicked() {
                                action = Some(BrowserAction::Open(node.relative.clone()));
                                ui.close_menu();
                            }
                            if ui.button("Rename").clicked() {
                                action = Some(BrowserAction::Rename(node.relative.clone()));
                                ui.close_menu();
                            }
                            if !node.directory && ui.button("Duplicate").clicked() {
                                action = Some(BrowserAction::Duplicate(node.relative.clone()));
                                ui.close_menu();
                            }
                            if ui.button("Delete...").clicked() {
                                action = Some(BrowserAction::Delete(node.relative.clone()));
                                ui.close_menu();
                            }
                            if ui.button("Reveal project folder").clicked() {
                                action = Some(BrowserAction::Reveal);
                                ui.close_menu();
                            }
                        });
                    });
                }
            });
        action
    }

    pub fn rename(&mut self, from: &Path, to: &Path) -> Result<()> {
        ensure!(
            !self
                .tabs
                .iter()
                .any(|tab| tab.path().starts_with(from) && tab.dirty()),
            "save or close modified tabs before renaming"
        );
        let graph = kairo_project::dependencies::DependencyGraph::scan(&self.files)?;
        ensure!(
            graph.problems().is_empty(),
            "resource analysis failed: {}",
            graph.problems().first().cloned().unwrap_or_default()
        );
        let affected: Vec<_> = graph
            .references()
            .iter()
            .filter(|reference| reference.target == from || reference.target.starts_with(from))
            .collect();
        if let Some(source) = graph.lua_mentions(from).first() {
            anyhow::bail!(
                "{} may load this resource dynamically; review that Lua path before renaming",
                source.display()
            );
        }
        let is_directory = self.files.resolve(from)?.is_dir();
        ensure!(
            affected.is_empty() || !is_directory,
            "folder move affects {} known references; move resources individually",
            affected.len()
        );
        ensure!(
            is_directory || from.extension() == to.extension(),
            "renaming cannot change a resource's file type"
        );
        ensure!(
            from.parent() == to.parent()
                || !matches!(
                    from.extension().and_then(|e| e.to_str()),
                    Some("tmj" | "tsj" | "json")
                ),
            "moving tilemap/animation metadata across folders can break relative references"
        );
        let mut edits = std::collections::BTreeMap::<PathBuf, (Vec<u8>, serde_json::Value)>::new();
        for reference in affected {
            ensure!(
                reference.owner != from,
                "self-referencing resource requires a manual move"
            );
            ensure!(
                matches!(
                    reference.owner.extension().and_then(|ext| ext.to_str()),
                    Some("scene" | "prefab")
                ),
                "{} uses a relative resource path; update it manually before renaming",
                reference.owner.display()
            );
            let entry = match edits.entry(reference.owner.clone()) {
                std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
                std::collections::btree_map::Entry::Vacant(entry) => {
                    let original = self.files.read(&reference.owner)?;
                    let document = serde_json::from_slice(&original)?;
                    entry.insert((original, document))
                }
            };
            let field = entry
                .1
                .pointer_mut(&reference.field)
                .context("resource field changed during rename")?;
            ensure!(
                field.as_str() == Some(&reference.raw),
                "resource field changed during rename"
            );
            *field = serde_json::Value::String(to.to_string_lossy().replace('\\', "/"));
        }
        ensure!(
            !self
                .tabs
                .iter()
                .any(|tab| tab.dirty() && edits.contains_key(tab.path())),
            "save modified referencing scenes before renaming"
        );
        let writes: Vec<(PathBuf, Vec<u8>, Vec<u8>)> = edits
            .into_iter()
            .map(|(path, (original, document))| {
                Ok((path, original, serde_json::to_vec_pretty(&document)?))
            })
            .collect::<Result<_>>()?;
        self.files.rename(from, to)?;
        let mut written = Vec::new();
        for (path, original, replacement) in &writes {
            if let Err(error) = self.files.write(path, replacement) {
                for (path, original) in written.into_iter().rev() {
                    let _ = self.files.write(path, original);
                }
                let _ = self.files.rename(to, from);
                return Err(error).with_context(|| {
                    format!(
                        "cannot update {} after rename; rollback attempted",
                        path.display()
                    )
                });
            }
            written.push((path.as_path(), original.as_slice()));
        }
        for tab in &mut self.tabs {
            if let Ok(suffix) = tab.path().strip_prefix(from) {
                let destination = to.join(suffix);
                tab.rename(destination);
            }
            if writes.iter().any(|(path, _, _)| tab.path() == path) {
                let path = tab.path().to_path_buf();
                *tab = Tab::open(&self.files, path)?;
            }
        }
        self.selected = Some(to.to_owned());
        self.refresh()
    }

    pub fn delete(&mut self, path: &Path) -> Result<()> {
        ensure!(
            !self
                .tabs
                .iter()
                .any(|tab| tab.path().starts_with(path) && tab.dirty()),
            "save or close modified tabs before deleting this file/folder"
        );
        self.files.delete(path)?;
        self.tabs.retain(|tab| !tab.path().starts_with(path));
        self.active = self.active.min(self.tabs.len().saturating_sub(1));
        self.selected = None;
        self.refresh()
    }
}

#[cfg(test)]
mod runtime_scene_tests {
    use super::*;
    use kairo_core::inspector::{InspectKind, InspectMetadata, RuntimeNodeKey, RuntimeTreePage};
    use kairo_core::scene_graph::NodeKind;

    #[test]
    fn apply_runtime_value_checks_authored_node_and_persistence_permission() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("main.lua"), "-- test").unwrap();
        std::fs::write(
            root.path().join("kairo.toml"),
            "[game]\ntitle='Test'\n[save]\nidentity='kairo-test-inspector'\n",
        )
        .unwrap();
        std::fs::create_dir(root.path().join("scenes")).unwrap();
        let mut authored = SceneGraph::new("Arena").unwrap();
        let boss = authored
            .create(authored.root(), NodeKind::Node2D, "Boss")
            .unwrap();
        authored
            .set_property(boss, "attack_delay", serde_json::json!(1.25))
            .unwrap();
        let source = "scenes/arena.scene";
        std::fs::write(root.path().join(source), authored.to_json().unwrap()).unwrap();
        let mut runtime = SceneGraph::from_json(&authored.to_json().unwrap()).unwrap();
        let boss = runtime.find_by_file_id(2).unwrap();
        let metadata = InspectMetadata {
            kind: InspectKind::Number,
            writable: true,
            persist: true,
            min: Some(0.1),
            max: Some(5.0),
            choices: Vec::new(),
        };
        runtime
            .expose_inspector_field(boss, "property.attack_delay", metadata)
            .unwrap();
        runtime
            .set_property(boss, "attack_delay", serde_json::json!(0.8))
            .unwrap();
        let key = RuntimeNodeKey {
            graph: runtime.graph_id(),
            id: 2,
        };
        let details = RuntimeTreePage::from_graph(&runtime, 0, Some(key))
            .unwrap()
            .selected
            .unwrap();
        let field = details
            .exposed
            .iter()
            .find(|field| field.path == "property.attack_delay")
            .unwrap();
        let mut workspace = Workspace::open(root.path()).unwrap();
        assert_eq!(
            workspace.runtime_scene_values(source, &details).unwrap()["property.attack_delay"],
            1.25
        );
        assert!(workspace
            .apply_runtime_to_scene(source, &details, field, serde_json::json!(0.5))
            .is_err());
        workspace
            .apply_runtime_to_scene(source, &details, field, serde_json::json!(1.25))
            .unwrap();
        let saved =
            SceneGraph::from_json(&std::fs::read_to_string(root.path().join(source)).unwrap())
                .unwrap();
        assert_eq!(
            saved
                .node(saved.find_by_file_id(2).unwrap())
                .unwrap()
                .data()
                .properties["attack_delay"],
            0.8
        );
        let mut transient = field.clone();
        transient.metadata.persist = false;
        assert!(workspace
            .apply_runtime_to_scene(source, &details, &transient, serde_json::json!(0.8))
            .is_err());
    }

    #[test]
    fn rename_updates_known_scene_references_and_preserves_unrelated_assets() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("main.lua"), "-- test").unwrap();
        std::fs::write(
            root.path().join("kairo.toml"),
            "[game]\ntitle='Test'\n[save]\nidentity='kairo-test-rename'\n",
        )
        .unwrap();
        std::fs::write(root.path().join("image.png"), b"x").unwrap();
        std::fs::write(root.path().join("spare.png"), b"x").unwrap();
        let mut graph = SceneGraph::new("Arena").unwrap();
        let image = graph
            .create(graph.root(), NodeKind::Sprite, "Image")
            .unwrap();
        graph
            .set_property(image, "texture", serde_json::json!("image.png"))
            .unwrap();
        std::fs::write(root.path().join("arena.scene"), graph.to_json().unwrap()).unwrap();
        let mut workspace = Workspace::open(root.path()).unwrap();
        workspace
            .rename(Path::new("image.png"), Path::new("renamed.png"))
            .unwrap();
        assert!(!root.path().join("image.png").exists());
        assert!(root.path().join("renamed.png").exists());
        let saved = SceneGraph::from_json(
            &std::fs::read_to_string(root.path().join("arena.scene")).unwrap(),
        )
        .unwrap();
        let image = saved.find_by_file_id(2).unwrap();
        assert_eq!(
            saved.node(image).unwrap().data().properties["texture"],
            "renamed.png"
        );
        workspace
            .rename(Path::new("spare.png"), Path::new("spare-new.png"))
            .unwrap();
        assert!(root.path().join("spare-new.png").exists());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kairo_project::{create_project, SettingsDocument, Template};

    fn project() -> (tempfile::TempDir, Workspace) {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("game");
        create_project(&path, "Feature test", Template::Empty).unwrap();
        let workspace = Workspace::open(&path).unwrap();
        (root, workspace)
    }

    #[test]
    fn feature_settings_persist_and_refresh_an_open_config_tab() {
        let (_root, mut workspace) = project();
        workspace.open_file("kairo.toml".into()).unwrap();
        let mut settings = SettingsDocument::load(&workspace.files).unwrap();
        settings.config.micro.enabled = true;
        settings.config.micro.palette = "olive4".into();
        settings.config.micro.width = 160;
        settings.config.micro.height = 144;
        settings.config.replay.enabled = true;
        workspace.save_settings(&mut settings).unwrap();
        assert!(workspace.config.micro.enabled);
        assert!(workspace.config.replay.enabled);
        assert_eq!(workspace.config.micro.palette, "olive4");
        let Tab::Code(document) = &workspace.tabs[workspace.active] else {
            panic!("config should be editable text");
        };
        assert!(!document.dirty());
        assert_eq!(
            Config::parse(&document.text).unwrap().micro,
            workspace.config.micro
        );
    }

    #[test]
    fn feature_settings_do_not_overwrite_unsaved_config_edits() {
        let (_root, mut workspace) = project();
        workspace.open_file("kairo.toml".into()).unwrap();
        let mut settings = SettingsDocument::load(&workspace.files).unwrap();
        settings.config.micro.enabled = true;
        let before = workspace.files.read("kairo.toml").unwrap();
        if let Tab::Code(document) = &mut workspace.tabs[workspace.active] {
            document.text.push_str("\n# unsaved edit\n");
        }
        assert!(workspace.save_settings(&mut settings).is_err());
        assert_eq!(workspace.files.read("kairo.toml").unwrap(), before);
        assert!(workspace.tabs[workspace.active].dirty());
    }

    #[test]
    fn feature_settings_refuse_stale_documents_and_preserve_custom_fields() {
        let (_root, mut workspace) = project();
        let mut stale = SettingsDocument::load(&workspace.files).unwrap();
        let mut source = String::from_utf8(workspace.files.read("kairo.toml").unwrap()).unwrap();
        source.push_str("\n# custom project metadata\n[custom]\nlevel = 7\n");
        workspace
            .files
            .write("kairo.toml", source.as_bytes())
            .unwrap();
        stale.config.micro.enabled = true;
        assert!(workspace.save_settings(&mut stale).is_err());
        let mut fresh = SettingsDocument::load(&workspace.files).unwrap();
        fresh.config.micro.enabled = true;
        workspace.save_settings(&mut fresh).unwrap();
        let saved = workspace
            .files
            .filesystem()
            .read_text("kairo.toml")
            .unwrap();
        assert!(saved.contains("# custom project metadata"));
        assert!(saved.contains("[custom]"));
        assert!(saved.contains("level = 7"));
    }
}
