mod app;
mod dispatch;
mod ecs;
mod entity;
mod game;
mod game_common;
mod game_logics;
mod game_loop;
mod input;
mod physics;
mod rendering;
mod systems;
mod worker;

fn main() {
    app::run();
}
