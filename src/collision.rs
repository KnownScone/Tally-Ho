use ::utility::{Rect2, Rect3};
use ::component::collider;

use std::collections::{VecDeque, HashSet, HashMap};
use std::cmp::{min, max};
use std::iter::Map;

use cgmath::{Vector2, Vector3, Zero};
use specs;

const CELL_BOUND: Vector2<f32> = Vector2 { x: 0.1, y: 0.1 };

#[derive(Clone, Debug)]
pub struct Object {
    pub bound: collider::Bound,
    pub entity: specs::Entity,
}

pub struct BroadPhase {
    objects: Vec<Option<Object>>,
    free_idxs: VecDeque<usize>,
    cells: HashMap<Vector2<i32>, Cell>,
    active_cells: HashSet<Vector2<i32>>,
}

impl BroadPhase {
    pub fn new() -> Self {
        BroadPhase {
            objects: Vec::new(),
            free_idxs: VecDeque::new(),
            cells: HashMap::new(),
            active_cells: HashSet::new(),
        }
    }

    pub fn insert(&mut self, obj: Object) -> usize {
        let grid_bound = self.grid_bound(&obj.bound.rect);

        let obj_idx =
            if let Some(idx) = self.free_idxs.pop_front() {
                // If there's a free index, insert the object there.
                self.objects[idx] = Some(obj);
                idx
            } else {
                // If there's no free indices, allocate more space for the object.
                self.objects.push(Some(obj));
                self.objects.len() - 1
            };

        // Insert the object into each of its cells.
        for cell_x in grid_bound.min.x..grid_bound.max.x {
            for cell_y in grid_bound.min.y..grid_bound.max.y {
                let pos = Vector2::new(cell_x, cell_y);

                self.cells.entry(pos).or_insert(Cell::new())
                    .objects.push(obj_idx);

                // Make sure we update the collisions in this cell.
                self.active_cells.insert(pos);
            }
        }

        obj_idx
    }

    pub fn update(&mut self, idx: usize, bound: collider::Bound) {
        let grid_bound = self.grid_bound(&bound.rect);
        let old_grid_bound = self.grid_bound(&self.objects[idx].as_ref().unwrap().bound.rect);

        // Update the bound.
        self.objects[idx].as_mut().unwrap().bound = bound;

        // Remove the object from the cells it has left.
        // * We don't need to update collisions in old cells because the collision pairs from the last tick have already been drained.
        for cell_x in old_grid_bound.min.x..old_grid_bound.max.x {
            for cell_y in old_grid_bound.min.y..old_grid_bound.max.y {
                let pos = Vector2::new(cell_x, cell_y);

                // If the new grid bound is still using this cell, we don't need to remove it.
                if cell_x >= grid_bound.min.x && cell_x < grid_bound.max.x 
                && cell_y >= grid_bound.min.y && cell_y < grid_bound.max.y {
                    continue;
                }

                let cell = self.cells.get_mut(&pos).expect("Grid cell should exist");
                let cell_idx = cell.objects.iter().position(|x| *x == idx)
                    .expect("Grid cell does not contain the collision object");

                cell.objects.swap_remove(cell_idx);
            }
        }

        // Add the object to the new cells (and update their collisions).
        for cell_x in grid_bound.min.x..grid_bound.max.x {
            for cell_y in grid_bound.min.y..grid_bound.max.y {
                let pos = Vector2::new(cell_x, cell_y);

                // If the old grid bound included this cell, we don't need to insert.
                if cell_x >= old_grid_bound.min.x && cell_x < old_grid_bound.max.x 
                && cell_y >= old_grid_bound.min.y && cell_y < old_grid_bound.max.y {
                    // but we do need to be sure we update the collision in this cell first...
                    self.active_cells.insert(pos);
                    continue;
                }

                self.cells.entry(pos)
                    .or_insert(Cell::new())
                        .objects.push(idx);

                self.active_cells.insert(pos);
            }
        }
    }

