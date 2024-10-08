use std::path::Path;

use diesel::{RunQueryDsl, SqliteConnection};
use image::DynamicImage;
use log::{info, trace, warn};
use ndarray::{Array, Array2, Dim, IxDyn, Axis};
use ort::{self, inputs, GraphOptimizationLevel};
use ort::DirectMLExecutionProvider;
use anyhow;

use crate::models::{NewFailedEncoding, NewImageFeaturesVitL14336Px};
use crate::preprocessing::{self, FEATURE_VECTOR_LENGTH};
use crate::uuid::UUID;
use crate::{ann, queries};
use crate::state::{ClipState, SearchState};

pub struct ForwardResults
{
    pub logits_per_image: Array<f32, IxDyn>,
    pub logits_per_text: Array<f32, IxDyn>,
}

/// The CLIP search model.
/// Supports 3 model functions: `encode_text()`, `encode_image()`, and `forward()`.
/// The encoding functions outputs the latent feature vectore of text and images.
/// The forward function outputs the similarity score between the text and image.
/// See the [OpenAI CLIP paper](https://arxiv.org/abs/2103.00020) for more details.
/// 
/// Note that our encode_text() and encode_image() functions also normalize the output feature vectors.
/// This differs from the CLIP implementation, which only performs that normalization in the forward() function.
/// We do this because our HNSW index assumes L2 normalized vectors, so that the cosine similarity is equivalent to the dot product, which is cheaper.
/// 
/// Uses an ONNX representation of the model so that it can be used in Rust,
/// and to execute the model on a wide variety of hardware using the ONNX runtime.
/// The ONNX representation is created by: https://github.com/jalberse/CLIP-to-onnx-converter
pub struct Clip
{
    visual_session: ort::Session,
    text_session: ort::Session,
    logit_scale: f32,
}

impl Clip
{
    pub fn new() -> Result<Self, ort::Error>
    {
        // TODO Switch to load-dynamic, possibly
        // TODO Ensure we can load models when shipping executables;
        //    We will ship the ONNX files.
        // TODO Right now I am using the download strategy (faster) to get the ONNX runtime.
        //    Switch to the load-dynamic strategy instead to avoid shared library hell.
        //    https://crates.io/crates/ort#strategies
        //    Ensure we've got it for all the target platforms....
        // TODO Ensure that the libonnxruntime.so is in the same directory as the executable when we ship it.
        //       Possibly need some build.rs stuff.
        //      including when we ship it.
        // TODO And ensure that the env variable for the process points to it:
        //      std::env::set_var("ORT_DYLIB_PATH", "./libonnxruntime.so");

        // TODO It looks like takes some time to load these. I think we'll want separate models so we can load them individually and asynchronously?
        //      They aren't actually that related to each other other than semantically, since they're separate ONNX graphs.
        //      Like, waiting a bit to initialize visual aint bad because that's mostly in the background.
        //      Text we do want to be fast + first since that's the main thing we're doing.
        //      The combined model we'll use for something like suggested tags, but that's it.
        //       It's advantageous to split these.
        //   We also definitely want to initiate the session once on startup and keep it around the whole process.

        // TODO When we package the app, we'll be copying the ONNX files to be local to the executable.
        //      That will change the path to the models (unless tauri is doing some magic with the path).
        //      So, we should update these accordingly.
        let visual_session = ort::Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .with_execution_providers([DirectMLExecutionProvider::default().build()])?
            .commit_from_file(Path::new(env!("CARGO_MANIFEST_DIR")).join("models").join("ViT-L_14_336px_visual.onnx"))?;

        let text_session = ort::Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .with_execution_providers([DirectMLExecutionProvider::default().build()])?
            .commit_from_file(Path::new(env!("CARGO_MANIFEST_DIR")).join("models").join("ViT-L_14_336px_transformer.onnx"))?;

        let logit_scale = f32::ln(1.0 / 0.01);

        Ok( Clip { visual_session, text_session, logit_scale } )
    }

