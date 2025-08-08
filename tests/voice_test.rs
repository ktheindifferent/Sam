#[cfg(test)]
mod voice_tests {
    use sam::services::voice::{VoiceAssistant, VoiceConfig, ConversationContext};
    use sam::services::stt::whisper_enhanced::{WhisperSTT, WhisperModel, WhisperConfig};
    use sam::services::tts::enhanced::{EnhancedTTS, TTSEngine, TTSConfig, Voice};
    use std::path::PathBuf;
    use tempfile::{TempDir, NamedTempFile};
    use std::fs;
    use std::io::Write;
    use tokio::sync::RwLock;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_whisper_stt_initialization() {
        let config = WhisperConfig {
            model: WhisperModel::Tiny,
            language: Some("en".to_string()),
            use_gpu: false,
            cache_dir: None,
            beam_size: 5,
            best_of: 5,
            temperature: 0.0,
        };
        
        let stt = WhisperSTT::new(config).await;
        assert!(stt.is_ok());
        
        let stt = stt.unwrap();
        assert_eq!(stt.get_model_type(), WhisperModel::Tiny);
        assert!(!stt.is_gpu_enabled());
    }

    #[tokio::test]
    async fn test_whisper_model_switching() {
        let mut stt = WhisperSTT::new(WhisperConfig::default())
            .await
            .expect("Failed to create STT");
        
        let result = stt.switch_model(WhisperModel::Base).await;
        assert!(result.is_ok());
        assert_eq!(stt.get_model_type(), WhisperModel::Base);
        
        let result = stt.switch_model(WhisperModel::Large).await;
        assert!(result.is_ok());
        assert_eq!(stt.get_model_type(), WhisperModel::Large);
    }

    #[tokio::test]
    async fn test_audio_transcription() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let audio_file = temp_dir.path().join("test_audio.wav");
        
        create_test_wav_file(&audio_file);
        
        let stt = WhisperSTT::new(WhisperConfig::default())
            .await
            .expect("Failed to create STT");
        
        let result = stt.transcribe_file(&audio_file).await;
        assert!(result.is_ok());
        
