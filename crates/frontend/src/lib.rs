use polymap::painter::{Painter, Validation};
use rand::Rng;
use raylib::prelude::*;
use worldgen::{ViewMode, WorldGenerator};

pub fn main() {
    let seed = 270493;

    const WIDTH: i32 = 800;
    const HEIGHT: i32 = 600;

    let poly_map = polymap::PolyMap::new(WIDTH as usize, HEIGHT as usize, 8.0);
    let mut world = WorldGenerator::new().generate(&poly_map, seed);

    let mut world_view_mode = worldgen::ViewMode::Heightmap;

    let (mut rl, thread) = raylib::init().size(WIDTH, HEIGHT).title("Hello, World").build();
    rl.set_target_fps(60);

    let mut polymap_texture = Painter::new(&mut rl, &thread, &poly_map).unwrap();

    const VIEW_MODES: &'static [(worldgen::ViewMode, raylib::consts::KeyboardKey)] = &[
        (ViewMode::Heightmap, raylib::consts::KeyboardKey::KEY_ONE),
        (ViewMode::Terrain, raylib::consts::KeyboardKey::KEY_TWO),
    ];

    while !rl.window_should_close() {
        if rl.is_key_down(raylib::consts::KeyboardKey::KEY_A) {
            polymap_texture.invalidate(Validation::Invalid);
        }
        if rl.is_key_pressed(raylib::consts::KeyboardKey::KEY_G) {
            let seed = rand::thread_rng().gen();
            world = WorldGenerator::new().generate(&poly_map, seed);
            polymap_texture.invalidate(Validation::Invalid)
        }

        if let Some(mode) = VIEW_MODES.iter().find_map(|(mode, key)| {
            if rl.is_key_pressed(*key) && &world_view_mode != mode {
                Some(*mode)
            } else {
                None
            }
        }) {
            world_view_mode = mode;
            polymap_texture.invalidate(Validation::Invalid)
        }

        if rl.is_mouse_button_pressed(MouseButton::MOUSE_LEFT_BUTTON) {
            let mouse_pos = rl.get_mouse_position();
            if let Some(cell_id) = poly_map.polygon_at(mouse_pos.x, mouse_pos.y) {
                println!("{:?}:{}", cell_id, world.heightmap.cells[cell_id]);
            }
        }

        let fps = format!("FPS: {}", rl.get_fps());
        rl.set_window_title(&thread, &fps);

        let mut ctx = rl.begin_drawing(&thread);
        // Uniform access...
        let ctx = &mut ctx;

        ctx.clear_background(Color::WHITE);

        polymap_texture.draw(
            ctx,
            &thread,
            0,
            0,
            &poly_map,
            &worldgen::WorldMapView::new(&world, world_view_mode),
        );
    }
}
