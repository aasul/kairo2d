//! Finite orthogonal Tiled JSON maps with array-encoded tile layers.
use crate::AssetManager;
use anyhow::{bail, ensure, Context, Result};
use kairo_core::{ProjectFs, TextureHandle};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

const FLIP_H: u32 = 0x8000_0000;
const FLIP_V: u32 = 0x4000_0000;
const FLIP_D: u32 = 0x2000_0000;
const ROTATE_HEX: u32 = 0x1000_0000;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Property {
    pub name: String,
    #[serde(rename = "type", default)]
    pub kind: String,
    pub value: serde_json::Value,
}
fn properties(values: Vec<Property>) -> Result<BTreeMap<String, serde_json::Value>> {
    ensure!(values.len() <= 256, "too many custom properties");
    let mut result = BTreeMap::new();
    for property in values {
        ensure!(
            !property.name.is_empty() && property.name.len() <= 128,
            "invalid custom property name"
        );
        ensure!(
            serde_json::to_vec(&property.value)?.len() <= 65536,
            "custom property exceeds 64 KiB"
        );
        ensure!(
            result.insert(property.name, property.value).is_none(),
            "duplicate custom property"
        );
    }
    Ok(result)
}
#[derive(Clone, Debug, Deserialize)]
struct RawTile {
    id: u32,
    #[serde(default)]
    properties: Vec<Property>,
    #[serde(default)]
    animation: Vec<AnimatedFrame>,
}
#[derive(Clone, Debug, Deserialize)]
pub struct AnimatedFrame {
    pub tileid: u32,
    pub duration: u32,
}
#[derive(Clone, Debug, Serialize)]
pub struct CollisionRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rotation: f32,
}

#[derive(Clone, Debug)]
pub struct TileLayer {
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub offset: [f32; 2],
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<u32>,
    pub parallax: [f32; 2],
    pub properties: BTreeMap<String, serde_json::Value>,
}

impl TileLayer {
    pub fn tile(&self, x: u32, y: u32) -> Option<u32> {
        (x < self.width && y < self.height).then(|| self.tiles[(y * self.width + x) as usize])
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MapObject {
    pub id: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "type", alias = "class")]
    pub kind: String,
    pub x: f32,
    pub y: f32,
    #[serde(default)]
    pub width: f32,
    #[serde(default)]
    pub height: f32,
    #[serde(default)]
    pub rotation: f32,
    #[serde(default)]
    pub properties: Vec<Property>,
    #[serde(default, skip_deserializing)]
    pub layer: String,
    #[serde(default, skip_deserializing)]
    pub collision: bool,
    #[serde(default)]
    pub ellipse: bool,
    #[serde(default)]
    pub point: bool,
    #[serde(default)]
    pub polygon: Option<serde_json::Value>,
    #[serde(default)]
    pub polyline: Option<serde_json::Value>,
    #[serde(default)]
    pub gid: Option<u32>,
    #[serde(default)]
    pub text: Option<serde_json::Value>,
}

pub struct Tileset {
    pub first_gid: u32,
    pub count: u32,
    pub columns: u32,
    pub width: u32,
    pub height: u32,
    pub spacing: u32,
    pub margin: u32,
    pub texture: TextureHandle,
    pub texture_size: [u32; 2],
    pub properties: BTreeMap<u32, BTreeMap<String, serde_json::Value>>,
    pub animations: BTreeMap<u32, Vec<AnimatedFrame>>,
}

pub struct TileMap {
    pub width: u32,
    pub height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub layers: Vec<TileLayer>,
    pub objects: Vec<MapObject>,
    pub tilesets: Vec<Tileset>,
    pub properties: BTreeMap<String, serde_json::Value>,
    pub parallax_origin: [f32; 2],
}

#[derive(Deserialize)]
struct RawMap {
    orientation: String,
    #[serde(default)]
    infinite: bool,
    width: u32,
    height: u32,
    tilewidth: u32,
    tileheight: u32,
    layers: Vec<RawLayer>,
    tilesets: Vec<RawSet>,
    #[serde(default)]
    properties: Vec<Property>,
    #[serde(default)]
    parallaxoriginx: f32,
    #[serde(default)]
    parallaxoriginy: f32,
}