    /// Given a batch of images, returns the image features encoded by the vision portion of the CLIP model.
    /// Use the preprocessing::load_image() function to load the image
    /// and convert it into an array for this input.
    /// 
    /// Returns a 2D array of shape (batch_size, FEATURE_VECTOR_LENGTH).
    pub fn encode_image(&self, images: Array<f32, Dim<[usize; 4]>>) -> anyhow::Result<Array2<f32>>
    {
        let images_len = images.len_of(Axis(0));
        if images_len == 0 {
            return Err(anyhow::anyhow!("No images to encode!"));
        }
        info!("Images len: {}", images_len);
        info!("Running visual session...");
        let outputs = self.visual_session.run(inputs![images]?)?;
        info!("Ran visual session");


        let output = &outputs["FEATURES_EMBEDDED"];

        // First dimension is for each image in the batch; the second is the feature vector per image.
        let output = output.try_extract_tensor::<f32>()?;

        let mut output = output.to_shape((images_len, FEATURE_VECTOR_LENGTH))?.to_owned();

        // For example:
        // With one image in the batch....
        // Output: [[0.51000077, ..., -0.37341008]],
        //    shape=[1, 768], strides=[768, 1], layout=CFcf (0xf), dynamic ndim=2
        // With two images in batch...
        // Output: [[0.51000077, ..., -0.37341008], [-0.061004326, ..., -0.3435823]],
        //    shape=[2, 768], strides=[768, 1], layout=Cc (0x5), dynamic ndim=2

        // Normalize the output feature vectors. Our HNSW index assumes L2 normalized vectors,
        // so that the cosine similarity is equivalent to the dot product, which is cheaper.
        Self::normalize_feature_vectors(&mut output);

        Ok(output)
    }

    /// Given a batch of text tokens, returns the text features encoded by the language portion of the CLIP model.
    /// Generate tokens using preprocessing::tokenize_batch().
    /// 
    /// Returns a 2D array of shape (batch_size, FEATURE_VECTOR_LENGTH).
    pub fn encode_text(&self, tokens: Array2<i32>) -> anyhow::Result<Array2<f32>>
    {
        if tokens.len_of(Axis(0)) == 0 {
            return Err(anyhow::anyhow!("No text to encode!"));
        }
        let tokens_len = tokens.len_of(Axis(0));
        let outputs = self.text_session.run(inputs![tokens]?)?;

        let output = &outputs["FEATURES_EMBEDDED"];

        // First dimension is for each text in the batch; the second is the feature vector per text.
        let output = output.try_extract_tensor::<f32>()?;

        let mut output: Array2<f32> = output.to_shape((tokens_len, FEATURE_VECTOR_LENGTH))?.to_owned();

        // Normalize the output feature vectors. Our HNSW index assumes L2 normalized vectors,
        // so that the cosine similarity is equivalent to the dot product, which is cheaper.
        Self::normalize_feature_vectors(&mut output);

        Ok(output)
    }

    /// Given a batch of images and a batch of text tokens, returns two Tensors,
    /// containing the logit scores corresponding to each image and text input.
    pub fn forward(&self, images: Array<f32, Dim<[usize; 4]>>, tokens: Array2<i32>) -> anyhow::Result<ForwardResults>
    {
        let image_features = self.encode_image(images)?;
        let text_features = self.encode_text(tokens)?;
    
        // Note that these are already normalized (this convention differs from CLIP)

        // cosine similarity as logits
        let logit_scale = self.logit_scale.exp();
        let logits_per_image = image_features.dot(&text_features.t()) * logit_scale;
        let logits_per_text = logits_per_image.t().to_owned();

        Ok(
            ForwardResults{
                logits_per_image: logits_per_image.into_dyn(),
                logits_per_text: logits_per_text.into_dyn(),
            }
        )
    }

