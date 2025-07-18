mod app;
mod dispatch;
mod game;
mod game_common;
mod game_ecs;
mod game_entity;
mod game_logics;
mod game_loop;
mod game_systems;
mod input;
mod physics;
mod rendering;
mod worker;

fn main() {
    app::run();
}