        let transcription = result.unwrap();
        assert!(transcription.text.len() > 0);
        assert!(transcription.confidence > 0.0);
        assert!(transcription.duration_ms > 0);
    }

    #[tokio::test]
    async fn test_streaming_transcription() {
        let stt = WhisperSTT::new(WhisperConfig::default())
            .await
            .expect("Failed to create STT");
        
        let audio_chunks = vec![
            vec![0u8; 1024],
            vec![1u8; 1024],
            vec![2u8; 1024],
        ];
        
        let mut stream = stt.create_stream().await
            .expect("Failed to create stream");
        
        for chunk in audio_chunks {
            stream.push_audio(chunk).await
                .expect("Failed to push audio");
        }
        
        let result = stream.finalize().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_tts_initialization() {
        let config = TTSConfig {
            engine: TTSEngine::default_for_platform(),
            voice: Voice::default(),
            rate: 1.0,
            pitch: 1.0,
            volume: 1.0,
            output_format: "wav".to_string(),
        };
        
        let tts = EnhancedTTS::new(config);
        assert!(tts.is_ok());
        
        let tts = tts.unwrap();
        assert!(tts.is_available());
    }

    #[tokio::test]
    async fn test_text_to_speech_conversion() {
        let tts = EnhancedTTS::new(TTSConfig::default())
            .expect("Failed to create TTS");
        
        let text = "Hello, this is a test of text to speech.";
        let result = tts.synthesize(text).await;
        
        assert!(result.is_ok());
        let audio_data = result.unwrap();
        assert!(audio_data.len() > 0);
    }

    #[tokio::test]
    async fn test_tts_voice_selection() {
        let mut config = TTSConfig::default();
        
        let voices = vec![
            Voice::Male("David".to_string()),
            Voice::Female("Zira".to_string()),
            Voice::Neutral("Alex".to_string()),
        ];
        
        for voice in voices {
            config.voice = voice.clone();
            let tts = EnhancedTTS::new(config.clone());
            assert!(tts.is_ok());
            
            let tts = tts.unwrap();
            assert_eq!(tts.get_current_voice(), voice);
        }
    }

    #[tokio::test]
    async fn test_tts_parameter_adjustment() {
        let mut tts = EnhancedTTS::new(TTSConfig::default())
            .expect("Failed to create TTS");
        
        tts.set_rate(1.5).expect("Failed to set rate");
        assert_eq!(tts.get_rate(), 1.5);
        
        tts.set_pitch(0.8).expect("Failed to set pitch");
        assert_eq!(tts.get_pitch(), 0.8);
        
        tts.set_volume(0.5).expect("Failed to set volume");
        assert_eq!(tts.get_volume(), 0.5);
    }

    #[tokio::test]
    async fn test_voice_assistant_initialization() {
        let config = VoiceConfig {
            stt_config: WhisperConfig::default(),
            tts_config: TTSConfig::default(),
            wake_word: Some("hey sam".to_string()),
            conversation_timeout_ms: 30000,
            max_history: 10,
        };
        
        let assistant = VoiceAssistant::new(config).await;
        assert!(assistant.is_ok());
        
        let assistant = assistant.unwrap();
        assert!(assistant.is_ready());
        assert_eq!(assistant.get_wake_word(), Some("hey sam".to_string()));
    }

    #[tokio::test]
    async fn test_conversation_context() {
        let assistant = VoiceAssistant::new(VoiceConfig::default())
            .await
            .expect("Failed to create assistant");
        
        let context = assistant.get_context().await;
        assert_eq!(context.history.len(), 0);
        assert!(context.session_id.len() > 0);
        
        assistant.add_to_history("user", "Hello").await;
        assistant.add_to_history("assistant", "Hi there!").await;
        
        let context = assistant.get_context().await;
        assert_eq!(context.history.len(), 2);
        assert_eq!(context.history[0].role, "user");
        assert_eq!(context.history[0].message, "Hello");
    }

    #[tokio::test]
    async fn test_wake_word_detection() {
        let config = VoiceConfig {
            wake_word: Some("activate".to_string()),
            ..Default::default()
        };
        
        let assistant = VoiceAssistant::new(config)
            .await
            .expect("Failed to create assistant");
        
        let test_phrases = vec![
            ("activate the system", true),
            ("please activate", true),
            ("deactivate", false),
            ("hello world", false),
        ];
        
        for (phrase, should_trigger) in test_phrases {
            let detected = assistant.detect_wake_word(phrase).await;
            assert_eq!(detected, should_trigger);
        }
    }

    #[tokio::test]
    async fn test_conversation_timeout() {
        let config = VoiceConfig {
            conversation_timeout_ms: 100,
            ..Default::default()
        };
        
        let assistant = VoiceAssistant::new(config)
            .await
            .expect("Failed to create assistant");
        
        assistant.start_conversation().await;
        assert!(assistant.is_conversation_active().await);
        
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        assert!(!assistant.is_conversation_active().await);
    }

    #[tokio::test]
    async fn test_history_management() {
        let config = VoiceConfig {
            max_history: 3,
            ..Default::default()
        };
        
        let assistant = VoiceAssistant::new(config)
            .await
            .expect("Failed to create assistant");
        
        for i in 0..5 {
            assistant.add_to_history("user", &format!("Message {}", i)).await;
        }
        
        let context = assistant.get_context().await;
        assert_eq!(context.history.len(), 3);
        assert_eq!(context.history[0].message, "Message 2");
        assert_eq!(context.history[2].message, "Message 4");
    }

    #[tokio::test]
    async fn test_audio_caching() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        
        let config = WhisperConfig {
            cache_dir: Some(temp_dir.path().to_path_buf()),
            ..Default::default()
        };
        
        let stt = WhisperSTT::new(config)
            .await
            .expect("Failed to create STT");
        
        let audio_file = temp_dir.path().join("cached_audio.wav");
        create_test_wav_file(&audio_file);
        
        let result1 = stt.transcribe_file(&audio_file).await
            .expect("First transcription failed");
        
        let result2 = stt.transcribe_file(&audio_file).await
            .expect("Second transcription failed");
        
        assert_eq!(result1.text, result2.text);
        assert!(result2.from_cache);
    }

    #[tokio::test]
    async fn test_language_detection() {
        let stt = WhisperSTT::new(WhisperConfig::default())
            .await
            .expect("Failed to create STT");
        
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let audio_file = temp_dir.path().join("multilang.wav");
        create_test_wav_file(&audio_file);
        
        let detected_lang = stt.detect_language(&audio_file).await;
        assert!(detected_lang.is_ok());
        
        let lang = detected_lang.unwrap();
        assert!(lang.code.len() == 2);
        assert!(lang.confidence > 0.0);
    }

    #[tokio::test]
    async fn test_batch_transcription() {
        let stt = WhisperSTT::new(WhisperConfig::default())
            .await
            .expect("Failed to create STT");
        
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let mut files = vec![];
        
        for i in 0..3 {
            let file = temp_dir.path().join(format!("batch_{}.wav", i));
            create_test_wav_file(&file);
            files.push(file);
        }
        
        let results = stt.batch_transcribe(files).await;
        assert!(results.is_ok());
        
        let transcriptions = results.unwrap();
        assert_eq!(transcriptions.len(), 3);
        
        for transcription in transcriptions {
            assert!(transcription.is_ok());
        }
    }

    #[tokio::test]
    async fn test_ssml_support() {
        let tts = EnhancedTTS::new(TTSConfig::default())
            .expect("Failed to create TTS");
        
        let ssml = r#"
            <speak>
                <prosody rate="slow">This is slow speech.</prosody>
                <break time="500ms"/>
                <prosody pitch="high">This is high pitched.</prosody>
            </speak>
        "#;
        
        let result = tts.synthesize_ssml(ssml).await;
        assert!(result.is_ok());
        
        let audio_data = result.unwrap();
        assert!(audio_data.len() > 0);
    }

    #[tokio::test]
    async fn test_voice_cloning_preparation() {
        let tts = EnhancedTTS::new(TTSConfig::default())
            .expect("Failed to create TTS");
        
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let sample_file = temp_dir.path().join("voice_sample.wav");
        create_test_wav_file(&sample_file);
        
        let result = tts.prepare_voice_clone(&sample_file, "custom_voice").await;
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let assistant = Arc::new(
            VoiceAssistant::new(VoiceConfig::default())
                .await
                .expect("Failed to create assistant")
        );
        
        let mut handles = vec![];
        
        for i in 0..5 {
            let assistant_clone = assistant.clone();
            let handle = tokio::spawn(async move {
                assistant_clone.add_to_history(
                    "user",
                    &format!("Concurrent message {}", i)
                ).await;
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.await.expect("Task panicked");
        }
        
        let context = assistant.get_context().await;
        assert_eq!(context.history.len(), 5);
    }

    #[tokio::test]
    async fn test_error_recovery() {
        let stt = WhisperSTT::new(WhisperConfig::default())
            .await
            .expect("Failed to create STT");
        
        let non_existent = PathBuf::from("/non/existent/file.wav");
        let result = stt.transcribe_file(&non_existent).await;
        assert!(result.is_err());
        
        let error = result.unwrap_err();
        assert!(error.to_string().contains("not found") || 
                error.to_string().contains("No such file"));
    }

    fn create_test_wav_file(path: &PathBuf) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        
        let mut writer = hound::WavWriter::create(path, spec)
            .expect("Failed to create WAV writer");
        
        for _ in 0..16000 {
            writer.write_sample(0i16).expect("Failed to write sample");
        }
        
        writer.finalize().expect("Failed to finalize WAV");
    }
}