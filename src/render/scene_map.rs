use std::hash::{Hash, Hasher};

use bevy_ecs::entity::Entity;
use bevy_platform::collections::{HashMap, HashSet};
use glam::{IVec3, Vec3};

use crate::{ecs::parts::Part, render::parts::PartInstance};
use anyhow::{Result, anyhow};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SpatialKey {
    pub part: Part,
    pub position: IVec3,
}

impl SpatialKey {
    pub fn new(part: Part, position: Vec3, cell_size: usize) -> Self {
        let mut position: IVec3 = position.as_ivec3();
        position = (position / cell_size as i32) * cell_size as i32;

        SpatialKey {
            part: part,
            position: position,
        }
    }
}

impl Hash for SpatialKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.part as u8, self.position).hash(state);
    }
}

#[derive(Debug)]
pub struct SpatialCell<T> {
    pub entities: HashSet<Entity>,
    pub buffers: Vec<T>,
}

impl<T> SpatialCell<T> {
    pub fn new() -> Self {
        Self {
            entities: HashSet::new(),
            buffers: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct SceneMap<const SIZE: usize> {
    /// Keeps track of entity keys and indices into buffers as a generational arena.
    /// Behavior chained onto this:
    ///     Accessing entities happens through checking this then going into spatial map.
    ///     If the new position on an update falls in a different location, have to update spatial_map.
    /// Don't need generations, just indices.
    pub entity_keys: Vec<Option<(SpatialKey, usize)>>,
    pub spatial_map: HashMap<SpatialKey, SpatialCell<PartInstance>>,
}

impl<const SIZE: usize> SceneMap<SIZE> {
    const INTERNAL_ERR: &'static str = "SceneMap internal error";

    pub fn new() -> Self {
        SceneMap {
            entity_keys: vec![None; 1024], // Start off generational arena with 1024
            spatial_map: HashMap::new(),
        }
    }

    pub fn get_size(&self) -> usize {
        SIZE
    }

    pub fn set(
        &mut self,
        entity: Entity,
        part: Part,
        position: Vec3,
        uniform: PartInstance,
    ) -> Result<()> {
        let e_index = entity.index() as usize;
        let key = SpatialKey::new(part, position, SIZE);

        // Size-doubling if entity index is past arena
        if self.entity_keys.len() <= e_index {
            self.entity_keys.resize(self.entity_keys.len() * 2, None);
        }

        let new_entity = self.entity_keys[e_index].is_none();
        let (needs_switch, other_key, other_index) = {
            if new_entity {
                (false, None, None)
            } else if let Some((other_key, other_index)) = self.entity_keys[e_index]
                && other_key != key
            {
                (true, Some(other_key), Some(other_index))
            } else {
                (false, None, None)
            }
        };

        if needs_switch {
            let old_key = other_key.unwrap();
            let old_index = other_index.unwrap();

            let remove = {
                let other_cell = self.spatial_map.get_mut(&old_key).unwrap();
                other_cell.entities.remove(&entity);
                other_cell.buffers.remove(old_index);

                for other in &other_cell.entities {
                    let Some((_, other)) = &mut self.entity_keys[other.index() as usize] else {
                        panic!("{}", Self::INTERNAL_ERR);
                    };

                    if *other > old_index {
                        *other -= 1;
                    }
                }
                other_cell.entities.len() == 0
            };
            if remove {
                self.spatial_map.remove(&old_key);
            }
        }

        let cell = self.spatial_map.entry(key).or_insert(SpatialCell::new());
        if !new_entity && !needs_switch {
            let index = self.entity_keys[e_index].unwrap().1;
            cell.buffers[index] = uniform;
        } else {
            let index = cell.buffers.len();
            cell.buffers.push(uniform);
            cell.entities.insert(entity);
            self.entity_keys[e_index] = Some((key, index));
        }

        Ok(())
    }

    pub fn remove(&mut self, entity: Entity) -> Result<PartInstance> {
        let e_index = entity.index() as usize;

        if self.entity_keys.len() < e_index {
            return Err(anyhow!(
                "SceneMap::remove(), can't update an entity that doesn't exist"
            ));
        }

        let Some((key, index)) = self.entity_keys[e_index] else {
            return Err(anyhow!(
                "SceneMap::remove(), can't update an entity that doesn't exist"
            ));
        };

        let cell = self.spatial_map.get_mut(&key).expect(Self::INTERNAL_ERR);

        self.entity_keys[e_index] = None;
        cell.entities.remove(&entity);
        let uniform = cell.buffers.remove(index);

        for other in &cell.entities {
            let o_index = other.index() as usize;

            let Some((_, other)) = &mut self.entity_keys[o_index] else {
                panic!("{}", Self::INTERNAL_ERR);
            };

            if *other > index {
                *other -= 1;
            }
        }

        Ok(uniform)
    }

    // Camera Frustrum Culling
    pub fn sweep() {
        todo!()
    }
}
