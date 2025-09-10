use ndarray::{Array4, Array2, Array1, Axis};
use image::{DynamicImage, GenericImageView, imageops::FilterType, Luma};
use std::io;
use rand::Rng;
use image::{imageops, Luma};

/// WordDetector is a logistic regression classifier for detecting words in images.
pub struct WordDetector {
    weights: Array2<f32>, // shape: (num_features, 2)
    bias: Array1<f32>,    // shape: (2,)
}

impl WordDetector {
    /// Create a new WordDetector with random weights.
    pub fn new(input_size: usize) -> Self {
        use rand::thread_rng;
        use rand::distributions::{Distribution, Uniform};
        let mut rng = thread_rng();
        let between = Uniform::from(-0.01..0.01);
        let weights = Array2::from_shape_fn((input_size, 2), |_| between.sample(&mut rng));
        let bias = Array1::from_shape_fn(2, |_| between.sample(&mut rng));
        Self { weights, bias }
    }

    /// Preprocess an image: grayscale, resize, normalize, and flatten to 1D feature vector.
    pub fn preprocess_image(image: &DynamicImage, size: u32) -> io::Result<Array1<f32>> {
        let mut rng = rand::thread_rng();
        // Convert to grayscale
        let mut gray = image.to_luma8();

        // Random horizontal flip
        if rng.gen_bool(0.5) {
            imageops::flip_horizontal_in_place(&mut gray);
        }
        // Random vertical flip
        if rng.gen_bool(0.5) {
            imageops::flip_vertical_in_place(&mut gray);
        }
        // Random rotation (0, 90, 180, 270 degrees)
        let angle = [0, 90, 180, 270][rng.gen_range(0..4)];
        let rotated = match angle {
            90 => imageops::rotate90(&gray),
            180 => imageops::rotate180(&gray),
            270 => imageops::rotate270(&gray),
            _ => gray.clone(),
        };
        // Resize
        let mut resized = imageops::resize(&rotated, size, size, FilterType::Nearest);
        // Add random noise
        for pixel in resized.pixels_mut() {
            let noise: i16 = rng.gen_range(-10..10);
            let val = pixel[0] as i16 + noise;
            pixel[0] = val.clamp(0, 255) as u8;
        }
        // Normalize using mean and std (default: mean=0.5, std=0.5 for grayscale)
        let mean = 0.5;
        let std = 0.5;
        let data: Vec<f32> = resized
            .pixels()
            .map(|p| ((p[0] as f32 / 255.0) - mean) / std)
            .collect();
        Ok(Array1::from(data))
    }

    /// Run inference on a single input. Returns probability vector (softmax).
    pub fn forward(&self, input: &Array1<f32>) -> Array1<f32> {
        let logits = input.dot(&self.weights) + &self.bias;
        let max = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        let exp: Array1<f32> = logits.mapv(|x| (x - max).exp());
        let sum = exp.sum();
        &exp / sum
    }

    /// Train using SGD for given epochs and learning rate.
    pub fn train(
        &mut self,
        dataset: &[(Array1<f32>, usize)],
        epochs: usize,
        learning_rate: f32,
    ) -> Result<(), String> {
        for _ in 0..epochs {
            for (x, &label) in dataset.iter() {
                let y_pred = self.forward(x);
                let mut y_true = Array1::<f32>::zeros(2);
                y_true[label] = 1.0;
                let error = &y_pred - &y_true;
                // Gradient for weights and bias
                let grad_w = x.view().insert_axis(Axis(1)).dot(&error.view().insert_axis(Axis(0)));
                let grad_b = error.clone();
                self.weights = &self.weights - &(learning_rate * grad_w);
                self.bias = &self.bias - &(learning_rate * grad_b);
            }
        }
        Ok(())
    }

    /// Save the model to a file (weights and bias as .npz).
    pub fn save(&self, path: &str) -> io::Result<()> {
        ndarray_npy::write_npz(
            std::fs::File::create(path)?,
            vec![
                ("weights", &self.weights),
                ("bias", &self.bias),
            ],
        ).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
    }

    /// Load the model from a file (weights and bias from .npz).
    pub fn load(path: &str) -> io::Result<Self> {
        let mut npz = ndarray_npy::NpzReader::new(std::fs::File::open(path)?).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let weights: Array2<f32> = npz.by_name("weights.npy").map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let bias: Array1<f32> = npz.by_name("bias.npy").map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(Self { weights, bias })
    }
}