param([Parameter(Mandatory)][string]$Script)
$name = Split-Path -Leaf $Script
scp -q -o BatchMode=yes $Script "xing@100.106.147.75:C:/Users/xing/$name"
ssh -o BatchMode=yes xing@100.106.147.75 "powershell -NoProfile -ExecutionPolicy Bypass -File C:\Users\xing\$name"
