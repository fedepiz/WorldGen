use polymap::{painter::{Painter, Validation}, map_shader::HighlightShader, element_set::ElementSet};
use raylib::prelude::*;

mod polymap;

pub fn main() {
    let polymap = polymap::PolyMap::new(800, 600,5.0);

    let (mut rl, thread) = raylib::init()
        .size(800, 600)
        .title("Hello, World") 
        .build();
    rl.set_target_fps(60);


    let shader = HighlightShader::new();
    //let shader = polymap::map_shader::RandomColorShader::new(&polymap);
    let mut polymap_texture = Painter::new(&mut rl, &thread, &polymap, shader).unwrap();

    let mut selected_cell = None;
    while !rl.window_should_close() {

        {
            let mouse_pos = rl.get_mouse_position();
            let picked = polymap.polygon_at(mouse_pos.x, mouse_pos.y);
            if picked != selected_cell {
                polymap_texture.update_shader(|shader| {   
                    shader.0.clear();
                    let mut changes = ElementSet::new();
                    
                    if let Some(cell) = selected_cell {
                        changes.add_cell(cell, &polymap);
                    }

                    if let Some(cell) = picked {
                        shader.0.add_cell(cell, &polymap);
                        changes.add_cell(cell, &polymap);
                        changes.join(&shader.0);
                    }
                    Validation::Partial(changes)
                })
            }
            selected_cell = picked;
        }

        if rl.is_key_down(raylib::consts::KeyboardKey::KEY_A) {
            polymap_texture.invalidate();
        }

        let fps = format!("FPS: {}", rl.get_fps());
        rl.set_window_title(&thread,&fps);

        let mut ctx = rl.begin_drawing(&thread);
        // Uniform access...
        let ctx = &mut ctx; 

        ctx.clear_background(Color::WHITE);

        polymap_texture.draw(ctx, &thread, 0, 0, &polymap);
    }
}
