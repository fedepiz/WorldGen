use ::rand::Rng;
use macroquad::prelude as mq;
use macroquad::prelude::{KeyCode, MouseButton};
use polymap::painter::Validation;
use worldgen::{conf::WorldGenConf, ViewMode, WorldGenerator};

const WIDTH: i32 = 1600;
const HEIGHT: i32 = 900;

const VIEW_MODES: &'static [(worldgen::ViewMode, KeyCode)] = &[
    (ViewMode::Heightmap, KeyCode::Key1),
    (ViewMode::Terrain, KeyCode::Key2),
    (ViewMode::Hydrology, KeyCode::Key3),
    (ViewMode::Thermology, KeyCode::Key4),
];

pub fn main() {
    let mut config = mq::Conf::default();
    config.high_dpi = true;
    config.window_width = WIDTH;
    config.window_height = HEIGHT;
    config.window_title = "Worldgen".to_owned();
    macroquad::Window::from_config(config, async {
        let mut seed = 27049319951022;

        let make_world_gen = || {
            let file = std::fs::read_to_string("./config.toml").unwrap();
            let conf: WorldGenConf = toml::from_str(file.as_str()).unwrap();
            WorldGenerator::new(conf)
        };
        let mut world_gen = make_world_gen();
        let poly_map = polymap::PolyMap::new(WIDTH as usize, HEIGHT as usize, 8.0);
        let mut world = world_gen.generate(&poly_map, seed);
        let mut world_view_mode = worldgen::ViewMode::Heightmap;

        let mut polymap_texture = polymap::painter::Painter::new(&poly_map).unwrap();

        let mut show_gui = false;

        loop {
            if mq::is_key_pressed(KeyCode::G) {
                seed = rand::thread_rng().gen();
                world = world_gen.generate(&poly_map, seed);
                polymap_texture.invalidate(Validation::Invalid)
            }
            if mq::is_key_pressed(KeyCode::R) {
                world_gen = make_world_gen();
                seed = rand::thread_rng().gen();
                world = world_gen.generate(&poly_map, seed);
                polymap_texture.invalidate(Validation::Invalid)
            }
            if mq::is_key_down(KeyCode::F) {
                world.reflow_rivers(&poly_map);
                polymap_texture.invalidate(Validation::Invalid)
            }

            if mq::is_key_pressed(KeyCode::Space) {
                show_gui = !show_gui;
            }

            if let Some(mode) = VIEW_MODES.iter().find_map(|(mode, key)| {
                if mq::is_key_pressed(*key) && &world_view_mode != mode {
                    Some(*mode)
                } else {
                    None
                }
            }) {
                world_view_mode = mode;
                polymap_texture.invalidate(Validation::Invalid)
            }

            if mq::is_mouse_button_pressed(MouseButton::Left) {
                let (mx, my) = mq::mouse_position();
                if let Some(cell_id) = poly_map.polygon_at(mx, my) {
                    println!("{:?}:{}", cell_id, world.heightmap().cell_height(cell_id));
                }
            }

            mq::clear_background(mq::WHITE);

            polymap_texture.draw(
                0.0,
                0.0,
                &poly_map,
                &worldgen::WorldMapView::new(&world, world_view_mode),
            );

            // Process keys, mouse etc.
            egui_macroquad::ui(|egui_ctx| {
                egui::Window::new("Toolbox")
                    .open(&mut show_gui)
                    .show(egui_ctx, |ui| {
                        ui.label(&format!("FPS: {}", mq::get_fps()));
                        ui.horizontal_top(|ui| {
                            for &(mode, _) in VIEW_MODES {
                                let selected = world_view_mode == mode;
                                let text_color = if selected {
                                    egui::Color32::RED
                                } else {
                                    egui::Color32::WHITE
                                };

                                if ui
                                    .add(egui::Button::new(mode.name()).text_color(text_color))
                                    .clicked() {
                                    world_view_mode = mode;
                                }
                            }
                        })
                    });
            });

            // Draw things before egui
            egui_macroquad::draw();

            mq::next_frame().await
        }
    });
}