#[derive(Deserialize)]
struct RawLayer {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    name: String,
    #[serde(default = "yes")]
    visible: bool,
    #[serde(default = "one")]
    opacity: f32,
    #[serde(default)]
    offsetx: f32,
    #[serde(default)]
    offsety: f32,
    #[serde(default)]
    x: i32,
    #[serde(default)]
    y: i32,
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
    #[serde(default)]
    data: Option<Vec<u32>>,
    #[serde(default)]
    objects: Vec<MapObject>,
    #[serde(default)]
    properties: Vec<Property>,
    #[serde(default = "one")]
    parallaxx: f32,
    #[serde(default = "one")]
    parallaxy: f32,
}
fn yes() -> bool {
    true
}
fn one() -> f32 {
    1.0
}

#[derive(Deserialize)]
struct RawSet {
    #[serde(default)]
    firstgid: u32,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    image: Option<String>,
    #[serde(default)]
    tilewidth: u32,
    #[serde(default)]
    tileheight: u32,
    #[serde(default)]
    tilecount: u32,
    #[serde(default)]
    columns: u32,
    #[serde(default)]
    spacing: u32,
    #[serde(default)]
    margin: u32,
    #[serde(default)]
    tiles: Vec<RawTile>,
}

/// Resolve a map-relative asset reference without permitting project-root escape.
pub fn relative_asset(base_file: &str, reference: &str) -> Result<String> {
    ensure!(
        reference.len() <= 2048 && !reference.contains(['\\', ':', '\0']),
        "invalid asset reference"
    );
    ensure!(
        !Path::new(reference).is_absolute(),
        "asset reference must be relative"
    );
    let mut result = PathBuf::from(base_file)
        .parent()
        .unwrap_or(Path::new(""))
        .to_owned();
    for part in Path::new(reference).components() {
        match part {
            Component::Normal(name) => result.push(name),
            Component::CurDir => {}
            Component::ParentDir => ensure!(result.pop(), "asset reference escapes project root"),
            _ => bail!("invalid asset path"),
        }
    }
    Ok(result
        .to_str()
        .context("asset paths must be UTF-8")?
        .replace('\\', "/"))
}

