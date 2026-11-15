//! The pictures of the creatures and of the effects, and the drawing of
//! them.
//!
//! The pictures are made in `render::sprites` and put in one atlas.
//! This module keeps the atlas on the card and draws a picture out of
//! it.

use crate::render::sprites;
use macroquad::prelude::*;

/// The atlas of the pictures, put on the card once and kept.
pub fn atlas_texture() -> &'static Texture2D {
    use std::sync::OnceLock;
    static ATLAS: OnceLock<Texture2D> = OnceLock::new();
    ATLAS.get_or_init(|| {
        let image = Image {
            bytes: sprites::atlas().to_vec(),
            width: sprites::ATLAS_W as u16,
            height: sprites::ATLAS_H as u16,
        };
        let texture = Texture2D::from_image(&image);
        texture.set_filter(FilterMode::Linear);
        texture
    })
}

/// Draw one picture of the atlas, in the middle of the given place.
pub fn draw_sprite(tile: usize, sx: f32, sy: f32, size: f32, rot: f32, tint: Color) {
    let (tx, ty) = sprites::tile_origin(tile);
    let pic = sprites::SPRITE as f32;
    draw_texture_ex(
        atlas_texture(),
        sx - size * 0.5,
        sy - size * 0.5,
        tint,
        DrawTextureParams {
            source: Some(Rect {
                x: tx,
                y: ty,
                w: pic,
                h: pic,
            }),
            dest_size: Some(vec2(size, size)),
            rotation: rot,
            ..Default::default()
        },
    );
}
