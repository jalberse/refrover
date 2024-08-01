/// Preprocessing functions for input data for the CLIP model.

use std::path::Path;
use image::{imageops::FilterType, GenericImageView};
use ndarray::{Array, Dim};

pub const IMAGE_INPUT_SIZE: usize = 336;
pub const CONTEXT_LENGTH: usize = 77;

pub fn load_image(path: &Path) -> Array<f32, Dim<[usize; 4]>>
{
    let original_img = image::open(path).unwrap();
	// let (img_width, img_height) = (original_img.width(), original_img.height());
	let img = original_img.resize_exact(IMAGE_INPUT_SIZE as u32, IMAGE_INPUT_SIZE as u32, FilterType::CatmullRom);
	let mut image_input = Array::zeros((1, 3, IMAGE_INPUT_SIZE, IMAGE_INPUT_SIZE));
	for pixel in img.pixels() {
		let x = pixel.0 as _;
		let y = pixel.1 as _;
		let [r, g, b, _] = pixel.2.0;
		image_input[[0, 0, y, x]] = (r as f32) / 255.;
		image_input[[0, 1, y, x]] = (g as f32) / 255.;
		image_input[[0, 2, y, x]] = (b as f32) / 255.;
	}

    image_input
}
