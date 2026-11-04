# ATRIUM ops agent: brain chain setup ("feather": LM Studio library + Ollama, Nov 3 2026).
#   B) loads an LM Studio model as "atrium-ops" (localhost only, OpenAI-compatible API, 32k context)
#   A) imports the same GGUF into Ollama as "atrium-ops", ONLY if Ollama's model store is on D:
# then writes D:\ATRIUM\ops\agent\brains.txt, which ops.exe reads. Changes nothing in any repo.
# Default model: google/gemma-4-12b-qat (non-reasoning instruct, ~7 GB, leaves VRAM for 32k context).
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File D:\ATRIUM\ops\agent\brains-setup.ps1 [-Key qwen/qwen3-8b]
param([string]$Key = 'google/gemma-4-12b-qat')
# 'Continue': lms writes progress to stderr, which Windows PowerShell 5 would turn into a terminating error.
# Failures are caught by exit codes below instead.
$ErrorActionPreference = 'Continue'
$out = 'D:\ATRIUM\ops\agent\brains.txt'
$ollamaUrl = 'http://100.106.147.75:11434'

$lms = (Get-Command lms -ErrorAction SilentlyContinue).Source
if (-not $lms) { $lms = Join-Path $env:USERPROFILE '.lmstudio\bin\lms.exe' }
if (-not (Test-Path $lms)) { Write-Output "FAIL: lms CLI not found at $lms"; exit 1 }

# B: LM Studio, localhost only, 32k context, stable id "atrium-ops".
& $lms server start *> $null
& $lms unload atrium-ops *> $null
$load = & $lms load $Key --identifier atrium-ops --context-length 32768 -y 2>&1 | Out-String
if ($LASTEXITCODE -ne 0) { Write-Output "FAIL: lms load $Key`n$load"; exit 1 }
Write-Output "Loaded $Key as atrium-ops (32k context)"
$body = '{"model":"atrium-ops","messages":[{"role":"user","content":"Reply with exactly {\"say\":\"ok\"}"}],"max_tokens":512}'
try {
  $r = Invoke-RestMethod -Uri http://127.0.0.1:1234/v1/chat/completions -Method Post -Body $body -ContentType 'application/json' -TimeoutSec 120
  Write-Output "LM Studio smoke: $($r.choices[0].message.content)"
} catch { Write-Output "FAIL: LM Studio smoke: $_"; exit 1 }

$lines = @('# ATRIUM ops brain chain, tried top to bottom (written by brains-setup.ps1)')

# A: Ollama copy, only when its model store is on D: (an import copies the GGUF).
$store = $env:OLLAMA_MODELS
if (-not $store) { $store = [Environment]::GetEnvironmentVariable('OLLAMA_MODELS', 'User') }
if ($store -and $store.StartsWith('D:', 'OrdinalIgnoreCase')) {
  $dl = (Get-Content "$env:USERPROFILE\.lmstudio\settings.json" -Raw | ConvertFrom-Json).downloadsFolder
  $name = ($Key -split '/')[-1]
  $g = Get-ChildItem $dl -Recurse -Filter *.gguf | Where-Object { $_.FullName -match [regex]::Escape($name) -and $_.Name -notmatch 'mmproj' } | Sort-Object Length -Descending | Select-Object -First 1
  if ($g) {
    Set-Content D:\ATRIUM\ops\agent\Modelfile "FROM $($g.FullName)`nPARAMETER num_ctx 32768`nPARAMETER temperature 0.2"
    & "$env:LOCALAPPDATA\Programs\Ollama\ollama.exe" create atrium-ops -f D:\ATRIUM\ops\agent\Modelfile *> $null
    if ($LASTEXITCODE -eq 0) { $lines += "ollama|$ollamaUrl|atrium-ops"; Write-Output "Ollama copy: atrium-ops in $store" }
    else { Write-Output "Ollama copy failed; chain continues without it" }
  } else { Write-Output "Ollama copy skipped: GGUF for $Key not found under $dl" }
} else {
  Write-Output "Ollama copy skipped: OLLAMA_MODELS is not on D: (import would write to C:)."
}

$lines += "lmstudio|http://127.0.0.1:1234|atrium-ops|$Key"
$lines += "ollama|$ollamaUrl|gpt-oss:20b"
Set-Content $out $lines
Write-Output "Wrote $out :"; Get-Content $out
