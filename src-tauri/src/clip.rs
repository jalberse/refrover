use std::path::Path;

use ndarray::{Array, Array2, ArrayView, Dim, IxDyn};
use ort::{self, inputs, GraphOptimizationLevel, Tensor};

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
    // TODO We also need to store tensor data for the text encoding (+ forward methods once we switch off the combined model).
}

impl Clip
{
    pub fn new() -> Result<Self, ort::Error>
    {
        // TODO We need to ensure that we are using the CUDA execution provide when available,
        //   with CPU as a fallback. There's some examples of this online.

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

        let visual_session = ort::Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(Path::new(env!("CARGO_MANIFEST_DIR")).join("models").join("ViT-L_14_336px_visual.onnx"))?;

        let text_session = ort::Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(Path::new(env!("CARGO_MANIFEST_DIR")).join("models").join("ViT-L_14_336px_transformer.onnx"))?;

        let forward_session = ort::Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(Path::new(env!("CARGO_MANIFEST_DIR")).join("models").join("ViT-L_14_336px.onnx"))?;

        // TOOD Also load in the tensor data/params... I think we need tch for that?

        Ok( Clip { visual_session, text_session, forward_session } )
    }


    /// Given a batch of images, returns the image features encoded by the vision portion of the CLIP model.
    /// Use the preprocessing::load_image() function to load the image
    /// and convert it into an array for this input.
    pub fn encode_image(&self, images: &Array<f32, Dim<[usize; 4]>>) -> Result<Tensor<f32>, ort::Error>
    {
        // TODO Implement this
        todo!()
    }

    /// Given a batch of text tokens, returns the text features encoded by the language portion of the CLIP model.
    /// Generate tokens using preprocessing::tokenize_batch().
    pub fn encode_text(&self, tokens: Array2<i32>) -> Result<Tensor<f32>, ort::Error>
    {
        // TODO Implement this
        todo!()
    }

    /// Given a batch of images and a batch of text tokens, returns two Tensors,
    /// containing the logit scores corresponding to each image and text input.
    /// The values are cosine similarities between the corresponding image and text features,
    /// times 100.
    pub fn forward(&self, images: Array<f32, Dim<[usize; 4]>>, tokens: Array2<i32>) -> Result<ForwardResults, ort::Error>
    {
        // TODO For now, we will just run the forward model.
        //   But we will want to instead mimic CLIP.forward() and use the constituent models,
        //   and then just get the cosine similarity. This will save space on the disk, since
        //   the combined ONNX graph is just storing a lot (1.6GB) of redundant data.

        let outputs = self.forward_session.run(inputs![
            images,
            tokens
        ]?)?;

        let logits_per_image: ArrayView<f32, IxDyn> = outputs[0].try_extract_tensor::<f32>()?;
        let logits_per_text: ArrayView<f32, IxDyn> = outputs[1].try_extract_tensor::<f32>()?;

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