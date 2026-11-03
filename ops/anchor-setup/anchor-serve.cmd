@echo off
:loop
C:\Users\there\anchor-peer\bin\ductei-sync.exe serve C:\Users\there\anchor-peer --listen 100.121.3.55:7421 --scope mcp:atrium:anchor --scope mcp:video-watch:anchor --allow-ip 100.106.147.75 >> C:\Users\there\anchor-peer\serve.log 2>&1
echo [%date% %time%] serve exited %errorlevel%, restarting in 5s >> C:\Users\there\anchor-peer\serve.log
ping -n 6 127.0.0.1 >nul
goto loop
