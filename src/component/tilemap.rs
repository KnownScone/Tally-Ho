use ::Vertex;
use ::utility::Rect2;
use ::script::ComponentParser;
use ::parse;

use std::cmp;
use std::sync::Arc;

use rlua::{Table, Value as LuaValue, Result as LuaResult, Error as LuaError};
use cgmath::{Point2, Point3, Vector2, Vector3, Transform};
use vulkano as vk;
use specs;

pub const STRIP_LENGTH: u32 = 10;

pub struct TileMap {
    pub instance_set: Option<Arc<vk::descriptor::DescriptorSet + Send + Sync>>,
    pub tile_dims: Vector3<f32>,
    
    // how many sub-textures in each dimension
    pub tex_dims: Vector2<u32>,

    pub image_index: u32,

    pub strips: Vec<Strip>,
}

impl TileMap {
    pub fn new(tile_dims: Vector3<f32>, tex_dims: Vector2<u32>, image_index: u32) -> TileMap {
        TileMap {
            instance_set: None,
            tile_dims,
            tex_dims,
            image_index,
            strips: Vec::new()
        }
    }

    pub fn load(&mut self, tile_map: parse::TileMap) {
        for chunk in tile_map.chunks.iter() {
            if let Some(layer) = chunk.layers.iter().find(|x| x.property == parse::LayerProperty::TileIndex) {
                for (idx, strip) in layer.strips.iter().enumerate() {
                    let strip_pos = Point3::new(
                        idx as u32 % chunk.dimensions.x,
                        (idx as f32 / chunk.dimensions.x as f32).floor() as u32,
                        chunk.pos.z
                    );

                    let data: Vec<Option<Rect2<f32>>> = strip.iter()
                        .map(|idx| {
                            let subtex_dims = Vector2::new(
                                1.0 / self.tex_dims.x as f32,
                                1.0 / self.tex_dims.y as f32
                            ); 

                            assert!(
                                (*idx as u32) < self.tex_dims.x * self.tex_dims.y, 
                                "Texture index ({}) is too large (should be less than {}).", idx, self.tex_dims.x * self.tex_dims.y
                            );

                            let pos = Point2::new(
                                (*idx as f32 % self.tex_dims.x as f32) * subtex_dims.x,
                                (*idx as f32 / self.tex_dims.x as f32).floor() * subtex_dims.y,
                            );

                            info!("{} - {:?}", idx, pos);

                            Some(Rect2::new(
                                Vector2::new(pos.x, pos.y),
                                Vector2::new(pos.x + subtex_dims.x, pos.y + subtex_dims.y)
                            ))
                        })
                    .collect();

                    let mut uvs = [None; 10];
                    uvs.copy_from_slice(&data[..]);

                    // If the strip exists, set its uvs.
                    if let Some(strip) = self.strips.iter_mut().find(|x| x.pos == strip_pos) {
                        strip.set_uvs(uvs);
                        continue;
                    }
                    
                    // If the strip doesn't exist, make a new one with these uvs.
                    self.strips.push(Strip {
                        pos: strip_pos,
                        uvs: {
                            uvs
                        },
                        vertex_buf: None,
                        index_buf: None
                    });
                }
            }
        }
    }
}

pub struct Strip {
    pos: Point3<u32>,
    uvs: [Option<Rect2<f32>>; STRIP_LENGTH as usize],
    
    pub vertex_buf: Option<Arc<vk::buffer::ImmutableBuffer<[Vertex]>>>,
    pub index_buf: Option<Arc<vk::buffer::ImmutableBuffer<[u32]>>>,
}

impl Strip {
    pub fn pos(&self) -> Point3<u32> {
        self.pos
    }

    pub fn uvs(&self) -> [Option<Rect2<f32>>; STRIP_LENGTH as usize] {
        self.uvs
    }

    pub fn set_uvs(&mut self, uvs: [Option<Rect2<f32>>; STRIP_LENGTH as usize]) {
        uvs.iter().enumerate().for_each(|(i, x)| { 
            // Only override the strip's uv if the overriding value is Some
            if let Some(_) = *x {
                self.uvs[i] = *x;
            }
        });
        self.vertex_buf = None;
        self.index_buf = None;
    }

    pub fn set_uv(&mut self, pos: usize, uv: Rect2<f32>) {
        self.uvs[pos] = Some(uv);
        self.vertex_buf = None;
        self.index_buf = None;
    }
}

impl specs::Component for TileMap {
    type Storage = specs::FlaggedStorage<Self, specs::storage::BTreeStorage<Self>>;
}

impl ComponentParser for TileMap { 
    fn parse(v: LuaValue) -> LuaResult<Self> {
        match v {
            LuaValue::Table(t) => {
                // TODO: Load parse::TileMap from this, then call the component's load function w/ it.
                let path: String = t.get("path").expect("Couldn't get tile map file path");

                let tile_dims = {
                    let t: Table = t.get("tile_dims").expect("Couldn't get tile dimensions");
                    Vector3::new(
                        t.get("x").expect("Couldn't get x-dim"), 
                        t.get("y").expect("Couldn't get y-dim"), 
                        t.get("z").expect("Couldn't get z-dim")
                    )
                };

                let tex_dims = {
                    let t: Table = t.get("tex_dims").expect("Couldn't get texture dimensions");
                    Vector2::new(
                        t.get("x").expect("Couldn't get x-dim"), 
                        t.get("y").expect("Couldn't get y-dim"), 
                    )
                };

                Ok(TileMap::new(
                    tile_dims,
                    tex_dims,
                    t.get("image_index").expect("Couldn't get image index")
                ))
            },
            LuaValue::Error(err) => Err(err),
            _ => Err(LuaError::FromLuaConversionError {
                from: "_",
                to: "table",
                message: None, 
            }),
        }
    }
}