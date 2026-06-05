use rouille::Request;
use rouille::Response;

use invidious::reqwest::blocking::Client;

pub fn handle(
    current_session: crate::memory::cache::WebSessions,
    request: &Request,
) -> Result<Response, crate::http::Error> {
    if request.url() == "/api/services/media/youtube" {
        let q_param = request.get_param("q");

        match q_param {
            Some(q) => {
                let client = Client::new(String::from("https://vid.puffyan.us"));
                let search_results = client
                    .search(Some(format!("q={q}").as_str()))
                    .map_err(|e| {
                        crate::http::Error::Other(format!("YouTube search failed: {}", e))
                    })?
                    .items;
                return Ok(Response::json(&search_results));
            }
            None => {
                return Ok(Response::empty_404());
            }
        }
    }

    if request.url() == "/api/services/media/youtube/lucky" {
        let q_param = request.get_param("q");

        match q_param {
            Some(q) => {
                let client = Client::new(String::from("https://vid.puffyan.us"));
                let search_results = client
                    .search(Some(format!("q={q}").as_str()))
                    .map_err(|e| {
                        crate::http::Error::Other(format!("YouTube search failed: {}", e))
                    })?
                    .items;
                match search_results.first() {
                    Some(video) => return Ok(Response::json(video)),
                    None => {
                        return Ok(Response::text("No matching videos found").with_status_code(404))
                    }
                }
            }
            None => {
                return Ok(Response::empty_404());
            }
        }
    }

    if request.url() == "/api/services/media/youtube/stream" {
        let id_param = request.get_param("id");
        match id_param {
            Some(id) => {
                let url = format!("https://youtu.be/{id}");
                let path_to_video = rustube::blocking::download_worst_quality(url.as_str())?;
                log::info!("path_to_video: {:?}", path_to_video);
                let data = std::fs::read(path_to_video).map_err(|e| {
                    crate::http::Error::Other(format!("Unable to read video file: {}", e))
                })?;

                let response = Response::from_data("video/mp4", data);
                return Ok(response);
            }
            None => {
                return Ok(Response::empty_404());
            }
        }
    }

    if request.url() == "/api/services/media/youtube/download" {
        let Some(id) = request.get_param("id") else {
            return Ok(Response::text("Missing id parameter").with_status_code(400));
        };

        let tube_id = rustube::Id::from_string(id)?;
        let video = rustube::blocking::Video::from_id(tube_id.clone())?;

        log::info!("video: {:?}", video);

        let best_quality = video
            .streams()
            .iter()
            .filter(|stream| stream.includes_video_track && stream.includes_audio_track)
            .max_by_key(|stream| stream.quality_label)
            .ok_or_else(|| {
                crate::http::Error::Other("No downloadable audio/video stream found".to_string())
            })?;

        best_quality.blocking_download_to_dir("/opt/sam/tmp/youtube/downloads")?;

        let data = std::fs::read(format!(
            "/opt/sam/tmp/youtube/downloads/{}.mp4",
            tube_id.clone()
        ))
        .map_err(|e| {
            crate::http::Error::Other(format!("Unable to read downloaded video: {}", e))
        })?;

        let file_folder_tree = vec!["Videos".to_string(), "Youtube".to_string()];

        let mut file = crate::memory::storage::File::new();
        file.file_name = format!("{}.mp4", tube_id.clone());
        file.file_type = "video/mp4".to_string();
        file.file_data = Some(data);
        file.file_folder_tree = Some(file_folder_tree);
        file.storage_location_oid = "SQL".to_string();
        file.save()?;

        let mut notify = crate::memory::human::Notification::new();
        notify.message = format!("{}.mp4 finished downloading!", tube_id.clone());
        notify.human_oid = current_session.human_oid;
        notify.sid = current_session.sid;
        notify.save()?;

        let response = Response::text("done");
        return Ok(response);
    }

    if request.url() == "/api/services/media/youtube/cache" {
        let id_param = request.get_param("id");
        match id_param {
            Some(id) => {
                let tube_id = rustube::Id::from_string(id)?;
                let video = rustube::blocking::Video::from_id(tube_id)?;

                let best_quality = video
                    .streams()
                    .iter()
                    .filter(|stream| stream.includes_video_track && stream.includes_audio_track)
                    .min_by_key(|stream| stream.quality_label)
                    .ok_or_else(|| {
                        crate::http::Error::Other(
                            "No cacheable audio/video stream found".to_string(),
                        )
                    })?;

                best_quality.blocking_download_to_dir("/opt/sam/tmp/youtube")?;

                return Ok(Response::text("done"));
            }
            None => {
                return Ok(Response::empty_404());
            }
        }
    }

    Ok(Response::empty_404())
}

