use ::component::tilemap::STRIP_LENGTH;

use std::str::from_utf8;

use num_traits::FromPrimitive;
use cgmath::{Vector3, Vector2};
use nom::*;

#[derive(Debug)]
pub struct TileMap {
    pub name: String,
    pub chunks: Vec<Chunk>
}

#[derive(Debug)]
pub struct Chunk {
    pub pos: Vector3<u32>,
    pub dimensions: Vector2<u32>,
    pub layers: Vec<Layer>
}

#[derive(Debug, PartialEq, FromPrimitive)]
pub enum LayerProperty {
    TileIndex = 0,
    Blocking,
}

#[derive(Debug)]
pub struct Layer {
    pub property: LayerProperty,
    pub strips: Vec<[u16; STRIP_LENGTH as usize]>
}

named!(pub tile_map<TileMap>, do_parse!(
    _header_length: be_u8 >>
    name: map_res!(length_bytes!(be_u8), from_utf8) >>
    chunks: many0!(complete!(chunk)) >>
    (TileMap { name: String::from(name), chunks })
));

named!(pub chunk<Chunk>, do_parse!(
    _header_length: be_u8 >>
    pos: map!(count_fixed!(u32, be_u32, 3), Vector3::from) >>
    dimensions: map!(count_fixed!(u32, be_u32, 2), Vector2::from) >>
    layers: length_count!(be_u8, call!(layer, dimensions.x * dimensions.y)) >>
    (Chunk { pos, dimensions, layers })
));

named_args!(pub layer(strips_length: u32)<Layer>, do_parse!(
    property: map_opt!(be_u8, FromPrimitive::from_u8) >>
    strips: count!(strip, strips_length as usize) >>
    (Layer { property, strips })
));

named!(pub strip<[u16; STRIP_LENGTH as usize]>, count_fixed!(u16, be_u16, STRIP_LENGTH as usize));

#[test]
fn parse() {
    let tile_map = tile_map(b"\x05\x04dust\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01\x00\x00\x00\x01\x01\x00\
    \x00\x00\x00\x01\x00\x00\x00\x01\x00\x01\x00\x01\x00\x01\x00\x01\x00\x01\x00\x01").unwrap().1;
    assert_eq!(
        tile_map.name.as_str(),
        "dust"
    );
    assert_eq!(
        tile_map.chunks[0].pos,
        Vector3::new(0, 0, 0)
    );
    assert_eq!(
        tile_map.chunks[0].dimensions,
        Vector2::new(1, 1)
    );

    assert_eq!(
        tile_map.chunks[0].layers[0].property,
        LayerProperty::TileIndex
    );
    assert_eq!(
        tile_map.chunks[0].layers[0].strips[0],
        [0, 1, 0, 1, 1, 1, 1, 1, 1, 1]
    );
}