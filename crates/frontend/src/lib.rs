use parameters::Space;
use ::rand::Rng;
use macroquad::prelude as mq;
use macroquad::prelude::{KeyCode, MouseButton};
use worldgen::{conf::WorldGenConf, view::*, WorldGenerator};

use gui::GuiEvent;

mod painter;
mod gui;


const WIDTH: i32 = 1600;
const HEIGHT: i32 = 900;

const VIEW_MODES: &'static [ViewMode] = &[
    ViewMode::Heightmap, ViewMode::Terrain, 
    ViewMode::Hydrology, ViewMode::Thermology,
];

pub fn main() {
    let mut config = mq::Conf::default();
    config.high_dpi = true;
    config.window_width = WIDTH;
    config.window_height = HEIGHT;
    config.window_title = "Worldgen".to_owned();


    macroquad::Window::from_config(config, async {
        let mut seed = 27049319951022;

        
        let screen_scale_x = WIDTH as f32 / mq::screen_width();
        let screen_scale_y = HEIGHT as f32 / mq::screen_height();

        let mut parameters = worldgen::WorldParams::make_params();    

       
        let file = std::fs::read_to_string("./config.toml").unwrap();
        let conf: WorldGenConf = toml::from_str(file.as_str()).unwrap();
    
        parameters.define(parameters::Info {
            tag: worldgen::Param::RainToRiver,
            name: "Rain to River".to_string(),
            min: Some(0.0),
            max: Some(0.1)
        }, 0.010);

        parameters.define(parameters::Info {
            tag: worldgen::Param::RiverCutoff,
            name: "River Cutoff".to_string(),
            min: Some(0.0),
            max: Some(1.0),
        }, conf.hydrology.min_river_flux.into());

        let mut world_gen = WorldGenerator::new(conf, parameters);
        let poly_map = polymap::PolyMap::new(WIDTH as usize, HEIGHT as usize, 8.0);
        let mut world = world_gen.generate(&poly_map, seed);
        let mut world_view_mode = ViewMode::Heightmap;

        let mut polymap_texture = painter::Painter::new(&poly_map).unwrap();

        let mut show_gui = false;
        let mut image_caching = true;

        loop {
            mq::clear_background(mq::WHITE);

            if !image_caching {
                polymap_texture.invalidate(painter::Validation::Invalid);
            }

            polymap_texture.draw(
                0.0,
                0.0,
                &poly_map,
                &WorldMapView::new(&world, world_view_mode),
            );

            let mut block_clicks = false;
            if show_gui {
                let (hovered, events) = gui::gui(seed, world_gen.parameters(), &world_view_mode, image_caching);
                block_clicks = hovered;
                for event in events {
                    match event {
                        GuiEvent::Close => {
                            show_gui = false;
                        }
                        GuiEvent::ChangeMode(mode) => {
                            world_view_mode = mode;
                            polymap_texture.invalidate(painter::Validation::Invalid)
                        }
                        GuiEvent::SetWorldTextureCaching(b) => {
                            image_caching = b;
                        }
                        GuiEvent::ChangeParam(id, value) => {
                            world_gen.parameters_mut().set_param(id, value)
                        }
                    }
                }
            }

            if mq::is_key_pressed(KeyCode::G) {
                seed = rand::thread_rng().gen();
                world = world_gen.generate(&poly_map, seed);
                polymap_texture.invalidate(painter::Validation::Invalid)
            }

            if mq::is_key_pressed(KeyCode::R) {
                world = world_gen.generate(&poly_map, seed);
                polymap_texture.invalidate(painter::Validation::Invalid)
            }


            if mq::is_key_down(KeyCode::F) {
                world.reflow_rivers(world_gen.parameters(), &poly_map);
                polymap_texture.invalidate(painter::Validation::Invalid)
            }

            if mq::is_key_pressed(KeyCode::Space) {
                show_gui = !show_gui;
            }            
                
            if !block_clicks {


                if mq::is_mouse_button_pressed(MouseButton::Left) {
                    let (mx, my) = mq::mouse_position();
                    if let Some(cell_id) = poly_map.polygon_at(mx * screen_scale_x, my * screen_scale_y) {
                        println!("{:?}:{}", cell_id, world.heightmap().cell_height(cell_id));
                    }
                }
            }


            mq::next_frame().await
        }
    });
}

