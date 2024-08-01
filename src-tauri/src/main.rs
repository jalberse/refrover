#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::path::Path;

use app::preprocessing::CONTEXT_LENGTH;
use app::{db, preprocessing};
use app::clip::Clip;

use ort::{inputs, ArrayExtensions, SessionOutputs};
use ndarray::{Array, Axis, Dim};

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

fn inference() -> Result<(), ort::Error>
{   
    let input_texts = vec![
        "a man playing the saxophone",
        "a man playing baseball",
        "cathedral",
        ];
        
    let all_tokens = preprocessing::tokenize_batch(input_texts);
    let image_input: Array<f32, Dim<[usize; 4]>> = preprocessing::load_image(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data").join("baseball.jpg"));
        
    let clip = Clip::new()?;
    let forward_results = clip.forward(image_input, all_tokens)?;

    let probabilities = forward_results.logits_per_image.softmax(Axis(1));
    
    // Convert the probabilities to a Vec
    // let probabilities = logits_per_image.iter().map(|x| x.to_owned()).collect::<Vec<_>>();
    println!("Probabilities: {:?}", probabilities);

    // TODO This comment below is from the onnx_converter. Listen to it.
    //        Need to transfer project to Ubuntu to actually run + test though lol (I hate python!!)
    // TODO Okay, I have typed in the stuff I need to get exported. Test it, try loading
    //      it into Rust with tch (?). Use all 3 models for now, implement the encode_text thing
    //      with the additional Parameter and LayerNorm stuff and permuations.
    //      ndarray provides a common interface for our "x" between ONNX and tch.
    //        (that stuff is done in the forward() ONNX, but wouldn't be for a plain export
    //         of the transformer model, so we'll need to do that - can't just call the transformer ONNX for text).
    //         Remember the token embedding stuff is *I think* handled by the instant-clip-tokenizer crate.

    // TODO great, I got the models and I'm quite confident that they are correct based on tests in the exporter.
    //    Try the 3 model functions out on some data and see how it goes :)
    // We'll probably want an ImageSearchModel class that has the 3 model functions and the tokenizer.

    // TODO Then we'll go into the spatial search stuff.
    //  That's exciting I can get zero-shot image search into our UI/app pretty soon!!!

    Ok(())
}