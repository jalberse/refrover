/// Preprocessing functions for input data for the CLIP model.
/// Do not use these functions for any other purpose (for example,
/// to load images for purposes other than CLIP encoding).

use std::path::PathBuf;
use image::{imageops::FilterType, DynamicImage, GenericImageView};
use ndarray::{Array, Array2, Dim};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

pub const IMAGE_INPUT_SIZE: usize = 336;
pub const CONTEXT_LENGTH: usize = 77;
pub const FEATURE_VECTOR_LENGTH: usize = 768;

// TODO This is fallible for image::open(), reflect that and return a result
// TODO Handle skip;ping unsupported file types?  Related to the above I guess. But I think we want to succesfully load the others, and just maybe warn here?
pub fn load_image_batch(paths: &Vec<PathBuf>) -> Array<f32, Dim<[usize; 4]>>
{
	let mut image_input = Array::zeros((paths.len(), 3, IMAGE_INPUT_SIZE, IMAGE_INPUT_SIZE));

	// Load the images in parallel
	let images:  Vec<DynamicImage> = paths.par_iter().map(
	{
		| path |
		{
			let img = image::open(path);
			match img
			{
				Ok(img) => img,
				Err(e) =>
				{
					// TODO We'll want proper error handling here; it becomes a bit complex since we assume
					//   that indices will match from the result of this to the calling code's list of file IDs,
					//   so we'll actually want to take a vec of some struct that holds the file ID + path, and filter + return that list
					//   without the ones that error out.
					//   For dev purposes, I'm just printing the error here so I can test the ONNX functionality is working.
					//   I have a ticket open for this kind of error handling TODO.
					//   So we'll end up with some silly blank images in the batch for now which could have some odd feature vectors.
					println!("Error loading image: {:?} {:?}", path , e);
					DynamicImage::new_rgb8(IMAGE_INPUT_SIZE as u32, IMAGE_INPUT_SIZE as u32)
				}
			}
		}
	}).collect::<Vec<DynamicImage>>();

	// Resize the images in parallel
	let resized_images = images.par_iter().map(
	{
		|original_img|
		{
			let img = original_img.resize(IMAGE_INPUT_SIZE as u32, IMAGE_INPUT_SIZE as u32, FilterType::CatmullRom);
			img
		}
	}).collect::<Vec<DynamicImage>>();

	// Convert the images to arrays in parallel
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