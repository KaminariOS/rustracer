use gui::imgui::{Condition, Ui};
use app::anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gui {
    pub aperture: f32,
    pub focus_distance: f32,
    pub number_of_samples: u32,
    pub number_of_bounces: u32,
    pub ray_tracing: bool,
    pub acc: bool,
    pub heatmap: bool,
    pub sky: bool,
    pub heatmap_scale: f32,
    pub max_number_of_samples: u32,
}


impl app::Gui for Gui {
    fn new() -> Result<Self> {
        Ok(Gui {
            // light: Light {
            //     direction: [-2.0, -1.0, -2.0],
            //     color: [1.0; 3],
            //
            // },
            aperture: 0.01,
            focus_distance: 10.0,
            number_of_samples: 3,
            number_of_bounces: 5,
            ray_tracing: true,
            acc: true,
            heatmap: false,
            heatmap_scale: 1.0,
            max_number_of_samples: 5000,
            sky: false
        })
    }

    fn build(&mut self, ui: &Ui) {
        ui.window("Vulkan RT")
            .size([300.0, 400.0], Condition::FirstUseEver)
            .build(|| {
                // RT controls
                ui.text_wrapped("Rays");

                let mut number_of_samples = self.number_of_samples as _;
                ui.input_int("Number of samples", &mut number_of_samples)
                    .build();
                self.number_of_samples = number_of_samples as _;

                let mut max_number_of_samples = self.max_number_of_samples as _;
                ui.input_int("Max Number of samples", &mut max_number_of_samples)
                    .build();
                self.max_number_of_samples = max_number_of_samples as _;

                let mut number_of_bounces = self.number_of_bounces as _;
                ui.input_int("Max Number of bounces", &mut number_of_bounces)
                    .build();
                self.number_of_bounces = number_of_bounces as _;

                ui.slider("Apertures", 0., 1., &mut self.aperture);
                ui.slider("Focus", 0.1, 20., &mut self.focus_distance);
                ui.slider("Heatmap Scale", 0.1, 10., &mut self.heatmap_scale);

                // Light control
                ui.text_wrapped("Light");
                ui.separator();

                // ui.input_float3("direction", &mut self.light.direction)
                //     .build();

                if ui.radio_button_bool("Ray tracing", self.ray_tracing) {
                    self.ray_tracing = !self.ray_tracing;
                }

                if ui.radio_button_bool("Acc", self.acc) {
                    self.acc = !self.acc;
                }

                if ui.radio_button_bool("Heatmap", self.heatmap) {
                    self.heatmap = !self.heatmap;
                }
                if ui.radio_button_bool("sky", self.sky) {
                    self.sky = !self.sky;
                }
                // ui.color_picker3_config("color", &mut self.light.color)
                //     .display_rgb(true)
                //     .build();
            });
    }
}
