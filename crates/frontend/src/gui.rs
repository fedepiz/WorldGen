use parameters::ParamId;
use worldgen::view::ViewMode;

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

    let mut rainfall_to_river = parameters.get(&worldgen::Param::RainToRiver);
    let mut river_cutoff = parameters.get(&worldgen::Param::RiverCutoff);

    let mut pointer_over_gui = false;

     // Process keys, mouse etc.
     egui_macroquad::ui(|egui_ctx| {
        egui::Window::new("Hello!....Colleague")
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

                ui.horizontal(|ui| {
                    let id = parameters.lookup(&worldgen::Param::RainToRiver);
                    let info = parameters.info(id);
                    ui.label(info.name.as_str());
                    
                    let range = info.min.unwrap()..=info.max.unwrap();
                    if ui.add(
                        egui::widgets::Slider::new(&mut rainfall_to_river, range)
                            .logarithmic(true)
                    ).changed() {
                        events.push(GuiEvent::ChangeParam(id, rainfall_to_river.into()))
                    }
                });

                ui.horizontal(|ui| {
                    let id = parameters.lookup(&worldgen::Param::RiverCutoff);
                    let info = parameters.info(id);
                    ui.label(info.name.as_str());
                    
                    if ui.add(egui::widgets::Slider::new(&mut river_cutoff, info.min.unwrap()..=info.max.unwrap())).changed() {
                        events.push(GuiEvent::ChangeParam(id, river_cutoff.into()))
                    }
                })
            });

    });

    // Draw things before egui
    egui_macroquad::draw();

    if !show_gui {
        events.push(GuiEvent::Close);
    }

    (pointer_over_gui, events)
}