    pub fn encode_image_files(&self, files: &[UUID], connection: &mut SqliteConnection) -> anyhow::Result<()>
    {
        use crate::schema::image_features_vit_l_14_336_px;
        use crate::schema::failed_encodings;
        
        let files = queries::get_filepaths(files, connection)?;

        info!("Encoding images with CLIP...");

        // Encode images with CLIP and add encodings to the database
        for chunk in files.chunks(32)
        {
            info!("Images in chunk: \n {:?}", chunk);
            // Load and preprocess our images.
            let images = preprocessing::load_image_batch(&chunk);
             
            info!("Loaded images");

            // Split images into those that succesfully loaded and those that failed.
            // Images may fail to load because they are not images, not found, etc.
            let (images, failed_images) = images.into_iter().partition::<Vec<_>, _>(|(_, img)| img.is_ok());
 
            info!("Successfully loaded images: {}", images.len());
            info!("Failed to load images: {}", failed_images.len());

            if images.is_empty()
            {
                warn!("No images to encode! Either you're throwing me a lot of junk data or there's likely a bug!");
            }
            else
            {
                // Handle images that succesfully loaded.
                // Unwrap the succesful images. This is safe because we just partitioned.
                let images: Vec<(UUID, Box<DynamicImage>)> = images.into_iter().map(|(uuid, img)| (uuid, img.expect("Couldn't unwrap image!"))).collect();
                trace!("Resizing...");
                let resized_images = preprocessing::resize_images(images);
                trace!("Resized images");
                trace!("Converting to clip format...");
                let image_clip_input = preprocessing::image_to_clip_format(resized_images);
                trace!("Converted to clip format");
                
                // TODO We need to handle failures here. They seem to come up occasionally?
                //      If we do get a failure, then to avoid just failing for the whole batch,
                //      we could try for each individual image. Then individual failures can be put in the failed_encodings table.
                //      Also, it just crashes right now - we'd rather log it properly and continue?
                //      At least, it was crashing in our db::init() logic, but that might just be because our error
                //        handling was set to crash on error and we weren't logging or something.
                //        Hopefully there's not just a panic-type thing in ORT, I doubt it,
                //        unless it's something odd with the GPU side?
                trace!("Encoding images...");
                let image_encodings: Array2<f32> = self.encode_image(image_clip_input)?;
                trace!("Encoded images");
                
                // Serialize each image encodings with bincode; convert the first axis of the ndarray to a vec
                trace!("Serializing encodings...");
                let serialized_encodings: anyhow::Result<Vec<Vec<u8>>> = image_encodings.outer_iter().map(|row| {
                    Ok(bincode::serialize(&row.to_vec())?)
                }).collect();
                let serialized_encodings = serialized_encodings?;
                trace!("Serialized encodings");
                
                // Insert the image encodings into the image_features_vit_l_14_336_px table
                // The encoding is serialized with serde.
                // The ID of the encoding is the same as the file ID.
                let new_image_features: Vec<NewImageFeaturesVitL14336Px> = chunk.iter().zip(serialized_encodings.iter()).map(|((file_id, _), encoding)| {
                    NewImageFeaturesVitL14336Px {
                        id: file_id.to_string(),
                        feature_vector: encoding
                    }
                }).collect();
                
                diesel::insert_into(image_features_vit_l_14_336_px::table)
                .values(&new_image_features)
                .execute(connection)?;
            }
 
            if !failed_images.is_empty()
            {
                // Convert the failed images into NewFailedEncoding structs and insert them into the failed_encodings table.
                // The unwrap is safe because we just partitioned, so these are all Err results.
                let new_failed_encodings: Vec<NewFailedEncoding> = failed_images.into_iter().map(|(uuid, img)| {
                    NewFailedEncoding {
                        id: uuid,
                        error: img.as_ref().err().expect("Expected error!").to_string(),
                        failed_at: None
                    }
                }).collect();
                
                diesel::insert_into(failed_encodings::table)
                .values(&new_failed_encodings)
                .execute(connection)?;
            }
        }

        Ok(())
    }

