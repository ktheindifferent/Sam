//   Neural Style Transfer
//   This is inspired by the Neural Style tutorial from PyTorch.org
//   https://pytorch.org/tutorials/advanced/neural_style_tutorial.html
//   The pre-trained weights for the VGG16 model can be downloaded from:
//   https://github.com/LaurentMazare/tch-rs/releases/download/mw/vgg16.ot
// use tch::vision::{imagenet, vgg};
// use tch::{nn, nn::OptimizerConfig, Device, Tensor};
use std::path::Path;
use std::fs;
use std::fs::File;
use std::io::{Write};
use titlecase::titlecase;
use serde::{Serialize, Deserialize};
use rouille::post_input;
use rouille::Request;
use rouille::Response;
use std::thread;

 // Add missing import for tools module

const _STYLE_WEIGHT: f64 = 1e6;
const _LEARNING_RATE: f64 = 1e-1;
const _TOTAL_STEPS: i64 = 10000;
const _STYLE_INDEXES: [usize; 5] = [0, 2, 5, 7, 10];
const _CONTENT_INDEXES: [usize; 1] = [7];


pub fn handle(_current_session: crate::sam::memory::cache::WebSessions, request: &Request) -> Result<Response, crate::sam::http::Error> {
    if request.url().contains("/styles"){
        return Ok(Response::json(&styles().unwrap()));
    }

    if request.url().contains("/run"){

        let input = post_input!(request, {
            image_id: String, // oid:<oid>, dropbox:<id>
            nst_style: String, // Fra Angelico, Vincent Van Gogh
        })?;

        let mut selected_style = "/opt/sam/models/nst/vincent_van_gogh.jpg".to_string();
        for style in styles()?{
            if style.name == input.nst_style.as_str() {
                selected_style = style.file_path.to_string();
            }
        }

        // file
        if input.image_id.contains("oid:") {
            let oid = input.image_id.replace("oid:", "");
            if Path::new(format!("/opt/sam/files/{oid}").as_str()).exists(){
                let _ = thread::Builder::new().name("nst_thread".to_string()).spawn(move || {
                    let _ = run(&selected_style, format!("/opt/sam/files/{oid}").as_str(), oid, input.nst_style);
                });
            }
        }

        

        // return Ok(Response::json(&styles().unwrap()));
    }
    Ok(Response::empty_404())
}

// fn gram_matrix(m: &Tensor) -> Tensor {
//     let (a, b, c, d) = m.size4().unwrap();
//     let m = m.view([a * b, c * d]);
//     let g = m.matmul(&m.tr());
//     g / (a * b * c * d)
// }

// fn style_loss(m1: &Tensor, m2: &Tensor) -> Tensor {
//     gram_matrix(m1).mse_loss(&gram_matrix(m2), tch::Reduction::Mean)
// }

