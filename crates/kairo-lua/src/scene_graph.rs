//! Lua view over the validated Rust scene graph. No native graph storage escapes.
use crate::state::{lua_error, SharedState};
use kairo_core::scene_graph::{NodeFile, NodeId, NodeKind, SceneFile, SceneGraph};
use kairo_core::ProjectFs;
use mlua::{
    AnyUserData, Function, Lua, LuaSerdeExt, MultiValue, Table, UserData, UserDataFields,
    UserDataMethods, Value, Variadic,
};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

type SharedGraph = Rc<RefCell<SceneGraph>>;

struct GraphRef(
    SharedGraph,
    SharedState,
    RefCell<crate::scene_physics::ScenePhysics>,
    RefCell<crate::scene_audio::SceneAudio>,
);

#[derive(Clone)]
struct NodeRef {
    graph: SharedGraph,
    id: NodeId,
}

fn borrowed() -> mlua::Error {
    mlua::Error::RuntimeError("scene graph is already in use".into())
}

fn kind(name: &str) -> mlua::Result<NodeKind> {
    Ok(match name {
        "Node" => NodeKind::Node,
        "Node2D" => NodeKind::Node2D,
        "Sprite" => NodeKind::Sprite,
        "AnimatedSprite" => NodeKind::AnimatedSprite,
        "Camera2D" => NodeKind::Camera2D,
        "PhysicsBody2D" => NodeKind::PhysicsBody2D,
        "StaticBody2D" => NodeKind::StaticBody2D,
        "DynamicBody2D" => NodeKind::DynamicBody2D,
        "CharacterBody2D" => NodeKind::CharacterBody2D,
        "Collider2D" => NodeKind::Collider2D,
        "Area2D" => NodeKind::Area2D,
        "TileMap" => NodeKind::TileMap,
        "ParticleEmitter" => NodeKind::ParticleEmitter,
        "AudioSource" => NodeKind::AudioSource,
        "Text" => NodeKind::Text,
        "CanvasLayer" => NodeKind::CanvasLayer,
        "Control" => NodeKind::Control,
        "ScriptNode" => NodeKind::ScriptNode,
        _ => {
            return Err(mlua::Error::RuntimeError(format!(
                "unknown scene node type '{name}'"
            )))
        }
    })
}

fn vector(lua: &Lua, value: [f32; 2]) -> mlua::Result<Table> {
    let result = lua.create_table()?;
    result.set("x", value[0])?;
    result.set("y", value[1])?;
    Ok(result)
}

fn components(value: Table) -> mlua::Result<[f32; 2]> {
    Ok([value.get("x")?, value.get("y")?])
}

fn read_node<T>(
    data: AnyUserData,
    f: impl FnOnce(&kairo_core::scene_graph::Node) -> T,
) -> mlua::Result<T> {
    let this = data.borrow::<NodeRef>()?;
    let graph = this.graph.try_borrow().map_err(|_| borrowed())?;
    Ok(f(graph.node(this.id).map_err(lua_error)?))
}

fn edit_node<T>(
    data: AnyUserData,
    f: impl FnOnce(&mut SceneGraph, NodeId) -> anyhow::Result<T>,
) -> mlua::Result<T> {
    let this = data.borrow::<NodeRef>()?;
    let mut graph = this.graph.try_borrow_mut().map_err(|_| borrowed())?;
    f(&mut graph, this.id).map_err(lua_error)
}

