# Search for opencl.lib and clang.dll, set LIB and LIBCLANG_PATH

# Define common search locations
$searchPaths = @(
    "C:\\Program Files\\NVIDIA GPU Computing Toolkit\\CUDA\\v12.8\\lib\\x64",
    "C:\\Program Files\\LLVM\\bin"
)
$arch = if ([Environment]::Is64BitOperatingSystem) { 'x64' } else { 'x86' }


function Prompt-For-Path($libName) {
    while ($true) {
        $userPath = Read-Host "Could not find $libName. Please enter the full path to $libName (or leave blank to skip)"
        if ([string]::IsNullOrWhiteSpace($userPath)) { return $null }
        if (Test-Path $userPath -PathType Leaf -and (Split-Path $userPath -Leaf) -ieq $libName) {
            return Get-Item $userPath
        } else {
            Write-Host "Invalid path or file name. Please try again."
        }
    }
}

function Get-Dll-Arch($dllPath) {
    $fs = [System.IO.File]::OpenRead($dllPath)
    $br = New-Object System.IO.BinaryReader($fs)
    $fs.Seek(0x3C, 'Begin') | Out-Null
    $peOffset = $br.ReadInt32()
    $fs.Seek($peOffset + 4, 'Begin') | Out-Null
    $machine = $br.ReadUInt16()
    $fs.Close()
    switch ($machine) {
        0x8664 { return 'x64' }
        0x014c { return 'x86' }
        default { return 'unknown' }
    }
}

function Get-Lib-Arch($libPath) {
    $fs = [System.IO.File]::OpenRead($libPath)
    $br = New-Object System.IO.BinaryReader($fs)
    $fs.Seek(0x20, 'Begin') | Out-Null
    $machine = $br.ReadUInt16()
    $fs.Close()
    switch ($machine) {
        0x8664 { return 'x64' }
        0x014c { return 'x86' }
        default { return 'unknown' }
    }
}

# Optimized search for opencl.lib
Write-Host "Searching for opencl.lib in common locations..."
$openclLib = $null
foreach ($path in $searchPaths) {
    if (Test-Path $path) {
        try {
            Write-Host "Confirmed path: $path...`nChecking..."
            Get-ChildItem -Path $path -Filter opencl.lib -Recurse -ErrorAction SilentlyContinue -Force | ForEach-Object {
                
                if ($_.Name -ieq 'opencl.lib' -or $_.Name -ieq 'OpenCL.lib') {
                    Write-Host "Found opencl.lib: $($_.FullName)"
                    $libArch = Get-Lib-Arch $_.FullName
                    Write-Host "Detected architecture: $libArch"
                    if ($libArch -eq $arch -or $_.FullName -match $arch) {
                        $openclLib = $_
                        break
                    }
                }
            }
            if ($openclLib) { break }
            Write-Host "No opencl.lib found in $path."
        } catch {}
    }
}
if (-not $openclLib) {
    $openclLib = Prompt-For-Path "opencl.lib"
    if ($openclLib) {
        $libArch = Get-Lib-Arch $openclLib.FullName
        if ($libArch -ne $arch) {
            Write-Host "Warning: The provided opencl.lib may not match your system architecture ($arch)."
        }
    }
}
if ($openclLib) {
    $libDir = $openclLib.DirectoryName
    Write-Host "Found opencl.lib at $($openclLib.FullName)"
    $env:LIB = $libDir
    Write-Host "LIB environment variable set to $libDir"
} else {
    Write-Host "opencl.lib not found and not provided. LIB will not be set."
}

# Optimized search for libclang.dll
Write-Host "Searching for libclang.dll in common locations..."
$clangDll = $null
foreach ($path in $searchPaths) {
    if (Test-Path $path) {
        try {
            Get-ChildItem -Path $path -Filter libclang.dll -Recurse -ErrorAction SilentlyContinue -Force | ForEach-Object {
                if ($_.Name -ieq 'libclang.dll') {
                    $dllArch = Get-Dll-Arch $_.FullName
                    if ($dllArch -eq $arch) {
                        $clangDll = $_
                        break
                    }
                }
            }
            if ($clangDll) { break }
        } catch {}
    }
}
if (-not $clangDll) {
    $clangDll = Prompt-For-Path "libclang.dll"
    if ($clangDll) {
        $dllArch = Get-Dll-Arch $clangDll.FullName
        if ($dllArch -ne $arch) {
            Write-Host "Warning: The provided libclang.dll may not match your system architecture ($arch)."
        }
    }
}
if ($clangDll) {
    $clangDir = $clangDll.DirectoryName
    Write-Host "Found libclang.dll at $($clangDll.FullName)"
    $env:LIBCLANG_PATH = $clangDir
    Write-Host "LIBCLANG_PATH environment variable set to $clangDir"
} else {
    Write-Host "libclang.dll not found and not provided. LIBCLANG_PATH will not be set."
}

Write-Host "Done. These environment variables are set for this session."

# Run cargo installer and sam
Write-Host "Running cargo run --bin installer..."
$cargoInstaller = Start-Process cargo -ArgumentList 'run --bin installer' -NoNewWindow -Wait -PassThru
if ($cargoInstaller.ExitCode -eq 0) {
    Write-Host "installer succeeded. Running cargo run --bin sam..."
    Start-Process cargo -ArgumentList 'run --bin sam' -NoNewWindow -Wait
} else {
    Write-Host "installer failed. Not running sam."
}