impl TileMap {
    pub fn load(fs: &ProjectFs, assets: &mut AssetManager, path: &str) -> Result<Self> {
        let source = fs.read(path)?;
        ensure!(source.len() <= 16 * 1024 * 1024, "tilemap exceeds 16 MiB");
        let raw: RawMap = serde_json::from_slice(&source)
            .context("invalid Tiled JSON map; use finite maps with CSV/array data")?;
        ensure!(
            raw.orientation == "orthogonal" && !raw.infinite,
            "only finite orthogonal Tiled maps are supported"
        );
        ensure!(
            raw.width > 0
                && raw.height > 0
                && u64::from(raw.width) * u64::from(raw.height) <= 1_000_000,
            "map must contain 1..=1000000 cells per layer"
        );
        ensure!(
            (1..=2048).contains(&raw.tilewidth) && (1..=2048).contains(&raw.tileheight),
            "invalid map tile dimensions"
        );
        ensure!(
            raw.layers.len() <= 128 && raw.tilesets.len() <= 64,
            "map layer/tileset limit exceeded"
        );
        kairo_core::finite(
            "map parallax origin",
            &[raw.parallaxoriginx, raw.parallaxoriginy],
        )?;
        let map_properties = properties(raw.properties)?;
        let mut layers = Vec::new();
        let mut objects = Vec::new();
        let mut cell_count = 0;
        for layer in raw.layers {
            ensure!(layer.name.len() <= 128, "map layer name is too long");
            kairo_core::finite("layer parallax", &[layer.parallaxx, layer.parallaxy])?;
            ensure!(
                layer.parallaxx.abs() <= 100.0 && layer.parallaxy.abs() <= 100.0,
                "layer parallax exceeds supported range"
            );
            let layer_properties = properties(layer.properties)?;
            ensure!(
                layer.offsetx.is_finite()
                    && layer.offsety.is_finite()
                    && (0.0..=1.0).contains(&layer.opacity),
                "invalid map layer opacity or offset"
            );
            match layer.kind.as_str() {
                "tilelayer" => {
                    ensure!(
                        layer.width == raw.width && layer.height == raw.height,
                        "layer dimensions must match the finite map"
                    );
                    let data = layer.data.context("tile layer has no integer-array data")?;
                    ensure!(
                        data.len() as u64 == u64::from(layer.width) * u64::from(layer.height),
                        "tile layer data length mismatch"
                    );
                    cell_count += data.len();
                    ensure!(
                        cell_count <= 4_000_000,
                        "map exceeds four million tile cells"
                    );
                    // Reject unsupported flips rather than silently drawing the wrong tile.
                    ensure!(
                        data.iter().all(|gid| gid & (FLIP_D | ROTATE_HEX) == 0),
                        "diagonal/hexagonal tile flips are not supported"
                    );
                    layers.push(TileLayer {
                        name: layer.name,
                        visible: layer.visible,
                        opacity: layer.opacity,
                        offset: [
                            layer.offsetx + layer.x as f32 * raw.tilewidth as f32,
                            layer.offsety + layer.y as f32 * raw.tileheight as f32,
                        ],
                        width: layer.width,
                        height: layer.height,
                        tiles: data,
                        parallax: [layer.parallaxx, layer.parallaxy],
                        properties: layer_properties,
                    });
                }
                "objectgroup" => {
                    ensure!(
                        objects.len() + layer.objects.len() <= 10_000,
                        "map object limit exceeded"
                    );
                    for mut object in layer.objects {
                        kairo_core::finite(
                            "map object",
                            &[
                                object.x,
                                object.y,
                                object.width,
                                object.height,
                                object.rotation,
                            ],
                        )?;
                        ensure!(
                            object.width >= 0.0 && object.height >= 0.0,
                            "negative map object size"
                        );
                        properties(object.properties.clone())?;
                        object.layer = layer.name.clone();
                        object.collision = layer_properties
                            .get("collision")
                            .and_then(serde_json::Value::as_bool)
                            == Some(true)
                            || object.properties.iter().any(|p| {
                                p.name == "collision" && p.value == serde_json::Value::Bool(true)
                            });
                        object.x += layer.offsetx;
                        object.y += layer.offsety;
                        objects.push(object);
                    }
                }
                other => {
                    bail!("unsupported Tiled layer type '{other}' (flatten groups before export)")
                }
            }
        }
        let mut sets = Vec::new();
        for mut set in raw.tilesets {
            let first_gid = set.firstgid;
            let mut base = path.to_owned();
            if let Some(source) = set.source {
                base = relative_asset(path, &source)?;
                set = serde_json::from_slice(&fs.read(&base)?)
                    .context("invalid external JSON tileset (.tsj)")?;
                ensure!(
                    set.source.is_none(),
                    "nested tileset references are not supported"
                );
            }
            ensure!(
                set.tilewidth == raw.tilewidth && set.tileheight == raw.tileheight,
                "tileset and map tile sizes must match"
            );
            ensure!(
                first_gid > 0
                    && set.columns > 0
                    && set.columns <= 8192
                    && set.tilecount > 0
                    && u64::from(first_gid) + u64::from(set.tilecount) <= 0x1000_0000,
                "invalid tileset gid/columns/count"
            );
            ensure!(
                set.spacing <= 8192 && set.margin <= 8192,
                "tileset spacing/margin exceeds texture limits"
            );
            ensure!(set.tiles.len() <= 65536, "tileset metadata limit exceeded");
            let mut tile_properties = BTreeMap::new();
            let mut animations = BTreeMap::new();
            for tile in set.tiles {
                ensure!(tile.id < set.tilecount, "metadata tile ID exceeds tileset");
                ensure!(
                    tile_properties
                        .insert(tile.id, properties(tile.properties)?)
                        .is_none(),
                    "duplicate tile metadata"
                );
                ensure!(
                    tile.animation.len() <= 1024
                        && tile
                            .animation
                            .iter()
                            .all(|v| v.tileid < set.tilecount && (1..=60000).contains(&v.duration)),
                    "invalid animated tile frames/durations"
                );
                if !tile.animation.is_empty() {
                    animations.insert(tile.id, tile.animation);
                }
            }
            let image = relative_asset(
                &base,
                &set.image
                    .context("image-collection tilesets are not supported")?,
            )?;
            let texture = assets.load_texture(&image)?;
            let size = assets.texture(texture)?;
            let rows = set.tilecount.div_ceil(set.columns);
            ensure!(
                u64::from(set.margin) * 2
                    + u64::from(set.columns) * u64::from(set.tilewidth)
                    + u64::from(set.columns.saturating_sub(1)) * u64::from(set.spacing)
                    <= u64::from(size.width)
                    && u64::from(set.margin) * 2
                        + u64::from(rows) * u64::from(set.tileheight)
                        + u64::from(rows.saturating_sub(1)) * u64::from(set.spacing)
                        <= u64::from(size.height),
                "tileset rectangles exceed image bounds"
            );
            sets.push(Tileset {
                first_gid,
                count: set.tilecount,
                columns: set.columns,
                width: set.tilewidth,
                height: set.tileheight,
                spacing: set.spacing,
                margin: set.margin,
                texture,
                texture_size: [size.width, size.height],
                properties: tile_properties,
                animations,
            });
        }
        sets.sort_by_key(|set| set.first_gid);
        for pair in sets.windows(2) {
            ensure!(
                u64::from(pair[0].first_gid) + u64::from(pair[0].count)
                    <= u64::from(pair[1].first_gid),
                "overlapping tileset GIDs"
            );
        }
        let map = Self {
            width: raw.width,
            height: raw.height,
            tile_width: raw.tilewidth,
            tile_height: raw.tileheight,
            layers,
            objects,
            tilesets: sets,
            properties: map_properties,
            parallax_origin: [raw.parallaxoriginx, raw.parallaxoriginy],
        };
        for layer in &map.layers {
            for &raw in &layer.tiles {
                if raw != 0 {
                    map.tile_source(raw)?;
                }
            }
        }
        Ok(map)
    }

