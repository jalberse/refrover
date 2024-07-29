#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::path::Path;

use app::db;

use image::{imageops::FilterType, GenericImageView};
use ort::{inputs, ArrayExtensions, CUDAExecutionProvider, GraphOptimizationLevel, Session, SessionOutputs};
use ndarray::{s, Array, Axis};
use ndarray::Array4;

#[tauri::command]
async fn on_button_clicked() -> String {
    let connection = &mut db::establish_db_connection();

    // TODO I'd need to check the DB to see what the ID is, lol.
    //   Or I can make a query to query the ID by the name, which I might want anyways.
    // let results = query_files_with_tag(2, connection);
    // serde_json::to_string(&results).unwrap()

    "Hello from Rust!".to_string()
}



fn main() {
    // TODO Test ORT reading and executing the ONNX model.

    // TODO If my models aren't working, try converting like this instead; https://huggingface.co/docs/transformers/serialization
    //   Specifically, https://pytorch.org/docs/stable/onnx.html

    // TODO Compile libonnxruntime.so ?
    // TODO Compile libonnxruntime.so for other target platforms (?)

    // TODO Right now I am using the download strategy (faster) to get the ONNX runtime.
    //    Switch to the load-dynamic strategy instead to avoid shared library hell.
    //    https://crates.io/crates/ort#strategies
    // 
    // Set the ENV variable: ORT_DYLIB_PATH=./libonnxruntime.so
    // This is for the load-dynamic feature of ORT.
    // This avoids shared library hell.
    // We do expect libonnxruntime.so to be in the same directory as the executable;
    // TODO Ensure that the libonnxruntime.so is in the same directory as the executable,
    //      including when we ship it.
    // std::env::set_var("ORT_DYLIB_PATH", "./libonnxruntime.so");

    let result = inference();
    match result {
        Ok(_) => println!("Inference successful!"),
        Err(e) => println!("Inference failed: {:?}", e),
    }

    tauri::Builder::default()
        .setup(|_app| {
            db::init();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![on_button_clicked])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// TODO Ensure we actually use the correct execution provider (ie CUDA for now).
fn inference() -> Result<(), ort::Error>
{
    // TODO Ensure we can load models when shipping executables;
    //    We will ship the ONNX files.
    //    Note we might do something like: Path::new(env!("CARGO_MANIFEST_DIR")).join("data").join("tokenizer.json")

    // TODO The ONNX runtime and tokenizer (etc) should be initialized once and passed about,
    //      not created within this function.

    let session = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(4)?
        .commit_from_file("models/ViT-L_14_336px.onnx")?;

    // TODO Verify this works. I think it should be fine, but it's just some random crate. 
    use instant_clip_tokenizer::{Token, Tokenizer};
    let tokenizer = Tokenizer::new();

    let context_length = 77;

    let input_texts = vec![
        "a diagram",
        "a cat",
        "a dog",
    ];
    let mut all_tokens = Vec::new();
    for text in &input_texts {
        let mut tokens = Vec::new();
        tokenizer.encode(text, &mut tokens);
        let tokens = tokens.into_iter().map(Token::to_u16).collect::<Vec<_>>();
        // Transform the tokens to i32, which is what the model expects.
        let mut tokens = tokens.into_iter().map(|x| x as i32).collect::<Vec<_>>();
        // TODO If we truncate the tokens here, then I think we miss the <endTex> token that the model expects. See their clip.py tokenization.
        // Pad to 77 tokens, the expected input size per row.
        tokens.resize(context_length, 0);
        all_tokens.append(&mut tokens);
    }
    // Maintain const.
    let all_tokens = all_tokens;

    // TODO Then juist figure out image stuff and then get predictions as below.
    // TODO Test the model against a sample photo, with a sample query.
    // Do we get high confidence for sentences which match the photo?
    // Do we get low confidence for sentences which do not match the photo?
    // https://github.com/pykeio/ort/blob/main/examples/sentence-transformers/examples/semantic-similarity.rs
    // I think this is somewhat similar to how we would preprocess our input, but we need
    //   to set it up for CLIP.
    // https://github.com/openai/CLIP

    // TODO Note I think we only need the ViT model right now.
    
    let original_img = image::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("data").join("CLIP.png")).unwrap();
	let (img_width, img_height) = (original_img.width(), original_img.height());
    let image_input_size = 336;
	let img = original_img.resize_exact(image_input_size, image_input_size, FilterType::CatmullRom);
	let mut image_input = Array::zeros((1, 3, image_input_size as usize, image_input_size as usize));
	for pixel in img.pixels() {
		let x = pixel.0 as _;
		let y = pixel.1 as _;
		let [r, g, b, _] = pixel.2.0;
		image_input[[0, 0, y, x]] = (r as f32) / 255.;
		image_input[[0, 1, y, x]] = (g as f32) / 255.;
		image_input[[0, 2, y, x]] = (b as f32) / 255.;
	}

    // TODO With the image and the tokens, I think we can get the predictions now?
    //   Unsure if I pre-processed the image correctly.

    // Create Arrays of the tokens and image_input

    // The model expects the following for the text token input:
    // A two-dimensional tensor containing the resulting tokens, shape = [number of input strings, context_length
    println!("Tokens: {:?}", all_tokens);
    println!("Shape: {:?}", (all_tokens.len(), context_length));
    let tokens = Array::from_shape_vec((input_texts.len(), context_length), all_tokens).unwrap();
    let image_input = image_input.into_shape((1, 3, image_input_size as usize, image_input_size as usize)).unwrap();

    let outputs: SessionOutputs = session.run(inputs![
        image_input,
        tokens
    ]?)?;
    let logits_per_image = &outputs[0].try_extract_tensor::<f32>()?;
    // TODO Not needed?
    let _logits_per_text = &outputs[1].try_extract_tensor::<f32>()?;

    // Softmax is used for "which of these is likely", but not if we just want one query (our use case)
    let probabilities = logits_per_image.softmax(Axis(1));
    
    // TODO Do the OpenAI example on Clip.PNG to see if we get the same/similar numbers.
    //   If so, yippee we've likely configured it correctly.
    //   If not, we need to figure out what we did wrong.
    //   I think it's looking OK - baseball had a value twice as high as motorcycling, which is what I expected.

    // Convert the probabilities to a Vec
    // let probabilities = logits_per_image.iter().map(|x| x.to_owned()).collect::<Vec<_>>();
    println!("Probs: {:?}", probabilities);

    Ok(())
}