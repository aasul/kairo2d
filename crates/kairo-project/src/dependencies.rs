//! Conservative static resource relationships for editor analysis.
use crate::ProjectFiles;
use anyhow::Result;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceUse {
    pub owner: PathBuf,
    pub target: PathBuf,
    /// JSON pointer to the serialized property that contains the reference.
    pub field: String,
    pub raw: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Usage {
    Referenced,
    DynamicUncertain,
    ApparentlyUnused,
}

#[derive(Clone, Debug, Default)]
pub struct DependencyGraph {
    files: BTreeSet<PathBuf>,
    uses: Vec<ResourceUse>,
    problems: Vec<String>,
    lua_sources: BTreeMap<PathBuf, String>,
}

impl DependencyGraph {
    pub fn scan(files: &ProjectFiles) -> Result<Self> {
        let nodes = files.list()?;
        let mut graph = Self {
            files: nodes
                .iter()
                .filter(|node| !node.directory)
                .map(|node| node.relative.clone())
                .collect(),
            ..Self::default()
        };
        for owner in &graph.files {
            if owner
                .extension()
                .is_some_and(|extension| extension == "lua")
            {
                let bytes = files.read(owner)?;
                if bytes.len() <= 1024 * 1024 {
                    if let Ok(source) = String::from_utf8(bytes) {
                        graph.lua_sources.insert(owner.clone(), source);
                    } else {
                        graph.problems.push(format!(
                            "{}: Lua source is not UTF-8; dynamic references cannot be checked",
                            owner.display()
                        ));
                    }
                } else {
                    graph.problems.push(format!("{}: Lua source exceeds 1 MiB scan limit; dynamic references cannot be checked", owner.display()));
                }
            }
        }
        let structured: Vec<_> = graph
            .files
            .iter()
            .filter(|path| structured(path))
            .cloned()
            .collect();
        for owner in structured {
            let bytes = files.read(&owner)?;
            if bytes.len() > 8 * 1024 * 1024 {
                graph.problems.push(format!(
                    "{}: structured resource exceeds 8 MiB scan limit",
                    owner.display()
                ));
                continue;
            }
            match serde_json::from_slice::<Value>(&bytes) {
                Ok(value) => walk(&owner, &value, "", "", &mut graph.uses, &mut graph.problems),
                Err(error) => graph
                    .problems
                    .push(format!("{}: invalid JSON: {error}", owner.display())),
            }
        }
        Ok(graph)
    }

    pub fn files(&self) -> &BTreeSet<PathBuf> {
        &self.files
    }
    pub fn references(&self) -> &[ResourceUse] {
        &self.uses
    }
    pub fn dependencies_of(&self, owner: &Path) -> Vec<&ResourceUse> {
        self.uses
            .iter()
            .filter(|reference| reference.owner == owner)
            .collect()
    }
    pub fn used_by(&self, target: &Path) -> Vec<&ResourceUse> {
        self.uses
            .iter()
            .filter(|reference| reference.target == target)
            .collect()
    }
    pub fn broken(&self) -> Vec<&ResourceUse> {
        self.uses
            .iter()
            .filter(|reference| !self.files.contains(&reference.target))
            .collect()
    }
    pub fn problems(&self) -> &[String] {
        &self.problems
    }
    pub fn usage(&self, path: &Path) -> Usage {
        if !self.used_by(path).is_empty() {
            Usage::Referenced
        } else if !self.lua_mentions(path).is_empty() {
            Usage::DynamicUncertain
        } else {
            Usage::ApparentlyUnused
        }
    }
    pub fn lua_mentions(&self, path: &Path) -> Vec<&PathBuf> {
        let portable = path.to_string_lossy().replace('\\', "/");
        self.lua_sources
            .iter()
            .filter_map(|(owner, source)| {
                if source.contains(&portable)
                    || path
                        .file_name()
                        .is_some_and(|name| source.contains(name.to_string_lossy().as_ref()))
                {
                    Some(owner)
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn transitive_dependencies(&self, owner: &Path) -> BTreeSet<PathBuf> {
        let mut result = BTreeSet::new();
        let mut pending = vec![owner.to_path_buf()];
        while let Some(next) = pending.pop() {
            for reference in self.dependencies_of(&next) {
                if result.insert(reference.target.clone()) {
                    pending.push(reference.target.clone());
                }
            }
        }
        result.remove(owner);
        result
    }
    pub fn validate_references(&self) -> Result<()> {
        let broken = self.broken();
        anyhow::ensure!(
            broken.is_empty(),
            "{} broken resource reference(s), first: {} -> {} ({})",
            broken.len(),
            broken[0].owner.display(),
            broken[0].target.display(),
            broken[0].field
        );
        Ok(())
    }
}

fn structured(path: &Path) -> bool {
    path.to_string_lossy().ends_with(".anim.json")
        || matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("scene" | "prefab" | "tmj" | "tsj")
        )
}

fn reference_key(owner: &Path, key: &str) -> bool {
    match key {
        "texture" | "image" | "audio" | "sound" | "clip" | "script" | "prefab" | "animation"
        | "shader" | "font" | "tileset" | "scene" => true,
        "source" => matches!(
            owner.extension().and_then(|value| value.to_str()),
            Some("tmj" | "tsj")
        ),
        _ => false,
    }
}

fn resource_path(raw: &str) -> bool {
    let lower = raw.to_ascii_lowercase();
    [
        ".png",
        ".jpg",
        ".jpeg",
        ".wav",
        ".ogg",
        ".lua",
        ".scene",
        ".prefab",
        ".wgsl",
        ".ttf",
        ".otf",
        ".tmj",
        ".tsj",
        ".anim.json",
    ]
    .iter()
    .any(|extension| lower.ends_with(extension))
}

fn resolve_reference(owner: &Path, raw: &str) -> Option<PathBuf> {
    if raw.contains('\\') || raw.contains(':') || !resource_path(raw) {
        return None;
    }
    let relative_owner = owner.to_string_lossy().ends_with(".anim.json")
        || matches!(
            owner.extension().and_then(|value| value.to_str()),
            Some("tmj" | "tsj")
        );
    let path = if relative_owner {
        owner.parent()?.join(raw)
    } else {
        PathBuf::from(raw)
    };
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(name) => result.push(name),
            Component::CurDir => {}
            Component::ParentDir => {
                if !result.pop() {
                    return None;
                }
            }
            _ => return None,
        }
    }
    if result.as_os_str().is_empty() {
        None
    } else {
        Some(result)
    }
}

fn walk(
    owner: &Path,
    value: &Value,
    field: &str,
    key: &str,
    uses: &mut Vec<ResourceUse>,
    problems: &mut Vec<String>,
) {
    match value {
        Value::Object(values) => {
            for (name, child) in values {
                let escaped = name.replace('~', "~0").replace('/', "~1");
                walk(
                    owner,
                    child,
                    &format!("{field}/{escaped}"),
                    name,
                    uses,
                    problems,
                );
            }
        }
        Value::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                walk(
                    owner,
                    child,
                    &format!("{field}/{index}"),
                    key,
                    uses,
                    problems,
                );
            }
        }
        Value::String(raw) if reference_key(owner, key) => {
            if let Some(target) = resolve_reference(owner, raw) {
                uses.push(ResourceUse {
                    owner: owner.to_path_buf(),
                    target,
                    field: field.to_owned(),
                    raw: raw.clone(),
                });
            } else if resource_path(raw) {
                problems.push(format!(
                    "{} {}: unsafe or invalid resource path '{raw}'",
                    owner.display(),
                    field
                ));
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_reverse_missing_recursive_and_uncertain_resources() -> Result<()> {
        let root = tempfile::tempdir()?;
        std::fs::write(
            root.path().join("main.lua"),
            "graphics.loadTexture('dynamic.png')",
        )?;
        for name in ["player.png", "dynamic.png", "unused.png"] {
            std::fs::write(root.path().join(name), b"x")?;
        }
        std::fs::write(
            root.path().join("boss.prefab"),
            r#"{"root":{"script":"boss.lua","properties":{"texture":"player.png"},"prefab":"child.prefab"}}"#,
        )?;
        std::fs::write(
            root.path().join("child.prefab"),
            r#"{"root":{"prefab":"boss.prefab"}}"#,
        )?;
        let files = ProjectFiles::open(root.path())?;
        let graph = DependencyGraph::scan(&files)?;
        assert_eq!(
            graph.used_by(Path::new("player.png"))[0].field,
            "/root/properties/texture"
        );
        assert_eq!(graph.broken().len(), 1);
        assert_eq!(graph.broken()[0].target, Path::new("boss.lua"));
        assert_eq!(graph.usage(Path::new("player.png")), Usage::Referenced);
        assert_eq!(
            graph.usage(Path::new("dynamic.png")),
            Usage::DynamicUncertain
        );
        assert_eq!(
            graph.usage(Path::new("unused.png")),
            Usage::ApparentlyUnused
        );
        assert_eq!(
            graph
                .transitive_dependencies(Path::new("boss.prefab"))
                .len(),
            3
        );
        assert!(graph.validate_references().is_err());
        Ok(())
    }

    #[test]
    fn reports_escaping_resource_paths_without_following_them() -> Result<()> {
        let root = tempfile::tempdir()?;
        std::fs::write(
            root.path().join("bad.prefab"),
            r#"{"root":{"texture":"../outside.png"}}"#,
        )?;
        let graph = DependencyGraph::scan(&ProjectFiles::open(root.path())?)?;
        assert!(graph.references().is_empty());
        assert!(graph.problems()[0].contains("unsafe or invalid resource path"));
        Ok(())
    }
}
