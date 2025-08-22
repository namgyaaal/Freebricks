use bevy_ecs::prelude::*;

pub trait State<T: Resource> {
    fn consume(world: &mut World, state: T) {
        world.insert_resource(state);
    }
}
