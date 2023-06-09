use crate::gui_state::Scene::DragAndDrop;
use app::anyhow::Result;
use asset_loader::light::LightRaw;
use gui::imgui::{Condition, Ui};
use std::borrow::Cow;
use std::convert::AsRef;
use std::time::Duration;
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumIter, IntoStaticStr};

const FPS: f32 = 40.;
const BUDGET: f32 = 1. / FPS;

#[derive(Debug, Clone, PartialEq)]
pub struct Gui {
    pub aperture: f32,
    pub focus_distance: f32,
    pub number_of_samples: u32,
    pub dynamic_samples: bool,
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
    pub point_light_intensity: f32,
    pub orthographic_fov_dis: f32,
    pub point_light_radius: f32,
    pub exposure: f32,
    pub selected_tone_map_mode: usize,
}

#[derive(IntoStaticStr, AsRefStr, EnumIter, PartialEq, Clone, Debug, Default)]
pub enum Scene {
    ZeroDay,
    TransmissionRoughnessTest,
    // Passed tests:
    // UnlitTest,
    // VertexColorTest,
    SpecularTest,
    Bathroom,

    BistroExt,

    Emerald,
    BistroInterior,
    CornellBoxLucy,
    CornellBox,
    Game,
    ABeautifulGame,
    Sponza,

    Type59,

    DamagedHelmet,
    MosquitoInAmber,
    EmissiveStrengthTest,
    LightsPunctualLamp,
    BoomBoxWithAxes,

    Test,
    Ferris,
    Erato,
    Teapot,
    Anime,
    LaocoonInBox,
    Garage,
    Triss,
    EVA,
    Anakin,
    Ford,
    Ironman,
    Knight,
    // RiggedSimple,
    CesiumMan,
    // RiggedFigure,
    // SimpleSkin,
    Loba,
    Hulkbuster,
    // KikuHoshimi,
    SparkLence,
    Apollo,

    LamborghiniInterior,
    #[default]
    Skull,
    Ironman85,
    Laocoon,
    LamborghiniBlue,
    Room,
    Spartan,
    SciFiGirlWalk,

    SciFiGirl,
    BathroomRPT,
    Xeno,
    CL4P,
    MillenniumFalcon,
    MillenniumFalconHighPoly,

    BusterDrone,
    Lamborghini2021,
    SkullSimple,
    LouisXIV,
    DragonArmor,
    StormTrooper,
    Nier,
    Crusader,
    Thinker,
    Enterprise,

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

    BrainStem,
    Fox,
    NegativeScaleTest,
    TextureCoordinateTest,
    TextureLinearInterpolationTest,
    TextureSettingsTest,
    ToyCar,
    AttenuationTest,
    Earth,

    // MultiUVTest,
    // FerrisCrab,
    LA_Night,
    // WinterForest,
    // Panocube,
    CemeteryAngelCampanella,
    CemeteryAngelMiller,
    BaptismalAngelKneeling,
    // Batman,
    GingaSpark,
    Katana,
    Valkyrie,
    ValkyrieBronze,
    AngelScottino,
    Aurelius,
    Nile,
    Liberty,
    Pagoda,
    Tardis,
    Colosseum,
    Kentaur,
    Tritonen,
    FrankAngel,
    Angels,

    // Chief,
    // Avengers,
    // ShutterGirl,
    DaedricGauntlet,
    Curiosity,
    DragonThree,
    DragonFlying,
    Chernovan,
    LotusFlower,
    IcyDragon,
    Zombie,
    // Trex,
    Phoenix,
    Wolf,
    DragAndDrop(String),
}

