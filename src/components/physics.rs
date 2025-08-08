use bevy_ecs::prelude::*;
use rapier3d::prelude::*;


#[derive(Component, Debug)]
pub struct Physical {
    pub anchored: bool 
}

impl Default for Physical {
    fn default() -> Self {
        Physical { 
            anchored: true  
        } 
    }
}

impl Physical {
    pub fn dynamic() -> Self {
        Physical {
            anchored : false 
        }
    }

    pub fn anchored() -> Self {
        Physical::default()
    }
}

#[derive(Component, Debug)]
pub struct BodyHandle(pub RigidBodyHandle);

#[derive(Component, Debug)]
pub struct ShapeHandle(pub ColliderHandle);
