//   Neural Style Transfer
//   This is inspired by the Neural Style tutorial from PyTorch.org
//   https://pytorch.org/tutorials/advanced/neural_style_tutorial.html
//   The pre-trained weights for the VGG16 model can be downloaded from:
//   https://github.com/LaurentMazare/tch-rs/releases/download/mw/vgg16.ot
use tch::vision::{imagenet, vgg};
use tch::{nn, nn::OptimizerConfig, Device, Tensor};
use std::path::Path;
use std::fs;
use std::fs::File;
use std::io::{Write};
use titlecase::titlecase;
use serde::{Serialize, Deserialize};
use std::process::{Command, Stdio};
use rouille::post_input;
use rouille::Request;
use rouille::Response;

const STYLE_WEIGHT: f64 = 1e6;
const LEARNING_RATE: f64 = 1e-1;
const TOTAL_STEPS: i64 = 3000;
const STYLE_INDEXES: [usize; 5] = [0, 2, 5, 7, 10];
const CONTENT_INDEXES: [usize; 1] = [7];


pub fn handle(current_session: crate::sam::memory::WebSessions, request: &Request) -> Result<Response, crate::sam::http::Error> {
    if request.url().contains("/styles"){
        return Ok(Response::json(&styles().unwrap()));
    }

    if request.url().contains("/run"){

        let style = request.get_param("style");
        let image = request.get_param("image");


        return Ok(Response::json(&styles().unwrap()));
    }
    return Ok(Response::empty_404());
}

fn gram_matrix(m: &Tensor) -> Tensor {
    let (a, b, c, d) = m.size4().unwrap();
    let m = m.view([a * b, c * d]);
    let g = m.matmul(&m.tr());
    g / (a * b * c * d)
}

fn style_loss(m1: &Tensor, m2: &Tensor) -> Tensor {
    gram_matrix(m1).mse_loss(&gram_matrix(m2), tch::Reduction::Mean)
}

pub fn run2(style_img: &str, content_img: &str) -> Result<(), crate::sam::services::Error> {
    let device = Device::cuda_if_available();


    let mut net_vs = tch::nn::VarStore::new(device);
    let net = vgg::vgg16(&net_vs.root(), imagenet::CLASS_COUNT);
    net_vs.load("/opt/sam/models/vgg16.ot")?;
    net_vs.freeze();

    let style_img = imagenet::load_image(&style_img)?
        .unsqueeze(0)
        .to_device(device);
    let content_img = imagenet::load_image(&content_img)?
        .unsqueeze(0)
        .to_device(device);
    let max_layer = STYLE_INDEXES.iter().max().unwrap() + 1;
    let style_layers = net.forward_all_t(&style_img, false, Some(max_layer));
    let content_layers = net.forward_all_t(&content_img, false, Some(max_layer));

    let vs = nn::VarStore::new(device);
    let input_var = vs.root().var_copy("img", &content_img);
    let mut opt = nn::Adam::default().build(&vs, LEARNING_RATE)?;

    for step_idx in 1..(1 + TOTAL_STEPS) {
        let input_layers = net.forward_all_t(&input_var, false, Some(max_layer));
        let style_loss: Tensor =
            STYLE_INDEXES.iter().map(|&i| style_loss(&input_layers[i], &style_layers[i])).sum();
        let content_loss: Tensor = CONTENT_INDEXES
            .iter()
            .map(|&i| input_layers[i].mse_loss(&content_layers[i], tch::Reduction::Mean))
            .sum();
        let loss = style_loss * STYLE_WEIGHT + content_loss;
        opt.backward_step(&loss);
        log::info!("{} {}", step_idx, f64::from(loss.clone(&loss)));
        if step_idx % 1000 == 0 {
            log::info!("{} {}", step_idx, f64::from(loss));
            imagenet::save_image(&input_var, &format!("out{}.jpg", step_idx))?;
        }
    }

    Ok(())
}

