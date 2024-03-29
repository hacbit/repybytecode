@echo off
setlocal

set IS_LINUX=0

for %%a in (%*) do (
    if "%%a" == "--wsl" (
        set IS_LINUX=1
    )
)

if %IS_LINUX% equ 1 (
    echo Building for Linux...
    cargo build --release --target x86_64-unknown-linux-musl
    echo Copying binary to /usr/local/bin/pyrev...
    wsl sudo cp target/x86_64-unknown-linux-musl/release/pyrev /usr/local/bin/pyrev
) else (
    echo Building for Windows...
    cargo build --release
    echo Copying binary to %USERPROFILE%\.cargo\bin\pyrev.exe...
    copy .\target\release\pyrev.exe %USERPROFILE%\.cargo\bin\pyrev.exe
)