impl UserData for GraphRef {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("name", |_, this, ()| {
            Ok(this
                .0
                .try_borrow()
                .map_err(|_| borrowed())?
                .name()
                .to_owned())
        });
        methods.add_method("_nodeCount", |_, this, ()| {
            Ok(this.0.try_borrow().map_err(|_| borrowed())?.len())
        });
        methods.add_method(
            "_runtimeContains",
            |_, this, (graph_id, file_id): (u64, u64)| {
                let graph = this.0.try_borrow().map_err(|_| borrowed())?;
                Ok(graph.graph_id() == graph_id && graph.find_by_file_id(file_id).is_some())
            },
        );
        methods.add_method(
            "_runtimePage",
            |lua, this, (offset, graph_id, file_id): (usize, Option<u64>, Option<u64>)| {
                let graph = this.0.try_borrow().map_err(|_| borrowed())?;
                let selected = graph_id
                    .zip(file_id)
                    .map(|(graph, id)| kairo_core::inspector::RuntimeNodeKey { graph, id });
                let page =
                    kairo_core::inspector::RuntimeTreePage::from_graph(&graph, offset, selected)
                        .map_err(lua_error)?;
                lua.to_value(&page)
            },
        );
        methods.add_method(
            "_runtimeEdit",
            |lua, this, (graph_id, file_id, update): (u64, u64, Value)| {
                let update: kairo_core::inspector::InspectUpdate = lua.from_value(update)?;
                let session = this
                    .1
                    .try_borrow()
                    .map_err(|_| borrowed())?
                    .inspection_session;
                let mut graph = this.0.try_borrow_mut().map_err(|_| borrowed())?;
                kairo_core::inspector::apply_runtime_edit(
                    &mut graph,
                    kairo_core::inspector::RuntimeNodeKey {
                        graph: graph_id,
                        id: file_id,
                    },
                    &update,
                    session,
                )
                .map_err(lua_error)
            },
        );
        methods.add_method("root", |_, this, ()| {
            let id = this.0.try_borrow().map_err(|_| borrowed())?.root();
            Ok(NodeRef {
                graph: this.0.clone(),
                id,
            })
        });
        methods.add_method(
            "createNode",
            |_, this, (node_type, name): (String, String)| {
                let parent = this.0.try_borrow().map_err(|_| borrowed())?.root();
                let id = this
                    .0
                    .try_borrow_mut()
                    .map_err(|_| borrowed())?
                    .create(parent, kind(&node_type)?, name)
                    .map_err(lua_error)?;
                Ok(NodeRef {
                    graph: this.0.clone(),
                    id,
                })
            },
        );
        methods.add_method("findPath", |_, this, path: String| {
            let id = this
                .0
                .try_borrow()
                .map_err(|_| borrowed())?
                .find_path(&path)
                .map_err(lua_error)?;
            Ok(id.map(|id| NodeRef {
                graph: this.0.clone(),
                id,
            }))
        });
        methods.add_method("findByTag", |_, this, tag: String| {
            let ids = this
                .0
                .try_borrow()
                .map_err(|_| borrowed())?
                .find_by_tag(&tag);
            Ok(ids
                .into_iter()
                .map(|id| NodeRef {
                    graph: this.0.clone(),
                    id,
                })
                .collect::<Vec<_>>())
        });
        methods.add_method("findByName", |_, this, name: String| {
            let ids = this
                .0
                .try_borrow()
                .map_err(|_| borrowed())?
                .find_by_name(&name);
            Ok(ids
                .into_iter()
                .map(|id| NodeRef {
                    graph: this.0.clone(),
                    id,
                })
                .collect::<Vec<_>>())
        });
        methods.add_method("findById", |_, this, file_id: u64| {
            let id = this
                .0
                .try_borrow()
                .map_err(|_| borrowed())?
                .find_by_file_id(file_id);
            Ok(id.map(|id| NodeRef {
                graph: this.0.clone(),
                id,
            }))
        });
        methods.add_method("toJson", |_, this, ()| {
            this.0
                .try_borrow()
                .map_err(|_| borrowed())?
                .to_json()
                .map_err(lua_error)
        });
        methods.add_method("drawNative", |_, this, ()| {
            let mut graph = this.0.try_borrow_mut().map_err(|_| borrowed())?;
            let mut state = this
                .1
                .try_borrow_mut()
                .map_err(|_| mlua::Error::RuntimeError("engine state is already in use".into()))?;
            crate::scene_render::draw(&mut graph, &mut state).map_err(lua_error)
        });
        methods.add_method("hitControl", |_, this, (x, y): (f32, f32)| {
            let mut graph = this.0.try_borrow_mut().map_err(|_| borrowed())?;
            let state = this.1.try_borrow().map_err(|_| borrowed())?;
            let hit = crate::scene_render::hit_control(
                &mut graph,
                state.width,
                state.height,
                glam::Vec2::new(x, y),
            )
            .map_err(lua_error)?;
            Ok(hit.map(|id| NodeRef {
                graph: this.0.clone(),
                id,
            }))
        });
        methods.add_method("syncPhysicsBefore", |_, this, ()| {
            let mut graph = this.0.try_borrow_mut().map_err(|_| borrowed())?;
            let mut state = this.1.try_borrow_mut().map_err(|_| borrowed())?;
            this.2
                .try_borrow_mut()
                .map_err(|_| borrowed())?
                .before(&mut graph, &mut state.physics)
                .map_err(lua_error)
        });
        methods.add_method("syncPhysicsAfter", |_, this, ()| {
            let mut graph = this.0.try_borrow_mut().map_err(|_| borrowed())?;
            let state = this.1.try_borrow().map_err(|_| borrowed())?;
            this.2
                .try_borrow_mut()
                .map_err(|_| borrowed())?
                .after(&mut graph, &state.physics)
                .map_err(lua_error)
        });
        methods.add_method("collisionPairs", |lua, this, ()| {
            let state = this.1.try_borrow().map_err(|_| borrowed())?;
            let pairs = this
                .2
                .try_borrow()
                .map_err(|_| borrowed())?
                .contacts(&state.physics);
            let result = lua.create_table()?;
            for (index, (a, b)) in pairs.into_iter().enumerate() {
                let pair = lua.create_table()?;
                pair.set(
                    1,
                    NodeRef {
                        graph: this.0.clone(),
                        id: a,
                    },
                )?;
                pair.set(
                    2,
                    NodeRef {
                        graph: this.0.clone(),
                        id: b,
                    },
                )?;
                result.set(index + 1, pair)?;
            }
            Ok(result)
        });
        methods.add_method("deactivatePhysics", |_, this, ()| {
            let mut state = this.1.try_borrow_mut().map_err(|_| borrowed())?;
            this.2
                .try_borrow_mut()
                .map_err(|_| borrowed())?
                .deactivate(&mut state.physics)
                .map_err(lua_error)
        });
        methods.add_method("syncAudio", |_, this, ()| {
            let graph = this.0.try_borrow().map_err(|_| borrowed())?;
            let mut state = this.1.try_borrow_mut().map_err(|_| borrowed())?;
            this.3
                .try_borrow_mut()
                .map_err(|_| borrowed())?
                .sync(&graph, &mut state.audio)
                .map_err(lua_error)
        });
        methods.add_method("deactivateAudio", |_, this, ()| {
            let mut state = this.1.try_borrow_mut().map_err(|_| borrowed())?;
            this.3
                .try_borrow_mut()
                .map_err(|_| borrowed())?
                .deactivate(&mut state.audio);
            Ok(())
        });
    }
}

