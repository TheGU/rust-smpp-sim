@echo off
setlocal

REM Check if version argument is provided
if "%~1"=="" (
    echo Usage: RELEASE_VERSION
    exit /b 1
)

set VERSION=%~1
echo [INFO] Preparing release for version %VERSION%...

REM Check if git is clean
git diff-index --quiet HEAD --
if %errorlevel% neq 0 (
    echo [ERROR] Git workspace is not clean. Commit changes first.
    exit /b 1
)

REM Verify tests
echo [INFO] Running tests...
cargo test
if %errorlevel% neq 0 (
    echo [ERROR] Tests failed. Aborting release.
    exit /b 1
)

REM Build release binary
echo [INFO] Building release binary...
cargo build --release
if %errorlevel% neq 0 (
    echo [ERROR] Build failed. Aborting release.
    exit /b 1
)

REM Create distribution archive
echo [INFO] Creating distribution package...
mkdir dist
copy target\release\rust-smpp-sim.exe dist\
xcopy templates dist\templates\ /E /I
xcopy static dist\static\ /E /I
copy .env.example dist\.env

echo [INFO] Release package created in dist/
echo [INFO] Release %VERSION% preparation complete.
echo [INFO] To publish Docker image: docker build -t rust-smpp-sim:%VERSION% .

endlocal
