//! The renderer that paints the picture on the processor.
//!
//! It walks the region one cell at a time and writes the colour
//! of every cell into a buffer, which is then handed to the
//! graphics card to be drawn. It is the reference: the renderer on
//! the card has to paint exactly the same picture.

use super::fx;
use super::{bloom, clamp_u8, is_shore, noise, sample_cloud, sky_state, smoothstep};
use super::{Clock, PaintGeometry, PaintTables, Region, SkyState, Tables, GSX, GSY};

mod paint;

use crate::creature::CreatureSystem;
use crate::world::World;

/// The renderer that paints the picture on the processor.
pub struct CpuRenderer {
    tables: Tables,
}

impl CpuRenderer {
    #[must_use]
    pub fn new() -> Self {
        CpuRenderer {
            tables: Tables::new(),
        }
    }

    pub fn render_world(
        &mut self,
        world: &World,
        creatures: &CreatureSystem,
        buf: &mut [u8],
        clock: Clock,
    ) {
        self.render_region(
            world,
            creatures,
            buf,
            Region {
                x: 0,
                y: 0,
                w: world.w as i32,
                h: world.h as i32,
            },
            clock,
        );
    }

    pub fn render_region(
        &mut self,
        world: &World,
        _creatures: &CreatureSystem,
        buf: &mut [u8],
        region: Region,
        clock: Clock,
    ) {
        let (lw, lh, gx0, gy0) = self.tables.build(world, region, &clock);

        let tables = PaintTables {
            cloud_row: &self.tables.cloud_row,
            fx: &self.tables.fx,
            light_rows: &self.tables.light_rows,
            aurora_col: &self.tables.aurora_col,
            biome_delta: &self.tables.biome_delta,
            cloud_shadow: &self.tables.cloud_shadow,
            sun_shaft: &self.tables.sun_shaft,
            vig_col: &self.tables.vig_col,
            bloom: &self.tables.bloom,
            sky_base: &self.tables.sky_base,
            sky_slope: &self.tables.sky_slope,
        };
        let geometry = PaintGeometry {
            region,
            light_w: lw,
            light_h: lh,
            light_x: gx0,
            light_y: gy0,
        };

        paint::paint(world, buf, &tables, &geometry, &clock);
    }
}

impl Default for CpuRenderer {
    fn default() -> Self {
        Self::new()
    }
}
