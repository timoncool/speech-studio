use super::jobs::{self as speech_jobs, Scope, SpeechPaths};
use crate::app::refuse;
use crate::jobs;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::{json, Value};

#[cfg(test)]
mod tests;

pub(super) async fn enqueue(paths: &SpeechPaths, queue: &jobs::JobQueue, pid: String, body: Value) -> Response {
    let project = match crate::speech::load(&paths.workspace, &pid) {
        Ok(project) => project,
        Err(e) => return refuse(StatusCode::NOT_FOUND, "no_project", e),
    };
    let scope = Scope {
        ids: body
            .get("ids")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()),
        from: body.get("from").and_then(Value::as_str).map(str::to_string),
        force: body.get("force").and_then(Value::as_bool).unwrap_or(false),
        seed: body.get("seed").and_then(Value::as_u64),
    };
    // blocks with no voice are told before voicing starts, so the window takes the person to choosing one
    if let Some(e) = speech_jobs::no_voice(paths, &project, &speech_jobs::wanted(&project, &scope)) {
        return refuse(StatusCode::CONFLICT, "no_voice", e);
    }
    let aloud = body.get("aloud").and_then(Value::as_bool).unwrap_or(false);
    for _ in 0..40 {
        if let Some((job_id, jobs::JobKind::Synth)) = queue.active_for(&pid).await {
            if let Some(n) = speech_jobs::try_queue(&pid, &scope) {
                return Json(json!({ "job_id": job_id, "queued": n })).into_response();
            }
        }
        let p = paths.clone();
        let (pid2, scope2) = (pid.clone(), scope.clone());
        let playing = match aloud.then(|| crate::speech::live::aloud(&pid)).transpose() {
            Ok(playing) => playing,
            Err(e) => return refuse(StatusCode::CONFLICT, "no_window", e),
        };
        let job: jobs::JobFn = Box::new(move |progress: jobs::ProgressFn| {
            let _playing = playing;
            let cb = |ev: Value| progress(ev);
            speech_jobs::synth(&p, &pid2, &scope2, &cb)
        });
        match queue.enqueue(jobs::JobMeta::new(jobs::JobKind::Synth, Some(&pid)), job).await {
            Ok(job_id) => return Json(json!({ "job_id": job_id })).into_response(),
            Err(jobs::EnqueueError::Conflict {
                kind: jobs::JobKind::Synth,
                ..
            }) => tokio::time::sleep(std::time::Duration::from_millis(50)).await,
            Err(e) => return crate::app::enqueue_error(e),
        }
    }
    refuse(StatusCode::CONFLICT, "busy", "synthesis is finishing, try again")
}
