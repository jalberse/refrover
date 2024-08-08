use std::path::Path;

use ndarray::{Array, Array2, ArrayView, Dim, IxDyn, Axis};
use ort::{self, inputs, GraphOptimizationLevel};
use ort::DirectMLExecutionProvider;

use crate::preprocessing::FEATURE_VECTOR_LENGTH;

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
/// Uses an ONNX representation of the model so that it can be used in Rust,
/// and to execute the model on a wide variety of hardware using the ONNX runtime.
/// The ONNX representation is created by: https://github.com/jalberse/CLIP-to-onnx-converter
pub struct Clip
{
    visual_session: ort::Session,
    text_session: ort::Session,
    forward_session: ort::Session
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

        let forward_session = ort::Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .with_execution_providers([DirectMLExecutionProvider::default().build()])?
            .commit_from_file(Path::new(env!("CARGO_MANIFEST_DIR")).join("models").join("ViT-L_14_336px.onnx"))?;

        Ok( Clip { visual_session, text_session, forward_session } )
    }

    /// Given a batch of images, returns the image features encoded by the vision portion of the CLIP model.
    /// Use the preprocessing::load_image() function to load the image
    /// and convert it into an array for this input.
    /// 
    /// Returns a 2D array of shape (batch_size, FEATURE_VECTOR_LENGTH).
    pub fn encode_image(&self, images: Array<f32, Dim<[usize; 4]>>) -> Result<Array2<f32>, ort::Error>
    {
        let images_len = images.len_of(Axis(0));
        let outputs = self.visual_session.run(inputs![images]?)?;

        let output = &outputs["FEATURES_EMBEDDED"];

        // First dimension is for each image in the batch; the second is the feature vector per image.
        let output = output.try_extract_tensor::<f32>()?;

        let output: Array2<f32> = output.to_shape((images_len, FEATURE_VECTOR_LENGTH)).unwrap().to_owned();

        // For example:
        // With one image in the batch....
        // Output: [[0.51000077, ..., -0.37341008]],
        //    shape=[1, 768], strides=[768, 1], layout=CFcf (0xf), dynamic ndim=2
        // With two images in batch...
        // Output: [[0.51000077, ..., -0.37341008], [-0.061004326, ..., -0.3435823]],
        //    shape=[2, 768], strides=[768, 1], layout=Cc (0x5), dynamic ndim=2

        Ok(output)
    }

    /// Given a batch of text tokens, returns the text features encoded by the language portion of the CLIP model.
    /// Generate tokens using preprocessing::tokenize_batch().
    /// 
    /// Returns a 2D array of shape (batch_size, FEATURE_VECTOR_LENGTH).
    pub fn encode_text(&self, tokens: Array2<i32>) -> Result<Array2<f32>, ort::Error>
    {
        let tokens_len = tokens.len_of(Axis(0));
        let outputs = self.text_session.run(inputs![tokens]?)?;

        let output = &outputs["FEATURES_EMBEDDED"];

        // First dimension is for each text in the batch; the second is the feature vector per text.
        let output = output.try_extract_tensor::<f32>()?;

        let output: Array2<f32> = output.to_shape((tokens_len, FEATURE_VECTOR_LENGTH)).unwrap().to_owned();

        Ok(output)
    }

    /// Given a batch of images and a batch of text tokens, returns two Tensors,
    /// containing the logit scores corresponding to each image and text input.
    /// The values are cosine similarities between the corresponding image and text features,
    /// times 100.
    pub fn forward(&self, images: Array<f32, Dim<[usize; 4]>>, tokens: Array2<i32>) -> Result<ForwardResults, ort::Error>
    {
        let outputs = self.forward_session.run(inputs![
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
}
