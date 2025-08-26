use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;

#[derive(Component, Debug, Deref, DerefMut, Clone, Copy)]
/// Component for indicating where it is in GPU memory.
/// Very minimal for now but will be expanded in the future, currently points to offset in instance buffer.
pub struct BufferIndex(pub Option<u32>);

impl Default for BufferIndex {
    fn default() -> Self {
        BufferIndex(None)
    }
}

#[derive(Debug)]
pub enum RenderModeOption {
    None = 0x00,    // Not Added
    Uniform = 0x01, // Not Added
    Instanced = 0x02,
}

#[derive(Component, Debug, Deref, DerefMut)]

/// Whether it's rendered with instancing, uniform buffer, etc..
pub struct RenderMode(pub RenderModeOption);

impl Default for RenderMode {
    fn default() -> Self {
        RenderMode(RenderModeOption::None)
    }
}
