use std::cell;

use polymap::{painter::{Painter, Validation}, map_shader::{HighlightShader, MapShader}, element_set::ElementSet, PolyMap, CornerId, Corner, CellId, Cell};
use raylib::prelude::*;
use worldgen::WorldGenerator;

pub fn main() {
    let seed = 270493;

    let poly_map = polymap::PolyMap::new(800, 600, 10.0);
    let world = WorldGenerator::new()
        .generate(&poly_map, seed);

    let (mut rl, thread) = raylib::init()
        .size(800, 600)
        .title("Hello, World") 
        .build();
    rl.set_target_fps(60);


    let mut polymap_texture = Painter::new(&mut rl, &thread, &poly_map, world).unwrap();

    while !rl.window_should_close() {

        if rl.is_key_down(raylib::consts::KeyboardKey::KEY_A) {
            polymap_texture.invalidate();
        }

        let fps = format!("FPS: {}", rl.get_fps());
        rl.set_window_title(&thread,&fps);

        let mut ctx = rl.begin_drawing(&thread);
        // Uniform access...
        let ctx = &mut ctx; 

        ctx.clear_background(Color::WHITE);

        polymap_texture.draw(ctx, &thread, 0, 0, &poly_map);
    }
}
