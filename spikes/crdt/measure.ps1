param(
    [string]$Executable = "$PSScriptRoot/target/release/trinedocs-crdt-spike.exe"
)

$ErrorActionPreference = "Stop"
$outputDirectory = Join-Path $PSScriptRoot "results"
New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null
$os = Get-CimInstance Win32_OperatingSystem
$cpu = Get-CimInstance Win32_Processor | Select-Object -First 1
$hostInfo = @{
    os = $os.Caption
    build = $os.BuildNumber
    architecture = $os.OSArchitecture
    total_memory_kib = $os.TotalVisibleMemorySize
    processor = $cpu.Name
    logical_processors = $cpu.NumberOfLogicalProcessors
    acceptance_scope = "Infrastructure benchmark; Windows Server is not Windows 11 GUI acceptance"
}
$hostInfo | ConvertTo-Json | Set-Content -Encoding utf8 (Join-Path $outputDirectory "host.json")
$hostInfo | ConvertTo-Json -Compress | Write-Output

foreach ($case in @("docs", "sheets", "slides", "history", "merge", "properties", "compaction")) {
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo.FileName = (Resolve-Path $Executable).Path
    $process.StartInfo.Arguments = $case
    $process.StartInfo.UseShellExecute = $false
    $process.StartInfo.RedirectStandardOutput = $true
    $process.StartInfo.RedirectStandardError = $true
    [void]$process.Start()
    $peak = [int64]0
    while (-not $process.HasExited) {
        try {
            $process.Refresh()
            $peak = [Math]::Max($peak, $process.PeakWorkingSet64)
        } catch [System.InvalidOperationException] {
            # Exit may race with sampling. Preserve the last successful sample.
        }
        Start-Sleep -Milliseconds 10
    }
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    if ($process.ExitCode -ne 0) { throw "Benchmark $case failed: $stderr" }
    $record = $stdout | ConvertFrom-Json
    $record | Add-Member -NotePropertyName peak_working_set_sampled_bytes -NotePropertyValue $peak
    $record | Add-Member -NotePropertyName memory_method -NotePropertyValue "OS peak working set sampled every 10ms; final sub-10ms spike may be missed"
    $record | ConvertTo-Json -Compress | Write-Output
    $record | ConvertTo-Json | Set-Content -Encoding utf8 (Join-Path $outputDirectory "$case.json")
    $process.Dispose()
}
