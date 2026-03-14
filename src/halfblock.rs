//! Half-block image rendering for terminals with truecolor support.
//!
//! Uses `▀` (upper half block) with foreground/background colors to render
//! two vertical pixels per terminal cell, achieving 2x vertical resolution.

#[cfg(feature = "image")]
use image::DynamicImage;

use crate::style::Color;

/// A terminal-renderable image stored as a grid of [`Color`] values.
///
/// Each cell contains a foreground color (upper pixel) and background color
/// (lower pixel), rendered using the `▀` half-block character.
///
/// Create from an [`image::DynamicImage`] with [`HalfBlockImage::from_dynamic`]
/// (requires `image` feature), or construct manually from raw RGB data with
/// [`HalfBlockImage::from_rgb`].
pub struct HalfBlockImage {
    /// Width in terminal columns.
    pub width: u32,
    /// Height in terminal rows (each row = 2 image pixels).
    pub height: u32,
    /// Row-major pairs of (upper_color, lower_color) for each cell.
    pub pixels: Vec<(Color, Color)>,
}

#[cfg(feature = "image")]
impl HalfBlockImage {
    /// Create a half-block image from a [`DynamicImage`], resized to fit
    /// the given terminal cell dimensions.
    ///
    /// The image is resized to `width x (height * 2)` pixels using Lanczos3
    /// filtering, then each pair of vertically adjacent pixels is packed
    /// into one terminal cell.
    pub fn from_dynamic(img: &DynamicImage, width: u32, height: u32) -> Self {
        let pixel_height = height * 2;
        let resized = img.resize_exact(width, pixel_height, image::imageops::FilterType::Lanczos3);
        let rgba = resized.to_rgba8();

        let mut pixels = Vec::with_capacity((width * height) as usize);
        for row in 0..height {
            for col in 0..width {
                let upper_y = row * 2;
                let lower_y = row * 2 + 1;

                let up = rgba.get_pixel(col, upper_y);
                let lo = rgba.get_pixel(col, lower_y);

                let upper = Color::Rgb(up[0], up[1], up[2]);
                let lower = Color::Rgb(lo[0], lo[1], lo[2]);
                pixels.push((upper, lower));
            }
        }

        Self {
            width,
            height,
            pixels,
        }
    }
}

impl HalfBlockImage {
    /// Create a half-block image from raw RGB pixel data.
    ///
    /// `rgb_data` must contain `width x pixel_height x 3` bytes in row-major
    /// RGB order, where `pixel_height = height * 2`.
    pub fn from_rgb(rgb_data: &[u8], width: u32, height: u32) -> Self {
        let pixel_height = height * 2;
        let stride = (width * 3) as usize;
        let mut pixels = Vec::with_capacity((width * height) as usize);

        for row in 0..height {
            for col in 0..width {
                let upper_y = (row * 2) as usize;
                let lower_y = (row * 2 + 1) as usize;
                let x = (col * 3) as usize;

                let (ur, ug, ub) = if upper_y < pixel_height as usize {
                    let offset = upper_y * stride + x;
                    if offset + 2 < rgb_data.len() {
                        (rgb_data[offset], rgb_data[offset + 1], rgb_data[offset + 2])
                    } else {
                        (0, 0, 0)
                    }
                } else {
                    (0, 0, 0)
                };

                let (lr, lg, lb) = if lower_y < pixel_height as usize {
                    let offset = lower_y * stride + x;
                    if offset + 2 < rgb_data.len() {
                        (rgb_data[offset], rgb_data[offset + 1], rgb_data[offset + 2])
                    } else {
                        (0, 0, 0)
                    }
                } else {
                    (0, 0, 0)
                };

                pixels.push((Color::Rgb(ur, ug, ub), Color::Rgb(lr, lg, lb)));
            }
        }

        Self {
            width,
            height,
            pixels,
        }
    }
}