    pub fn tile_properties(&self, raw: u32) -> Result<BTreeMap<String, serde_json::Value>> {
        if raw == 0 {
            return Ok(BTreeMap::new());
        }
        let (set, _, _, _) = self.tile_source(raw)?;
        let local = (raw & !(FLIP_H | FLIP_V)) - set.first_gid;
        Ok(set.properties.get(&local).cloned().unwrap_or_default())
    }
    pub fn tile_source_at(&self, raw: u32, time: f64) -> Result<(&Tileset, [u32; 4], bool, bool)> {
        ensure!(
            time.is_finite() && time >= 0.0,
            "invalid tile animation time"
        );
        let (set, _, _, _) = self.tile_source(raw)?;
        let local = (raw & !(FLIP_H | FLIP_V)) - set.first_gid;
        if let Some(frames) = set.animations.get(&local) {
            let duration = frames.iter().map(|v| u64::from(v.duration)).sum::<u64>() as f64;
            let mut clock = (time * 1000.0) % duration;
            for frame in frames {
                if clock < f64::from(frame.duration) {
                    return self
                        .tile_source((set.first_gid + frame.tileid) | (raw & (FLIP_H | FLIP_V)));
                }
                clock -= f64::from(frame.duration);
            }
        }
        self.tile_source(raw)
    }
    pub fn collision_rects(&self, layer_name: Option<&str>) -> Result<Vec<CollisionRect>> {
        if let Some(name) = layer_name {
            ensure!(
                self.layers.iter().any(|v| v.name == name)
                    || self.objects.iter().any(|v| v.layer == name),
                "unknown collision layer"
            );
        }
        let mut rects = Vec::new();
        for layer in &self.layers {
            let selected = layer_name.map_or_else(
                || {
                    layer
                        .properties
                        .get("collision")
                        .and_then(serde_json::Value::as_bool)
                        == Some(true)
                },
                |name| name == layer.name,
            );
            if !selected {
                continue;
            }
            ensure!(
                layer.parallax == [1.0, 1.0],
                "collision tile layers must use parallax 1,1"
            );
            for y in 0..layer.height {
                for x in 0..layer.width {
                    if layer.tile(x, y).unwrap_or(0) != 0 {
                        ensure!(
                            rects.len() < 8192,
                            "collision import exceeds 8192 rectangles"
                        );
                        rects.push(CollisionRect {
                            x: layer.offset[0] + (x as f32 + 0.5) * self.tile_width as f32,
                            y: layer.offset[1] + (y as f32 + 0.5) * self.tile_height as f32,
                            width: self.tile_width as f32,
                            height: self.tile_height as f32,
                            rotation: 0.0,
                        });
                    }
                }
            }
        }
        for object in &self.objects {
            if !layer_name.map_or(object.collision, |name| name == object.layer) {
                continue;
            }
            ensure!(
                !object.ellipse
                    && !object.point
                    && object.polygon.is_none()
                    && object.polyline.is_none()
                    && object.gid.is_none()
                    && object.text.is_none(),
                "collision objects must be rectangles"
            );
            ensure!(
                object.width > 0.0 && object.height > 0.0 && rects.len() < 8192,
                "invalid collision rectangle or capacity exceeded"
            );
            let rotation = object.rotation.to_radians();
            let (sin, cos) = rotation.sin_cos();
            rects.push(CollisionRect {
                x: object.x + cos * object.width * 0.5 - sin * object.height * 0.5,
                y: object.y + sin * object.width * 0.5 + cos * object.height * 0.5,
                width: object.width,
                height: object.height,
                rotation,
            });
        }
        Ok(rects)
    }

