//! Wire-safe inspection data. No evaluator, addresses, or Lua registry handles.
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub const MAX_INSPECTION_BYTES: usize = 24 * 1024;
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
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectSnapshot {
    pub session: u64,
    pub nodes: Vec<InspectNode>,
    pub scenes: SceneStatus,
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
                && self.scenes.stack.len() <= 16,
            "inspection limit exceeded"
        );
        for node in &self.nodes {
            node.validate()?;
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
}
