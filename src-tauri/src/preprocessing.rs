/// Preprocessing functions for input data for the CLIP model.

use std::path::{Path, PathBuf};
use image::{imageops::FilterType, DynamicImage, GenericImageView};
use ndarray::{Array, Array2, Dim};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

pub const IMAGE_INPUT_SIZE: usize = 336;
pub const CONTEXT_LENGTH: usize = 77;
pub const FEATURE_VECTOR_LENGTH: usize = 768;

pub fn load_image(path: &Path) -> Array<f32, Dim<[usize; 4]>>
{
	load_image_batch(&[path.to_path_buf()].to_vec())
}

// TODO This is fallible for image::open(), reflect that and return a result
// TODO Handle skip;ping unsupported file types?  Related to the above I guess. But I think we want to succesfully load the others, and just maybe warn here?
pub fn load_image_batch(paths: &Vec<PathBuf>) -> Array<f32, Dim<[usize; 4]>>
{
	let mut image_input = Array::zeros((paths.len(), 3, IMAGE_INPUT_SIZE, IMAGE_INPUT_SIZE));

	// Load the images in parallel
	println!("Loading images...");
	let images:  Vec<DynamicImage> = paths.par_iter().map(
	{
		| path |
		{
			image::open(path).unwrap()
		}
	}).collect::<Vec<DynamicImage>>();

	// Resize the images in parallel
	println!("Resizing images...");
	let resized_images = images.par_iter().map(
	{
		|original_img|
		{
			let img = original_img.resize(IMAGE_INPUT_SIZE as u32, IMAGE_INPUT_SIZE as u32, FilterType::CatmullRom);
			img
		}
	}).collect::<Vec<DynamicImage>>();

	// Convert the images to arrays in parallel
	println!("Converting images to arrays...");
	for (idx, img) in resized_images.iter().enumerate()
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

	print!("Done pre-processing image batch.");
    image_input
}

pub fn tokenize(text: &str) -> Array2<i32>
{
	tokenize_batch([text].to_vec())
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