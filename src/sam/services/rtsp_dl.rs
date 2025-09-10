// RTSP Deep Learning Module
// Provides computer vision and deep learning capabilities for RTSP streams

use crate::sam::memory::{Observation, ObservationType, ObservationObjects};
// use crate::sam::services::errors::CommonError;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::task;

#[cfg(feature = "opencv")]
use std::process::{Command, Stdio};

// Mock types for testing when OpenCV is not available
#[cfg(not(feature = "opencv"))]
pub struct MockMat;

#[cfg(not(feature = "opencv"))]
pub struct MockRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

// Deep Learning Detection Results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    pub timestamp: i64,
    pub detections: Vec<Detection>,
    pub motion_detected: bool,
    pub anomaly_score: f32,
    pub faces: Vec<FaceDetection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Detection {
    pub class: String,
    pub confidence: f32,
    pub bbox: BoundingBox,
    pub track_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceDetection {
    pub bbox: BoundingBox,
    pub confidence: f32,
    pub encoding: Option<Vec<f32>>,
    pub identity: Option<String>,
}

// Motion Detection State
#[cfg(feature = "opencv")]
#[derive(Clone)]
pub struct MotionDetector {
    background_subtractor: Arc<Mutex<core::Ptr<opencv::video::BackgroundSubtractorMOG2>>>,
    motion_threshold: f32,
    min_area: f32,
}

#[cfg(not(feature = "opencv"))]
#[derive(Clone)]
pub struct MotionDetector {
    motion_threshold: f32,
    min_area: f32,
}

impl MotionDetector {
    #[cfg(feature = "opencv")]
    pub fn new(motion_threshold: f32, min_area: f32) -> Result<Self> {
        let bg_subtractor = opencv::video::create_background_subtractor_mog2(500, 16.0, true)?;
        Ok(Self {
            background_subtractor: Arc::new(Mutex::new(bg_subtractor)),
            motion_threshold,
            min_area,
        })
    }

    #[cfg(not(feature = "opencv"))]
    pub fn new(motion_threshold: f32, min_area: f32) -> Result<Self> {
        Ok(Self {
            motion_threshold,
            min_area,
        })
    }

    #[cfg(feature = "opencv")]
    pub fn detect_motion(&self, frame: &core::Mat) -> Result<(bool, Vec<core::Rect>)> {
        let mut fg_mask = core::Mat::default();
        let mut bg_sub = self.background_subtractor.lock().unwrap();
        
        bg_sub.apply(frame, &mut fg_mask, -1.0)?;
        
        // Find contours
        let mut contours = core::Vector::<core::Vector<core::Point>>::new();
        imgproc::find_contours(
            &fg_mask,
            &mut contours,
            imgproc::RETR_EXTERNAL,
            imgproc::CHAIN_APPROX_SIMPLE,
            core::Point::new(0, 0),
        )?;
        
        let mut motion_areas = Vec::new();
        let mut motion_detected = false;
        
        for i in 0..contours.len() {
            let contour = contours.get(i)?;
            let area = imgproc::contour_area(&contour, false)?;
            
            if area > self.min_area as f64 {
                let rect = imgproc::bounding_rect(&contour)?;
                motion_areas.push(rect);
                motion_detected = true;
            }
        }
        
        Ok((motion_detected, motion_areas))
    }

    #[cfg(not(feature = "opencv"))]
    pub fn detect_motion(&self, _frame: &MockMat) -> Result<(bool, Vec<MockRect>)> {
        // Mock implementation for testing
        Ok((false, Vec::new()))
    }
}

// Object Detection with YOLO
#[cfg(feature = "opencv")]
pub struct YoloDetector {
    net: opencv::dnn::Net,
    classes: Vec<String>,
    confidence_threshold: f32,
    nms_threshold: f32,
}

#[cfg(not(feature = "opencv"))]
pub struct YoloDetector {
    classes: Vec<String>,
    confidence_threshold: f32,
    nms_threshold: f32,
}

impl YoloDetector {
    #[cfg(feature = "opencv")]
    pub fn new(config_path: &str, weights_path: &str, names_path: &str) -> Result<Self> {
        // Load YOLO model
        let net = opencv::dnn::read_net_from_darknet(config_path, weights_path)?;
        
        // Load class names
        let classes = std::fs::read_to_string(names_path)?
            .lines()
            .map(|s| s.to_string())
            .collect();
        
        Ok(Self {
            net,
            classes,
            confidence_threshold: 0.5,
            nms_threshold: 0.4,
        })
    }

    #[cfg(not(feature = "opencv"))]
    pub fn new(_config_path: &str, _weights_path: &str, names_path: &str) -> Result<Self> {
        // Load class names
        let classes = std::fs::read_to_string(names_path)?
            .lines()
            .map(|s| s.to_string())
            .collect();
        
        Ok(Self {
            classes,
            confidence_threshold: 0.5,
            nms_threshold: 0.4,
        })
    }

    #[cfg(feature = "opencv")]
    pub fn detect(&mut self, frame: &core::Mat) -> Result<Vec<Detection>> {
        // Create blob from image
        let blob = opencv::dnn::blob_from_image(
            frame,
            1.0 / 255.0,
            core::Size::new(416, 416),
            core::Scalar::new(0.0, 0.0, 0.0, 0.0),
            true,
            false,
            core::CV_32F,
        )?;
        
        self.net.set_input(&blob, "", 1.0, core::Scalar::default())?;
        
        // Get output layer names
        let output_names = self.net.get_unconnected_out_layers_names()?;
        let mut outputs = core::Vector::<core::Mat>::new();
        self.net.forward(&mut outputs, &output_names)?;
        
        let mut detections = Vec::new();
        let frame_height = frame.rows();
        let frame_width = frame.cols();
        
        // Process outputs
        for i in 0..outputs.len() {
            let output = outputs.get(i)?;
            let rows = output.rows();
            
            for j in 0..rows {
                let scores_start = 5;
                let num_classes = output.cols() - scores_start;
                let mut max_score = 0.0f32;
                let mut class_id = 0;
                
                // Find class with maximum score
                for k in 0..num_classes {
                    let score = *output.at_2d::<f32>(j, scores_start + k)?;
                    if score > max_score {
                        max_score = score;
                        class_id = k;
                    }
                }
                
                if max_score > self.confidence_threshold {
                    let center_x = *output.at_2d::<f32>(j, 0)? * frame_width as f32;
                    let center_y = *output.at_2d::<f32>(j, 1)? * frame_height as f32;
                    let width = *output.at_2d::<f32>(j, 2)? * frame_width as f32;
                    let height = *output.at_2d::<f32>(j, 3)? * frame_height as f32;
                    
                    detections.push(Detection {
                        class: self.classes.get(class_id).unwrap_or(&"unknown".to_string()).clone(),
                        confidence: max_score,
                        bbox: BoundingBox {
                            x: center_x - width / 2.0,
                            y: center_y - height / 2.0,
                            width,
                            height,
                        },
                        track_id: None,
                    });
                }
            }
        }
        
        // Apply NMS
        let mut boxes = Vec::new();
        let mut confidences = Vec::new();
        let mut class_ids = Vec::new();
        
        for detection in &detections {
            boxes.push(core::Rect::new(
                detection.bbox.x as i32,
                detection.bbox.y as i32,
                detection.bbox.width as i32,
                detection.bbox.height as i32,
            ));
            confidences.push(detection.confidence);
            class_ids.push(detections.iter().position(|d| d.class == detection.class).unwrap() as i32);
        }
        
        let mut indices = Vec::new();
        opencv::dnn::nms_boxes(
            &boxes,
            &confidences,
            self.confidence_threshold,
            self.nms_threshold,
            &mut indices,
            1.0,
            0,
        )?;
        
        let mut final_detections = Vec::new();
        for &idx in &indices {
            final_detections.push(detections[idx as usize].clone());
        }
        
        Ok(final_detections)
    }

    #[cfg(not(feature = "opencv"))]
    pub fn detect(&mut self, _frame: &[u8]) -> Result<Vec<Detection>> {
        // Stub implementation when opencv is not available
        Ok(vec![])
    }
}

// Face Detection and Recognition
#[cfg(feature = "opencv")]
pub struct FaceRecognizer {
    face_cascade: objdetect::CascadeClassifier,
    face_embeddings: HashMap<String, Vec<f32>>,
}

#[cfg(not(feature = "opencv"))]
pub struct FaceRecognizer {
    face_embeddings: HashMap<String, Vec<f32>>,
}

impl FaceRecognizer {
    #[cfg(feature = "opencv")]
    pub fn new() -> Result<Self> {
        let cascade_path = "/usr/share/opencv4/haarcascades/haarcascade_frontalface_default.xml";
        let face_cascade = objdetect::CascadeClassifier::new(cascade_path)?;
        
        Ok(Self {
            face_cascade,
            face_embeddings: HashMap::new(),
        })
    }

    #[cfg(not(feature = "opencv"))]
    pub fn new() -> Result<Self> {
        Ok(Self {
            face_embeddings: HashMap::new(),
        })
    }

    #[cfg(feature = "opencv")]
    pub fn detect_faces(&mut self, frame: &core::Mat) -> Result<Vec<FaceDetection>> {
        let mut gray = core::Mat::default();
        imgproc::cvt_color(frame, &mut gray, imgproc::COLOR_BGR2GRAY, 0)?;
        
        let mut faces = core::Vector::<core::Rect>::new();
        self.face_cascade.detect_multi_scale(
            &gray,
            &mut faces,
            1.1,
            3,
            0,
            core::Size::new(30, 30),
            core::Size::new(0, 0),
        )?;
        
        let mut face_detections = Vec::new();
        for i in 0..faces.len() {
            let face = faces.get(i)?;
            face_detections.push(FaceDetection {
                bbox: BoundingBox {
                    x: face.x as f32,
                    y: face.y as f32,
                    width: face.width as f32,
                    height: face.height as f32,
                },
                confidence: 1.0,
                encoding: None,
                identity: None,
            });
        }
        
        Ok(face_detections)
    }

    #[cfg(not(feature = "opencv"))]
    pub fn detect_faces(&mut self, _frame: &[u8]) -> Result<Vec<FaceDetection>> {
        // Stub implementation when opencv is not available
        Ok(vec![])
    }

    pub fn add_known_face(&mut self, name: String, encoding: Vec<f32>) {
        self.face_embeddings.insert(name, encoding);
    }

    pub fn recognize_face(&self, encoding: &[f32]) -> Option<String> {
        let mut best_match = None;
        let mut best_distance = f32::MAX;
        
        for (name, known_encoding) in &self.face_embeddings {
            let distance = euclidean_distance(encoding, known_encoding);
            if distance < 0.6 && distance < best_distance {
                best_distance = distance;
                best_match = Some(name.clone());
            }
        }
        
        best_match
    }
}

// Anomaly Detection
pub struct AnomalyDetector {
    baseline_stats: Option<FrameStatistics>,
    sensitivity: f32,
    history: Vec<FrameStatistics>,
    history_size: usize,
}

#[derive(Clone, Debug)]
struct FrameStatistics {
    mean_intensity: f32,
    std_deviation: f32,
    edge_density: f32,
    motion_level: f32,
}

impl AnomalyDetector {
    pub fn new(sensitivity: f32, history_size: usize) -> Self {
        Self {
            baseline_stats: None,
            sensitivity,
            history: Vec::new(),
            history_size,
        }
    }

    #[cfg(feature = "opencv")]
    pub fn detect_anomaly(&mut self, frame: &core::Mat, motion_level: f32) -> Result<f32> {
        let stats = self.calculate_frame_statistics(frame, motion_level)?;
        
        // Update history
        self.history.push(stats.clone());
        if self.history.len() > self.history_size {
            self.history.remove(0);
        }
        
        // Calculate baseline if needed
        if self.baseline_stats.is_none() && self.history.len() >= 10 {
            self.calculate_baseline();
        }
        
        // Calculate anomaly score
        if let Some(baseline) = &self.baseline_stats {
            let intensity_diff = (stats.mean_intensity - baseline.mean_intensity).abs() / baseline.mean_intensity;
            let std_diff = (stats.std_deviation - baseline.std_deviation).abs() / baseline.std_deviation.max(0.001);
            let edge_diff = (stats.edge_density - baseline.edge_density).abs() / baseline.edge_density.max(0.001);
            let motion_diff = (stats.motion_level - baseline.motion_level).abs() / baseline.motion_level.max(0.001);
            
            let anomaly_score = (intensity_diff + std_diff + edge_diff + motion_diff) / 4.0;
            Ok(anomaly_score * self.sensitivity)
        } else {
            Ok(0.0)
        }
    }

    #[cfg(feature = "opencv")]
    fn calculate_frame_statistics(&self, frame: &core::Mat, motion_level: f32) -> Result<FrameStatistics> {
        let mut gray = core::Mat::default();
        imgproc::cvt_color(frame, &mut gray, imgproc::COLOR_BGR2GRAY, 0)?;
        
        // Calculate mean and std deviation
        let mut mean = core::Mat::default();
        let mut stddev = core::Mat::default();
        core::mean_std_dev(&gray, &mut mean, &mut stddev, &core::no_array())?;
        
        let mean_intensity = *mean.at::<f64>(0)? as f32;
        let std_deviation = *stddev.at::<f64>(0)? as f32;
        
        // Calculate edge density
        let mut edges = core::Mat::default();
        imgproc::canny(&gray, &mut edges, 50.0, 150.0, 3, false)?;
        let edge_pixels = core::count_non_zero(&edges)?;
        let total_pixels = (edges.rows() * edges.cols()) as f32;
        let edge_density = edge_pixels as f32 / total_pixels;
        
        Ok(FrameStatistics {
            mean_intensity,
            std_deviation,
            edge_density,
            motion_level,
        })
    }

    #[cfg(not(feature = "opencv"))]
    fn calculate_frame_statistics(&self, _frame: &[u8], motion_level: f32) -> Result<FrameStatistics> {
        Ok(FrameStatistics {
            mean_intensity: 128.0,  // Default gray value
            std_deviation: 0.0,
            edge_density: 0.0,
            motion_level,
        })
    }

    fn calculate_baseline(&mut self) {
        if self.history.is_empty() {
            return;
        }
        
        let n = self.history.len() as f32;
        let mean_intensity = self.history.iter().map(|s| s.mean_intensity).sum::<f32>() / n;
        let std_deviation = self.history.iter().map(|s| s.std_deviation).sum::<f32>() / n;
        let edge_density = self.history.iter().map(|s| s.edge_density).sum::<f32>() / n;
        let motion_level = self.history.iter().map(|s| s.motion_level).sum::<f32>() / n;
        
        self.baseline_stats = Some(FrameStatistics {
            mean_intensity,
            std_deviation,
            edge_density,
            motion_level,
        });
    }
}

// Alerting System
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub timestamp: i64,
    pub alert_type: AlertType,
    pub description: String,
    pub severity: AlertSeverity,
    pub data: Option<serde_json::Value>,
    pub thing_oid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    MotionDetected,
    ObjectDetected(String),
    FaceDetected,
    UnknownFaceDetected,
    AnomalyDetected,
    TamperingDetected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

pub struct AlertManager {
    alert_tx: mpsc::Sender<Alert>,
    cooldown_duration: Duration,
    last_alert_times: Arc<Mutex<HashMap<String, SystemTime>>>,
}

impl AlertManager {
    pub fn new(alert_tx: mpsc::Sender<Alert>, cooldown_seconds: u64) -> Self {
        Self {
            alert_tx,
            cooldown_duration: Duration::from_secs(cooldown_seconds),
            last_alert_times: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn send_alert(&self, alert: Alert) -> Result<()> {
        let alert_key = format!("{:?}_{}", alert.alert_type, alert.thing_oid);
        
        // Check cooldown
        let mut last_times = self.last_alert_times.lock().unwrap();
        if let Some(last_time) = last_times.get(&alert_key) {
            if last_time.elapsed().unwrap_or(Duration::MAX) < self.cooldown_duration {
                return Ok(()); // Skip alert due to cooldown
            }
        }
        
        // Send alert
        self.alert_tx.send(alert).await?;
        last_times.insert(alert_key, SystemTime::now());
        
        Ok(())
    }
}

// RTSP Stream Processor
pub struct RtspStreamProcessor {
    thing_oid: String,
    rtsp_url: String,
    motion_detector: MotionDetector,
    yolo_detector: Option<YoloDetector>,
    face_recognizer: FaceRecognizer,
    anomaly_detector: AnomalyDetector,
    alert_manager: AlertManager,
}

impl RtspStreamProcessor {
    pub fn new(
        thing_oid: String,
        rtsp_url: String,
        alert_tx: mpsc::Sender<Alert>,
    ) -> Result<Self> {
        let motion_detector = MotionDetector::new(0.02, 500.0)?;
        let face_recognizer = FaceRecognizer::new()?;
        let anomaly_detector = AnomalyDetector::new(1.5, 100);
        let alert_manager = AlertManager::new(alert_tx, 60);
        
        // Try to load YOLO if available
        let yolo_detector = if Path::new("/opt/sam/models/yolo/yolov4.cfg").exists() 
            && Path::new("/opt/sam/models/yolo/yolov4.weights").exists() 
            && Path::new("/opt/sam/models/yolo/coco.names").exists() {
            Some(YoloDetector::new(
                "/opt/sam/models/yolo/yolov4.cfg",
                "/opt/sam/models/yolo/yolov4.weights",
                "/opt/sam/models/yolo/coco.names",
            )?)
        } else {
            log::warn!("YOLO model files not found, object detection disabled");
            None
        };
        
        Ok(Self {
            thing_oid,
            rtsp_url,
            motion_detector,
            yolo_detector,
            face_recognizer,
            anomaly_detector,
            alert_manager,
        })
    }

    #[cfg(feature = "opencv")]
    pub async fn process_stream(&mut self) -> Result<()> {
        let mut cap = videoio::VideoCapture::new(&self.rtsp_url, videoio::CAP_ANY)?;
        
        if !cap.is_opened()? {
            return Err(anyhow::anyhow!("Failed to open RTSP stream"));
        }
        
        let mut frame = core::Mat::default();
        let mut frame_count = 0u64;
        
        loop {
            if !cap.read(&mut frame)? || frame.empty() {
                log::warn!("Failed to read frame or empty frame");
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
            
            frame_count += 1;
            
            // Process every Nth frame to reduce CPU load
            if frame_count % 5 != 0 {
                continue;
            }
            
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;
            
            // Motion detection
            let (motion_detected, motion_areas) = self.motion_detector.detect_motion(&frame)?;
            let motion_level = motion_areas.len() as f32 / 10.0; // Normalized motion level
            
            // Object detection
            let mut object_detections = Vec::new();
            if let Some(yolo) = &mut self.yolo_detector {
                object_detections = yolo.detect(&frame)?;
                
                // Send alerts for detected objects
                for detection in &object_detections {
                    if detection.confidence > 0.7 {
                        let alert = Alert {
                            timestamp,
                            alert_type: AlertType::ObjectDetected(detection.class.clone()),
                            description: format!("{} detected with {:.2}% confidence", detection.class, detection.confidence * 100.0),
                            severity: if detection.class == "person" {
                                AlertSeverity::High
                            } else {
                                AlertSeverity::Medium
                            },
                            data: Some(serde_json::json!({
                                "class": detection.class,
                                "confidence": detection.confidence,
                                "bbox": detection.bbox,
                            })),
                            thing_oid: self.thing_oid.clone(),
                        };
                        self.alert_manager.send_alert(alert).await?;
                    }
                }
            }
            
            // Face detection
            let faces = self.face_recognizer.detect_faces(&frame)?;
            for face in &faces {
                let alert = Alert {
                    timestamp,
                    alert_type: if face.identity.is_some() {
                        AlertType::FaceDetected
                    } else {
                        AlertType::UnknownFaceDetected
                    },
                    description: if let Some(identity) = &face.identity {
                        format!("Known person detected: {}", identity)
                    } else {
                        "Unknown face detected".to_string()
                    },
                    severity: AlertSeverity::Medium,
                    data: Some(serde_json::json!({
                        "bbox": face.bbox,
                        "identity": face.identity,
                    })),
                    thing_oid: self.thing_oid.clone(),
                };
                self.alert_manager.send_alert(alert).await?;
            }
            
            // Anomaly detection
            let anomaly_score = self.anomaly_detector.detect_anomaly(&frame, motion_level)?;
            if anomaly_score > 0.8 {
                let alert = Alert {
                    timestamp,
                    alert_type: AlertType::AnomalyDetected,
                    description: format!("Anomaly detected with score {:.2}", anomaly_score),
                    severity: if anomaly_score > 1.5 {
                        AlertSeverity::Critical
                    } else {
                        AlertSeverity::High
                    },
                    data: Some(serde_json::json!({
                        "anomaly_score": anomaly_score,
                        "motion_level": motion_level,
                    })),
                    thing_oid: self.thing_oid.clone(),
                };
                self.alert_manager.send_alert(alert).await?;
            }
            
            // Store observation in database
            if motion_detected || !object_detections.is_empty() || !faces.is_empty() || anomaly_score > 0.5 {
                self.store_observation(
                    timestamp,
                    motion_detected,
                    object_detections,
                    faces,
                    anomaly_score,
                ).await?;
            }
            
            // Small delay to prevent CPU overload
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    async fn store_observation(
        &self,
        timestamp: i64,
        motion_detected: bool,
        detections: Vec<Detection>,
        faces: Vec<FaceDetection>,
        anomaly_score: f32,
    ) -> Result<()> {
        let mut observation = Observation::new();
        observation.timestamp = timestamp;
        observation.observation_type = if motion_detected {
            ObservationType::Motion
        } else if !detections.is_empty() {
            ObservationType::Object
        } else {
            ObservationType::UNKNOWN
        };
        
        // Clone detections before moving them
        let detections_clone = detections.clone();
        
        // Add detected objects
        for detection in detections {
            observation.observation_objects.push(ObservationObjects {
                name: detection.class.clone(),
                confidence: detection.confidence,
                location: Some(format!("{},{},{},{}", 
                    detection.bbox.x, detection.bbox.y, 
                    detection.bbox.width, detection.bbox.height)),
            });
        }
        
        // Add detection notes
        if motion_detected {
            observation.observation_notes.push("Motion detected".to_string());
        }
        if anomaly_score > 0.5 {
            observation.observation_notes.push(format!("Anomaly score: {:.2}", anomaly_score));
        }
        if !faces.is_empty() {
            observation.observation_notes.push(format!("{} face(s) detected", faces.len()));
        }
        
        // Store deep vision results
        observation.deep_vision_json = Some(serde_json::json!({
            "motion_detected": motion_detected,
            "detections": detections_clone,
            "faces": faces,
            "anomaly_score": anomaly_score,
        }).to_string());
        
        // Save to database
        task::spawn_blocking(move || {
            if let Err(e) = observation.save() {
                log::error!("Failed to save observation: {}", e);
            }
        }).await?;
        
        Ok(())
    }
}

// Utility functions
fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

impl RtspStreamProcessor {
    #[cfg(not(feature = "opencv"))]
    pub async fn process_stream(&mut self) -> Result<()> {
        // Stub implementation when opencv is not available
        log::info!("OpenCV not available - RTSP processing disabled");
        Ok(())
    }
}

// Public API for starting DL processing
pub async fn start_deep_learning_processor(
    thing_oid: String,
    rtsp_url: String,
    alert_tx: mpsc::Sender<Alert>,
) -> Result<()> {
    let mut processor = RtspStreamProcessor::new(thing_oid, rtsp_url, alert_tx)?;
    processor.process_stream().await
}