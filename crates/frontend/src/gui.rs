use macroquad::prelude as mq;
use strum::IntoEnumIterator;

use crate::painter::ViewMode;

pub enum GuiEvent {
    Close,
    SetViewMode(ViewMode),
}


pub(crate) fn gui(seed:u64, view_mode: ViewMode) -> (bool, Vec<GuiEvent>) {
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
                ui.horizontal(|ui| {
                    for mode in ViewMode::iter() {
                        let selected = view_mode == mode;
                        let color = if selected { egui::Color32::RED } else { egui::Color32::WHITE };
                        if ui.add(egui::Button::new(mode.name()).text_color(color)).clicked() {
                            events.push(GuiEvent::SetViewMode(mode))
                        }
                    }
                });
            
            });
    });

    // Draw things before egui
    egui_macroquad::draw();

    if !show_gui {
        events.push(GuiEvent::Close);
    }

    (pointer_over_gui, events)
}
