# Speech Studio 0.5.0 and Dub Studio 4.6.0 on Windows: ONNX Runtime from a mirror while
# github.com/microsoft/onnxruntime is unavailable. Every archive is checked against the SHA-256 the program pins.
# Run from the program's folder (where Speech-Studio.exe or Dub-Studio.exe is), or pass it: -Studio "D:\Dub Studio"
param([string]$Studio = (Get-Location).Path)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

if (Test-Path (Join-Path $Studio 'Speech-Studio.exe')) {
  $version = '1.28.2'
  $parts = @(
    @{ record = '_ort.zip.json'; name = "onnxruntime-win-x64-$version"; sha = 'c4eedd29489d5feca21866d054638416f3655bf6b18851b3b6b85c8313e95c35';
       mirrors = @("https://registry.npmmirror.com/-/binary/onnxruntime/v$version/onnxruntime-win-x64-$version.zip") }
  )
  $gpuNote = "In setup untick 'ONNX Runtime $version GPU (CUDA)': no mirror has it yet, recognition runs on the processor meanwhile."
} elseif (Test-Path (Join-Path $Studio 'Dub-Studio.exe')) {
  $version = '1.31.1'
  $parts = @(
    @{ record = '_ort.zip.json'; name = "onnxruntime-win-x64-$version"; sha = '5869e26bb25314375f2d5ef1877e3ea06b1543d6c710e6e4a1ec2f7177ab4b2c';
       mirrors = @("https://registry.npmmirror.com/-/binary/onnxruntime/v$version/onnxruntime-win-x64-$version.zip",
                   "https://dl.photoprism.app/onnx/runtime/v$version/onnxruntime-win-x64-$version.zip") },
    @{ record = '_ort_gpu.zip.json'; name = "onnxruntime-win-x64-gpu_cuda13-$version"; sha = '6c64508de9c04f9d938297287e9d81fa47a2fbbb30cc04e6fac0db88f6cb68c1';
       mirrors = @("https://dl.photoprism.app/onnx/runtime/v$version/onnxruntime-win-x64-gpu_cuda13-$version.zip") }
  )
  $gpuNote = ''
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
  Write-Host "[OK] $($p.name): SHA-256 matches the file the program expects"
  $target = Join-Path $runtime $p.name
  if (Test-Path $target) { Remove-Item $target -Recurse -Force }
  Expand-Archive -Path $zip -DestinationPath $runtime -Force
  Remove-Item $zip
  $files = Get-ChildItem $target -Recurse -File | Sort-Object FullName | ForEach-Object {
    [ordered]@{ path = $_.FullName.Substring($root.Length + 1).Replace('\', '/'); size = $_.Length }
  }
  $original = "https://github.com/microsoft/onnxruntime/releases/download/v$version/$($p.name).zip"
  $json = [ordered]@{ sha256 = $p.sha; url = $original; files = @($files) } | ConvertTo-Json -Depth 4
  [System.IO.File]::WriteAllText((Join-Path $runtime $p.record), $json, (New-Object System.Text.UTF8Encoding($false)))
  Write-Host "[OK] $($p.name) installed ($($files.Count) files)"
}
Write-Host "[OK] Done. Restart the program and continue setup. $gpuNote"
