@echo off
setlocal

set "ROOT=%~dp0.."

if "%~1"=="build" (
  shift
  call "%ROOT%\node_modules\.bin\vite.cmd" build %*
  exit /b %errorlevel%
)

if "%~1"=="dev" (
  shift
  call "%ROOT%\node_modules\.bin\vite.cmd" dev %*
  exit /b %errorlevel%
)

echo Unsupported local pnpm shim command: %*
exit /b 1