    pub fn tile_source(&self, raw: u32) -> Result<(&Tileset, [u32; 4], bool, bool)> {
        let gid = raw & !(FLIP_H | FLIP_V);
        let set = self
            .tilesets
            .iter()
            .rev()
            .find(|set| gid >= set.first_gid)
            .context("tile has no tileset")?;
        let local = gid - set.first_gid;
        ensure!(local < set.count, "tile GID is outside its tileset");
        let x = set.margin + (local % set.columns) * (set.width + set.spacing);
        let y = set.margin + (local / set.columns) * (set.height + set.spacing);
        Ok((
            set,
            [x, y, set.width, set.height],
            raw & FLIP_H != 0,
            raw & FLIP_V != 0,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn relative_tilesets_may_use_a_shared_directory_but_cannot_escape() {
        assert_eq!(
            relative_asset("maps/level.json", "../tiles/terrain.tsj").unwrap(),
            "tiles/terrain.tsj"
        );
        assert!(relative_asset("map.json", "../secret.png").is_err());
        assert!(relative_asset("map.json", "C:\\secret.png").is_err());
    }
    #[test]
    fn tile_lookup_checks_both_axes() {
        let layer = TileLayer {
            name: "ground".into(),
            visible: true,
            opacity: 1.0,
            offset: [0.0; 2],
            width: 2,
            height: 1,
            parallax: [1.0, 1.0],
            properties: BTreeMap::new(),
            tiles: vec![1, 2],
        };
        assert_eq!(layer.tile(1, 0), Some(2));
        assert_eq!(layer.tile(2, 0), None);
        assert_eq!(layer.tile(0, 1), None);
    }
}

#[cfg(test)]
mod workflow_tests {
    use super::*;
    #[test]
    fn tiled_properties_animations_and_marked_rectangles_are_retained() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("tiles.png"),
            include_bytes!("../../kairo-lua/tests/fixtures/pixel.png"),
        )
        .unwrap();
        let data = serde_json::json!({"orientation":"orthogonal","width":2,"height":1,"tilewidth":1,"tileheight":1,
            "properties":[{"name":"biome","type":"string","value":"garden"}],
            "tilesets":[{"firstgid":1,"image":"tiles.png","imagewidth":2,"imageheight":2,"tilewidth":1,"tileheight":1,"tilecount":4,"columns":2,
                "tiles":[{"id":0,"properties":[{"name":"solid","type":"bool","value":true}],"animation":[{"tileid":0,"duration":100},{"tileid":1,"duration":100}]}]}],
            "layers":[{"name":"walls","type":"tilelayer","width":2,"height":1,"data":[1,0],"properties":[{"name":"collision","type":"bool","value":true}]},
                {"name":"objects","type":"objectgroup","objects":[{"id":1,"name":"door","x":4,"y":5,"width":2,"height":2,"properties":[{"name":"collision","type":"bool","value":true}]}]}]});
        std::fs::write(
            root.path().join("map.tmj"),
            serde_json::to_vec(&data).unwrap(),
        )
        .unwrap();
        let fs = ProjectFs::new(root.path()).unwrap();
        let mut assets = AssetManager::new(fs.clone());
        let map = TileMap::load(&fs, &mut assets, "map.tmj").unwrap();
        assert_eq!(map.properties["biome"], "garden");
        assert_eq!(map.tile_properties(1).unwrap()["solid"], true);
        assert_eq!(map.collision_rects(None).unwrap().len(), 2);
        assert_eq!(map.objects[0].layer, "objects");
        let first = map.tile_source_at(1, 0.0).unwrap();
        let second = map.tile_source_at(1, 0.15).unwrap();
        assert_ne!(first.1, second.1);
    }
}
