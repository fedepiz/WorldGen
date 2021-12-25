use macroquad::prelude as mq;
use macroquad::prelude::{KeyCode, MouseButton};

use gui::GuiEvent;
use painter::ViewMode;
use polymap::PolyMap;
use rand::{Rng, SeedableRng};

mod gui;
mod tessellation;
mod painter;


const WIDTH: i32 = 1600;
const HEIGHT: i32 = 900;

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

        let poly = PolyMap::new(1600, 900, 8.0);
        let mut world = world::World::new(&poly);
        world.generate(&mut rand::rngs::SmallRng::seed_from_u64(seed));

        let mut view_mode = ViewMode::Geography;
        let mut dirty = true;

        let mut painter = painter::Painter::new(&poly);

        let mut show_gui = false;


        loop {

            if dirty {
                painter.update(&world, view_mode);
                dirty = false;
            }

            mq::clear_background(mq::WHITE);

            painter.draw();


            let mut block_clicks = false;
            if show_gui {
                let (hovered, events) = gui::gui(seed, view_mode);
                block_clicks = hovered;
                for event in events {
                    match event {
                        GuiEvent::Close => {
                            show_gui = false;
                        }
                        GuiEvent::SetViewMode(mode) => {
                            view_mode = mode;
                            dirty = true;
                        }
                    }
                }
            }

            if !block_clicks {
                let (smx, smy) = mq::mouse_position();
                // Scale th mouse coordinate appropriately
                let mx = screen_scale_x * smx;
                let my = screen_scale_y * smy;

                if mq::is_mouse_button_pressed(MouseButton::Left) {
                    if let Some(clicked_poly) = poly.cell_at(mx as f64, my as f64) {
                        println!("Clicked cell:{}", clicked_poly.idx())
                    }
                }
            }

            if mq::is_key_pressed(KeyCode::Space) {
                show_gui = !show_gui;
            }    
            
            if mq::is_key_pressed(KeyCode::R) {
                seed = rand::thread_rng().gen();
                world.generate(&mut rand::rngs::SmallRng::seed_from_u64(seed));         
                dirty = true;
            }        
                
            if !block_clicks {
                if mq::is_mouse_button_pressed(MouseButton::Left) {
                    
                }
            }

            mq::next_frame().await
        }
    });
}