    pub fn encode_files_and_add_to_search(
        file_ids: &[UUID],
        connection: &mut SqliteConnection,
        clip_state: tauri::State<'_, ClipState>,
        search_state: tauri::State<'_, SearchState<'_>>
    ) -> anyhow::Result<()>
    {
        // Note that we pass in the clip_state rather than using &self here
        // so that we can quickly release the lock on the app's CLIP state after
        // encoding the images.
        {
            let clip = &clip_state.0.lock().unwrap().clip;
            clip.encode_image_files(file_ids, connection)?;
        }

        // Add the resulting encodings to the HNSW search index.
        let image_features = queries::get_image_feature_data(file_ids, connection)?; 
        {
            let hnsw_elements = ann::convert_rows_to_hnsw_elements(&image_features)?;
            let mut search_inner = search_state.0.lock().unwrap();
            let hnsw = &mut search_inner.hnsw;
            hnsw.insert_slice(hnsw_elements);
        }

        Ok(())
    }

    fn normalize_feature_vectors(feature_vectors: &mut Array2<f32>)
    {
        feature_vectors.axis_iter_mut(Axis(0)).for_each(|mut row| {
            let norm = row.dot(&row).sqrt();
            if norm == 0.0 {
                return;
            }
            row /= norm;
        });
    }
}

#[cfg(test)]
mod tests {
    use ndarray::ArrayView;
    use approx;
    use ort::CPUExecutionProvider;

    use super::*;

    fn forward_onnx(images: Array<f32, Dim<[usize; 4]>>, tokens: Array2<i32>) -> anyhow::Result<ForwardResults>
    {
        let forward_session = ort::Session::builder().unwrap()
            .with_optimization_level(GraphOptimizationLevel::Level3).unwrap()
            .with_intra_threads(4).unwrap()
            .with_execution_providers([CPUExecutionProvider::default().build()]).unwrap()
            .commit_from_file(Path::new(env!("CARGO_MANIFEST_DIR")).join("models").join("ViT-L_14_336px.onnx")).unwrap();

        let outputs = forward_session.run(inputs![
            images,
            tokens
        ]?)?;

        let logits_per_image: ArrayView<f32, IxDyn> = outputs["LOGITS_PER_IMAGE"].try_extract_tensor::<f32>()?;
        let logits_per_text: ArrayView<f32, IxDyn> = outputs["LOGITS_PER_TEXT"].try_extract_tensor::<f32>()?;
        let logits_per_image = logits_per_image.to_owned();
        let logits_per_text = logits_per_text.to_owned();
        Ok(
            ForwardResults{
                logits_per_image,
                logits_per_text,
            }
        )
    }

    #[test]
    fn forward_equivalence()
    {
        let clip = Clip::new().unwrap();

        let texts = vec![
            "A photo of a duck",
            "A photo of a cat.",
        ];

        let tokenizer = instant_clip_tokenizer::Tokenizer::new();

        let tokens = preprocessing::tokenize_batch(texts, &tokenizer);

        let images = preprocessing::load_image_batch(&[
            (uuid::Uuid::new_v4().into(), Path::new(env!("CARGO_MANIFEST_DIR")).join("test_images").join("duck.jpg")),
            (uuid::Uuid::new_v4().into(), Path::new(env!("CARGO_MANIFEST_DIR")).join("test_images").join("benji.jpg")),
            ]);

        // Unwrap the images
        let images: Vec<(UUID, Box<DynamicImage>)> = images.into_iter().map(|(uuid, img)| (uuid, img.unwrap())).collect();

        let resized_images = preprocessing::resize_images(images);

        let image_clip_input = preprocessing::image_to_clip_format(resized_images);

        let expected_forward_results = forward_onnx(image_clip_input.clone(), tokens.clone()).unwrap();
        let actual_forward_results = clip.forward(image_clip_input, tokens).unwrap();

        assert!(approx::relative_eq!(expected_forward_results.logits_per_image, actual_forward_results.logits_per_image, epsilon = 0.0001));
        assert!(approx::relative_eq!(expected_forward_results.logits_per_text, actual_forward_results.logits_per_text, epsilon = 0.0001));
    }
}