use parameters::ParamId;
use worldgen::{view::ViewMode, WorldParams};

use crate::VIEW_MODES;

use macroquad::prelude as mq;

pub enum GuiEvent {
    Close,
    ChangeMode(ViewMode),
    SetWorldTextureCaching(bool),
    ChangeParam(ParamId, f64),
}


pub(crate) fn gui(seed:u64, 
                 parameters:&parameters::Parameters<worldgen::WorldParams>, 
                  world_view_mode: &ViewMode, 
                  mut image_caching: bool
                ) -> (bool, Vec<GuiEvent>) {
    let mut events = vec![];
    let mut show_gui = true;

    let mut pointer_over_gui = false;

     // Process keys, mouse etc.
     egui_macroquad::ui(|egui_ctx| {
        egui::Window::new("Toolbox")
            .open(&mut show_gui)
            .show(egui_ctx, |ui| {
                pointer_over_gui = egui_ctx.is_pointer_over_area();
                ui.label(&format!("Seed: {}", seed));
                ui.label(&format!("FPS: {}", mq::get_fps()));
                if ui.checkbox(&mut image_caching, "Image Caching").clicked() {
                    events.push(GuiEvent::SetWorldTextureCaching(image_caching));
                }
                ui.horizontal_top(|ui| {
                    for mode in VIEW_MODES {
                        let selected = world_view_mode == mode;
                        let text_color = if selected {
                            egui::Color32::RED
                        } else {
                            egui::Color32::WHITE
                        };

                        if ui
                            .add(egui::Button::new(mode.name()).text_color(text_color))
                            .clicked() {
                                events.push(GuiEvent::ChangeMode(*mode));
                        }
                    } 
                });
                
                edit_parameter(ui, &mut events, parameters, &worldgen::Param::RainToRiver);
                edit_parameter(ui, &mut events, parameters, &worldgen::Param::RiverCutoff);
            });
    });

    // Draw things before egui
    egui_macroquad::draw();

    if !show_gui {
        events.push(GuiEvent::Close);
    }

    (pointer_over_gui, events)
}

fn edit_parameter(ui:&mut egui::Ui, 
                  events: &mut Vec<GuiEvent>,
                  parameters:&parameters::Parameters<WorldParams>, 
                  tag: &worldgen::Param) {
    ui.horizontal(|ui| {
        let id = parameters.lookup(tag);
        let info = parameters.info(id);
        let mut value = parameters[id];
        ui.label(info.name.as_str());
        let range = info.min.unwrap()..=info.max.unwrap();
        
        let slider = egui::widgets::Slider::new(&mut value, range)
            .logarithmic(info.logarithmic);

        if ui.add(slider).changed() {
            events.push(GuiEvent::ChangeParam(id, value))
        }
    });
}
