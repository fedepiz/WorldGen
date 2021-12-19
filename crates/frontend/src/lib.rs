use polymap::painter::{Painter, Validation};
use rand::Rng;
use raylib::consts::KeyboardKey;
use raylib::prelude::*;
use worldgen::{conf::WorldGenConf, ViewMode, WorldGenerator};

pub fn main() {
    let mut seed = 27049319951022;

    const WIDTH: i32 = 1600;
    const HEIGHT: i32 = 900;

    let make_world_gen = || {
        let file = std::fs::read_to_string("./config.toml").unwrap();
        let conf: WorldGenConf = toml::from_str(file.as_str()).unwrap();
        WorldGenerator::new(conf)
    };
    let mut world_gen = make_world_gen();

    let poly_map = polymap::PolyMap::new(WIDTH as usize, HEIGHT as usize, 8.0);

    let mut world = world_gen.generate(&poly_map, seed);

    let mut world_view_mode = worldgen::ViewMode::Heightmap;

    raylib::core::logging::set_trace_log(raylib::consts::TraceLogLevel::LOG_NONE);
    let (mut rl, thread) = raylib::init()
        .size(WIDTH, HEIGHT)
        .title("Hello, World")
        .build();
    rl.set_target_fps(60);

    let mut polymap_texture = Painter::new(&mut rl, &thread, &poly_map).unwrap();

    const VIEW_MODES: &'static [(worldgen::ViewMode, KeyboardKey)] = &[
        (ViewMode::Heightmap, KeyboardKey::KEY_ONE),
        (ViewMode::Terrain, KeyboardKey::KEY_TWO),
        (ViewMode::Hydrology, KeyboardKey::KEY_THREE),
    ];

    while !rl.window_should_close() {
        if rl.is_key_down(KeyboardKey::KEY_A) {
            polymap_texture.invalidate(Validation::Invalid);
        }
        if rl.is_key_pressed(KeyboardKey::KEY_G) {
            seed = rand::thread_rng().gen();
            world = world_gen.generate(&poly_map, seed);
            polymap_texture.invalidate(Validation::Invalid)
        }
        if rl.is_key_pressed(KeyboardKey::KEY_R) {
            world_gen = make_world_gen();
            seed = rand::thread_rng().gen();
            world = world_gen.generate(&poly_map, seed);
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
                println!("{:?}:{}", cell_id, world.heightmap.cell_height(cell_id));
            }
        }

        let fps = format!("FPS: {}, Seed: {}", rl.get_fps(), seed);
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
