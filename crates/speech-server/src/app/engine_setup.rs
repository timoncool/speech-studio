//! Component preparation, readiness policy and activation before engine execution.

use std::path::Path;

use serde::Serialize;
use serde_json::{json, Value};

use crate::setup::{
    component_status, free_bytes, manifest, Component, ComponentStatus, Delivery, DlError, ProgressCb, Requirement, GH_LLAMA_BUILD,
};
use crate::{jobs, setup};
use crate::engines::models;

// ── Полный статус (для GET /setup/status) ────────────────────────────────────

/// A component this release updates, with why.
#[derive(Clone, Debug, Serialize)]
pub struct ComponentUpdate {
    pub id: String,
    pub name: String,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatus {
    pub components: Vec<ComponentStatus>,
    /// The required components this release replaces on a computer that has their earlier version, each with the
    /// reason: with them and nothing else missing, the window shows an update, not a first start.
    pub updates: Vec<ComponentUpdate>,
    /// Всё ли обязательное на месте (тогда фронт показывает обычный hero, а не «первый запуск»).
    pub ready: bool,
    /// Совокупный размер того, что ещё надо скачать (обязательное+рекомендованное, что missing и Download).
    pub download_pending: u64,
    /// Драйвер NVIDIA найден (nvcuda.dll грузится); годится ли он под CUDA 13 — в gpu.
    pub driver_ok: bool,
    /// build-строки для диагностики (llama-билд и т.п.).
    pub llama_build: String,
    /// Why the assistant runs another build than the card's (a CUDA build the system cannot start); None when it does not.
    pub llama_note: Option<String>,
    /// Папка моделей (куда кладёт «Первый запуск»).
    pub models_dir: String,
    /// Свободно на томе моделей, байт; None — ОС не ответила, место не проверяется.
    pub free_bytes: Option<u64>,
    /// Видеокарта и драйвер против требований CUDA 13.
    pub gpu: crate::engines::hw::GpuReport,
    /// Фоновая закачка (идёт, на паузе, прервана перезапуском, упала или завершилась). Заполняет обработчик.
    pub active: Option<super::downloads::DownloadJob>,
}

/// Запас видеопамяти под рабочий стол и окна поверх оценки модели.
const VRAM_HEADROOM: u64 = 512 * 1024 * 1024;

/// Модели по видеопамяти карты (`total`, байт; 0 — неизвестна): каждой отмечено, влезет ли она, а в группе
/// квантов одной модели (`higgs`, `higgs-q6_k`, ...) обязательным становится самый большой влезающий вариант.
/// Группа, где не влезает ничего, готовность не держит: эта стадия идёт на сервере или в облаке. Группу, где
/// вариант уже скачан, не трогает: выбор сделан.
fn fit_to_vram(comps: &mut [ComponentStatus], total: u64) {
    if total == 0 {
        return;
    }
    let group = |id: &str| id.split('-').next().unwrap_or(id).to_string();
    for c in comps.iter_mut().filter(|c| c.vram > 0) {
        c.fits_vram = Some(c.vram + VRAM_HEADROOM <= total);
    }
    let groups: std::collections::BTreeSet<String> = comps
        .iter()
        .filter(|c| c.vram > 0 && c.requirement == Requirement::Required && c.fits_vram == Some(false))
        .map(|c| group(&c.id))
        .filter(|g| !comps.iter().any(|c| c.vram > 0 && c.installed && group(&c.id) == *g))
        .collect();
    for g in groups {
        let best = comps
            .iter()
            .filter(|c| c.vram > 0 && group(&c.id) == g && c.fits_vram == Some(true))
            .max_by_key(|c| c.vram)
            .map(|c| c.id.clone());
        for c in comps.iter_mut().filter(|c| c.vram > 0 && group(&c.id) == g) {
            if Some(&c.id) == best.as_ref() {
                c.requirement = Requirement::Required;
            } else if c.requirement == Requirement::Required {
                c.requirement = Requirement::Optional;
            }
        }
    }
}

/// Варианты одной модели (`parakeet`, `parakeet-ultra`, ...) взаимозаменяемы: скачанный вариант закрывает
/// группу, и обязательный по умолчанию больше не держит готовность — иначе выбор юзера на первом запуске
/// упирается в докачку варианта, который он не выбирал.
fn installed_variant_covers_group(comps: &mut [ComponentStatus]) {
    let group = |id: &str| id.split('-').next().unwrap_or(id).to_string();
    let covered: std::collections::BTreeSet<String> = comps.iter().filter(|c| c.vram > 0 && c.installed).map(|c| group(&c.id)).collect();
    for c in comps.iter_mut() {
        if c.vram > 0 && !c.installed && c.requirement == Requirement::Required && covered.contains(&group(&c.id)) {
            c.requirement = Requirement::Optional;
        }
    }
}

pub fn setup_status(repo_root: &Path) -> SetupStatus {
    let mut comps: Vec<ComponentStatus> = manifest().iter().map(|c| component_status(repo_root, c)).collect();
    // Облачный пресет (OpenRouter) снимает ОБЯЗАТЕЛЬНОСТЬ тяжёлых локальных движков: если стадия перевода/
    // TTS идёт через облако (флаг + ключ), её локальную модель качать НЕ обязательно — не гейтит ready и не
    // преселектится на первом запуске (юзер выбрал облако -> не тянет ненужные гигабайты Gemma/Higgs).
    let mroot = repo_root.join("models");
    let cloud_llm = !crate::engines::models::local_gemma_needed(&mroot);
    let cloud_tts = crate::engines::models::cloud_tts_on(&mroot);
    let cloud_asr = crate::engines::models::openrouter_asr_on(&mroot);
    for c in comps.iter_mut() {
        if c.requirement != Requirement::Required {
            continue;
        }
        let is_gemma = c.id.starts_with("gemma") || c.id == "llama";
        let is_higgs = c.id.starts_with("higgs");
        let is_asr =
            c.id.starts_with("parakeet") || c.id.starts_with("crisperwhisper") || c.id.starts_with("transcribe-engine") || c.id == "langid";
        if (cloud_llm && is_gemma) || (cloud_tts && is_higgs) || (cloud_asr && is_asr) {
            c.requirement = Requirement::Optional;
        }
    }
    // Backend локальных стадий: в CPU-режиме (без NVIDIA) CUDA-компоненты не нужны — сепарация идёт
    // CPU-сборкой, тяжёлое в облаке. CPU-движок сепарации становится рекомендованным (преселект на
    // первом запуске), а CUDA-движок + CUDA-рантайм — необязательными (не тянем лишние гигабайты).
    // В GPU-режиме наоборот: CPU-движок не нужен.
    let backend = crate::engines::models::local_backend(&mroot);
    let asr_build = crate::engines::models::tcpp_build(&mroot);
    for c in comps.iter_mut() {
        if !cloud_asr && dub_asr::tcpp::Build::ALL.iter().any(|b| b.dir() == c.id) {
            c.requirement = if c.id == asr_build.dir() { Requirement::Required } else { Requirement::Optional };
        }
        if backend == "cpu" {
            if c.id == "bsroformer-engine-cpu" {
                c.requirement = Requirement::Recommended;
            } else if c.id == "bsroformer-engine" || c.id == "cuda-runtime" || c.id == "onnxruntime-gpu" || c.id == "cudnn" {
                c.requirement = Requirement::Optional;
            }
        } else if c.id == "bsroformer-engine-cpu" {
            c.requirement = Requirement::Optional;
        } else if c.id == "onnxruntime-gpu" || c.id == "cudnn" {
            // the process keeps the first ONNX Runtime it loads: on a GPU the CUDA build must be here before any session
            c.requirement = Requirement::Required;
        }
    }
    if backend == "gpu" {
        fit_to_vram(&mut comps, crate::engines::hw::total_vram());
    }
    installed_variant_covers_group(&mut comps);
    // ready = всё СКАЧИВАЕМОЕ/бандл-обязательное на месте. External (драйвер NVIDIA) НЕ гейтит: без NVIDIA
    // приложение работает на CPU/в облаке, а старый драйвер — предупреждение на экране, а не запертый вход.
    let ready = comps
        .iter()
        .filter(|c| c.requirement == Requirement::Required && c.delivery != Delivery::External)
        .all(|c| c.installed);
    let download_pending = comps
        .iter()
        .filter(|c| c.delivery == Delivery::Download && !c.installed)
        .map(|c| c.size.saturating_sub(c.bytes_on_disk))
        .sum();
    let gpu = crate::engines::hw::gpu_report();
    let updates = comps
        .iter()
        .filter(|c| c.requirement == Requirement::Required && c.delivery == Delivery::Download && !c.installed)
        .filter(|c| manifest().iter().find(|m| m.id == c.id).is_some_and(|m| setup::has_earlier_version(repo_root, m)))
        .map(|c| ComponentUpdate { id: c.id.clone(), name: c.name.clone(), reason: setup::update_reason(&c.id) })
        .collect();
    SetupStatus {
        updates,
        components: comps,
        ready,
        download_pending,
        driver_ok: gpu.nvidia,
        llama_build: GH_LLAMA_BUILD.to_string(),
        llama_note: crate::setup::llama_note(),
        models_dir: mroot.to_string_lossy().to_string(),
        free_bytes: free_bytes(&mroot),
        gpu,
        active: None,
    }
}

pub(crate) fn remove_components(repo_root: &Path, ids: &[String]) -> Result<setup::RemovalReport, DlError> {
    let mroot = crate::app::models_root(repo_root);
    setup::remove_files(repo_root, ids, |id| {
        let mut errors = Vec::new();
        for (engine, variant) in setup::component_selection(id) {
            if engine == "asr_engine" {
                continue;
            }
            if let Err(e) = models::clear_selection_if(&mroot, engine, &variant) {
                errors.push(format!("active.json ({engine}): {e}"));
            }
        }
        errors
    })
}

/// Итог закачки: статус каждого компонента; скачанный вариант модели становится активным (models/active.json),
/// иначе резолв взял бы дефолт и альт-квант бы не применился.
fn finish(repo_root: &Path, selected: &[&Component]) -> Value {
    let mroot = crate::app::models_root(repo_root);
    let mut results = Vec::new();
    for c in selected {
        let st = component_status(repo_root, c);
        if st.installed {
            for (engine, variant) in setup::component_selection(c.id) {
                if let Err(e) = crate::engines::models::set_selection(&mroot, engine, &variant) {
                    tracing::warn!("active.json: the choice {engine}={variant} was not written: {e}");
                }
            }
        }
        results.push(json!({ "id": c.id, "installed": st.installed, "missing": st.missing }));
    }
    let overall = setup_status(repo_root);
    json!({ "components": results, "ready": overall.ready })
}

/// Скачать набор компонентов по id (идемпотентно: целые файлы закреплённой версии пропускаются, уже лежащий
/// файл перед пропуском сверяется по SHA-256). Тело синхронное: зовут фоновый поток менеджера или джоба.
/// cancel — пауза: скачанное остаётся в .part с манифестом чанков и докачивается следующим запуском.
pub fn download_components(repo_root: &Path, ids: &[String], cancel: &dyn Fn() -> bool, progress: &ProgressCb) -> Result<Value, DlError> {
    let done = setup::fetch_components(repo_root, ids, cancel, progress, |selected| finish(repo_root, selected))?;
    if ids.iter().any(|id| id.starts_with("onnxruntime")) {
        crate::app::point_ort(repo_root);
    }
    Ok(done)
}

/// Перед задачей догружаются компоненты, нужные именно ей, если их нет на диске (прогресс — в тот же SSE):
/// проверка похожести голоса — WeSpeaker, GPU-стадия без CUDA-стека — cuFFT, onnxruntime-GPU, cuDNN.
/// Докачать в задаче недостающие компоненты из списка (отмена задачи прерывает закачку).
pub(crate) fn ensure_components(repo_root: &Path, ids: &[&str], progress: &setup::ProgressCb) -> Result<(), String> {
    ensure_components_with(repo_root, ids, progress, &download_components)
}

type Download<'a> = dyn Fn(&Path, &[String], &dyn Fn() -> bool, &ProgressCb) -> Result<Value, DlError> + 'a;

fn ensure_components_with(repo_root: &Path, ids: &[&str], progress: &ProgressCb, download: &Download<'_>) -> Result<(), String> {
    let manifest = setup::manifest();
    let need: Vec<String> = ids
        .iter()
        .filter(|id| {
            manifest
                .iter()
                .find(|c| c.id == **id)
                .is_some_and(|c| !setup::component_status(repo_root, c).installed)
        })
        .map(|id| id.to_string())
        .collect();
    if need.is_empty() {
        return Ok(());
    }
    progress(json!({ "stage": "download", "msg": t!("setup-fetching-missing") }));
    let ctl = jobs::current();
    let cancel = move || ctl.as_ref().is_some_and(|c| c.is_cancelled());
    download(repo_root, &need, &cancel, progress).map(|_| ()).map_err(|e| e.detail)
}

/// Для своей Gemma: llama-server и модель «Первого запуска», если их нет; сервер пользователя и OpenRouter ничего
/// не скачивают.
pub(crate) fn ensure_llm_components(repo_root: &Path, models_root: &Path, progress: &setup::ProgressCb) -> Result<(), String> {
    ensure_llm_components_with(repo_root, models_root, progress, &download_components)
}

fn ensure_llm_components_with(repo_root: &Path, models_root: &Path, progress: &ProgressCb, download: &Download<'_>) -> Result<(), String> {
    if models::llm_backend(models_root, "llm") != models::LlmBackend::Local {
        return Ok(());
    }
    let manifest = setup::manifest();
    let missing = |id: &str| {
        manifest
            .iter()
            .find(|c| c.id == id)
            .is_some_and(|c| !setup::component_status(repo_root, c).installed)
    };
    let (model, _) = models::resolve_mt(models_root, &models::load_selection(models_root));
    let mut need: Vec<String> = Vec::new();
    if missing("llama") {
        need.push("llama".to_string());
    }
    if !model.is_file() && missing("gemma") {
        need.push("gemma".to_string());
    }
    if need.is_empty() {
        return Ok(());
    }
    progress(json!({ "stage": "download", "msg": t!("setup-fetching-missing") }));
    let ctl = jobs::current();
    let cancel = move || ctl.as_ref().is_some_and(|c| c.is_cancelled());
    download(repo_root, &need, &cancel, progress).map_err(|e| e.detail)?;
    crate::app::augment_path_for_tools(repo_root);
    Ok(())
}

pub(crate) fn ensure_job_components(
    repo_root: &Path,
    models_root: &Path,
    similarity: bool,
    diarization: bool,
    progress: &setup::ProgressCb,
) -> Result<(), String> {
    ensure_job_components_with(repo_root, models_root, similarity, diarization, progress, &download_components)
}

fn ensure_job_components_with(
    repo_root: &Path,
    models_root: &Path,
    similarity: bool,
    diarization: bool,
    progress: &ProgressCb,
    download: &Download<'_>,
) -> Result<(), String> {
    let manifest = setup::manifest();
    let missing = |id: &str| -> bool {
        manifest
            .iter()
            .find(|c| c.id == id)
            .map(|c| !setup::component_status(repo_root, c).installed)
            .unwrap_or(false)
    };
    let mut need: Vec<String> = Vec::new();
    if similarity && missing("wespeaker") {
        need.push("wespeaker".to_string());
    }
    // Любая локальная стадия на GPU → полный CUDA-стек: ASR грузит onnxruntime-GPU + cuDNN,
    // ggml-движки (сепарация/Higgs) — cuda-runtime (cudart/cublas/cuFFT). Без cuFFT CUDA-EP не поднимется.
    let gpu = ["sep_backend", "asr_backend"]
        .iter()
        .any(|k| models::stage_backend(models_root, k) == "gpu");
    if gpu {
        for id in ["cuda-runtime", "onnxruntime-gpu", "cudnn"] {
            if missing(id) {
                need.push(id.to_string());
            }
        }
    }
    // Диаризация идёт на сервере audio.cpp: сборка устройства диаризации и её CUDA-рантайм.
    if diarization {
        let build = crate::engines::acpp::build_for(models::server_choice(models_root, "diar_backend"))?;
        let parts = std::iter::once(crate::engines::acpp::component(build)).chain(crate::engines::acpp::runtime_component(build));
        for id in parts {
            if missing(id) && !need.iter().any(|n| n == id) {
                need.push(id.to_string());
            }
        }
    }
    // Диаризация (дубляж/транскрипт) → модель Nemotron 3 Diarization. Её сбой закачки не валит джобу:
    // анализ штатно деградирует в single-speaker и пишет об этом в журнал (см. analyze::run).
    let need_diar = diarization && missing("sortformer");
    if need.is_empty() && !need_diar {
        return Ok(());
    }
    progress(json!({ "stage": "download", "msg": t!("setup-fetching-missing") }));
    let ctl = jobs::current();
    let cancel = move || ctl.as_ref().is_some_and(|c| c.is_cancelled());
    if !need.is_empty() {
        download(repo_root, &need, &cancel, progress).map_err(|e| e.detail)?;
    }
    if need_diar {
        if let Err(e) = download(repo_root, &["sortformer".to_string()], &cancel, progress) {
            progress(json!({
                "stage": "download",
                "msg": t!("setup-diarization-missing", error = e.detail.clone()),
            }));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
