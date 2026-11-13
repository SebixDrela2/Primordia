//! User interface: HUD, minimap, knowledge window and effects.
//!
//! Split by responsibility:
//! - effects.rs    transient animations for every interaction
//! - hud.rs        the heads-up display and the ecology panel
//! - knowledge.rs  hover tooltips and the encyclopedia
//! - minimap.rs    minimap, weather overlay and vignette
//! - victory.rs    the victory / extinction screen
//! - actors.rs     creatures, projectiles and world overlays
//! - diplomacy.rs  the relations between species and their treaties
//! - tech.rs       the tech tree window and the tech panel

pub mod actors;
pub mod diplomacy;
pub mod effects;
pub mod hud;
pub mod knowledge;
pub mod minimap;
pub mod tech;
pub mod victory;

pub use actors::*;
pub use diplomacy::*;
pub use effects::*;
pub use hud::*;
pub use knowledge::*;
pub use minimap::*;
pub use tech::*;
pub use victory::*;
