use crate::game_ecs::SystemArgs;

/// Synchronizes camera position with target position
pub fn camera_sync_system(args: SystemArgs) {
    let position = args
        .entity
        .camera()
        .filter(|camera| camera.follow)
        .and_then(|camera| camera.target)
        .and_then(|target| {
            args.get_entity(target)
                .map(|entity| entity.transform().position)
        });

    if let Some(position) = position {
        args.modify(move |entity| entity.transform_mut().position = position);
    }
}

/// Controls entities movement
pub fn movement_system(args: SystemArgs) {
    const BREAKING_ACCELERATION_EPSILON: f32 = 0.01;
    const BREAKING_VELOCITY_MULTIPLIER: f32 = 0.5;

    let position_velocity = args
        .entity
        .movement()
        .map(|movement| match movement.const_velocity {
            true => (
                args.entity.transform().position + args.elapsed * movement.velocity,
                movement.velocity,
            ),

            false if movement.acceleration.length() > BREAKING_ACCELERATION_EPSILON => (
                args.entity.transform().position + args.elapsed * movement.velocity,
                movement.velocity + args.elapsed * movement.acceleration,
            ),

            false => (
                args.entity.transform().position + args.elapsed * movement.velocity,
                movement.velocity - args.elapsed * BREAKING_VELOCITY_MULTIPLIER * movement.velocity,
            ),
        });

    if let Some((position, velocity)) = position_velocity {
        args.modify(move |entity| {
            entity.transform_mut().position = position;
            entity.movement_mut().unwrap().velocity = velocity;
        });
    }
}

/// Updates spacecraft weapon cooldown
pub fn spacecraft_cooldown_system(args: SystemArgs) {
    let cooldown = args
        .entity
        .spacecraft()
        .filter(|spacecraft| spacecraft.cooldown > 0.0)
        .map(|spacecraft| {
            if spacecraft.cooldown < args.elapsed {
                0.0
            } else {
                spacecraft.cooldown - args.elapsed
            }
        });

    if let Some(cooldown) = cooldown {
        args.modify(move |entity| entity.spacecraft_mut().unwrap().cooldown = cooldown);
    }
}
