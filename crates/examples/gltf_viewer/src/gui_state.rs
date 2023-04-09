use app::anyhow::Result;
use gui::imgui::{Condition, Ui};
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumIter, IntoStaticStr};
use std::convert::AsRef;
use asset_loader::light::LightRaw;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gui {
    pub aperture: f32,
    pub focus_distance: f32,
    pub number_of_samples: u32,
    pub number_of_bounces: u32,
    pub ray_tracing: bool,
    pub acc: bool,
    pub sky: bool,
    pub map_scale: f32,
    pub max_number_of_samples: u32,
    pub scale: f32,
    pub scene: Scene,
    pub mapping: Mapping,
    pub skybox: Skybox,
    pub animation: bool,
    pub animation_speed: f32,
    pub antialiasing: bool,
    pub debug: u32,
    pub sun: LightRaw,
    light_angle: [f32; 2],

    pub orthographic_fov_dis: f32,
}

#[derive(IntoStaticStr, AsRefStr, EnumIter, PartialEq, Clone, Copy, Debug, Default)]
pub enum Scene {
    TransmissionRoughnessTest,
    // Passed tests
    // UnlitTest,
    // VertexColorTest,
    SpecularTest,
    CornellBoxLucy,
    Cornell,
    ABeautifulGame,
    Sponza,
    Type59,
    DamagedHelmet,
    EmissiveStrengthTest,
    LightsPunctualLamp,
    BoomBoxWithAxes,
    Triss,
    EVA,
    Anakin,
    Ford,
    Ironman,
    Knight,
    Loba,
    Hulkbuster,
    KikuHoshimi,
    SparkLence,
    Apollo,
    CyberSamurai,
    Apex,
    Ra,
    BlueEye,
    Puzzle,
    Titan,
    MilleniumEye,
    VC,

    SunTemple,
    DragonAttenuation,
    TransmissionTest,
    MetalRoughSpheresNoTextures,
    MetalRoughSpheres,

    NormalTangentTest,
    NormalTangentMirrorTest,
    EnvironmentTest,
    SpecGlossVsMetalRough,
    AlphaBlendModeTest,
    // BrainStem,
    // Fox,
    NegativeScaleTest,
    TextureCoordinateTest,
    TextureLinearInterpolationTest,
    TextureSettingsTest,
    ToyCar,
    AttenuationTest,
    #[default]
    Earth,
    // MultiUVTest,

    FerrisCrab,LA_Night,WinterForest,Panocube
}

impl Scene {
    pub fn path(&self) -> &'static str {
        match self {
            Self::Cornell => "cornellBox.gltf",
            Self::Type59 => "type59.gltf",
            scene => scene.into(),
        }
    }
}

#[derive(Default, Debug, AsRefStr, IntoStaticStr, EnumIter, Copy, Clone, PartialEq)]
pub enum Skybox {
    LancellottiChapel,
    Yokohama,
    SaintPetersBasilica,
    LearnOpengl,
    #[default]
    UtahInteractiveGraphics
}

impl Skybox {
    pub fn path(&self) -> &'static str {
        self.into()
        // match self {
        //     Self::LancellottiChapel => "LancellottiChapel",
        //     Self::Yokohama => "Yokohama",
        //     Self::SaintPetersBasilica => "SaintPetersBasilica",
        //     Self::LearnOpengl => "LearnOpengl",
        //     Self::UtahInteractiveGraphics => "UtahInteractiveGraphics"
        // }
    }
}

#[derive(Default, Debug, AsRefStr, EnumIter, Copy, Clone, PartialEq)]
pub enum Mapping {
    #[default]
    RENDER = 0,
    HEAT = 1,
    INSTANCE = 2,
    TRIANGLE = 3,
    DISTANCE = 4,
    ALBEDO = 5,
}

impl Gui {
    pub fn is_mapping(&self) -> bool{
        self.mapping != Mapping::RENDER
    }

    pub fn ray_query(&self) -> bool {
        self.mapping != Mapping::RENDER && self.mapping != Mapping::HEAT
    }

    pub fn get_number_of_samples(&self, total_number_of_samples: u32) -> u32 {
        if self.max_number_of_samples <= total_number_of_samples {
            0
        } else {
            (self.max_number_of_samples - total_number_of_samples).min(self.number_of_samples)
        }
    }

    pub fn acc(&self) -> bool {
        self.acc && !self.is_mapping() && !self.animation
    }

    pub fn get_bounce(&self) -> u32 {
        if self.ray_query() {
            1
        } else {
            self.number_of_bounces
        }
    }
}

impl app::Gui for Gui {
    fn new() -> Result<Self> {
        Ok(Gui {
            aperture: 0.01,
            focus_distance: 10.0,
            number_of_samples: 3,
            number_of_bounces: 5,
            ray_tracing: true,
            acc: true,
            map_scale: 1.0,
            max_number_of_samples: 100,
            sky: !false,
            scene: Default::default(),
            scale: 1.,
            mapping: Default::default(),
            skybox: Default::default(),
            animation: false,
            animation_speed: 1.,
            antialiasing: true,
            debug: 0,
            sun: LightRaw::default(),
            light_angle: [1.; 2],

            orthographic_fov_dis: 0.0,
        })
    }

