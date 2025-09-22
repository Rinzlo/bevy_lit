mod light2d;
mod plugin;
mod post_process;
mod render;
mod settings;
mod shadows2d;

pub mod prelude {
    pub use crate::light2d::{
        point_light2d::PointLight2d,
        render::{CustomLight2dPlugin, Light2dMaterial},
        spot_light2d::SpotLight2d,
    };
    pub use crate::plugin::Lighting2dPlugin;
    pub use crate::settings::{
        AmbientLight2d, Lighting2dSettings, PenetrationSettings, RaymarchSettings,
    };
    pub use crate::shadows2d::LightOccluder2d;
}
