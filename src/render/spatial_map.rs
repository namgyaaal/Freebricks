use std::hash::{Hash, Hasher};

use bevy_ecs::entity::Entity;
use bevy_platform::collections::{HashMap, HashSet};
use glam::{IVec3, Vec3};

use crate::{ecs::parts::Part, render::parts::part_formats::PartInstance};
use anyhow::{Result, anyhow};

pub type DefaultSpatialKey = SpatialKey<32>;
pub type DefaultSpatialMap = SpatialMap<32>;

/// Key used for spatial hashing.
///
/// Alongside the position we also include the part type (1) and the opaqueness (2) since we want to
/// (1) separate different part types so that cells can be naively rendered
/// (2) separate opaque and transparent objects due to each having their own pipeline (depth writing)
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SpatialKey<const SIZE: usize> {
    pub part: Part,
    pub opaque: bool,
    pub position: IVec3,
}

impl<const SIZE: usize> SpatialKey<SIZE> {
    pub fn new(part: Part, position: Vec3, alpha: u8) -> Self {
        let mut position: IVec3 = position.as_ivec3();
        position = (position / SIZE as i32) * SIZE as i32;
        SpatialKey {
            part: part,
            opaque: alpha == u8::MAX,
            position: position,
        }
    }
}

impl<const SIZE: usize> Hash for SpatialKey<SIZE> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.part as u8, self.position, self.opaque).hash(state);
    }
}

/// Values for spatial hashing.
///
/// The broad idea is that they are continguous in memory and thus allow for easy copying into staging buffers.
/// They also contain a hashset of entities in the buffers (used with entity_keys in SpatialMap)
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
pub struct SpatialMap<const SIZE: usize> {
    /// Keeps track of entity keys and indices into buffers as a generational arena.
    ///
    /// Behavior chained onto this:
    ///     Accessing entities happens through checking this then going into spatial map.
    ///     If the new position on an update falls in a different location, have to update spatial_map.
    /// Don't need generations, just indices.
    pub entity_keys: Vec<Option<(SpatialKey<SIZE>, usize)>>,
    /// Actual spatial hashmap. Each value is a cell containing a contiguous block of per-object data.
    pub spatial_map: HashMap<SpatialKey<SIZE>, SpatialCell<PartInstance>>,
}

impl<const SIZE: usize> SpatialMap<SIZE> {
    const INTERNAL_ERR: &'static str = "SpatialMap internal error";

    pub fn new() -> Self {
        SpatialMap {
            entity_keys: vec![None; 1024], // Start off generational arena with 1024
            spatial_map: HashMap::new(),
        }
    }

    pub fn get_size(&self) -> usize {
        SIZE
    }

    /// Either inserts or updates a value of the spatial map.
    ///
    /// If it's an update, it'll move it into a different cell if the position is outside of the bounds of the
    ///     current cell.
    pub fn set(
        &mut self,
        entity: Entity,
        part: Part,
        position: Vec3,
        uniform: PartInstance,
    ) -> Result<()> {
        let e_index = entity.index() as usize;
        let key = SpatialKey::new(part, position, uniform.color[3]);

        // Size-doubling if entity index is past arena
        if self.entity_keys.len() <= e_index {
            self.entity_keys.resize(self.entity_keys.len() * 2, None);
        }

        // Three cases
        //  (1) It's a new entity
        //  (2) It's an already-existing entity and needs to be in a different cell
        //  (3) It's an already-existing entity that stays in the same cell.
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

        // Handles removing it from the old cell
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
                    // Have to shift everything above it down.
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

        // Actual update/insertion logic
        // First case is in-place update, others are new/cell change (same logic)
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

    /// Remove an entity from the spatial map.
    ///
    /// Returns an error if the entity doesn't exist
    pub fn remove(&mut self, entity: Entity) -> Result<PartInstance> {
        let e_index = entity.index() as usize;

        if self.entity_keys.len() < e_index {
            return Err(anyhow!(
                "SpatialMap::remove(), can't update an entity that doesn't exist"
            ));
        }

        let Some((key, index)) = self.entity_keys[e_index] else {
            return Err(anyhow!(
                "SpatialMap::remove(), can't update an entity that doesn't exist"
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
            // Have to shift everything above it down.
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
