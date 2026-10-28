use super::*;
use axum::body::to_bytes;
use axum::http::Request;
use axum::{routing::post, Router};
use tower::ServiceExt;

#[test]
fn bearer_authorisation_keeps_the_exact_prefix_and_openai_error_shape() {
    assert!(authorised_token(&HeaderMap::new(), None).is_ok());
    for value in [None, Some("bearer token"), Some("Bearer wrong"), Some("Bearer token ")] {
        let mut headers = HeaderMap::new();
        if let Some(value) = value {
            headers.insert(header::AUTHORIZATION, value.parse().unwrap());
        }
        let error = authorised_token(&headers, Some("token")).err().unwrap();
        assert_eq!(error.status, StatusCode::UNAUTHORIZED);
        assert_eq!(error.kind, "invalid_request_error");
        assert_eq!(error.message, "Incorrect API key provided.");
    }
    let mut headers = HeaderMap::new();
    headers.insert(header::AUTHORIZATION, "Bearer token".parse().unwrap());
    assert!(authorised_token(&headers, Some("token")).is_ok());
}

#[tokio::test]
async fn speech_passes_the_same_tool_arguments_and_reads_the_returned_file_and_pcm() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("spoken.wav");
    let mut wav = b"RIFF\0\0\0\0WAVEdata".to_vec();
    wav.extend(4u32.to_le_bytes());
    wav.extend([1, 2, 3, 4]);
    std::fs::write(&path, &wav).unwrap();
    for (format, mime, expected) in [("wav", "audio/wav", wav.clone()), ("pcm", "audio/pcm", vec![1, 2, 3, 4])] {
        let file = path.clone();
        let response = speak_with(
            &HeaderMap::new(),
            &json!({"input":" Hello ","model":"tts-1","voice":"Alice","speed":2.0,"instructions":" Quietly ","response_format":format}),
            |args| async move {
                assert_eq!(
                    args,
                    json!({"text":"[rate:2.00] Hello","engine":"higgs","voice":"Alice","direction":"Quietly","format":"wav"})
                );
                Ok(json!({"done":true,"file":file}))
            },
        )
        .await
        .ok()
        .unwrap();
        assert_eq!(response.headers()[header::CONTENT_TYPE], mime);
        assert_eq!(to_bytes(response.into_body(), usize::MAX).await.unwrap().as_ref(), expected);
    }
    let file = path.clone();
    speak_with(
        &HeaderMap::new(),
        &json!({"input":"Hello","model":"tts-1","seed":7,"temperature":0.7,"top_k":40,"top_p":0.9,"max_tokens":800,"loudness":"podcast","response_format":"wav"}),
        |args| async move {
            assert_eq!(
                args,
                json!({"text":"Hello","engine":"higgs","format":"wav","seed":7,"temperature":0.7,"top_k":40,"top_p":0.9,"max_tokens":800,"loudness":"podcast"})
            );
            Ok(json!({"done":true,"file":file}))
        },
    )
    .await
    .ok()
    .unwrap();
    let error = speak_with(&HeaderMap::new(), &json!({"input":"Hello"}), |_| async {
        Ok(json!({"done":false}))
    })
    .await
    .err()
    .unwrap();
    assert_eq!(error.status, StatusCode::GATEWAY_TIMEOUT);
    let error = speak_with(&HeaderMap::new(), &json!({"input":"Hello"}), |_| async {
        Err("tool failed".into())
    })
    .await
    .err()
    .unwrap();
    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    assert_eq!(error.message, "tool failed");
}

async fn fake_transcription(State(st): State<AppState>, headers: HeaderMap, multipart: Multipart) -> Response {
    let workspace = st.workspace.clone();
    let mut row = Logged::default();
    let result=transcribe_with(&st,&headers,multipart,&mut row,|args|async move {
        assert_eq!(args["lang"],"ru");
        let path=std::path::Path::new(args["path"].as_str().unwrap());
        assert_eq!(path.parent(),Some(workspace.join(".uploads").as_path()));
        assert_eq!(std::fs::read(path).unwrap(),b"audio bytes");
        if args["format"]=="srt" || args["format"]=="vtt" {
            return Ok(json!({"done":true,"transcript":"subtitle text"}));
        }
        Ok(json!({"done":true,"transcript":json!({"duration":1.0,"lines":[{"text":"Hello","start":0.0,"end":1.0,"speaker":"a","words":[{"text":"Hello","start":0.0,"end":1.0}]}]}).to_string()}))
    }).await;
    match result {
        Ok(response) => response,
        Err(error) => error_response(&error),
    }
}

#[tokio::test]
async fn multipart_import_reuses_uploads_and_preserves_each_transcription_format() {
    let root = tempfile::tempdir().unwrap();
    let state = AppState::new(root.path());
    let workspace = state.workspace.clone();
    let router = Router::new()
        .route("/v1/audio/transcriptions", post(fake_transcription))
        .with_state(state);
    for format in ["json", "text", "verbose_json", "srt", "vtt"] {
        let body=format!("--b\r\nContent-Disposition: form-data; name=\"file\"; filename=\"record.MP3\"\r\nContent-Type: audio/mpeg\r\n\r\naudio bytes\r\n--b\r\nContent-Disposition: form-data; name=\"language\"\r\n\r\n RU \r\n--b\r\nContent-Disposition: form-data; name=\"response_format\"\r\n\r\n{format}\r\n--b--\r\n");
        let response = router
            .clone()
            .oneshot(
                Request::post("/v1/audio/transcriptions")
                    .header(header::CONTENT_TYPE, "multipart/form-data; boundary=b")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        if format == "json" {
            assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), json!({"text":"Hello"}));
        } else if format == "verbose_json" {
            let value: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(value["words"][0]["word"], "Hello");
            assert_eq!(value["language"], "ru");
        } else {
            assert_eq!(
                std::str::from_utf8(&bytes).unwrap(),
                if format == "text" { "Hello" } else { "subtitle text" }
            );
        }
    }
    assert_eq!(std::fs::read_dir(workspace.join(".uploads")).unwrap().count(), 1);
    let uploaded = workspace
        .join(".uploads")
        .join(format!("{}.mp3", blake3::hash(b"audio bytes").to_hex()));
    assert_eq!(std::fs::read(uploaded).unwrap(), b"audio bytes");
}