    fn build(&mut self, ui: &Ui) {
        ui.window("Vulkan RT")
            .size([400.0, 400.0], Condition::FirstUseEver)
            .bg_alpha(0.5)
            .build(|| {
                // RT controls
                ui.text_wrapped("Rays");

                let mut number_of_samples = self.number_of_samples as _;
                ui.input_int("Number of samples", &mut number_of_samples)
                    .build();
                self.number_of_samples = number_of_samples.abs() as _;

                let mut max_number_of_samples = self.max_number_of_samples as _;
                ui.input_int("Max Number of samples", &mut max_number_of_samples)
                    .build();
                self.max_number_of_samples = max_number_of_samples.abs() as _;

                let mut number_of_bounces = self.number_of_bounces as _;
                ui.input_int("Max Number of bounces", &mut number_of_bounces)
                    .build();
                self.number_of_bounces = number_of_bounces as _;

                let mut debug_number = self.debug as _;
                ui.input_int("Debug control", &mut debug_number)
                    .build();
                self.debug = debug_number.abs() as _;
                ui.slider("scale", -20., 20., &mut self.scale);
                ui.slider("Apertures", 0., 1., &mut self.aperture);
                ui.slider("Focus", 0.1, 20., &mut self.focus_distance);
                ui.slider("Orthographic", 0., 100., &mut self.orthographic_fov_dis);

                let mut selected = self.scene;
                if let Some(_) = ui.begin_combo("Scene", format!("{}", selected.as_ref())) {
                    for cur in Scene::iter() {
                        if selected == cur {
                            // Auto-scroll to selected item
                            ui.set_item_default_focus();
                        }
                        // Create a "selectable"
                        let clicked = ui.selectable_config(cur).selected(selected == cur).build();
                        // When item is clicked, store it
                        if clicked {
                            selected = cur;
                        }
                    }
                    self.scene = selected;
                }

                ui.separator();

                let mut selected = self.mapping;
                if let Some(_) = ui.begin_combo("Mapping", format!("{}", selected.as_ref())) {
                    for cur in Mapping::iter() {
                        if selected == cur {
                            // Auto-scroll to selected item
                            ui.set_item_default_focus();
                        }
                        // Create a "selectable"
                        let clicked = ui.selectable_config(cur).selected(selected == cur).build();
                        // When item is clicked, store it
                        if clicked {
                            selected = cur;
                        }
                    }
                    self.mapping = selected;
                }
                match self.mapping {
                    Mapping::HEAT => ui.slider("Heatmap Scale", 0.1, 10., &mut self.map_scale),
                    Mapping::DISTANCE => {
                        ui.slider("dis_map Scale", 10., 1000., &mut self.map_scale)
                    },
                    _ => {false}
                };

                // ui.slider("Virtual light intensity",0., 2.0, &mut self.light_intensity);
                // Light control
                // ui.text_wrapped("Light");
                ui.separator();
                // ui.input_float3("direction", &mut self.light.direction)
                //     .build();
                let mut selected = self.skybox;
                if let Some(_) = ui.begin_combo("Skybox", format!("{}", selected.as_ref())) {
                    for cur in Skybox::iter() {
                        if selected == cur {
                            // Auto-scroll to selected item
                            ui.set_item_default_focus();
                        }
                        // Create a "selectable"
                        let clicked = ui.selectable_config(cur).selected(selected == cur).build();
                        // When item is clicked, store it
                        if clicked {
                            selected = cur;
                        }
                    }
                    self.skybox = selected;
                }
                ui.separator();
                if ui.radio_button_bool("Ray tracing", self.ray_tracing) {
                    self.ray_tracing = !self.ray_tracing;
                }

                if ui.radio_button_bool("Accumulation", self.acc) {
                    self.acc = !self.acc;
                }

                if ui.radio_button_bool("Animation", self.animation) {
                    self.animation = !self.animation;
                }
                if self.animation {
                    ui.slider("Animation speed", 0.1, 10., &mut self.animation_speed);
                }

                if ui.radio_button_bool("sky", self.sky) {
                    self.sky = !self.sky;
                }

                if ui.radio_button_bool("Anti-aliasing", self.antialiasing) {
                    self.antialiasing = !self.antialiasing;
                }
                ui.slider("light intensity", 0., 2.0, &mut self.sun.intensity);
                // let [mut theta, mut phi] = self.sun.get_angles();
                const PI: f32 = std::f32::consts::PI;
                ui.slider("light theta", 0.0, PI, &mut self.light_angle[0]);
                ui.slider("light phi", 0., 2. * PI, &mut self.light_angle[1]);
                self.sun.update_angles(self.light_angle);

                let [r, g, b, _] = self.sun.color.to_array();
                let mut color = [r, g, b];
                ui.color_picker3_config("color", &mut color)
                    .display_rgb(true)
                    .build();
                self.sun.update_color([color[0], color[1], color[2], 0.]);

            });
    }
}
