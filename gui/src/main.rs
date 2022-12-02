use eframe::{NativeOptions, CreationContext, App};
use egui::{Window, Context, RawInput, CentralPanel, Slider};
use serde::{Serialize, Deserialize};

fn main() {
    let native_options = NativeOptions::default();
    eframe::run_native(
        "Adder",
        native_options,
        Box::new(|creation_context| Box::new(Application::new(creation_context)))
    );

    let ctx = Context::default();

    loop {
        let raw_input = RawInput::default();

        let full_output = ctx.run(raw_input, |ctx| {
            Window::new("Adder")
                .show(&ctx, |ui| {

                });
        });

        
    }
}

#[derive(Serialize, Deserialize)]
struct Application {
    target: isize,
}

impl Default for Application {
    fn default() -> Self {
        Self {
            target: 0,
        }
    }
}

impl Application {
    fn new(cc: &CreationContext) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl App for Application {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Adder");

            ui.add(Slider::new(&mut self.target, -10..=10).text("Target"));
            if ui.button("Increment").clicked() {
                self.target += 1;
            }
        });
    }
}
