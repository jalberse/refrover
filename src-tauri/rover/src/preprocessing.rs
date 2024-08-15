/// Preprocessing functions for input data for the CLIP model.
/// Do not use these functions for any other purpose (for example,
/// to load images for purposes other than CLIP encoding).

use std::path::PathBuf;
use image::{imageops::FilterType, DynamicImage, GenericImageView};
use ndarray::{Array, Array2, Dim};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use uuid::Uuid;

pub const IMAGE_INPUT_SIZE: usize = 336;
pub const CONTEXT_LENGTH: usize = 77;
pub const FEATURE_VECTOR_LENGTH: usize = 768;

pub fn load_image_batch(paths: &Vec<(Uuid, PathBuf)>) -> Vec<(Uuid, anyhow::Result<Box<DynamicImage>>)>
{
	// Load the images in parallel
	let images = paths.par_iter().map(
	{
		| (uuid, path) |
		{
			let img = image::open(path);
			match img
			{
				Ok(img) => (*uuid, Ok(Box::new(img))),
				Err(e) =>
				{
					(*uuid, Err(anyhow::anyhow!("Error loading image: {:?} {:?}", path , e)))
				}
			}
		}
	}).collect::<Vec<(Uuid, anyhow::Result<Box<DynamicImage>>)>>();

	images
}

pub fn resize_images(images: Vec<(Uuid, Box<DynamicImage>)>) -> Vec<(Uuid, Box<DynamicImage>)>
{
	// Resize the images in parallel
	let resized_images = images.par_iter().map(
	{
		| (uuid, original_img) |
		{
			let img = original_img.as_ref()
				.resize(
					IMAGE_INPUT_SIZE as u32,
					IMAGE_INPUT_SIZE as u32,
					FilterType::CatmullRom);
			(*uuid, Box::new(img))
		}
	}).collect::<Vec<(Uuid, Box<DynamicImage>)>>();

	resized_images
}

// Convert the images to a 4D array expected by CLIP
pub fn image_to_clip_format(images: Vec<(Uuid, Box<DynamicImage>)>) -> Array<f32, Dim<[usize; 4]>>
{
	// Convert the images to arrays
	let mut image_input = Array::zeros((images.len(), 3, IMAGE_INPUT_SIZE, IMAGE_INPUT_SIZE));
	for (idx, (_, img)) in images.iter().enumerate()
	{
		for pixel in img.pixels() {
			let x = pixel.0 as _;
			let y = pixel.1 as _;
			let [r, g, b, _] = pixel.2.0;
			image_input[[idx, 0, y, x]] = (r as f32) / 255.;
			image_input[[idx, 1, y, x]] = (g as f32) / 255.;
			image_input[[idx, 2, y, x]] = (b as f32) / 255.;
		}
	}

	image_input
}

pub fn tokenize(text: &str, tokenizer: &instant_clip_tokenizer::Tokenizer) -> Array2<i32>
{
	tokenize_batch([text].to_vec(), tokenizer)
}

pub fn tokenize_batch(text: Vec<&str>, tokenizer: &instant_clip_tokenizer::Tokenizer) -> Array2<i32>
{
	let tokens = tokenizer.tokenize_batch(text, CONTEXT_LENGTH);

	// Convert to i32 (for ONNX)
	let tokens = tokens.mapv(|x| x as i32);

	tokens
}