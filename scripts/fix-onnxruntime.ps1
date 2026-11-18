# Speech Studio 0.5.0 and Dub Studio 4.6.0 on Windows: ONNX Runtime (CPU and CUDA 13) from our mirror while
# github.com/microsoft/onnxruntime is unavailable. Every archive is checked against its SHA-256 before it is unpacked.
# Run from the program's folder (where Speech-Studio.exe or Dub-Studio.exe is), or pass it: -Studio "D:\Dub Studio"
param([string]$Studio = (Get-Location).Path)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$ours = 'https://github.com/timoncool/speech-studio/releases/download/onnxruntime-mirror'

# record: the install record the program reads; pinned: the SHA-256 the program pins for the original archive;
# sha: the SHA-256 of the archive downloaded here (the same as pinned, except the CUDA 1.28.2 build, repacked from
# the files of a verified install because no mirror has the original archive).
if (Test-Path (Join-Path $Studio 'Speech-Studio.exe')) {
  $version = '1.28.2'
  $parts = @(
    @{ record = '_ort.zip.json'; name = "onnxruntime-win-x64-$version";
       pinned = 'c4eedd29489d5feca21866d054638416f3655bf6b18851b3b6b85c8313e95c35'; sha = 'c4eedd29489d5feca21866d054638416f3655bf6b18851b3b6b85c8313e95c35';
       mirrors = @("$ours/onnxruntime-win-x64-$version.zip", "https://registry.npmmirror.com/-/binary/onnxruntime/v$version/onnxruntime-win-x64-$version.zip") },
    @{ record = '_ort_gpu.zip.json'; name = "onnxruntime-win-x64-gpu_cuda13-$version";
       pinned = '4b7a2d01a3cc96b12d06c8266af2c8f42c96365c4a0100d45fd874c71b4a2e19'; sha = '4c033a5088254f167e49c4f2eb90e2d0d77667b2b9400e3607e8e15a5c9fa970';
       mirrors = @("$ours/onnxruntime-win-x64-gpu_cuda13-$version.zip") }
  )
} elseif (Test-Path (Join-Path $Studio 'Dub-Studio.exe')) {
  $version = '1.31.1'
  $parts = @(
    @{ record = '_ort.zip.json'; name = "onnxruntime-win-x64-$version";
       pinned = '5869e26bb25314375f2d5ef1877e3ea06b1543d6c710e6e4a1ec2f7177ab4b2c'; sha = '5869e26bb25314375f2d5ef1877e3ea06b1543d6c710e6e4a1ec2f7177ab4b2c';
       mirrors = @("$ours/onnxruntime-win-x64-$version.zip", "https://registry.npmmirror.com/-/binary/onnxruntime/v$version/onnxruntime-win-x64-$version.zip",
                   "https://dl.photoprism.app/onnx/runtime/v$version/onnxruntime-win-x64-$version.zip") },
    @{ record = '_ort_gpu.zip.json'; name = "onnxruntime-win-x64-gpu_cuda13-$version";
       pinned = '6c64508de9c04f9d938297287e9d81fa47a2fbbb30cc04e6fac0db88f6cb68c1'; sha = '6c64508de9c04f9d938297287e9d81fa47a2fbbb30cc04e6fac0db88f6cb68c1';
       mirrors = @("$ours/onnxruntime-win-x64-gpu_cuda13-$version.zip", "https://dl.photoprism.app/onnx/runtime/v$version/onnxruntime-win-x64-gpu_cuda13-$version.zip") }
  )
} else {
  throw "Neither Speech-Studio.exe nor Dub-Studio.exe is in $Studio"
}

$runtime = Join-Path $Studio 'models\runtime'
New-Item -ItemType Directory -Force $runtime | Out-Null
$root = (Resolve-Path $Studio).Path.TrimEnd('\')
foreach ($p in $parts) {
  $zip = Join-Path $runtime "$($p.name).download.zip"
  $ok = $false
  foreach ($url in $p.mirrors) {
    try {
      Write-Host "[..] $($p.name): $url"
      Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
      $got = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
      if ($got -eq $p.sha) { $ok = $true; break }
      Write-Host "[..] SHA-256 differs ($got), trying the next mirror"
    } catch { Write-Host "[..] $($_.Exception.Message)" }
  }
  if (-not $ok) { if (Test-Path $zip) { Remove-Item $zip }; throw "No mirror gave $($p.name) with SHA-256 $($p.sha)" }
  Write-Host "[OK] $($p.name): SHA-256 matches"
  $target = Join-Path $runtime $p.name
  if (Test-Path $target) { Remove-Item $target -Recurse -Force }
  Expand-Archive -Path $zip -DestinationPath $runtime -Force
  Remove-Item $zip
  $files = Get-ChildItem $target -Recurse -File | Sort-Object FullName | ForEach-Object {
    [ordered]@{ path = $_.FullName.Substring($root.Length + 1).Replace('\', '/'); size = $_.Length }
  }
  $original = "https://github.com/microsoft/onnxruntime/releases/download/v$version/$($p.name).zip"
  $json = [ordered]@{ sha256 = $p.pinned; url = $original; files = @($files) } | ConvertTo-Json -Depth 4
  [System.IO.File]::WriteAllText((Join-Path $runtime $p.record), $json, (New-Object System.Text.UTF8Encoding($false)))
  Write-Host "[OK] $($p.name) installed ($($files.Count) files)"
}
Write-Host "[OK] Done. Restart the program and continue setup."
