use super::*;

impl World {
    pub(crate) fn maintain(&mut self) {
        let frame = self.frame;
        let w = self.w as i32;

        self.update_pressure();

        // ---- Earthquakes ----
        //
        // The ground moves now and then: dust falls, weak rock comes
        // down and the shape of the caves changes under the colonies.
        if frame % 600 == 0 && frame > 0 && self.rng.chance(0.05 + self.pressure.extremes * 0.06) {
            let ex = self.rng.range(w as u32) as i32;
            let ey = self.surface_y(ex) + 20 + self.rng.range(40) as i32;
            let radius = 40 + self.rng.range(30) as i32;
            self.tremor(ex, ey, radius);
        }

        // ---- The seasons ----
        //
        // Summer dries the world out and winter freezes it over. Both change
        // what a colony can rely on.
        if frame % 120 == 0 {
            match self.season {
                // Summer: the shallow water at the edge of a lake dries up.
                1 => {
                    for _ in 0..6 {
                        let x = self.rng.range(w as u32) as i32;
                        let sy = self.surface_y(x);
                        for y in (sy - 6..sy).rev() {
                            if self.get(x, y) == Elem::Water {
                                self.set_xy(x, y, Elem::Empty);
                                break;
                            }
                        }
                    }
                }
                // Winter: the surface freezes over.
                3 => {
                    for _ in 0..4 {
                        let x = self.rng.range(w as u32) as i32;
                        let sy = self.surface_y(x);
                        for y in (sy - 4..sy).rev() {
                            if self.get(x, y) == Elem::Water {
                                self.set_xy(x, y, Elem::Ice);
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // ---- The volcano ----
        // The mountain keeps its own time. The deeper the pressure under it,
        // the sooner it goes off.
        self.tick_volcano();
        if self.volcano_state == Volcano::Grumbling
            && self.rng.chance(0.004 + self.pressure.volcanic * 0.02)
        {
            self.erupt();
        }

        // Keep the cached ground level fresh (the renderer reads it).
        if frame % 15 == 0 {
            self.update_ground();
        }

        // ---- Weather ----
        // Harshness pressure makes the weather turn violent: storms and
        // meteors become more likely, and the seasons swing wider.
        if self.weather_timer == 0 {
            let harsh = self.pressure.harshness;
            let roll = self.rng.range(100) as f32;
            // The season has a say in the weather: a fog belongs to the
            // wet months and a heatwave to the dry ones.
            let summer = self.season == 1;
            let winter = self.season == 3;
            self.weather = if roll < 34.0 - harsh * 16.0 {
                Weather::Clear
            } else if roll < 58.0 - harsh * 6.0 {
                Weather::Rain
            } else if roll < 68.0 + harsh * 10.0 {
                Weather::Storm
            } else if roll < 76.0 + harsh * 4.0 {
                Weather::Snow
            } else if roll < 84.0 {
                if winter {
                    Weather::Snow
                } else if summer {
                    Weather::Heatwave
                } else {
                    Weather::Fog
                }
            } else if roll < 92.0 {
                if summer {
                    Weather::Heatwave
                } else {
                    Weather::Fog
                }
            } else if roll < 96.0 {
                // The desert keeps its own weather: a sandstorm comes out
                // of the dry ground, and it comes more often in the heat.
                if summer {
                    Weather::Sandstorm
                } else {
                    Weather::Fog
                }
            } else {
                Weather::Meteor
            };
            self.weather_timer = 500 + self.rng.range(700);
        }
        self.weather_timer = self.weather_timer.saturating_sub(1);

        match self.weather {
            Weather::Rain | Weather::Storm => {
                // Rain falls from the sky (stormy weather falls harder).
                // It starts just above the local terrain rather than at the
                // top of the world, so it lands quickly instead of hanging
                // in the sky for hundreds of steps.
                let period = if self.weather == Weather::Storm { 15 } else { 30 };
                let want = if self.weather == Weather::Storm { 3 } else { 2 };
                if frame % period == 0 {
                    let mut added = 0;
                    let mut attempts = 0;
                    while added < want && attempts < 40 {
                        attempts += 1;
                        let x = self.rng.range(w as u32) as i32;
                        let surf = self.surface_y(x);
                        let y = surf - 1 - self.rng.range(6) as i32;
                        if y > 0 && self.get(x, y) == Elem::Empty {
                            let is_acid = self.weather == Weather::Storm && self.rng.chance(0.05);
                            let e = if is_acid { Elem::Acid } else { Elem::Water };
                            self.set_xy(x, y, e);
                            added += 1;
                        }
                    }
                }
                // Storms throw lightning bolts that scorch the ground.
                if self.weather == Weather::Storm && self.rng.chance(0.02) {
                    let x = self.rng.range(w as u32) as i32;
                    let surf = self.surface_y(x);
                    for dy in 0..6 {
                        let y = surf - dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Fire);
                        }
                    }
                }
            }
            Weather::Snow => {
                // A light dusting of snow on the surface.
                if frame % 60 == 0 {
                    let mut added = 0;
                    let mut attempts = 0;
                    while added < 1 && attempts < 20 {
                        attempts += 1;
                        let x = self.rng.range(w as u32) as i32;
                        let surf = self.surface_y(x);
                        let y = surf - 1 - self.rng.range(4) as i32;
                        if y > 0 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Ice);
                            added += 1;
                        }
                    }
                }
            }
            Weather::Fog => {
                // A fog settles on the low ground: the hollows fill with
                // it first, and the water underneath goes bad.
                if frame % 90 == 0 {
                    for _ in 0..3 {
                        let x = self.rng.range(w as u32) as i32;
                        let surf = self.surface_y(x);
                        let y = surf + self.rng.range(3) as i32;
                        if y > 1 && y < self.h as i32 - 6 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Smoke);
                        }
                    }
                }
            }
            Weather::Heatwave => {
                // The sun beats down on the surface and the shallow water
                // boils off. The deep is untouched.
                if frame % 70 == 0 {
                    for _ in 0..2 {
                        let x = self.rng.range(w as u32) as i32;
                        let surf = self.surface_y(x);
                        let y = surf - 1 - self.rng.range(3) as i32;
                        if y > 1 && self.get(x, y) == Elem::Water {
                            self.set_xy(x, y, Elem::Steam);
                        }
                    }
                }
            }
            Weather::Meteor => {
                // Meteors streak down and explode where they land. Harsh
                // weather makes them more frequent.
                if self.rng.chance(0.005 + self.pressure.harshness * 0.02) {
                    let x = self.rng.range(w as u32) as i32;
                    let surf = self.surface_y(x);
                    let impact_y = surf + self.rng.range(4) as i32;
                    for dy in 0..10 {
                        let y = surf - dy;
                        if self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Fire);
                        }
                    }
                    // Impact crater + scorched earth.
                    for dx in -2..=2 {
                        for dy in -1..=1 {
                            let (cx, cy) = (x + dx, impact_y + dy);
                            if self.get(cx, cy) == Elem::Empty {
                                self.set_xy(cx, cy, Elem::Lava);
                            }
                        }
                    }
                    self.meteor_events.push((x as f32, impact_y as f32, 22));
                }
            }
            Weather::Sandstorm => {
                // The wind carries the desert itself across the world: dust
                // streams over the ground and the shallow water is drunk
                // by the air.
                if frame % 24 == 0 {
                    for _ in 0..4 {
                        let x = self.rng.range(w as u32) as i32;
                        let surf = self.surface_y(x);
                        let y = surf - 1 - self.rng.range(5) as i32;
                        if y > 0 && self.get(x, y) == Elem::Empty {
                            self.set_xy(x, y, Elem::Dust);
                        }
                    }
                }
                if frame % 45 == 0 {
                    for _ in 0..2 {
                        let x = self.rng.range(w as u32) as i32;
                        let surf = self.surface_y(x);
                        let y = surf - 1 - self.rng.range(2) as i32;
                        if y > 1 && self.get(x, y) == Elem::Water {
                            self.set_xy(x, y, Elem::Steam);
                        }
                    }
                }
            }
            Weather::Clear => {}
        }

        // Underground springs - water seeping up from aquifers (rare)
        if frame % 200 == 0 {
            for _ in 0..1 {
                let x = self.rng.range(w as u32) as i32;
                let y = self.h as i32 - 2 - self.rng.range(20) as i32;
                if self.get(x, y) == Elem::Empty && self.get(x, y + 1).is_solid() {
                    self.set_xy(x, y, Elem::Water);
                }
            }
        }

        // Surface water bodies maintenance - keep lakes from drying (very rare, minimal)
        if frame % 1000 == 0 {
            for _ in 0..1 {
                let x = self.rng.range(w as u32) as i32;
                let sy = self.surface_y(x);
                // The water goes into the ground, not on top of it.
                if self.get(x, sy) == Elem::Stone && self.rng.chance(0.1) {
                    self.set_xy(x, sy, Elem::Water);
                }
            }
        }

        // Vegetation regrowth. Slower and pressure-sensitive: plants establish
        // near water, so over-grazing causes real scarcity that then recovers.
        // Scarcity pressure cuts the number of seedlings that survive.
        if frame % 15 == 0 && self.census.plants < 4000 {
            let scarcity = self.pressure.scarcity;
            let base = if self.census.plants < 1200 {
                24
            } else if self.census.plants < 3000 {
                12
            } else {
                6
            };
            // The season decides how much grows: spring is the green time,
            // winter is the lean one.
            let season_growth = match self.season {
                0 => 1.8,
                1 => 1.0,
                2 => 0.9,
                _ => 0.3,
            };
            // Under pressure, fewer seedlings take root.
            let batches = ((base as f32) * (1.0 - scarcity * 0.75) * season_growth) as u32;
            for _ in 0..batches {
                let x = self.rng.range((w - 4) as u32) as i32 + 2;
                let sy = self.surface_y(x);
                // Plants grow on the ground, not floating above it and not
                // buried under it: the seedling takes the first free cell
                // above the surface, and a bush grows one cell higher on
                // top of it.
                let mut y = sy - 1;
                if self.get(x, y) != Elem::Empty {
                    continue;
                }
                // Slightly favour spots near moisture.
                if !self.has_water_near(x, y, 5) && !self.rng.chance(0.5) {
                    continue;
                }
                let e = if self.rng.chance(0.12) {
                    Elem::Spore
                } else if self.rng.chance(0.10) {
                    Elem::Fungus
                } else {
                    Elem::Plant
                };
                self.set_xy(x, y, e);
                // A thicket: sometimes a second plant on top of the first.
                if self.rng.chance(0.3) && self.get(x, y - 1) == Elem::Empty {
                    self.set_xy(x, y - 1, e);
                }
            }
        }

        // Hard cap: thin out vegetation if it starts smothering the world.
        // NOTE: compare decoded elements, never raw byte ranges.
        if frame % 20 == 0 && self.census.plants > 2000 {
            // The thicker the mat, the harder it is thinned: a world that is
            // only a little over the line loses a few plants, a world that
            // has run away with the surface loses a lot.
            let over = (self.census.plants - 2000) as f32 / 1000.0;
            let n = (60.0 + over * 240.0) as u32;
            for _ in 0..n {
                let i = self.rng.range((self.w * self.h) as u32) as usize;
                if matches!(
                    Elem::from_u8(self.kind[i]),
                    Elem::Plant | Elem::Spore | Elem::Fungus | Elem::Moss
                ) {
                    self.set_at(i, Elem::Empty);
                }
            }
        }

        // Food drifting just above the ground
        if frame % 25 == 0 && self.census.energy < 150 {
            for _ in 0..4 {
                let x = self.rng.range(w as u32) as i32;
                let sy = self.surface_y(x).max(14);
                // Energy hovers a little above the ground, which is what makes
                // it visible as a drifting mote.
                let y = sy - 1 - self.rng.range(4) as i32;
                if y > 0 && self.get(x, y) == Elem::Empty {
                    self.set_xy(x, y, Elem::Energy);
                }
            }
        }

        // Resource spawns near surface
        if frame % 50 == 0 {
            // Oil seeps
            if self.census.oil < 200 {
                for _ in 0..3 {
                    let x = self.rng.range(w as u32) as i32;
                    let sy = self.surface_y(x);
                    let y = sy + 2 + self.rng.range(8) as i32;
                    if self.get(x, y) == Elem::Empty {
                        self.set_xy(x, y, Elem::Oil);
                    }
                }
            }
            // Sulfur vents near lava
            if self.census.sulfur < 150 {
                for _ in 0..2 {
                    let x = self.rng.range(w as u32) as i32;
                    let y = self.h as i32 - 5 - self.rng.range(10) as i32;
                    if self.get(x, y) == Elem::Empty && self.get(x, y + 1) == Elem::Lava {
                        self.set_xy(x, y, Elem::Sulfur);
                    }
                }
            }
        }

        // Meteor strike. The impact site is on the *surface* of the world, so
        // the crater is placed relative to the terrain height. Using absolute
        // row numbers here dropped lava into the sky, where it fell back down
        // and pooled on the surface.
        if frame % 1800 == 900 {
            let cx = self.rng.range_i(8, w - 8);
            let surf = self.surface_y(cx);
            for dy in 0..6 {
                for dx in -6..7 {
                    if dx * dx + dy * dy <= 30 {
                        let x = cx + dx;
                        let y = surf + dy;
                        // The impact melts the ground it hits. Only solid
                        // cells are affected, so the crater never spills into
                        // the sky.
                        if self.get(x, y).is_solid() {
                            self.set_xy(x, y, Elem::Lava);
                        }
                    }
                }
            }
            // A column of fire marks the path of the meteor.
            for dy in 1..10 {
                let y = surf - dy;
                if self.get(cx, y) == Elem::Empty {
                    self.set_xy(cx, y, Elem::Fire);
                }
            }
        }

        // Geological: slow stone formation from mineral deposition
        if frame % 100 == 0 {
            for _ in 0..8 {
                let x = self.rng.range(self.w as u32) as i32;
                let y = self.rng.range(self.h as u32) as i32;
                if self.get(x, y) == Elem::Empty && self.get(x, y + 1).is_solid() {
                    self.set_xy(x, y, Elem::Stone);
                }
            }
        }

        // ---- The deep seeps ----
        //
        // The ZORG cave is the larder of the deep, and a larder has to be
        // restocked: the poison on its floor seeps back out of the rock.
        // Without this the ZORGs would eat the cave bare and then starve
        // in the hole they had made.
        if frame % 10 == 0 {
            self.regrow_zorg_larder();
        }

        // Slow ore genesis. The world's metal is finite: every vein that forms
        // brings the depletion pressure closer to the end of mining, and a
        // depleted world stops replacing what the colony digs out.
        if frame % 250 == 0 {
            let depletion = self.pressure.depletion;
            // A heavily depleted world forms almost no new ore.
            let attempts = ((3.0 * (1.0 - depletion)) as u32).max(1);
            for _ in 0..attempts {
                let x = self.rng.range(self.w as u32) as i32;
                let sy = self.surface_y(x);
                let y = sy + 8 + self.rng.range((self.h as i32 - sy - 10).max(1) as u32) as i32;
                if y < self.h as i32 - 1 && self.get(x, y) == Elem::Stone {
                    let ore = match self.rng.range(6) {
                        0 => Elem::Copper,
                        1 => Elem::Iron,
                        2 => Elem::Gold,
                        3 => Elem::Silver,
                        4 => Elem::Crystal,
                        _ => Elem::Tin,
                    };
                    self.set_xy(x, y, ore);
                }
            }
        }

        // Crystal growth in caves
        if frame % 200 == 0 && self.census.crystal < 300 {
            for _ in 0..3 {
                let x = self.rng.range(self.w as u32) as i32;
                let y = self.rng.range(self.h as u32) as i32;
                if self.get(x, y) == Elem::Empty && self.has_water_near(x, y, 2) {
                    self.set_xy(x, y, Elem::Crystal);
                }
            }
        }

        // Volcanic pressure: the deep rock heats up. Magma creeps further up
        // the shafts as the run goes on, so the deep mines get more dangerous
        // the longer a colony stays alive.
        if frame % 600 == 0 && frame > 0 {
            let volcanic = self.pressure.volcanic;
            if volcanic > 0.05 && self.rng.chance(volcanic * 0.5) {
                let x = self.rng.range(w as u32) as i32;
                // Start from the deepest rock and creep upwards.
                let mut y = self.h as i32 - 8;
                while y > self.surface_y(x) + 20 {
                    if self.get(x, y) == Elem::Stone {
                        self.set_xy(x, y, Elem::Magma);
                        break;
                    }
                    y -= 1;
                }
            }
        }

        // The streaks of the meteors that came down fade out.
        for m in self.meteor_events.iter_mut() {
            m.2 = m.2.saturating_sub(1);
        }
        self.meteor_events.retain(|m| m.2 > 0);

        // Trim runaway particle counts (firestorms can fill the sky)
        if frame % 300 == 0 {
            let total = self.census.particles as usize;
            if total > (self.w * self.h * 85) / 100 {
                self.compact();
            }
        }
    }

    /// Advance the environmental pressure.
    ///
    /// Each pressure rises for its own reason and is measured against its own
    /// part of the world, so the difficulty curve is the sum of several
    /// independent trends rather than one number scaled up.
    pub(crate) fn update_pressure(&mut self) {
        let frame = self.frame;
        let p = &mut self.pressure;

        // --- Scarcity: the surface is picked clean ------------------------
        // Measured against the plant population, but *relative* to what this
        // world has recently been able to support. The capacity is a
        // high-water mark that decays slowly, so:
        //   - a world that is grazed below its own capacity feels scarcity;
        //   - a world that is genuinely dry has a low capacity and does not
        //     spiral into permanent famine for having few plants;
        //   - the pressure eases again once the vegetation recovers.
        let plants = self.census.plants as f32;
        // Learn the capacity: climb quickly to the peak, forget it slowly.
        if plants > self.plant_capacity {
            self.plant_capacity = plants;
        } else if frame % 600 == 0 {
            self.plant_capacity = (self.plant_capacity * 0.995).max(plants);
        }
        // Fraction of the world's own capacity that is still standing.
        let cover = if self.plant_capacity > 50.0 {
            (plants / self.plant_capacity).clamp(0.0, 1.0)
        } else {
            1.0
        };
        if cover < 0.6 {
            // Below 60% cover the surface is being eaten into. The rate scales
            // with how far below the threshold the cover has fallen, so a
            // lightly grazed world barely notices and a stripped one hurts.
            p.scarcity = (p.scarcity + (0.6 - cover) * 0.0012).min(1.0);
        } else if cover > 0.85 {
            // Above 85% cover the surface is healthy again.
            p.scarcity = (p.scarcity - 0.0006).max(0.0);
        }

        // --- Depletion: the ore is finite ---------------------------------
        // Every ore genesis event is one step closer to the end of the
        // world's metal, so the pressure only ever rises.
        if frame % 250 == 0 {
            p.depletion = (p.depletion + 0.012).min(1.0);
        }

        // --- Harshness: the weather turns violent --------------------------
        // Driven by the violent weather that actually happens. The world gets
        // harsher the more time it spends being battered: storms wear the
        // climate down faster than meteor showers, and a clear spell lets it
        // recover a little.
        //
        // This is a feedback loop with a real mechanism: harsh weather
        // makes storms more likely (see the weather roll), and storms make
        // the world harsher.
        let severity = match self.weather {
            Weather::Storm => 0.00055,
            Weather::Meteor => 0.00030,
            Weather::Snow => 0.00008,
            _ => 0.0,
        };
        if severity > 0.0 {
            p.harshness = (p.harshness + severity).min(1.0);
        } else if self.weather == Weather::Clear {
            p.harshness = (p.harshness - 0.00018).max(0.0);
        }

        // --- Extremes: the seasons swing wider -----------------------------
        // Rises with each season that turns over, so a long run has harsher
        // winters and fiercer summers than a short one.
        if frame % 7200 == 0 && frame > 0 {
            p.extremes = (p.extremes + 0.25).min(1.0);
        }

        // --- Volcanic: the deep rock heats up ------------------------------
        // Rises steadily. The deeper you go, the more lava you meet.
        if frame % 1200 == 0 && frame > 0 {
            p.volcanic = (p.volcanic + 0.09).min(1.0);
        }

        // --- The difficulty ramp -------------------------------------------
        //
        // The world starts forgiving and gets harder, for two reasons.
        //
        // The first is the run itself: a colony that has just begun has
        // creatures that have not yet worked out how to feed themselves,
        // and a world that is cruel from the first step kills every line
        // before any of them can learn. The second is the environment: the
        // pressures above all rise with time.
        //
        // So the difficulty is the run's own progress, plus what the world
        // has become. A young world in good condition is gentle; a world
        // that has been picked clean and battered is not.
        //
        // The ramp is slow on purpose. A world that gets cruel too early
        // kills the colony before it has had time to learn how to live in
        // it, and then there is nothing left to evolve.
        let progress = (frame as f32 / 60_000.0).min(1.0);
        let strain = p.overall();
        self.difficulty = (progress * 0.6 + strain * 0.4).clamp(0.0, 1.0);
    }

    /// How hard the world is on a creature right now.
    ///
    /// This is what the hunger, the thirst and the deep's hunger for the
    /// surface all read. At the start of a run it is near zero and the
    /// world is gentle; by the end of a long run it is near one and
    /// nothing is easy any more.
    pub fn hardship(&self) -> f32 {
        self.difficulty
    }

    // Removes some airborne noise so the world cannot jam completely.
    pub(crate) fn compact(&mut self) {
        for _ in 0..(self.w * self.h / 20) {
            let x = self.rng.range(self.w as u32) as i32;
            let y = self.rng.range(self.h as u32) as i32;
            let e = self.get(x, y);
            if matches!(e, Elem::Smoke | Elem::Steam | Elem::Gas | Elem::Fire) {
                self.set_xy(x, y, Elem::Empty);
            }
        }
    }

}
