//   Neural Style Transfer
//   This is inspired by the Neural Style tutorial from PyTorch.org
//   https://pytorch.org/tutorials/advanced/neural_style_tutorial.html
//   The pre-trained weights for the VGG16 model can be downloaded from:
//   https://github.com/LaurentMazare/tch-rs/releases/download/mw/vgg16.ot

#[cfg(feature = "nst")]
use tch::vision::{imagenet, vgg};
#[cfg(feature = "nst")]
use tch::{nn, nn::OptimizerConfig, Device, Tensor};

use rouille::post_input;
use rouille::Request;
use rouille::Response;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::thread;
use titlecase::titlecase;

const STYLE_WEIGHT: f64 = 1e6;
const LEARNING_RATE: f64 = 1e-1;
const TOTAL_STEPS: i64 = 10000;
const STYLE_INDEXES: [usize; 5] = [0, 2, 5, 7, 10];
const CONTENT_INDEXES: [usize; 1] = [7];

pub fn handle(
    _current_session: crate::memory::cache::WebSessions,
    request: &Request,
) -> Result<Response, crate::http::Error> {
    if request.url().contains("/styles") {
        return Ok(Response::json(&styles()?));
    }

    if request.url().contains("/run") {
        #[cfg(not(feature = "nst"))]
        {
            return Ok(Response::json(&serde_json::json!({
                "error": "NST feature not enabled. Please build with --features nst"
            })));
        }

        #[cfg(feature = "nst")]
        {
            let input = post_input!(request, {
                image_id: String, // oid:<oid>, dropbox:<id>
                nst_style: String, // Fra Angelico, Vincent Van Gogh
            })?;

            let mut selected_style = "/opt/sam/models/nst/vincent_van_gogh.jpg".to_string();
            for style in styles()? {
                if style.name == input.nst_style.as_str() {
                    selected_style = style.file_path.to_string();
                }
            }

            // file
            if input.image_id.contains("oid:") {
                let oid = input.image_id.replace("oid:", "");
                if Path::new(format!("/opt/sam/files/{oid}").as_str()).exists() {
                    let _ =
                        thread::Builder::new()
                            .name("nst_thread".to_string())
                            .spawn(move || {
                                let _ = run(
                                    &selected_style,
                                    format!("/opt/sam/files/{oid}").as_str(),
                                    oid,
                                    input.nst_style,
                                );
                            });
                }
            }

            return Ok(Response::json(&styles()?));
        }
    }
    Ok(Response::empty_404())
}

#[cfg(feature = "nst")]
fn gram_matrix(m: &Tensor) -> Tensor {
    let (a, b, c, d) = m.size4().unwrap();
    let m = m.view([a * b, c * d]);
    let g = m.matmul(&m.tr());
    g / (a * b * c * d)
}

#[cfg(feature = "nst")]
fn style_loss(m1: &Tensor, m2: &Tensor) -> Tensor {
    gram_matrix(m1).mse_loss(&gram_matrix(m2), tch::Reduction::Mean)
}