impl UserData for NodeRef {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_function_get("name", |_, data| {
            read_node(data, |node| node.data().name.clone())
        });
        fields.add_field_function_set("name", |_, data, value: String| {
            edit_node(data, |graph, id| graph.set_name(id, value))
        });
        fields.add_field_function_get("nodeType", |_, data| {
            read_node(data, |node| format!("{:?}", node.data().kind))
        });
        fields.add_field_function_get("id", |_, data| read_node(data, |node| node.data().id));
        fields.add_field_function_get("position", |lua, data| {
            vector(lua, read_node(data, |node| node.data().transform.position)?)
        });
        fields.add_field_function_set("position", |_, data, value: Table| {
            let position = components(value)?;
            edit_node(data, |graph, id| {
                let mut transform = graph.node(id)?.data().transform;
                transform.position = position;
                graph.set_transform(id, transform)
            })
        });
        fields.add_field_function_get("rotation", |_, data| {
            read_node(data, |node| node.data().transform.rotation)
        });
        fields.add_field_function_set("rotation", |_, data, value: f32| {
            edit_node(data, |graph, id| {
                let mut transform = graph.node(id)?.data().transform;
                transform.rotation = value;
                graph.set_transform(id, transform)
            })
        });
        fields.add_field_function_get("scale", |lua, data| {
            vector(lua, read_node(data, |node| node.data().transform.scale)?)
        });
        fields.add_field_function_set("scale", |_, data, value: Table| {
            let scale = components(value)?;
            edit_node(data, |graph, id| {
                let mut transform = graph.node(id)?.data().transform;
                transform.scale = scale;
                graph.set_transform(id, transform)
            })
        });
        fields.add_field_function_get("pivot", |lua, data| {
            vector(lua, read_node(data, |node| node.data().transform.pivot)?)
        });
        fields.add_field_function_set("pivot", |_, data, value: Table| {
            let pivot = components(value)?;
            edit_node(data, |graph, id| {
                let mut transform = graph.node(id)?.data().transform;
                transform.pivot = pivot;
                graph.set_transform(id, transform)
            })
        });
        fields.add_field_function_get("globalPosition", |lua, data| {
            let this = data.borrow::<Self>()?;
            let value = this
                .graph
                .try_borrow_mut()
                .map_err(|_| borrowed())?
                .world_position(this.id)
                .map_err(lua_error)?;
            vector(lua, value.to_array())
        });
        fields.add_field_function_get("worldTransform", |lua, data| {
            let this = data.borrow::<Self>()?;
            let matrix = this
                .graph
                .try_borrow_mut()
                .map_err(|_| borrowed())?
                .world_matrix(this.id)
                .map_err(lua_error)?;
            let result = lua.create_table()?;
            result.set("a", matrix.x_axis.x)?;
            result.set("b", matrix.x_axis.y)?;
            result.set("c", matrix.y_axis.x)?;
            result.set("d", matrix.y_axis.y)?;
            result.set("tx", matrix.z_axis.x)?;
            result.set("ty", matrix.z_axis.y)?;
            Ok(result)
        });
        fields.add_field_function_get("enabled", |_, data| {
            read_node(data, |node| node.data().enabled)
        });
        fields.add_field_function_set("enabled", |_, data, value: bool| {
            edit_node(data, |graph, id| graph.set_enabled(id, value))
        });
        fields.add_field_function_get("visible", |_, data| {
            read_node(data, |node| node.data().visible)
        });
        fields.add_field_function_set("visible", |_, data, value: bool| {
            edit_node(data, |graph, id| graph.set_visible(id, value))
        });
        fields.add_field_function_get("script", |_, data| {
            read_node(data, |node| node.data().script.clone())
        });
        fields.add_field_function_set("script", |_, data, value: Option<String>| {
            edit_node(data, |graph, id| graph.set_script(id, value))
        });
        fields.add_field_function_get("prefab", |_, data| {
            read_node(data, |node| node.data().prefab.clone())
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("isAlive", |_, this, ()| {
            Ok(this
                .graph
                .try_borrow()
                .map_err(|_| borrowed())?
                .node(this.id)
                .is_ok())
        });
        methods.add_method("_signalKey", |_, this, ()| {
            this.graph
                .try_borrow()
                .map_err(|_| borrowed())?
                .node(this.id)
                .map_err(lua_error)?;
            Ok(format!("{:?}", this.id))
        });
        methods.add_method("on", |lua, this, (event, callback): (String, Function)| {
            this.graph
                .try_borrow()
                .map_err(|_| borrowed())?
                .node(this.id)
                .map_err(lua_error)?;
            let events: Table = lua.globals().get("Events")?;
            let on: Function = events.get("_onNode")?;
            on.call::<u64>((format!("{:?}", this.id), event, callback))
        });
        methods.add_method("off", |lua, this, token: u64| {
            this.graph
                .try_borrow()
                .map_err(|_| borrowed())?
                .node(this.id)
                .map_err(lua_error)?;
            let events: Table = lua.globals().get("Events")?;
            let off: Function = events.get("off")?;
            off.call::<bool>(token)
        });
        methods.add_method(
            "emit",
            |lua, this, (event, args): (String, Variadic<Value>)| {
                this.graph
                    .try_borrow()
                    .map_err(|_| borrowed())?
                    .node(this.id)
                    .map_err(lua_error)?;
                let events: Table = lua.globals().get("Events")?;
                let emit: Function = events.get("_emitNode")?;
                let mut values = vec![
                    Value::String(lua.create_string(format!("{:?}", this.id))?),
                    Value::String(lua.create_string(event)?),
                ];
                values.extend(args);
                emit.call::<()>(MultiValue::from_vec(values))
            },
        );
        methods.add_method(
            "createChild",
            |_, this, (node_type, name): (String, String)| {
                let id = this
                    .graph
                    .try_borrow_mut()
                    .map_err(|_| borrowed())?
                    .create(this.id, kind(&node_type)?, name)
                    .map_err(lua_error)?;
                Ok(NodeRef {
                    graph: this.graph.clone(),
                    id,
                })
            },
        );
        methods.add_method("children", |_, this, ()| {
            let ids = this
                .graph
                .try_borrow()
                .map_err(|_| borrowed())?
                .node(this.id)
                .map_err(lua_error)?
                .children()
                .to_vec();
            Ok(ids
                .into_iter()
                .map(|id| NodeRef {
                    graph: this.graph.clone(),
                    id,
                })
                .collect::<Vec<_>>())
        });
        methods.add_method("parent", |_, this, ()| {
            let id = this
                .graph
                .try_borrow()
                .map_err(|_| borrowed())?
                .node(this.id)
                .map_err(lua_error)?
                .parent();
            Ok(id.map(|id| NodeRef {
                graph: this.graph.clone(),
                id,
            }))
        });
        methods.add_method("findChild", |_, this, name: String| {
            let id = this
                .graph
                .try_borrow()
                .map_err(|_| borrowed())?
                .find_child(this.id, &name)
                .map_err(lua_error)?;
            Ok(id.map(|id| NodeRef {
                graph: this.graph.clone(),
                id,
            }))
        });
        methods.add_method("find", |_, this, name: String| {
            let id = this
                .graph
                .try_borrow()
                .map_err(|_| borrowed())?
                .find_recursive(this.id, &name)
                .map_err(lua_error)?;
            Ok(id.map(|id| NodeRef {
                graph: this.graph.clone(),
                id,
            }))
        });
        methods.add_method("path", |_, this, ()| {
            this.graph
                .try_borrow()
                .map_err(|_| borrowed())?
                .path(this.id)
                .map_err(lua_error)
        });
        methods.add_method("addTag", |_, this, tag: String| {
            this.graph
                .try_borrow_mut()
                .map_err(|_| borrowed())?
                .add_tag(this.id, tag)
                .map_err(lua_error)
        });
        methods.add_method("hasTag", |_, this, tag: String| {
            Ok(this
                .graph
                .try_borrow()
                .map_err(|_| borrowed())?
                .node(this.id)
                .map_err(lua_error)?
                .data()
                .tags
                .contains(&tag))
        });
        methods.add_method("removeTag", |_, this, tag: String| {
            this.graph
                .try_borrow_mut()
                .map_err(|_| borrowed())?
                .remove_tag(this.id, &tag)
                .map_err(lua_error)
        });
        methods.add_method(
            "setProperty",
            |lua, this, (name, value): (String, Value)| {
                let value = lua.from_value(value)?;
                this.graph
                    .try_borrow_mut()
                    .map_err(|_| borrowed())?
                    .set_property(this.id, &name, value)
                    .map_err(lua_error)
            },
        );
        methods.add_method("getProperty", |lua, this, name: String| {
            let graph = this.graph.try_borrow().map_err(|_| borrowed())?;
            let value = graph
                .node(this.id)
                .map_err(lua_error)?
                .data()
                .properties
                .get(&name);
            match value {
                Some(value) => lua.to_value(value),
                None => Ok(Value::Nil),
            }
        });
        methods.add_method("removeProperty", |_, this, name: String| {
            this.graph
                .try_borrow_mut()
                .map_err(|_| borrowed())?
                .remove_property(this.id, &name)
                .map_err(lua_error)
        });
        methods.add_method(
            "exposeInspector",
            |lua, this, (path, metadata): (String, Value)| {
                let metadata: kairo_core::inspector::InspectMetadata = lua.from_value(metadata)?;
                this.graph
                    .try_borrow_mut()
                    .map_err(|_| borrowed())?
                    .expose_inspector_field(this.id, &path, metadata)
                    .map_err(lua_error)
            },
        );
        methods.add_method(
            "reparent",
            |_, this, (parent, index): (AnyUserData, Option<usize>)| {
                let parent = parent.borrow::<Self>()?;
                this.graph
                    .try_borrow_mut()
                    .map_err(|_| borrowed())?
                    .reparent(
                        this.id,
                        parent.id,
                        index.unwrap_or(usize::MAX).saturating_sub(1),
                    )
                    .map_err(lua_error)
            },
        );
        methods.add_method("addChild", |_, this, child: AnyUserData| {
            let child = child.borrow::<Self>()?;
            this.graph
                .try_borrow_mut()
                .map_err(|_| borrowed())?
                .reparent(child.id, this.id, usize::MAX)
                .map_err(lua_error)
        });
        methods.add_method(
            "insertChild",
            |_, this, (child, index): (AnyUserData, usize)| {
                let child = child.borrow::<Self>()?;
                this.graph
                    .try_borrow_mut()
                    .map_err(|_| borrowed())?
                    .reparent(child.id, this.id, index.saturating_sub(1))
                    .map_err(lua_error)
            },
        );
        methods.add_method("removeChild", |_, this, child: AnyUserData| {
            let child = child.borrow::<Self>()?;
            let mut graph = this.graph.try_borrow_mut().map_err(|_| borrowed())?;
            if this.id == graph.root() {
                return Err(mlua::Error::RuntimeError(
                    "root children cannot be detached".into(),
                ));
            }
            if graph.node(child.id).map_err(lua_error)?.parent() != Some(this.id) {
                return Err(mlua::Error::RuntimeError(
                    "node is not a direct child".into(),
                ));
            }
            let root = graph.root();
            graph
                .reparent(child.id, root, usize::MAX)
                .map_err(lua_error)
        });
        methods.add_method("reorder", |_, this, index: usize| {
            this.graph
                .try_borrow_mut()
                .map_err(|_| borrowed())?
                .reorder(this.id, index.saturating_sub(1))
                .map_err(lua_error)
        });
        methods.add_method("duplicate", |_, this, ()| {
            let mut graph = this.graph.try_borrow_mut().map_err(|_| borrowed())?;
            let parent = graph
                .node(this.id)
                .map_err(lua_error)?
                .parent()
                .unwrap_or(graph.root());
            let id = graph.duplicate(this.id, parent).map_err(lua_error)?;
            Ok(NodeRef {
                graph: this.graph.clone(),
                id,
            })
        });
        methods.add_method("destroy", |lua, this, ()| {
            let mut graph = this.graph.try_borrow_mut().map_err(|_| borrowed())?;
            let mut pending = vec![this.id];
            let mut keys = Vec::new();
            while let Some(id) = pending.pop() {
                let node = graph.node(id).map_err(lua_error)?;
                pending.extend_from_slice(node.children());
                keys.push(format!("{id:?}"));
            }
            graph.destroy(this.id).map_err(lua_error)?;
            drop(graph);
            let events: Table = lua.globals().get("Events")?;
            let cleanup: Function = events.get("_destroy")?;
            cleanup.call::<()>(keys)
        });
    }
}

pub(crate) fn register(lua: &Lua, fs: ProjectFs, state: &SharedState) -> mlua::Result<()> {
    let module = lua.create_table()?;
    let new_state = state.clone();
    module.set(
        "new",
        lua.create_function(move |_, name: String| {
            Ok(GraphRef(
                Rc::new(RefCell::new(SceneGraph::new(name).map_err(lua_error)?)),
                new_state.clone(),
                RefCell::new(crate::scene_physics::ScenePhysics::default()),
                RefCell::new(crate::scene_audio::SceneAudio::default()),
            ))
        })?,
    )?;
    let json_state = state.clone();
    module.set(
        "fromJson",
        lua.create_function(move |_, source: String| {
            Ok(GraphRef(
                Rc::new(RefCell::new(
                    SceneGraph::from_json(&source).map_err(lua_error)?,
                )),
                json_state.clone(),
                RefCell::new(crate::scene_physics::ScenePhysics::default()),
                RefCell::new(crate::scene_audio::SceneAudio::default()),
            ))
        })?,
    )?;
    let load_fs = fs.clone();
    let load_state = state.clone();
    module.set(
        "load",
        lua.create_function(move |_, path: String| {
            if !path.ends_with(".scene") {
                return Err(mlua::Error::RuntimeError(
                    "scene path must end in .scene".into(),
                ));
            }
            let source = load_fs.read_text(&path).map_err(lua_error)?;
            Ok(GraphRef(
                Rc::new(RefCell::new(
                    SceneGraph::from_json(&source).map_err(lua_error)?,
                )),
                load_state.clone(),
                RefCell::new(crate::scene_physics::ScenePhysics::default()),
                RefCell::new(crate::scene_audio::SceneAudio::default()),
            ))
        })?,
    )?;
    module.set(
        "instantiatePrefab",
        lua.create_function(
            move |lua, (path, parent, overrides): (String, AnyUserData, Option<Value>)| {
                let mut stack = Vec::new();
                let mut template = load_prefab(&fs, &path, &mut stack).map_err(lua_error)?;
                if let Some(overrides) = overrides {
                    let properties: serde_json::Map<String, serde_json::Value> =
                        lua.from_value(overrides)?;
                    template.properties.extend(properties);
                }
                let parent = parent.borrow::<NodeRef>()?;
                let id = parent
                    .graph
                    .try_borrow_mut()
                    .map_err(|_| borrowed())?
                    .instantiate_subtree(parent.id, &template)
                    .map_err(lua_error)?;
                Ok(NodeRef {
                    graph: parent.graph.clone(),
                    id,
                })
            },
        )?,
    )?;
    lua.globals().set("sceneGraph", module)
}

fn load_prefab(fs: &ProjectFs, path: &str, stack: &mut Vec<PathBuf>) -> anyhow::Result<NodeFile> {
    anyhow::ensure!(
        path.starts_with("prefabs/") && path.ends_with(".prefab"),
        "prefab path must be a project-relative prefabs/*.prefab file"
    );
    let resolved = fs.resolve(path)?;
    anyhow::ensure!(
        !stack.contains(&resolved),
        "recursive prefab dependency: {} -> {}",
        stack
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(" -> "),
        resolved.display()
    );
    anyhow::ensure!(stack.len() < 32, "nested prefab depth exceeds 32");
    stack.push(resolved);
    let result: anyhow::Result<NodeFile> = (|| {
        let source = fs.read_text(path)?;
        anyhow::ensure!(source.len() <= 1024 * 1024, "prefab file exceeds 1 MiB");
        let file: SceneFile = serde_json::from_str(&source)?;
        SceneGraph::from_file(file.clone())?;
        let mut root = expand_prefab_refs(fs, file.root, stack)?;
        root.prefab = Some(path.to_owned());
        Ok(root)
    })();
    stack.pop();
    result
}

fn expand_prefab_refs(
    fs: &ProjectFs,
    mut node: NodeFile,
    stack: &mut Vec<PathBuf>,
) -> anyhow::Result<NodeFile> {
    if let Some(path) = node.prefab.take() {
        let mut nested = load_prefab(fs, &path, stack)?;
        nested.name = node.name;
        nested.transform = node.transform;
        nested.enabled = node.enabled;
        nested.visible = node.visible;
        nested.tags.extend(node.tags);
        nested.metadata.extend(node.metadata);
        nested.properties.extend(node.properties);
        if node.script.is_some() {
            nested.script = node.script;
        }
        nested.children.extend(node.children);
        node = nested;
    }
    node.children = node
        .children
        .into_iter()
        .map(|child| expand_prefab_refs(fs, child, stack))
        .collect::<anyhow::Result<_>>()?;
    Ok(node)
}
