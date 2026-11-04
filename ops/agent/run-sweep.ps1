# ATRIUM ops agent: hourly charter goal sweep. Skips if a sweep is still running.
$m = New-Object System.Threading.Mutex($false, 'Global\atrium-ops-sweep')
if (-not $m.WaitOne(0)) { exit 0 }
try {
  $task = 'Hourly goal sweep per charter: run fleet_alarms, fleet_status, cargo_test_watch, cargo_test_agent, cargo_test_ctl, git_status, limen_gpu_tier0 once each, then give ONE final report: open alarms with likely cause, test failures, uncommitted work, LIMEN GPU state. Only report what the tool output shows. Propose fixes only, change nothing.'
  & D:\ATRIUM\ops\agent\ops.exe $task *>> D:\ATRIUM\ops\atrium-state\agent\sweep.log
} finally { $m.ReleaseMutex() }
