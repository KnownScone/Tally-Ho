use ::Vertex;
use ::utility::{Rect2, Rect3};
use ::script::{ScriptResult, ScriptError, ComponentParser};
use ::parse;

use std::collections::HashMap;
use std::sync::Arc;

use rlua::{Table, Value as LuaValue, Result as LuaResult, Error as LuaError, UserData, UserDataMethods, Lua};
use cgmath::{Vector2, Vector3};
use vulkano as vk;
use specs;

pub const STRIP_LENGTH: usize = 10;

pub struct TileMap {
    tile_dims: Vector3<f32>,
    
    // How many sub-textures in each dimension.
    tex_dims: Vector2<u32>,
    image_index: u32,

    pub load: Option<parse::TileMap>
}

impl TileMap {
    pub fn new(tile_dims: Vector3<f32>, tex_dims: Vector2<u32>, image_index: u32, load: Option<parse::TileMap>) -> TileMap {
        TileMap {
            tile_dims,
            tex_dims,
            image_index,
            load,
        }
    }

    pub fn tile_dims(&self) -> Vector3<f32> {
        self.tile_dims
    }

    pub fn tex_dims(&self) -> Vector2<u32> {
        self.tex_dims
    }

    pub fn image_index(&self) -> u32 {
        self.image_index
    }
}

impl specs::Component for TileMap {
    type Storage = specs::storage::BTreeStorage<Self>;
}

pub struct RenderStrip {
    tile_map: specs::Entity,
    pos: Vector3<u32>,

    uvs: [Option<Rect2<f32>>; STRIP_LENGTH],
    
    // Vertex positions are relative to the tile map's origin (not moved by the tile map's instance set).
    pub vertex_buf: Option<Arc<vk::buffer::ImmutableBuffer<[Vertex]>>>,
    pub index_buf: Option<Arc<vk::buffer::ImmutableBuffer<[u32]>>>,
}

impl RenderStrip {
    pub fn new(
        tile_map: specs::Entity, 
        pos: Vector3<u32>, 
        uvs: [Option<Rect2<f32>>; STRIP_LENGTH],
    ) -> RenderStrip {
        RenderStrip {
            tile_map,
            pos,
            uvs,
            vertex_buf: None,
            index_buf: None,
        }
    }

    pub fn tile_map(&self) -> specs::Entity {
        self.tile_map
    }
    
    pub fn pos(&self) -> Vector3<u32> {
        self.pos
    }

    pub fn uvs(&self) -> [Option<Rect2<f32>>; STRIP_LENGTH] {
        self.uvs
    }

    pub fn set_uvs(&mut self, uvs: [Option<Rect2<f32>>; STRIP_LENGTH]) {
        self.uvs = uvs;
        self.vertex_buf = None;
        self.index_buf = None;
    }

    pub fn set_uv(&mut self, pos: usize, uv: Rect2<f32>) {
        self.uvs[pos] = Some(uv);
        self.vertex_buf = None;
        self.index_buf = None;
    }
}

impl specs::Component for RenderStrip {
    type Storage = specs::FlaggedStorage<Self, specs::storage::BTreeStorage<Self>>;
}

pub struct CollisionStrip {
    tile_map: specs::Entity,
    pos: Vector3<u32>,

    pub blocking: [bool; STRIP_LENGTH],

    pub colliders: Vec<specs::Entity>,
}

impl CollisionStrip {
    pub fn new(
        tile_map: specs::Entity, 
        pos: Vector3<u32>, 
        blocking: [bool; STRIP_LENGTH],
    ) -> CollisionStrip {
        CollisionStrip {
            tile_map,
            pos,
            blocking,
            colliders: Vec::new(),
        }
    }

    pub fn tile_map(&self) -> specs::Entity {
        self.tile_map
    }
    
    pub fn pos(&self) -> Vector3<u32> {
        self.pos
    }
}

impl specs::Component for CollisionStrip {
    type Storage = specs::FlaggedStorage<Self, specs::storage::BTreeStorage<Self>>;
}

impl ComponentParser for TileMap { 
    fn parse(v: LuaValue, _: &Lua) -> ScriptResult<Self> {
        match v {
            LuaValue::Table(t) => {
                // TODO: Load parse::TileMap from this, then call the component's load function w/ it.
                let path: String = t.get("path")?;

                let tile_dims = {
                    let t: Table = t.get("tile_dimensions")?;
                    Vector3::new(
                        t.get("x")?, 
                        t.get("y")?, 
                        t.get("z")?
                    )
                };

                let tex_dims = {
                    let t: Table = t.get("texture_dimensions")?;
                    Vector2::new(
                        t.get("x")?, 
                        t.get("y")?, 
                    )
                };

                Ok(TileMap::new(
                    tile_dims,
                    tex_dims,
                    t.get("image_index")?,
                    None
                ))
            },
            LuaValue::Error(err) => Err(ScriptError::LuaError(err)),
            _ => Err(ScriptError::LuaError(LuaError::FromLuaConversionError {
                from: "_",
                to: "table",
                message: None, 
            })),
        }
    }
}