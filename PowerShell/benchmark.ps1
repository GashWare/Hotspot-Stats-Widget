# Benchmark Script: Comparing Original Python Hotspot_Widget vs Native Rust hotspot_stats_widget
$ErrorActionPreference = "SilentlyContinue"

Write-Host "=================================================================" -ForegroundColor Cyan
Write-Host "   BENCHMARK: Original Python vs Native Rust Hotspot Widget     " -ForegroundColor Cyan
Write-Host "=================================================================" -ForegroundColor Cyan

function Measure-AppPerformance {
    param (
        [string]$Name,
        [string]$FilePath,
        [string[]]$ArgumentList,
        [string]$WorkingDirectory,
        [int]$DurationSeconds = 12
    )

    Write-Host "`n>>> Starting Benchmark: $Name..." -ForegroundColor Yellow
    Write-Host "    Executable: $FilePath"
    Write-Host "    Working Dir: $WorkingDirectory"

    $startTime = [System.Diagnostics.Stopwatch]::StartNew()
    $process = Start-Process -FilePath $FilePath -ArgumentList $ArgumentList -WorkingDirectory $WorkingDirectory -PassThru

    if (-not $process -or $process.HasExited) {
        Write-Error "Failed to start $Name"
        return $null
    }

    # Let UI initialize for 2 seconds
    Start-Sleep -Seconds 2
    $process.Refresh()
    $initWorkingSet = [math]::Round($process.WorkingSet64 / 1MB, 2)
    $initPrivate = [math]::Round($process.PrivateMemorySize64 / 1MB, 2)

    $samples = @()
    $prevCpu = $process.CPU
    $prevTimestamp = [DateTime]::UtcNow
    $numCores = [Environment]::ProcessorCount

    $endTime = (Get-Date).AddSeconds($DurationSeconds)

    while ((Get-Date) -lt $endTime) {
        Start-Sleep -Milliseconds 500
        $process.Refresh()
        if ($process.HasExited) { break }

        $now = [DateTime]::UtcNow
        $currCpu = $process.CPU
        $elapsedSec = ($now - $prevTimestamp).TotalSeconds

        $cpuPct = 0.0
        if ($elapsedSec -gt 0) {
            $cpuPct = (($currCpu - $prevCpu) / ($elapsedSec * $numCores)) * 100.0
            if ($cpuPct -lt 0) { $cpuPct = 0.0 }
        }

        $prevCpu = $currCpu
        $prevTimestamp = $now

        $workingSetMB = [math]::Round($process.WorkingSet64 / 1MB, 2)
        $privateMB = [math]::Round($process.PrivateMemorySize64 / 1MB, 2)
        $threads = $process.Threads.Count

        $samples += [PSCustomObject]@{
            CpuPercent = $cpuPct
            WorkingSetMB = $workingSetMB
            PrivateMB = $privateMB
            Threads = $threads
        }
    }

    $totalCpuSeconds = [math]::Round($process.CPU, 3)
    Stop-Process -Id $process.Id -Force
    $startTime.Stop()

    # Calculate statistics
    $avgCpu = [math]::Round(($samples | Measure-Object -Property CpuPercent -Average).Average, 2)
    $maxCpu = [math]::Round(($samples | Measure-Object -Property CpuPercent -Maximum).Maximum, 2)
    $avgRam = [math]::Round(($samples | Measure-Object -Property WorkingSetMB -Average).Average, 2)
    $maxRam = [math]::Round(($samples | Measure-Object -Property WorkingSetMB -Maximum).Maximum, 2)
    $avgPrivate = [math]::Round(($samples | Measure-Object -Property PrivateMB -Average).Average, 2)
    $avgThreads = [math]::Round(($samples | Measure-Object -Property Threads -Average).Average, 0)

    Write-Host "    Benchmark Complete! Total Samples: $($samples.Count)" -ForegroundColor Green

    return [PSCustomObject]@{
        Name = $Name
        InitialRAM_MB = $initWorkingSet
        AvgRAM_MB = $avgRam
        PeakRAM_MB = $maxRam
        PrivateBytes_MB = $avgPrivate
        AvgCPU_Percent = $avgCpu
        PeakCPU_Percent = $maxCpu
        TotalCpuTimeSec = $totalCpuSeconds
        AvgThreads = $avgThreads
    }
}

# 1. Benchmark Original Python Version
$origPyPath = "python.exe"
$origPyArgs = @("main.py")
$origPyDir = "C:\Users\User\Desktop\Python\Hotspot_Widget"
$pyResult = Measure-AppPerformance -Name "Original Python Widget (Tkinter/psutil)" `
                                   -FilePath $origPyPath `
                                   -ArgumentList $origPyArgs `
                                   -WorkingDirectory $origPyDir `
                                   -DurationSeconds 12

# 2. Benchmark Native Rust Version
$rustExe = "c:\Users\User\Desktop\Python\Stats Widget\Rust\hotspot_stats_widget.exe"
$rustDir = "c:\Users\User\Desktop\Python\Stats Widget\Rust"
$rustResult = Measure-AppPerformance -Name "Native Rust Widget (egui/sysinfo)" `
                                     -FilePath $rustExe `
                                     -ArgumentList @() `
                                     -WorkingDirectory $rustDir `
                                     -DurationSeconds 12

# Output Comparison Results
Write-Host "`n=================================================================" -ForegroundColor Cyan
Write-Host "                      BENCHMARK COMPARISON                      " -ForegroundColor Cyan
Write-Host "=================================================================" -ForegroundColor Cyan

$results = @($pyResult, $rustResult)
$results | Format-Table -Property Name, AvgRAM_MB, PeakRAM_MB, PrivateBytes_MB, AvgCPU_Percent, PeakCPU_Percent, TotalCpuTimeSec, AvgThreads -AutoSize

# Summary Ratio Calculations
if ($pyResult -and $rustResult) {
    $ramReduction = [math]::Round((($pyResult.AvgRAM_MB - $rustResult.AvgRAM_MB) / $pyResult.AvgRAM_MB) * 100, 1)
    $cpuReduction = [math]::Round((($pyResult.TotalCpuTimeSec - $rustResult.TotalCpuTimeSec) / $pyResult.TotalCpuTimeSec) * 100, 1)
    
    Write-Host "Key Takeaways:" -ForegroundColor Yellow
    Write-Host "  - Memory Usage: Rust uses $([math]::Round($pyResult.AvgRAM_MB / $rustResult.AvgRAM_MB, 1))x LESS RAM ($ramReduction% reduction: $($pyResult.AvgRAM_MB) MB -> $($rustResult.AvgRAM_MB) MB)" -ForegroundColor Green
    Write-Host "  - CPU Time:     Rust reduced total CPU consumption by $cpuReduction% ($($pyResult.TotalCpuTimeSec)s -> $($rustResult.TotalCpuTimeSec)s)" -ForegroundColor Green
}
Write-Host "=================================================================" -ForegroundColor Cyan
