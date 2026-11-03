Get-ScheduledTaskInfo atrium-anchor-peer | Select-Object LastRunTime,LastTaskResult | Format-List
(Get-ScheduledTask atrium-anchor-peer).State
(Get-ScheduledTask atrium-anchor-peer).Principal | Select-Object UserId,LogonType,RunLevel | Format-List
Get-Content C:\Users\there\anchor-peer\serve.log -Tail 8
query user 2>&1