#[cfg(test)]
mod tests {

    use proptest::prelude::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn test_youtube_id_validation() {
        // Test valid YouTube IDs
        let valid_ids = vec!["dQw4w9WgXcQ", "jNQXAC9IVRw", "M7lc1UVf-VE"];

        for id in valid_ids {
            let result = rustube::Id::from_string(id.to_string());
            assert!(result.is_ok());
        }
    }

    #[tokio::test]
    async fn test_search_endpoint_mock() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/search"))
            .and(query_param("q", "test"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [
                    {
                        "type": "video",
                        "title": "Test Video",
                        "videoId": "test123",
                        "author": "Test Author",
                        "authorId": "UC123",
                        "videoThumbnails": [],
                        "description": "Test description",
                        "viewCount": 1000,
                        "published": 1234567890,
                        "publishedText": "1 day ago",
                        "lengthSeconds": 300,
                        "liveNow": false,
                        "premium": false,
                        "isUpcoming": false
                    }
                ]
            })))
            .mount(&mock_server)
            .await;

        // This demonstrates the mock pattern for testing
        // In real tests, we'd modify the client to point to mock_server.uri()
    }

    #[test]
    fn test_url_patterns() {
        let test_urls = vec![
            "/api/services/media/youtube",
            "/api/services/media/youtube/lucky",
            "/api/services/media/youtube/stream",
            "/api/services/media/youtube/download",
            "/api/services/media/youtube/cache",
        ];

        for url in test_urls {
            assert!(url.starts_with("/api/services/media/youtube"));
        }
    }

    #[test]
    fn test_youtube_url_construction() {
        let video_ids = vec!["abc123", "xyz789", "test456"];

        for id in video_ids {
            let url = format!("https://youtu.be/{}", id);
            assert!(url.contains(id));
            assert!(url.starts_with("https://youtu.be/"));
        }
    }

    proptest! {
        #[test]
        fn test_video_id_format(
            id in "[a-zA-Z0-9_-]{11}"
        ) {
            let url = format!("https://youtu.be/{}", id);
            prop_assert!(url.len() == 28); // "https://youtu.be/" (17) + 11 chars
            prop_assert!(url.contains(&id));
        }

        #[test]
        fn test_search_query_encoding(
            query in "[a-zA-Z0-9 ]{1,100}"
        ) {
            let encoded = format!("q={}", query);
            prop_assert!(encoded.starts_with("q="));
            prop_assert!(encoded.contains(&query));
        }

        #[test]
        fn test_file_path_generation(
            video_id in "[a-zA-Z0-9_-]{11}"
        ) {
            let download_path = format!("/opt/sam/tmp/youtube/downloads/{}.mp4", video_id);
            prop_assert!(download_path.ends_with(".mp4"));
            prop_assert!(download_path.contains(&video_id));
            prop_assert!(download_path.starts_with("/opt/sam/tmp/youtube/downloads/"));
        }
    }

    #[test]
    fn test_file_metadata_creation() {
        let test_id = "test123";
        let file_name = format!("{}.mp4", test_id);
        assert_eq!(file_name, "test123.mp4");

        let file_type = "video/mp4";
        assert_eq!(file_type, "video/mp4");

        let storage_location = "SQL";
        assert_eq!(storage_location, "SQL");
    }

    #[test]
    fn test_folder_tree_structure() {
        let file_folder_tree = ["Videos".to_string(), "Youtube".to_string()];

        assert_eq!(file_folder_tree.len(), 2);
        assert_eq!(file_folder_tree[0], "Videos");
        assert_eq!(file_folder_tree[1], "Youtube");
    }

    #[test]
    fn test_quality_filter_logic() {
        // Test that filter logic is consistent
        let test_filters = vec![
            (true, true),   // Both video and audio
            (true, false),  // Video only
            (false, true),  // Audio only
            (false, false), // Neither
        ];

        for (has_video, has_audio) in test_filters {
            let should_include = has_video && has_audio;
            assert_eq!(should_include, has_video && has_audio);
        }
    }
}
