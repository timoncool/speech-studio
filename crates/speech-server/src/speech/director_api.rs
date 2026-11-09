//! Маршруты режиссёра-LLM: сценарий подкаста — новый проект Speech с репликами спикеров; разметка главы по
//! персонажам — блоки главы заменяются репликами рассказчика и персонажей. Обе задачи идут общей очередью: модель
//! (своя Gemma, сервер пользователя или OpenRouter — как выбрано в настройках) занимает видеокарту, поэтому озвучка
//! перед ней выгружается.

use axum::extract::{Path as AxPath, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::speech::{director, project as sp};
use crate::speech::director::Speaker;
use crate::app::refuse;
use crate::speech::project::Block;
use crate::{jobs, AppState};

struct LlmPaths {
    repo_root: std::path::PathBuf,
    llama_bin: std::path::PathBuf,
    models_root: std::path::PathBuf,
    workspace: std::path::PathBuf,
}

fn llm(p: &LlmPaths, progress: &dyn Fn(Value)) -> Result<crate::app::llm::LlmProvider, String> {
    crate::app::ensure_llm_components(&p.repo_root, &p.models_root, &|ev| progress(ev))?;
    if crate::engines::models::llm_backend(&p.models_root, "llm") == crate::engines::models::LlmBackend::Local {
        crate::engines::tts_host::unload();
    }
    let (mt_model, mmproj) = crate::engines::models::resolve_mt(&p.models_root, &crate::engines::models::load_selection(&p.models_root));
    let open = crate::app::llm::LlmOpen { llama_bin: &p.llama_bin, mt_model: &mt_model, mmproj: &mmproj, models_root: &p.models_root };
    crate::app::llm::open(&open, crate::app::llm::LlmMode::Text)
}

fn paths(st: &AppState) -> LlmPaths {
    LlmPaths { repo_root: st.repo_root.clone(), llama_bin: st.llama_bin.clone(), models_root: st.models_root.clone(), workspace: st.workspace.clone() }
}

fn step(progress: &dyn Fn(Value), msg: String, pct: i64) {
    progress(json!({ "type": "progress", "stage": "director", "msg": msg, "pct": pct }));
}

#[derive(Deserialize)]
pub struct PodcastReq {
    #[serde(default)]
    name: String,
    topic: String,
    speakers: Vec<Speaker>,
    #[serde(default = "auto")]
    lang: String,
    #[serde(default = "five")]
    minutes: u32,
}

fn auto() -> String {
    "auto".into()
}

fn five() -> u32 {
    5
}

/// POST /projects/podcast {topic, speakers:[{name, manner}], lang?, minutes?, name?} — сценарий подкаста задачей; итог —
/// {project_id}.
pub async fn podcast(State(st): State<AppState>, Json(req): Json<PodcastReq>) -> Response {
    if req.topic.trim().is_empty() || req.speakers.len() < 2 || req.speakers.iter().any(|s| s.name.trim().is_empty()) {
        return refuse(StatusCode::BAD_REQUEST, "bad_request", t!("director-need-topic"));
    }
    let p = paths(&st);
    let job: jobs::JobFn = Box::new(move |progress: jobs::ProgressFn| {
        let cb = |ev: Value| progress(ev);
        step(&cb, t!("director-starting"), 2);
        let provider = llm(&p, &cb)?;
        step(&cb, t!("director-writing", model = provider.describe()), 20);
        let lines = director::write_podcast(provider.client(), req.topic.trim(), &req.speakers, &req.lang, req.minutes)?;
        let name = if req.name.trim().is_empty() { req.topic.trim().chars().take(60).collect() } else { req.name.trim().to_string() };
        let mut project = sp::SpeechProject { name, created: sp::now(), ..Default::default() };
        project.settings.lang = req.lang.clone();
        let blocks = lines.into_iter().map(|l| Block { id: sp::new_id(), speaker: l.speaker, text: l.text.trim().to_string(), direction: l.direction.trim().to_string(), ..Default::default() }).collect();
        project.chapters.push(sp::Chapter { id: sp::new_id(), title: String::new(), blocks });
        let pid = sp::new_id();
        crate::speech::save(&p.workspace, &pid, &project)?;
        step(&cb, t!("director-done"), 100);
        Ok(json!({ "project_id": pid }))
    });
    match st.jobs.enqueue(jobs::JobMeta::new(jobs::JobKind::Write, None), job).await {
        Ok(job_id) => Json(json!({ "job_id": job_id })).into_response(),
        Err(e) => crate::app::enqueue_error(e),
    }
}

#[derive(Deserialize)]
pub struct CastReq {
    chapter: String,
}

/// POST /projects/{pid}/cast {chapter} — разметить главу на рассказчика и персонажей задачей; блоки главы заменяются.
pub async fn cast(State(st): State<AppState>, AxPath(pid): AxPath<String>, Json(req): Json<CastReq>) -> Response {
    let project = match crate::speech::load(&st.workspace, &pid) {
        Ok(p) => p,
        Err(e) => return refuse(StatusCode::NOT_FOUND, "no_project", e),
    };
    if !project.chapters.iter().any(|c| c.id == req.chapter) {
        return refuse(StatusCode::NOT_FOUND, "no_chapter", req.chapter);
    }
    let p = paths(&st);
    let pid2 = pid.clone();
    let job: jobs::JobFn = Box::new(move |progress: jobs::ProgressFn| {
        let cb = |ev: Value| progress(ev);
        let project = crate::speech::load(&p.workspace, &pid2)?;
        let chapter = project.chapters.iter().find(|c| c.id == req.chapter).ok_or_else(|| req.chapter.clone())?;
        let text = chapter.blocks.iter().filter(|b| !b.hidden).map(|b| b.text.trim()).filter(|t| !t.is_empty()).collect::<Vec<_>>().join("\n\n");
        let narrator = t!("speech-narrator");
        let mut known: Vec<director::Person> = Vec::new();
        for (name, c) in &project.characters {
            director::register(&mut known, director::Person { name: name.clone(), gender: c.gender.clone(), age: c.age.clone() }, &narrator);
        }
        let lines_speaker = t!("speech-lines");
        for b in project.blocks().filter(|b| b.speaker != lines_speaker) {
            director::register(&mut known, director::Person { name: b.speaker.clone(), ..Default::default() }, &narrator);
        }
        step(&cb, t!("director-starting"), 2);
        let provider = llm(&p, &cb)?;
        let (lines, people) = director::cast_audiobook(provider.client(), &text, &narrator, &known, &project.settings.lang, &|n, of| {
            step(&cb, t!("director-casting", n = n as i64, total = of as i64), (5 + n * 90 / of.max(1)) as i64);
        })?;
        let made = lines.len();
        crate::speech::edit(&p.workspace, &pid2, |fresh| {
            if let Some(ch) = fresh.chapters.iter_mut().find(|c| c.id == req.chapter) {
                ch.blocks = lines.into_iter().map(|l| Block { id: sp::new_id(), speaker: l.speaker, text: l.text.trim().to_string(), direction: l.direction.trim().to_string(), ..Default::default() }).collect();
            }
            for p in people.into_iter().filter(|p| !p.gender.is_empty() || !p.age.is_empty()) {
                let c = fresh.characters.entry(p.name).or_default();
                if c.gender.is_empty() {
                    c.gender = p.gender;
                }
                if c.age.is_empty() {
                    c.age = p.age;
                }
            }
        })?;
        step(&cb, t!("director-done"), 100);
        Ok(json!({ "blocks": made }))
    });
    match st.jobs.enqueue(jobs::JobMeta::new(jobs::JobKind::Cast, Some(&pid)), job).await {
        Ok(job_id) => Json(json!({ "job_id": job_id })).into_response(),
        Err(e) => crate::app::enqueue_error(e),
    }
}

#[derive(Deserialize)]
pub struct EnrichReq {
    #[serde(default)]
    ids: Option<Vec<String>>,
    /// `tags` (default) — expression tags by meaning; `punctuation` — punctuation for reading aloud; `rewrite` — the
    /// lines worded anew as `instruction` says.
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    instruction: Option<String>,
}

#[derive(Deserialize)]
pub struct ExtendReq {
    #[serde(default = "five")]
    minutes: u32,
    /// Where the script goes next, when the person has a wish.
    #[serde(default)]
    direction: String,
}

/// POST /projects/{pid}/extend {minutes?, direction?} — the director writes what comes next in the project's script, for
/// the speakers it has, as a job; the project is not changed: the result is the new lines, which the window or the
/// agent adds after the last block.
pub async fn extend(State(st): State<AppState>, AxPath(pid): AxPath<String>, Json(req): Json<ExtendReq>) -> Response {
    if let Err(e) = crate::speech::load(&st.workspace, &pid) {
        return refuse(StatusCode::NOT_FOUND, "no_project", e);
    }
    let p = paths(&st);
    let pid2 = pid.clone();
    let job: jobs::JobFn = Box::new(move |progress: jobs::ProgressFn| {
        let cb = |ev: Value| progress(ev);
        let project = crate::speech::load(&p.workspace, &pid2)?;
        let written: Vec<(String, String)> = project.blocks().filter(|b| !b.hidden && !crate::speech::project::says_nothing(&b.text)).map(|b| (b.speaker.clone(), b.text.clone())).collect();
        let mut speakers: Vec<director::Speaker> = Vec::new();
        for (name, _) in &written {
            if !speakers.iter().any(|s| &s.name == name) {
                speakers.push(director::Speaker { name: name.clone(), manner: String::new() });
            }
        }
        if written.is_empty() {
            return Err(t!("director-nothing-written"));
        }
        step(&cb, t!("director-starting"), 2);
        let provider = llm(&p, &cb)?;
        step(&cb, t!("director-writing", model = provider.describe()), 20);
        let lines = director::continue_script(provider.client(), &speakers, &written, &project.settings.lang, req.minutes, &req.direction)?;
        step(&cb, t!("director-done"), 100);
        Ok(json!({ "lines": lines.into_iter().map(|l| json!({ "speaker": l.speaker, "text": l.text.trim(), "direction": l.direction.trim() })).collect::<Vec<_>>() }))
    });
    match st.jobs.enqueue(jobs::JobMeta::new(jobs::JobKind::Write, Some(&pid)), job).await {
        Ok(job_id) => Json(json!({ "job_id": job_id })).into_response(),
        Err(e) => crate::app::enqueue_error(e),
    }
}

/// POST /projects/{pid}/enrich {ids?} — режиссёр предлагает теги экспрессии по смыслу (только теги движка блока,
/// слова не меняются) задачей; проект не меняется, ответ — что было и что станет: применяет окно или агент.
pub async fn enrich(State(st): State<AppState>, AxPath(pid): AxPath<String>, Json(req): Json<EnrichReq>) -> Response {
    if let Err(e) = crate::speech::load(&st.workspace, &pid) {
        return refuse(StatusCode::NOT_FOUND, "no_project", e);
    }
    let p = paths(&st);
    let pid2 = pid.clone();
    let job: jobs::JobFn = Box::new(move |progress: jobs::ProgressFn| {
        let cb = |ev: Value| progress(ev);
        let project = crate::speech::load(&p.workspace, &pid2)?;
        let wanted = |b: &&Block| !b.hidden && !crate::speech::project::says_nothing(&b.text) && req.ids.as_ref().is_none_or(|ids| ids.contains(&b.id));
        if req.mode.as_deref() == Some("punctuation") {
            let blocks: Vec<(String, String, String)> = project.blocks().filter(wanted).map(|b| (b.id.clone(), b.speaker.clone(), b.text.clone())).collect();
            step(&cb, t!("director-starting"), 2);
            let provider = llm(&p, &cb)?;
            let (made, refused) = director::punctuate(provider.client(), &blocks, &project.settings.lang, &|n, of| {
                step(&cb, t!("director-punctuating", n = (n + 1) as i64, total = of as i64), (5 + n * 90 / of.max(1)) as i64);
            })?;
            step(&cb, t!("director-done"), 100);
            return Ok(json!({ "changes": made.into_iter().map(|c| json!({ "id": c.id, "before": c.before, "after": c.after })).collect::<Vec<_>>(), "refused": refused }));
        }
        if req.mode.as_deref() == Some("rewrite") {
            let instruction = req.instruction.as_deref().map(str::trim).filter(|i| !i.is_empty()).ok_or_else(|| t!("director-no-instruction"))?;
            let blocks: Vec<(String, String, String)> = project.blocks().filter(wanted).map(|b| (b.id.clone(), b.speaker.clone(), b.text.clone())).collect();
            step(&cb, t!("director-starting"), 2);
            let provider = llm(&p, &cb)?;
            let (made, refused) = director::rewrite_as_asked(provider.client(), &blocks, &project.settings.lang, instruction, &|n, of| {
                step(&cb, t!("director-rewriting", n = (n + 1) as i64, total = of as i64), (5 + n * 90 / of.max(1)) as i64);
            })?;
            step(&cb, t!("director-done"), 100);
            return Ok(json!({ "changes": made.into_iter().map(|c| json!({ "id": c.id, "before": c.before, "after": c.after })).collect::<Vec<_>>(), "refused": refused }));
        }
        if let Some(m) = req.mode.as_deref().filter(|m| *m != "tags") {
            return Err(format!("mode {m} is not tags, punctuation or rewrite"));
        }
        let mut groups: Vec<(crate::engines::Engine, Vec<(String, String, String)>)> = Vec::new();
        for b in project.blocks().filter(wanted) {
            let Some(engine) = crate::engines::get(project.engine_of(b)).filter(|e| !e.tags.is_empty()) else { continue };
            match groups.iter_mut().find(|(e, _)| e.id == engine.id) {
                Some((_, list)) => list.push((b.id.clone(), b.speaker.clone(), b.text.clone())),
                None => groups.push((engine, vec![(b.id.clone(), b.speaker.clone(), b.text.clone())])),
            }
        }
        if groups.is_empty() {
            let name = crate::engines::get(&project.settings.engine).map_or(project.settings.engine.clone(), |e| e.name.to_string());
            return Err(t!("director-no-tags", engine = name));
        }
        step(&cb, t!("director-starting"), 2);
        let provider = llm(&p, &cb)?;
        let (mut changes, mut refused) = (Vec::new(), 0);
        for (engine, blocks) in &groups {
            let (made, no) = director::enrich(provider.client(), engine, blocks, &project.settings.lang, &|n, of| {
                step(&cb, t!("director-enriching", n = (n + 1) as i64, total = of as i64), (5 + n * 90 / of.max(1)) as i64);
            })?;
            changes.extend(made.into_iter().map(|c| json!({ "id": c.id, "before": c.before, "after": c.after })));
            refused += no;
        }
        step(&cb, t!("director-done"), 100);
        Ok(json!({ "changes": changes, "refused": refused }))
    });
    match st.jobs.enqueue(jobs::JobMeta::new(jobs::JobKind::Enrich, Some(&pid)), job).await {
        Ok(job_id) => Json(json!({ "job_id": job_id })).into_response(),
        Err(e) => crate::app::enqueue_error(e),
    }
}