pub fn run(_style_img: &str, _content_img: &str, _oid: String, _style: String) -> Result<(), crate::sam::services::Error> {

    // log::info!("NST");
    // log::info!("style image: {:?}", style_img);
    // log::info!("content image: {:?}", content_img);

    // let device = Device::cuda_if_available();


    // let mut net_vs = tch::nn::VarStore::new(device);
    // let net = vgg::vgg16(&net_vs.root(), imagenet::CLASS_COUNT);
    // net_vs.load("/opt/sam/models/vgg16.ot")?;
    // net_vs.freeze();

    // let style_img = imagenet::load_image(&style_img)?
    //     .unsqueeze(0)
    //     .to_device(device);
    // let content_img = imagenet::load_image(&content_img)?
    //     .unsqueeze(0)
    //     .to_device(device);
    // let max_layer = STYLE_INDEXES.iter().max().unwrap() + 1;
    // let style_layers = net.forward_all_t(&style_img, false, Some(max_layer));
    // let content_layers = net.forward_all_t(&content_img, false, Some(max_layer));

    // let vs = nn::VarStore::new(device);
    // let input_var = vs.root().var_copy("img", &content_img);
    // let mut opt = nn::Adam::default().build(&vs, LEARNING_RATE)?;

    // for step_idx in 1..(1 + TOTAL_STEPS) {
    //     let input_layers = net.forward_all_t(&input_var, false, Some(max_layer));
    //     let style_loss: Tensor =
    //         STYLE_INDEXES.iter().map(|&i| style_loss(&input_layers[i], &style_layers[i])).sum();
    //     let content_loss: Tensor = CONTENT_INDEXES
    //         .iter()
    //         .map(|&i| input_layers[i].mse_loss(&content_layers[i], tch::Reduction::Mean))
    //         .sum();
    //     let loss = style_loss * STYLE_WEIGHT + content_loss;
    //     opt.backward_step(&loss);
    //     log::info!("{} {}", step_idx, f64::from(loss.clone(&loss)));
    //     if step_idx % 1000 == 0 {
    //         log::info!("{} {}", step_idx, f64::from(loss));
    //         imagenet::save_image(&input_var, &format!("/opt/sam/files/out{}.jpg", step_idx))?;


    //         let mut file = File::open(format!("/opt/sam/files/out{}.jpg", step_idx))?;
    //         let mut buf = Vec::new();
    //         file.read_to_end(&mut buf);

    //         let mut file = crate::sam::memory::storage::File::new();
    //         file.file_name = format!("{}-{}-{}.jpg", oid, style, step_idx);
    //         file.file_type = format!("image/jpeg");
    //         file.file_data = Some(buf);
    //         // file.file_folder_tree = input.file_folder_tree;
    //         file.storage_location_oid = format!("SQL");
    //         file.save()?;
    //     }
    // }

    Ok(())
}




#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Style {
    pub name: String,
    pub file_path: String,
}

pub fn styles() -> Result<Vec<Style>, crate::sam::services::Error> {
    let mut styles: Vec<Style> = Vec::new();
    let paths = fs::read_dir("/opt/sam/models/nst/")?;
    for path in paths {

        let pth = path.unwrap().path().display().to_string();

        let style = Style{
            name: titlecase(&pth.clone().to_string().replace("/opt/sam/models/nst/", "").replace(".jpg", "").replace("_", " ")),
            file_path: pth.clone(),
        };

        styles.push(style);
    }
    Ok(styles)
}

pub fn install() -> Result<(), crate::sam::services::Error> {
    if !Path::new("/opt/sam/models/vgg16.ot").exists(){
        crate::sam::tools::cmd("wget -O /opt/sam/models/vgg16.ot https://github.com/LaurentMazare/tch-rs/releases/download/mw/vgg16.ot")?; // Fix `?` operator error by ensuring the command returns a compatible type
    }

    let data = include_bytes!("../../../../../packages/nst/fra_angelico.jpg");
    let mut pos = 0;
    let mut buffer = File::create("/opt/sam/models/nst/fra_angelico.jpg")?;
    while pos < data.len() {
        let bytes_written = buffer.write(&data[pos..])?;
        pos += bytes_written;
    }

    let data = include_bytes!("../../../../../packages/nst/paul_cézanne.jpg");
    let mut pos = 0;
    let mut buffer = File::create("/opt/sam/models/nst/paul_cézanne.jpg")?;
    while pos < data.len() {
        let bytes_written = buffer.write(&data[pos..])?;
        pos += bytes_written;
    }

    let data = include_bytes!("../../../../../packages/nst/sassetta.jpg");
    let mut pos = 0;
    let mut buffer = File::create("/opt/sam/models/nst/sassetta.jpg")?;
    while pos < data.len() {
        let bytes_written = buffer.write(&data[pos..])?;
        pos += bytes_written;
    }

    let data = include_bytes!("../../../../../packages/nst/vincent_van_gogh.jpg");
    let mut pos = 0;
    let mut buffer = File::create("/opt/sam/models/nst/vincent_van_gogh.jpg")?;
    while pos < data.len() {
        let bytes_written = buffer.write(&data[pos..])?;
        pos += bytes_written;
    }

    Ok(())
}