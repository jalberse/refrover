/// Preprocessing functions for input data for the CLIP model.

use std::path::Path;
use image::{imageops::FilterType, GenericImageView};
use ndarray::{Array, Array2, Dim};

pub const IMAGE_INPUT_SIZE: usize = 336;
pub const CONTEXT_LENGTH: usize = 77;

// TODO This should instead load a BATCH of images.
// See the [0, 1, y, x] eg - the 0 refers to the image index in the batch.

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

pub fn tokenize_batch(text: Vec<&str>) -> Array2<i32>
{
	// TODO Rather than initializing the tokenizer each time, Lazy load it.
	let tokenizer = instant_clip_tokenizer::Tokenizer::new();
	let tokens = tokenizer.tokenize_batch(text, CONTEXT_LENGTH);

	// Convert to i32 (for ONNX)
	let tokens = tokens.mapv(|x| x as i32);

	tokens
}