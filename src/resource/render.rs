use specs;

pub enum RenderId {
    Sprite(specs::Entity),
    TileStrip(specs::Entity)
}

#[derive(Default)]
pub struct SortedRender {
    pub ids: Vec<RenderId>,
    pub need_sort: bool,
}