pub fn run(style_img: &str, content_img: &str, weights: &str) -> Result<(), crate::sam::services::Error> {
    let device = Device::cuda_if_available();


    let mut net_vs = tch::nn::VarStore::new(device);
    let net = vgg::vgg16(&net_vs.root(), imagenet::CLASS_COUNT);
    net_vs.load(&weights)?;
    net_vs.freeze();

    let style_img = imagenet::load_image(&style_img)?
        .unsqueeze(0)
        .to_device(device);
    let content_img = imagenet::load_image(&content_img)?
        .unsqueeze(0)
        .to_device(device);
    let max_layer = STYLE_INDEXES.iter().max().unwrap() + 1;
    let style_layers = net.forward_all_t(&style_img, false, Some(max_layer));
    let content_layers = net.forward_all_t(&content_img, false, Some(max_layer));

    let vs = nn::VarStore::new(device);
    let input_var = vs.root().var_copy("img", &content_img);
    let mut opt = nn::Adam::default().build(&vs, LEARNING_RATE)?;

    for step_idx in 1..(1 + TOTAL_STEPS) {
        let input_layers = net.forward_all_t(&input_var, false, Some(max_layer));
        let style_loss: Tensor =
            STYLE_INDEXES.iter().map(|&i| style_loss(&input_layers[i], &style_layers[i])).sum();
        let content_loss: Tensor = CONTENT_INDEXES
            .iter()
            .map(|&i| input_layers[i].mse_loss(&content_layers[i], tch::Reduction::Mean))
            .sum();
        let loss = style_loss * STYLE_WEIGHT + content_loss;
        opt.backward_step(&loss);
        log::info!("{} {}", step_idx, f64::from(loss.clone(&loss)));
        if step_idx % 1000 == 0 {
            log::info!("{} {}", step_idx, f64::from(loss));
            imagenet::save_image(&input_var, &format!("out{}.jpg", step_idx))?;
        }
    }

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

        let mut style = Style{
            name: titlecase(&format!("{}", pth.clone()).replace("/opt/sam/models/nst/", "").replace(".jpg", "").replace("_", " ")),
            file_path: pth.clone(),
        };

        styles.push(style);
    }
    return Ok(styles);
}

pub fn install() -> Result<(), crate::sam::services::Error> {
    if !Path::new("/opt/sam/models/vgg16.ot").exists(){
        crate::sam::tools::linux_cmd(format!("wget -O /opt/sam/models/vgg16.ot https://github.com/LaurentMazare/tch-rs/releases/download/mw/vgg16.ot"));
    }

    let data = include_bytes!("../../../../packages/nst/fra_angelico.jpg");
    let mut pos = 0;
    let mut buffer = File::create("/opt/sam/models/nst/fra_angelico.jpg")?;
    while pos < data.len() {
        let bytes_written = buffer.write(&data[pos..])?;
        pos += bytes_written;
    }

    let data = include_bytes!("../../../../packages/nst/paul_cézanne.jpg");
    let mut pos = 0;
    let mut buffer = File::create("/opt/sam/models/nst/paul_cézanne.jpg")?;
    while pos < data.len() {
        let bytes_written = buffer.write(&data[pos..])?;
        pos += bytes_written;
    }

    let data = include_bytes!("../../../../packages/nst/sassetta.jpg");
    let mut pos = 0;
    let mut buffer = File::create("/opt/sam/models/nst/sassetta.jpg")?;
    while pos < data.len() {
        let bytes_written = buffer.write(&data[pos..])?;
        pos += bytes_written;
    }

    let data = include_bytes!("../../../../packages/nst/vincent_van_gogh.jpg");
    let mut pos = 0;
    let mut buffer = File::create("/opt/sam/models/nst/vincent_van_gogh.jpg")?;
    while pos < data.len() {
        let bytes_written = buffer.write(&data[pos..])?;
        pos += bytes_written;
    }

    return Ok(());
}