pub fn run(
    style_img: &str,
    content_img: &str,
    oid: String,
    style: String,
) -> Result<(), crate::services::Error> {
    #[cfg(not(feature = "nst"))]
    {
        log::warn!(
            "NST feature not enabled. Build with --features nst to enable Neural Style Transfer"
        );
        return Err(crate::services::Error::Other(
            "NST feature not enabled".to_string(),
        ));
    }

    #[cfg(feature = "nst")]
    {
        log::info!("Starting Neural Style Transfer");
        log::info!("Style image: {:?}", style_img);
        log::info!("Content image: {:?}", content_img);

        let device = Device::cuda_if_available();

        let mut net_vs = tch::nn::VarStore::new(device);
        let net = vgg::vgg16(&net_vs.root(), imagenet::CLASS_COUNT);

        // Load VGG16 weights
        if let Err(e) = net_vs.load("/opt/sam/models/vgg16.ot") {
            log::error!("Failed to load VGG16 model: {}. Run install() first.", e);
            return Err(crate::services::Error::Other(format!(
                "VGG16 model not found: {}",
                e
            )));
        }
        net_vs.freeze();

        // Load and preprocess images
        let style_img = match imagenet::load_image(style_img) {
            Ok(img) => img.unsqueeze(0).to_device(device),
            Err(e) => {
                log::error!("Failed to load style image: {}", e);
                return Err(crate::services::Error::Other(format!(
                    "Style image load error: {}",
                    e
                )));
            }
        };

        let content_img = match imagenet::load_image(content_img) {
            Ok(img) => img.unsqueeze(0).to_device(device),
            Err(e) => {
                log::error!("Failed to load content image: {}", e);
                return Err(crate::services::Error::Other(format!(
                    "Content image load error: {}",
                    e
                )));
            }
        };

        let max_layer = STYLE_INDEXES.iter().max().unwrap() + 1;
        let style_layers = net.forward_all_t(&style_img, false, Some(max_layer));
        let content_layers = net.forward_all_t(&content_img, false, Some(max_layer));

        let vs = nn::VarStore::new(device);
        let input_var = vs.root().var_copy("img", &content_img);
        let mut opt = nn::Adam::default().build(&vs, LEARNING_RATE)?;

        log::info!("Starting optimization with {} steps", TOTAL_STEPS);

        for step_idx in 1..(1 + TOTAL_STEPS) {
            let input_layers = net.forward_all_t(&input_var, false, Some(max_layer));

            let style_loss: Tensor = STYLE_INDEXES
                .iter()
                .map(|&i| style_loss(&input_layers[i], &style_layers[i]))
                .sum();

            let content_loss: Tensor = CONTENT_INDEXES
                .iter()
                .map(|&i| input_layers[i].mse_loss(&content_layers[i], tch::Reduction::Mean))
                .sum();

            let loss = style_loss * STYLE_WEIGHT + content_loss;
            opt.backward_step(&loss);

            if step_idx % 100 == 0 {
                let loss_val = f64::try_from(&loss).unwrap_or_else(|_| {
                    // Fallback: extract scalar value using double_value if tensor is scalar
                    loss.double_value(&[])
                });
                log::info!("Step {}: Loss = {:.6}", step_idx, loss_val);
            }

            if step_idx % 1000 == 0 {
                let loss_val = f64::try_from(&loss).unwrap_or_else(|_| {
                    // Fallback: extract scalar value using double_value if tensor is scalar
                    loss.double_value(&[])
                });
                log::info!(
                    "Saving intermediate result at step {}: Loss = {:.6}",
                    step_idx,
                    loss_val
                );

                if let Err(e) =
                    imagenet::save_image(&input_var, &format!("/opt/sam/files/out{}.jpg", step_idx))
                {
                    log::warn!("Failed to save intermediate image: {}", e);
                    continue;
                }

                // Read the saved image and store it in the database
                if let Ok(mut file) = File::open(format!("/opt/sam/files/out{}.jpg", step_idx)) {
                    let mut buf = Vec::new();
                    if file.read_to_end(&mut buf).is_ok() {
                        let mut db_file = crate::memory::storage::File::new();
                        db_file.file_name = format!("{}-{}-{}.jpg", oid, style, step_idx);
                        db_file.file_type = "image/jpeg".to_string();
                        db_file.file_data = Some(buf);
                        db_file.storage_location_oid = "SQL".to_string();
                        if let Err(e) = db_file.save() {
                            log::warn!("Failed to save file to database: {}", e);
                        }
                    }
                }
            }
        }

        log::info!("Neural Style Transfer completed successfully");
    }

    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Style {
    pub name: String,
    pub file_path: String,
}

pub fn styles() -> Result<Vec<Style>, crate::services::Error> {
    let mut styles: Vec<Style> = Vec::new();
    let paths = fs::read_dir("/opt/sam/models/nst/")?;
    for path in paths {
        let pth = path?.path().display().to_string();

        let style = Style {
            name: titlecase(
                &pth.clone()
                    .to_string()
                    .replace("/opt/sam/models/nst/", "")
                    .replace(".jpg", "")
                    .replace("_", " "),
            ),
            file_path: pth.clone(),
        };

        styles.push(style);
    }
    Ok(styles)
}

pub fn install() -> Result<(), crate::services::Error> {
    // Create models directory if it doesn't exist
    std::fs::create_dir_all("/opt/sam/models/nst").map_err(|e| {
        crate::services::Error::Other(format!("Failed to create models directory: {}", e))
    })?;

    // Download VGG16 model if it doesn't exist
    if !Path::new("/opt/sam/models/vgg16.ot").exists() {
        log::info!("Downloading VGG16 model weights...");
        match crate::tools::safe_cmd(
            "wget",
            &[
                "-O",
                "/opt/sam/models/vgg16.ot",
                "https://github.com/LaurentMazare/tch-rs/releases/download/mw/vgg16.ot",
            ],
        ) {
            Ok(output) => {
                log::info!("VGG16 model downloaded successfully: {}", output);
            }
            Err(e) => {
                log::error!("Failed to download VGG16 model: {}", e);
                return Err(crate::services::Error::Other(format!(
                    "VGG16 download failed: {}",
                    e
                )));
            }
        }
    } else {
        log::info!("VGG16 model already exists");
    }

    // Install default style images
    install_style_image(
        "fra_angelico.jpg",
        include_bytes!("../../../../../packages/nst/fra_angelico.jpg"),
    )?;
    install_style_image(
        "paul_cézanne.jpg",
        include_bytes!("../../../../../packages/nst/paul_cézanne.jpg"),
    )?;
    install_style_image(
        "sassetta.jpg",
        include_bytes!("../../../../../packages/nst/sassetta.jpg"),
    )?;
    install_style_image(
        "vincent_van_gogh.jpg",
        include_bytes!("../../../../../packages/nst/vincent_van_gogh.jpg"),
    )?;

    log::info!("NST installation completed successfully");
    Ok(())
}

fn install_style_image(filename: &str, data: &[u8]) -> Result<(), crate::services::Error> {
    let path = format!("/opt/sam/models/nst/{}", filename);

    if !Path::new(&path).exists() {
        log::info!("Installing style image: {}", filename);
        let mut file = File::create(&path).map_err(|e| {
            crate::services::Error::Other(format!("Failed to create file {}: {}", path, e))
        })?;

        file.write_all(data).map_err(|e| {
            crate::services::Error::Other(format!("Failed to write file {}: {}", path, e))
        })?;

        log::info!("Style image {} installed successfully", filename);
    } else {
        log::debug!("Style image {} already exists", filename);
    }

    Ok(())
}
