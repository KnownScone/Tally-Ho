use ::utility::{Rect2, Rect3};
use ::component::collider;

use std::collections::hash_map::{HashMap, Entry};
use std::collections::VecDeque;
use std::cmp::{min, max};

use cgmath::{Vector2, Vector3, Zero};
use dmsort;
use specs;

const CELL_BOUND: Vector2<f32> = Vector2 { x: 1.0, y: 1.0 };

#[derive(Clone)]
pub struct CollisionObject {
    bound: collider::Bound,
    entity: specs::Entity,
}

pub struct CollisionWorld {
    objects: Vec<Option<CollisionObject>>,
    free_idxs: VecDeque<usize>,
    cells: HashMap<Vector2<i32>, Cell>,
    // TODO: Physical response abstraction
    // TODO: Event system
}

impl CollisionWorld {
    pub fn new() -> Self {
        CollisionWorld {
            objects: Vec::new(),
            free_idxs: VecDeque::new(),
            cells: HashMap::new(),
        }
    }

    pub fn insert(&mut self, obj: CollisionObject) -> usize {
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
                self.cells.entry(Vector2::new(cell_x, cell_y))
                    .or_insert(Cell::new())
                        .objects.push(obj_idx);
            }
        }

        obj_idx
    }

    pub fn update(&mut self, idx: usize, bound: collider::Bound) {
        let (grid_bound, old_grid_bound) = {
            let obj = self.objects[idx].as_ref().unwrap();
            (
                self.grid_bound(&bound.rect),
                self.grid_bound(&self.objects[idx].as_ref().unwrap().bound.rect)
            )
        };

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

                let key = Vector2::new(cell_x, cell_y);

                {
                    let cell = self.cells.get_mut(&key).expect("Grid cell should exist.");
                    let cell_idx = cell.objects.iter().position(|x| *x == idx)
                        .expect("Grid cell does not contain the collision object.");

                    cell.objects.swap_remove(cell_idx);
                }

                if self.cells.get(&key).expect("Grid cell should exist.").objects.is_empty() {
                    self.cells.remove(&key);
                }
            }
        }

        // Add to new cells.
        for cell_x in grid_bound.min.x..grid_bound.max.x {
            for cell_y in grid_bound.min.y..grid_bound.max.y {
                // If the old grid bound included this cell, we don't need to update.
                if cell_x >= old_grid_bound.min.x && cell_x < old_grid_bound.max.x 
                && cell_y >= old_grid_bound.min.y && cell_y < old_grid_bound.max.y {
                    continue;
                }

                self.cells.entry(Vector2::new(cell_x, cell_y))
                    .or_insert(Cell::new())
                        .objects.push(idx);
            }
        }
    }

    pub fn remove(&mut self, idx: usize) {
        let grid_bound = self.grid_bound(&self.objects[idx].as_ref().unwrap().bound.rect);

        // Remove from cells.
        for cell_x in grid_bound.min.x..grid_bound.max.x {
            for cell_y in grid_bound.min.y..grid_bound.max.y {
                let key = Vector2::new(cell_x, cell_y);

                {
                    let cell = self.cells.get_mut(&key).expect("Grid cell should exist.");
                    let cell_idx = cell.objects.iter().position(|x| *x == idx)
                        .expect("Grid cell does not contain the collision object.");

                    cell.objects.swap_remove(cell_idx);
                }

                if self.cells.get(&key).expect("Grid cell should exist.").objects.is_empty() {
                    self.cells.remove(&key);
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

        println!("{:?}", rect);
        println!("{:?}, {:?}", min, max);

        Rect2::new(
            min,
            max
        )
    }
}

struct Cell {
    objects: Vec<usize>
}

impl Cell {
    pub fn new() -> Self {
        Cell {
            objects: Vec::new()
        }
    }
}

#[test]
fn collision() {
    let ecs = specs::World::new();
    let mut world = CollisionWorld::new();

    let e1 = ecs.create_entity_unchecked().build();
    let obj1 = CollisionObject {
        bound: collider::Bound {
            rect: Rect2::new(
                Vector2::new(0.0, 0.0),
                Vector2::new(1.0, 1.0)
            ),
            depth: 0.0,
        },
        entity: e1,
    };

    let idx1 = world.insert(obj1.clone());

    assert_eq!(world.cells.len(), 1);
    assert_eq!(world.cells.get(&Vector2::new(0, 0)).unwrap().objects, vec![0]);

    // Test update.
    world.update(idx1, collider::Bound {
        rect: Rect2::new(
            Vector2::new(1.0, 0.0),
            Vector2::new(2.0, 2.0)
        ),
        depth: 0.0,
    });

    assert_eq!(world.cells.len(), 2);
    assert_eq!(world.cells.get(&Vector2::new(1, 0)).unwrap().objects, vec![0]);
    assert_eq!(world.cells.get(&Vector2::new(1, 1)).unwrap().objects, vec![0]);

    // Test remove.
    world.remove(idx1);

    assert_eq!(world.cells.len(), 0);

    let idx1 = world.insert(obj1.clone());
    let idx2 = world.insert(obj1.clone());

    assert_eq!(world.cells.len(), 1);
    assert_eq!(world.cells.get(&Vector2::new(0, 0)).unwrap().objects, vec![0, 1]);

    world.remove(idx1);

    assert_eq!(world.cells.len(), 1);
    assert_eq!(world.cells.get(&Vector2::new(0, 0)).unwrap().objects, vec![1]);
}