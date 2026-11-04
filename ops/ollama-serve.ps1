# Starts Ollama only once the Tailscale IP it binds to (OLLAMA_HOST) exists.
# Fixes the boot race: at logon+2min Tailscale may not be up, bind fails, ollama exits.
$ip = '100.106.147.75'
$exe = "$env:LOCALAPPDATA\Programs\Ollama\ollama.exe"
for ($i = 0; $i -lt 120; $i++) {
  if (Get-NetIPAddress -IPAddress $ip -ErrorAction SilentlyContinue) { break }
  Start-Sleep -Seconds 5
}
while ($true) {
  & $exe serve
  Start-Sleep -Seconds 10   # restart if it ever exits
}
