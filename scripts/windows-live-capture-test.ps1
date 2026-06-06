param(
    [string]$ClocheExe = "cloche.exe",
    [string]$OutRoot = "$env:TEMP\cloche-live-test",
    [ValidateSet("active", "screen", "window")]
    [string]$Target = "active",
    [int]$StyleSeed = 424242,
    [int]$WindowWidth = 680,
    [int]$WindowHeight = 460,
    [switch]$LaunchNotepad
)

$ErrorActionPreference = 'Continue'

Remove-Item -Recurse -Force $OutRoot -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $OutRoot | Out-Null

$process = $null
if ($LaunchNotepad) {
    $samplePath = Join-Path $OutRoot "sample.txt"
    @"
Cloche Windows capture test

Target: Notepad
Backend: Win32 PrintWindow
Result: no occluding terminal pixels
"@ | Set-Content -Path $samplePath -Encoding UTF8
    $process = Start-Process notepad.exe -ArgumentList @($samplePath) -PassThru
    Start-Sleep -Seconds 2
    Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class FocusNative {
  public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr hWnd, int X, int Y, int nWidth, int nHeight, bool bRepaint);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int X, int Y, int cx, int cy, uint uFlags);
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumWindowsProc callback, IntPtr extraData);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern int GetWindowTextLength(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);

  public static IntPtr FindWindowForProcess(int targetPid) {
    IntPtr found = IntPtr.Zero;
    EnumWindows(delegate(IntPtr hWnd, IntPtr lParam) {
      if (!IsWindowVisible(hWnd)) { return true; }
      int length = GetWindowTextLength(hWnd);
      if (length == 0) { return true; }
      uint pid;
      GetWindowThreadProcessId(hWnd, out pid);
      if (pid == targetPid) {
        found = hWnd;
        return false;
      }
      return true;
    }, IntPtr.Zero);
    return found;
  }
}
"@
    try {
        $process.Refresh()
        $windowHandle = $process.MainWindowHandle
        if ($windowHandle -eq [IntPtr]::Zero) {
            $windowHandle = [FocusNative]::FindWindowForProcess($process.Id)
        }
        if ($windowHandle -ne [IntPtr]::Zero) {
            [void][FocusNative]::ShowWindow($windowHandle, 9)
            [void][FocusNative]::MoveWindow($windowHandle, 120, 120, $WindowWidth, $WindowHeight, $true)
            [void][FocusNative]::SetWindowPos($windowHandle, [IntPtr](-1), 120, 120, $WindowWidth, $WindowHeight, 0x0040)
            [void][FocusNative]::SetForegroundWindow($windowHandle)
        }
    } catch {
        $_ | Out-String | Set-Content -Path (Join-Path $OutRoot "focus-error.txt")
    }
    Start-Sleep -Seconds 1
}

$captureDir = Join-Path $OutRoot "capture"
$captureArgs = @(
    "capture",
    "--target",
    $Target,
    "--presentation",
    "both",
    "--style-seed",
    "$StyleSeed",
    "--out-dir",
    $captureDir,
    "--format",
    "json"
)
if ($LaunchNotepad -and $Target -eq "window") {
    $captureArgs += @("--app", "notepad")
}
& $ClocheExe @captureArgs *> (Join-Path $OutRoot "combined.log")
$exitCode = $LASTEXITCODE
$exitCode | Set-Content -Path (Join-Path $OutRoot "exit.txt")

if ($process -and -not $process.HasExited) {
    try {
        [void]$process.CloseMainWindow()
        Start-Sleep -Seconds 1
    } catch {}
    try {
        if (-not $process.HasExited) {
            $process.Kill()
        }
    } catch {}
}

exit $exitCode
