# Сборка релиза и staging бандла

Релиз собирает один скрипт: `scripts/build-release.ps1` (PowerShell 5.1, файл в UTF-8 с BOM). Версия берётся из
`desktop/src-tauri/tauri.conf.json` и обязана совпадать с `version` в `desktop/src-tauri/Cargo.toml`.

`tauri.conf.json` держит `bundle.active` = false (обычный `tauri build` даёт только exe), а его
`beforeBundleCommand` падает с подсказкой для `tauri build --bundles …` и `tauri bundle`: установщик без
ресурсов staging не собрать. `tauri.bundle.conf.json` включает бандл и снимает эту заглушку.

```powershell
# проверка входов без сборки
scripts\build-release.ps1 -DryRun -ModelsSource "F:\AI\Speech Studio"
# тесты + фронт + staging, без tauri build
scripts\build-release.ps1 -StopAfterStaging -ModelsSource "F:\AI\Speech Studio"
# полный релиз (ключ и пароль — в окружении или в %USERPROFILE%\.tauri)
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "..."
scripts\build-release.ps1 -ReleaseNotes "..." -ModelsSource "F:\AI\Speech Studio"
```

Что делает полный прогон:

1. Гейт: фронт (`npm ci` при отсутствии `node_modules`, `npm run test` если такой скрипт есть, `npm run build`),
   затем `cargo test --workspace` и `cargo test --manifest-path desktop/src-tauri/Cargo.toml`. Фронт идёт первым:
   оболочка вшивает `frontend/dist` при компиляции, а `dist` в git не лежит.
2. Staging в `desktop/src-tauri/staging/` (каталог генерируется, в git не попадает): `frontend/dist` и
   `models/higgs-engine` (только VC++-рантайм). Сервер встроен в
   exe оболочки, отдельный `speech-server.exe` в бандл не кладётся.
3. `tauri build --config tauri.bundle.conf.json` (он добавляет `bundle.resources` на staging, то же: `npm run bundle`
   в `desktop/`): NSIS и MSI с подписью
   обновлений из `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Обычная сборка и `tauri dev`
   идут по `tauri.conf.json` и staging не требуют.
4. `release/<версия>/`: `Speech.Studio_<версия>_x64-setup.exe` и `.sig`, MSI и `.sig`, портативный zip и
   `latest.json` (UTF-8 без BOM, `url` с точками вместо пробелов): платформы `windows-x86_64` (NSIS, запасная),
   `windows-x86_64-nsis` и `windows-x86_64-msi` с подписью своего установщика.

`models/higgs-engine` в git не лежит (gitignore): источник задаётся `-ModelsSource` (по умолчанию
корень репозитория). Модели, движки, CUDA и ffmpeg в бандл не входят, их качает «Первый запуск»
(`crates/speech-server/src/setup.rs`, `delivery: Download`).

## Раскладка и данные

Установщик и портативный zip кладут ресурсы рядом с `Speech-Studio.exe`. Оболочка
(`desktop/src-tauri/src/layout.rs`) выбирает каталог данных (`models`, `workspace`, `voices`,
`tools`, профиль WebView2, `temp`):

- рядом с exe, если туда можно писать (проверяется реальной записью);
- иначе `%LOCALAPPDATA%\Speech Studio`; поставляемые ресурсы копируются туда при старте;
- портативная копия (файл `portable.flag` рядом с exe) запасного пути не имеет: недоступная для записи папка — ошибка.

`TEMP`/`TMP` процесса переводятся в `<каталог данных>\temp`, `WEBVIEW2_USER_DATA_FOLDER` — в
`<каталог данных>\webview-data` (если не задан). Автообновление ставит на лету только копию из NSIS-установщика и в ту же
папку (`/D=`); портатив и копия из MSI (msiexec не принимает `/D=`) только открывают страницу релиза.

## NSIS

`installer-english.nsh` и `installer-russian.nsh` — языковые файлы установщика (английский и русский) с понятной
ошибкой WebView2; `installer-hooks.nsi` — пустые хуки установки и удаления.

## Проверка комплекта бандла

Все пять DLL VC++ runtime (`MSVCP140.dll`, `MSVCP140_1.dll`, `VCOMP140.DLL`, `VCRUNTIME140.dll`,
`VCRUNTIME140_1.dll`) — из одного VC++ Redistributable (например, `VC/Redist/MSVC/<версия>/x64/Microsoft.VC145.CRT`
и `.OpenMP` из Build Tools); `MSVCP140_1.dll` нужен onnxruntime 1.31. Без него релиз запирает на «Первом запуске»
всех, у кого VC++ Redistributable нет в системе: докачать Bundled-компонент нельзя. Комплект staging (и
распакованного портатива) проверяется по манифесту:

```bash
SPEECH_RELEASE_STAGING="$PWD/desktop/src-tauri/staging" cargo test -p speech-server --lib setup::tests::the_release_staging_carries_every_bundled_file -- --ignored
```
