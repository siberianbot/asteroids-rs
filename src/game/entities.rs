use std::{
    f32::consts::PI,
    ops::RangeInclusive,
    ptr::NonNull,
    sync::{Arc, Mutex, MutexGuard},
};

use bitflags::bitflags;
use glam::{Vec2, vec2};

use crate::{
    dispatch::{Command, Dispatcher, Event, Sender},
    entity::Entity,
    rendering::shaders::Vertex,
};

pub const SPACECRAFT_VERTICES: [Vertex; 3] = [
    Vertex {
        position: Vec2::new(0.0, 0.5),
    },
    Vertex {
        position: Vec2::new(0.35355339, -0.35355339),
    },
    Vertex {
        position: Vec2::new(-0.35355339, -0.35355339),
    },
];
pub const SPACECRAFT_INDICES: [u32; 3] = [0, 1, 2];

pub const ASTEROID_SEGMENTS: usize = 8;
pub const ASTEROID_SEGMENT_RANGE: RangeInclusive<f32> = 0.75..=1.0;
pub const ASTEROID_INDICES: [u32; 24] = [
    // TODO: try to enumerate in compile-time by using ASTEROID_SEGMENTS value
    0, 1, 2, //
    0, 2, 3, //
    0, 3, 4, //
    0, 4, 5, //
    0, 5, 6, //
    0, 6, 7, //
    0, 7, 8, //
    0, 8, 1, //
];

pub const ASTEROID_SIZE_RANGE: RangeInclusive<u32> = 1..=4;
pub const ASTEROID_VELOCITY_RANGE: RangeInclusive<f32> = 0.25..=3.0;
pub const ASTEROID_ROTATION_VELOCITY_RANGE: RangeInclusive<f32> = 0.25..=2.0;

pub const BULLET_VERTICES: [Vertex; 1] = [Vertex {
    position: Vec2::new(0.0, 0.0),
}];
pub const BULLET_INDICES: [u32; 1] = [0];

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct PlayerAction : u32{
        const ACCELERATE = 1 << 0;
        const DECELERATE = 1 << 1;
        const INCLINE_LEFT = 1 << 2;
        const INCLINE_RIGHT = 1 << 3;
        const FIRE = 1 << 4;
    }
}

pub const CAMERA_INITIAL_DISTANCE: f32 = 4.0;
pub const CAMERA_MIN_DISTANCE: f32 = 1.0;
pub const CAMERA_MAX_DISTANCE: f32 = 32.0;
pub const CAMERA_DISTANCE_MULTIPLIER: f32 = 2.0;