impl Scene {
    pub fn path(&self) -> String {
        match self {
            DragAndDrop(path) => path.to_string(),
            scene => scene.as_ref().to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToneMapMode {
    Default = 0,
    Uncharted,
    HejlRichard,
    Aces,
    None,
}

impl ToneMapMode {
    pub fn all() -> [ToneMapMode; 5] {
        use ToneMapMode::*;
        [Default, Uncharted, HejlRichard, Aces, None]
    }
}

#[derive(Default, Debug, AsRefStr, IntoStaticStr, EnumIter, Copy, Clone, PartialEq)]
pub enum Skybox {
    LancellottiChapel,
    Yokohama,
    SaintPetersBasilica,
    LearnOpengl,
    #[default]
    UtahInteractiveGraphics,
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
    Render = 0,
    Heat = 1,
    Instance = 2,
    Triangle = 3,
    Distance = 4,
    Albedo = 5,
    Metallic = 6,
    Roughness = 7,
    Normal = 8,
    Tangent = 9,
    Transmission = 10,
    GeoId = 11,
}

impl Gui {
    pub fn is_mapping(&self) -> bool {
        self.mapping != Mapping::Render
    }

    pub fn ray_query(&self) -> bool {
        self.mapping != Mapping::Render && self.mapping != Mapping::Heat
    }

    pub fn get_number_of_samples(
        &mut self,
        total_number_of_samples: u32,
        frame_time: Duration,
    ) -> u32 {
        if self.dynamic_samples {
            if frame_time.as_secs_f32() >= BUDGET && self.number_of_samples > 1 {
                self.number_of_samples -= 1;
            } else {
                self.number_of_samples += 1;
            }
        }
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
            aperture: 0.0,
            focus_distance: 10.0,
            number_of_samples: 3,
            dynamic_samples: false,
            number_of_bounces: 5,
            ray_tracing: true,
            acc: true,
            map_scale: 1.0,
            max_number_of_samples: 5000,
            sky: false,
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

            point_light_intensity: 2.0,
            point_light_radius: 10.0,
            orthographic_fov_dis: 0.0,
            exposure: 5.0,
            selected_tone_map_mode: 0,
        })
    }

    fn build(&mut self, ui: &Ui) {
        ui.window("Vulkan RT")
            .size([400.0, 600.0], Condition::FirstUseEver)
            .bg_alpha(0.5)
            .build(|| {
                // RT controls
                ui.text_wrapped("Rays");

                let mut number_of_samples = self.number_of_samples as _;
                ui.input_int("Number of samples", &mut number_of_samples)
                    .build();
                self.number_of_samples = number_of_samples.unsigned_abs();
                if ui.radio_button_bool(
                    format!("Dynamic sampling(target: {}fps)", FPS as u32),
                    self.dynamic_samples,
                ) {
                    self.dynamic_samples = !self.dynamic_samples;
                }

                let mut max_number_of_samples = self.max_number_of_samples as _;
                ui.input_int("Max Number of samples", &mut max_number_of_samples)
                    .build();
                self.max_number_of_samples = max_number_of_samples.unsigned_abs();

                let mut number_of_bounces = self.number_of_bounces as _;
                ui.input_int("Max Number of bounces", &mut number_of_bounces)
                    .build();
                self.number_of_bounces = number_of_bounces.unsigned_abs();

                let mut debug_number = self.debug as _;
                ui.input_int("Debug control", &mut debug_number).build();
                self.debug = debug_number.unsigned_abs();
                ui.slider("scale", -40., 40., &mut self.scale);
                ui.slider("Apertures", 0., 1., &mut self.aperture);
                ui.slider("Focus", 0.1, 20., &mut self.focus_distance);
                ui.slider("Orthographic", 0., 100., &mut self.orthographic_fov_dis);
                ui.slider("Exposure", 0.1, 10., &mut self.exposure);

                let mut scenes: Vec<_> = Scene::iter().collect();
                scenes.sort_by_key(|k| k.as_ref().to_string());
                let mut selected = self.scene.clone();
                if ui.begin_combo("Scene", selected.as_ref()).is_some() {
                    for cur in scenes.iter() {
                        if &selected == cur {
                            // Auto-scroll to selected item
                            ui.set_item_default_focus();
                        }
                        // Create a "selectable"
                        let clicked = ui.selectable_config(cur).selected(&selected == cur).build();
                        // When item is clicked, store it
                        if clicked {
                            selected = cur.clone();
                        }
                    }
                    self.scene = selected;
                }

                ui.separator();
                let _tone_map_mode_changed = ui.combo(
                    "Tone Map mode",
                    &mut self.selected_tone_map_mode,
                    &ToneMapMode::all(),
                    |mode| Cow::Owned(format!("{mode:?}")),
                );
                ui.separator();
                let mut selected = self.mapping;
                if ui.begin_combo("Mapping", selected.as_ref()).is_some() {
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
                    Mapping::Heat => ui.slider("Heatmap Scale", 0.1, 10., &mut self.map_scale),
                    Mapping::Distance => {
                        ui.slider("dis_map Scale", 10., 1000., &mut self.map_scale)
                    }
                    _ => false,
                };

                // ui.slider("Virtual light intensity",0., 2.0, &mut self.light_intensity);
                // Light control
                // ui.text_wrapped("Light");
                ui.separator();
                // ui.input_float3("direction", &mut self.light.direction)
                //     .build();
                let mut selected = self.skybox;
                if ui.begin_combo("Skybox", selected.as_ref()).is_some() {
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

                ui.slider(
                    "Point light intensity",
                    0.,
                    2.,
                    &mut self.point_light_intensity,
                );
                ui.slider(
                    "Point light distance",
                    1.,
                    100.,
                    &mut self.point_light_radius,
                );
                // let [r, g, b, _] = self.sun.color.to_array();
                // let mut color = [r, g, b];
                // ui.color_picker3_config("color", &mut color)
                //     .display_rgb(true)
                //     .build();
                // self.sun.update_color([color[0], color[1], color[2], 0.]);
            });
    }
}