    pub fn remove(&mut self, idx: usize) {
        let grid_bound = self.grid_bound(&self.objects[idx].as_ref().unwrap().bound.rect);

        // Lazy hack: set the rect so it doesn't collide with anything (takes up no space) before updating collision pairs.
        self.objects[idx].as_mut().unwrap().bound.rect = Rect3::new(Vector3::zero(), Vector3::zero());

        // Remove from cells.
        for cell_x in grid_bound.min.x..grid_bound.max.x {
            for cell_y in grid_bound.min.y..grid_bound.max.y {
                let pos = Vector2::new(cell_x, cell_y);

                let cell = self.cells.get_mut(&pos).expect("Grid cell should exist");
                let cell_idx = cell.objects.iter().position(|x| *x == idx)
                    .expect("Grid cell does not contain the collision object");

                cell.objects.swap_remove(cell_idx);
            }
        }

        // Erase the object data.
        self.objects[idx] = None;
        // Open up the index for new inserts.
        self.free_idxs.push_back(idx);
    }

    // Turns a bound in world space to a bound that encompasses it in grid space. 
    fn grid_bound(&self, rect: &Rect3<f32>) -> Rect2<i32> {
        let min = Vector2::new(
            (rect.min.x / CELL_BOUND.x).floor() as i32,
            (rect.min.y / CELL_BOUND.y).floor() as i32
        );

        let max = Vector2::new(
            (rect.max.x / CELL_BOUND.x).ceil() as i32,
            (rect.max.y / CELL_BOUND.y).ceil() as i32
        );

        Rect2::new(
            min,
            max
        )
    }

    pub fn for_each<F>(&mut self, func: F)
    where
        F: FnMut((specs::Entity, specs::Entity))
    {
        let mut coll_pairs: HashSet<(usize, usize)> = HashSet::new();
        
        // For each cell that has recently been edited.
        for cell_pos in self.active_cells.drain() {
            if let Some(cell) = self.cells.get(&cell_pos) {

                // Brute-force collision checking; intersection check on each pair.
                for c_i in 0..cell.objects.len() {
                    for c_j in c_i+1..cell.objects.len() {
                        let idx1 = cell.objects[c_i];
                        let idx2 = cell.objects[c_j];
                        let obj1 = self.objects[idx1].as_ref()
                            .expect("Object should exist");
                        let obj2 = self.objects[idx2].as_ref()
                            .expect("Object should exist");

                        let pair = (min(idx1, idx2), max(idx1, idx2)); 
                        if obj1.bound.rect.is_intersecting(obj2.bound.rect) {
                            coll_pairs.insert(pair);
                        }
                    }
                }
            }
        }

        // The collision pairs detected are turned into pairs of entities and we the call the function on each of them.
        let objects = &self.objects;
        coll_pairs.drain()
            .filter_map(|(i1, i2)| {
                let e1 = objects[i1].as_ref().map(|x| x.entity);
                let e2 = objects[i2].as_ref().map(|x| x.entity);
                if let Some(e1) = e1 { if let Some(e2) = e2 {
                    return Some((e1, e2));
                }}

                None
            })
            .for_each(func);
    }
}

struct Cell {
    objects: Vec<usize>,
}

impl Cell {
    pub fn new() -> Self {
        Cell {
            objects: Vec::new(),
        }
    }
}

use specs::Builder;

#[test]
fn collision_pairs() {
    let ecs = specs::World::new();
    let mut bp = BroadPhase::new();

    let e1 = ecs.create_entity_unchecked().build();
    let obj1 = Object {
        bound: collider::Bound {
            rect: Rect3::new(
                Vector3::new(0.0, 0.0, 0.0),
                Vector3::new(0.75, 0.75, 1.0)
            ),
        },
        entity: e1,
    };

    let e2 = ecs.create_entity_unchecked().build();
    let obj2 = Object {
        bound: collider::Bound {
            rect: Rect3::new(
                Vector3::new(0.74, 0.0, 0.0),
                Vector3::new(1.0, 1.0, 1.0)
            ),
        },
        entity: e1,
    };

    let e3 = ecs.create_entity_unchecked().build();
    let obj3 = Object {
        bound: collider::Bound {
            rect: Rect3::new(
                Vector3::new(0.0, 0.74, 0.0),
                Vector3::new(1.0, 1.0, 1.0)
            ),
        },
        entity: e1,
    };

    let idx1 = bp.insert(obj1.clone());
    let idx2 = bp.insert(obj2.clone());
    let idx3 = bp.insert(obj3.clone());

    let mut len = 0;
    bp.for_each(|x| {
        len+=1;
    });
    assert_eq!(len, 3);

    bp.remove(idx1);
    bp.update(idx2, obj2.clone().bound);
    bp.update(idx3, obj3.clone().bound);
    let mut len = 0;
    bp.for_each(|x| {
        len+=1;
    });
    assert_eq!(len, 1);
}