use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};

// Benchmark LIFX service endpoints
// fn bench_lifx_endpoint_selection(c: &mut Criterion) {
//     let mut group = c.benchmark_group("lifx_endpoints");

//     for public in &[true, false] {
//         group.bench_with_input(
//             BenchmarkId::from_parameter(if *public { "public" } else { "private" }),
//             public,
//             |b, &public| {
//                 b.iter(|| {
//                     sam::sam::services::lifx::select_lifx_endpoint(black_box(public))
//                 });
//             },
//         );
//     }
//     group.finish();
// }

// Benchmark Spotify state operations
// fn bench_spotify_status(c: &mut Criterion) {
//     c.bench_function("spotify_status", |b| {
//         b.iter(|| {
//             sam::sam::services::spotify::status()
//         });
//     });
// }

// Benchmark sound processing operations
fn bench_wav_spec_creation(c: &mut Criterion) {
    use hound::WavSpec;

    c.bench_function("wav_spec_creation", |b| {
        b.iter(|| WavSpec {
            channels: black_box(2),
            sample_rate: black_box(44100),
            bits_per_sample: black_box(16),
            sample_format: hound::SampleFormat::Int,
        });
    });
}

// Benchmark noise gate processing
// fn bench_noise_gate_processing(c: &mut Criterion) {
//     use noise_gate::NoiseGate;

//     let noise_gate = NoiseGate::<f32>::new(
//         -30.0,  // open_threshold
//         -40.0,  // close_threshold
//     );

//     let mut group = c.benchmark_group("noise_gate");
//     group.warm_up_time(Duration::from_secs(1));

//     // Benchmark single sample processing
//     group.bench_function("single_sample", |b| {
//         let mut sample = [0.5f32];
//         b.iter(|| {
//             noise_gate.process_frame(black_box(sample))
//         });
//     });

//     // Benchmark batch processing
//     group.bench_function("batch_1000_samples", |b| {
//         let samples: Vec<[f32; 1]> = (0..1000)
//             .map(|i| [(i as f32 * 0.01).sin()])
//             .collect();

//         b.iter(|| {
//             for sample in &samples {
//                 noise_gate.process_frame(black_box(*sample));
//             }
//         });
//     });

//     group.finish();
// }

// Benchmark YouTube URL construction
fn bench_youtube_url_construction(c: &mut Criterion) {
    let video_ids = vec!["dQw4w9WgXcQ", "jNQXAC9IVRw", "M7lc1UVf-VE"];

    c.bench_function("youtube_url_construction", |b| {
        b.iter(|| {
            for id in &video_ids {
                let _ = format!("https://youtu.be/{}", black_box(id));
            }
        });
    });
}

// Benchmark base64 encoding for Spotify auth
fn bench_base64_encoding(c: &mut Criterion) {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;

    let test_credentials = vec![
        ("short_id", "short_secret"),
        ("medium_length_client_id", "medium_length_client_secret"),
        (
            "very_long_client_id_for_testing_purposes",
            "very_long_client_secret_for_testing_purposes",
        ),
    ];

    let mut group = c.benchmark_group("base64_encoding");

    for (id, secret) in test_credentials {
        let label = format!("{}_{}", id.len(), secret.len());
        group.bench_with_input(
            BenchmarkId::from_parameter(&label),
            &(id, secret),
            |b, &(id, secret)| {
                b.iter(|| STANDARD.encode(format!("{}:{}", black_box(id), black_box(secret))));
            },
        );
    }

    group.finish();
}

// Benchmark concurrent operations
fn bench_concurrent_access(c: &mut Criterion) {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let mut group = c.benchmark_group("concurrent_access");
    group.sample_size(10); // Reduce sample size for thread-heavy benchmarks

    for num_threads in &[1, 2, 4, 8] {
        group.bench_with_input(
            BenchmarkId::from_parameter(num_threads),
            num_threads,
            |b, &num_threads| {
                b.iter(|| {
                    let data = Arc::new(Mutex::new(0));
                    let mut handles = vec![];

                    for _ in 0..num_threads {
                        let data = data.clone();
                        let handle = thread::spawn(move || {
                            for _ in 0..100 {
                                let mut guard = data
                                    .lock()
                                    .expect("Failed to acquire mutex lock in benchmark");
                                *guard += 1;
                            }
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.join().expect("Thread panicked during benchmark");
                    }
                });
            },
        );
    }

    group.finish();
}

// Benchmark file path operations
fn bench_file_path_operations(c: &mut Criterion) {
    use std::path::PathBuf;

    c.bench_function("path_join_operations", |b| {
        b.iter(|| {
            let base = PathBuf::from("/opt/sam/tmp");
            let _path = base.join("youtube").join("downloads").join("video.mp4");
        });
    });

    c.bench_function("path_string_format", |b| {
        b.iter(|| {
            let _ = format!(
                "/opt/sam/tmp/youtube/downloads/{}.mp4",
                black_box("test_video")
            );
        });
    });
}

criterion_group!(
    benches,
    // bench_lifx_endpoint_selection,
    // bench_spotify_status,
    bench_wav_spec_creation,
    // bench_noise_gate_processing,
    bench_youtube_url_construction,
    bench_base64_encoding,
    bench_concurrent_access,
    bench_file_path_operations
);

criterion_main!(benches);
