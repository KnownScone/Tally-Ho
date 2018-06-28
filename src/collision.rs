use ::utility::{Rect2};
use ::component::collider;

use std::collections::{VecDeque, HashSet, HashMap};
use std::cmp::{min, max};

use cgmath::{Vector2};
use specs;

const CELL_BOUND: Vector2<f32> = Vector2 { x: 1.0, y: 1.0 };

#[derive(Clone)]
pub struct Object {
    bound: collider::Bound,
    entity: specs::Entity,
}

pub struct BroadPhase {
    objects: Vec<Option<Object>>,
    free_idxs: VecDeque<usize>,
    cells: HashMap<Vector2<i32>, Cell>,

    coll_pairs: HashSet<(usize, usize)>,
}

impl BroadPhase {
    pub fn new() -> Self {
        BroadPhase {
            objects: Vec::new(),
            free_idxs: VecDeque::new(),
            cells: HashMap::new(),
            coll_pairs: HashSet::new(),
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

        for cell_x in grid_bound.min.x..grid_bound.max.x {
            for cell_y in grid_bound.min.y..grid_bound.max.y {
                let pos = Vector2::new(cell_x, cell_y);

                self.cells.entry(pos).or_insert(Cell::new())
                    .objects.push(obj_idx);

                self.update_collision_pairs(pos)
            }
        }

        obj_idx
    }

    pub fn update(&mut self, idx: usize, bound: collider::Bound) {
        let grid_bound = self.grid_bound(&bound.rect);
        let old_grid_bound = self.grid_bound(&self.objects[idx].as_ref().unwrap().bound.rect);

        // Update the bound.
        self.objects[idx].as_mut().unwrap().bound = bound;

        // No other updates needed if the new grid bound is the same as the previous grid bound.
        if grid_bound == old_grid_bound {
            return;
        }

        // Remove from old cells.
        for cell_x in old_grid_bound.min.x..old_grid_bound.max.x {
            for cell_y in old_grid_bound.min.y..old_grid_bound.max.y {
                // If the old grid bound is still using this cell, we don't need to remove it.
                if cell_x >= grid_bound.min.x && cell_x < grid_bound.max.x 
                && cell_y >= grid_bound.min.y && cell_y < grid_bound.max.y {
                    continue;
                }

                let pos = Vector2::new(cell_x, cell_y);

                {
                    let cell = self.cells.get_mut(&pos).expect("Grid cell should exist.");
                    let cell_idx = cell.objects.iter().position(|x| *x == idx)
                        .expect("Grid cell does not contain the collision object.");

                    cell.objects.swap_remove(cell_idx);
                }

                // If the cell is now empty of objects, remove it entirely.
                if self.cells.get(&pos).expect("Grid cell should exist.").objects.is_empty() {
                    self.cells.remove(&pos);
                // Otherwise, update it collision pairs.
                } else {
                    self.update_collision_pairs(pos);
                }
            }
        }

        // Add to new cells.
        for cell_x in grid_bound.min.x..grid_bound.max.x {
            for cell_y in grid_bound.min.y..grid_bound.max.y {
                let pos = Vector2::new(cell_x, cell_y);

                // If the old grid bound included this cell, we don't need to update.
                if cell_x >= old_grid_bound.min.x && cell_x < old_grid_bound.max.x 
                && cell_y >= old_grid_bound.min.y && cell_y < old_grid_bound.max.y {
                    continue;
                }

                self.cells.entry(pos)
                    .or_insert(Cell::new())
                        .objects.push(idx);
                
                self.update_collision_pairs(pos);
            }
        }
    }

    pub fn remove(&mut self, idx: usize) {
        let grid_bound = self.grid_bound(&self.objects[idx].as_ref().unwrap().bound.rect);

        // Remove from cells.
        for cell_x in grid_bound.min.x..grid_bound.max.x {
            for cell_y in grid_bound.min.y..grid_bound.max.y {
                let pos = Vector2::new(cell_x, cell_y);

                {
                    let cell = self.cells.get_mut(&pos).expect("Grid cell should exist.");
                    let cell_idx = cell.objects.iter().position(|x| *x == idx)
                        .expect("Grid cell does not contain the collision object.");

                    cell.objects.swap_remove(cell_idx);
                }

                // If the cell is now empty of objects, remove it entirely.
                if self.cells.get(&pos).expect("Grid cell should exist.").objects.is_empty() {
                    self.cells.remove(&pos);
                // Otherwise, update its collision pairs.
                } else {
                    self.update_collision_pairs(pos);
                }
            }
        }

        // Erase the object data.
        self.objects[idx] = None;
        // Open up the index for new inserts.
        self.free_idxs.push_back(idx);
    }

    fn grid_bound(&self, rect: &Rect2<f32>) -> Rect2<i32> {
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

    fn update_collision_pairs(&mut self, cell_pos: Vector2<i32>) {
        let cell = self.cells.get(&cell_pos).expect("Grid cell should exist.");

        // Brute-force collision checking
        for c_i in 0..cell.objects.len() {
            for c_j in c_i+1..cell.objects.len() {
                let idx1 = cell.objects[c_i];
                let idx2 = cell.objects[c_j];
                let obj1 = self.objects[idx1].as_ref()
                    .expect("Object should exist.");
                let obj2 = self.objects[idx2].as_ref()
                    .expect("Object should exist.");

                let pair = (min(idx1, idx2), max(idx1, idx2)); 
                if obj1.bound.rect.is_intersecting(obj2.bound.rect) {
                    self.coll_pairs.insert(pair);
                } else {
                    self.coll_pairs.remove(&pair);
                }

                // TODO: the insert and remove HashSet functions return bool indicating if the pair was already present; use these for enter/stay/exit collision tracking?
            }
        }
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

#[test]
fn collision_storage() {
    let ecs = specs::World::new();
    let mut bp = BroadPhase::new();

    let e1 = ecs.create_entity_unchecked().build();
    let obj1 = Object {
        bound: collider::Bound {
            rect: Rect2::new(
                Vector2::new(0.0, 0.0),
                Vector2::new(1.0, 1.0)
            ),
            depth: 0.0,
        },
        entity: e1,
    };

    let idx1 = bp.insert(obj1.clone());

    assert_eq!(bp.cells.len(), 1);
    assert_eq!(bp.cells.get(&Vector2::new(0, 0)).unwrap().objects, vec![0]);

    // Test update.
    bp.update(idx1, collider::Bound {
        rect: Rect2::new(
            Vector2::new(1.0, 0.0),
            Vector2::new(2.0, 2.0)
        ),
        depth: 0.0,
    });

    assert_eq!(bp.cells.len(), 2);
    assert_eq!(bp.cells.get(&Vector2::new(1, 0)).unwrap().objects, vec![0]);
    assert_eq!(bp.cells.get(&Vector2::new(1, 1)).unwrap().objects, vec![0]);

    // Test remove.
    bp.remove(idx1);

    assert_eq!(bp.cells.len(), 0);

    let idx1 = bp.insert(obj1.clone());
    let idx2 = bp.insert(obj1.clone());

    assert_eq!(bp.cells.len(), 1);
    assert_eq!(bp.cells.get(&Vector2::new(0, 0)).unwrap().objects, vec![0, 1]);

    bp.remove(idx1);

    assert_eq!(bp.cells.len(), 1);
    assert_eq!(bp.cells.get(&Vector2::new(0, 0)).unwrap().objects, vec![1]);
}

#[test]
fn collision_pairs() {
    let ecs = specs::World::new();
    let mut bp = BroadPhase::new();

    let e1 = ecs.create_entity_unchecked().build();
    let obj1 = Object {
        bound: collider::Bound {
            rect: Rect2::new(
                Vector2::new(0.25, 0.25),
                Vector2::new(0.75, 0.75)
            ),
            depth: 0.0,
        },
        entity: e1,
    };

    let e2 = ecs.create_entity_unchecked().build();
    let obj2 = Object {
        bound: collider::Bound {
            rect: Rect2::new(
                Vector2::new(0.0, 0.0),
                Vector2::new(1.0, 0.25)
            ),
            depth: 0.0,
        },
        entity: e1,
    };

    let e3 = ecs.create_entity_unchecked().build();
    let obj3 = Object {
        bound: collider::Bound {
            rect: Rect2::new(
                Vector2::new(0.75, 0.0),
                Vector2::new(1.0, 1.0)
            ),
            depth: 0.0,
        },
        entity: e1,
    };

    let idx1 = bp.insert(obj1.clone());
    let idx2 = bp.insert(obj2.clone());
    let idx3 = bp.insert(obj3.clone());

    println!("{:?}", bp.coll_pairs);

    assert_eq!(bp.coll_pairs.len(), 3);
}