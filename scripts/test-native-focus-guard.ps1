param(
  [Parameter(Mandatory=$true)][string]$ApplicationPath,
  [Parameter(Mandatory=$true)][string]$Output,
  [ValidateSet('true','false')][string]$Pinned = 'false',
  [ValidateRange(3,15)][int]$Seconds = 6
)
$ErrorActionPreference = 'Stop'
if ($env:CI -ne 'true') { throw 'This foreground probe is restricted to the disposable CI desktop.' }
Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class FocusWindowProbe {
  public delegate bool EnumProc(IntPtr window, IntPtr parameter);
  [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc callback, IntPtr data);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr window);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out Rect rect);
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
  [DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr window);
  [DllImport("user32.dll")] public static extern bool AttachThreadInput(uint first, uint second, bool attach);
  [DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
  [DllImport("user32.dll")] public static extern IntPtr GetWindowLongPtr(IntPtr window, int index);
  [DllImport("user32.dll")] public static extern IntPtr GetTopWindow(IntPtr window);
  [DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr window, uint direction);
  public static IntPtr MiniWindow(uint processId) {
    IntPtr found = IntPtr.Zero;
    EnumWindows((window, _) => {
      uint owner; GetWindowThreadProcessId(window, out owner);
      Rect rect; GetWindowRect(window, out rect);
      if (owner == processId && IsWindowVisible(window)
          && rect.Right - rect.Left < 640 && rect.Bottom - rect.Top < 500) {
        found = window;
      }
      return true;
    }, IntPtr.Zero);
    return found;
  }
  public static bool Topmost(IntPtr window) {
    return (GetWindowLongPtr(window, -20).ToInt64() & 8) != 0;
  }
  public static bool Above(IntPtr first, IntPtr second) {
    for (IntPtr window = GetTopWindow(IntPtr.Zero); window != IntPtr.Zero; window = GetWindow(window, 2)) {
      if (window == first) return true;
      if (window == second) return false;
    }
    return false;
  }
  public static void Focus(IntPtr window) {
    uint process;
    uint foregroundThread = GetWindowThreadProcessId(GetForegroundWindow(), out process);
    uint currentThread = GetCurrentThreadId();
    bool attached = foregroundThread != currentThread && AttachThreadInput(currentThread, foregroundThread, true);
    BringWindowToTop(window);
    SetForegroundWindow(window);
    if (attached) AttachThreadInput(currentThread, foregroundThread, false);
  }
}
'@
$application = @(Get-Process -Name 'focus-timer' -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $ApplicationPath })
if ($application.Count -ne 1) { throw 'Expected exactly one CI Focus process.' }
$mini = [FocusWindowProbe]::MiniWindow([uint32]$application[0].Id)
if ($mini -eq [IntPtr]::Zero) { throw 'The native mini timer is not visible.' }
$form = New-Object System.Windows.Forms.Form
$form.Text = 'Focus CI foreground guard'
$form.Width = 420
$form.Height = 180
$form.StartPosition = 'CenterScreen'
$form.TopMost = $false
$label = New-Object System.Windows.Forms.Label
$label.Text = 'Focus must keep this window in front while its timer ticks.'
$label.AutoSize = $true
$label.Left = 24
$label.Top = 48
$form.Controls.Add($label)
$observations = [System.Collections.Generic.List[object]]::new()
$expectedPinned = $Pinned -eq 'true'
try {
  $form.Show()
  $form.Activate()
  [FocusWindowProbe]::Focus($form.Handle)
  [System.Windows.Forms.Application]::DoEvents()
  Start-Sleep -Milliseconds 150
  if ([FocusWindowProbe]::GetForegroundWindow() -ne $form.Handle) {
    throw 'The runner did not provide an interactive foreground desktop.'
  }
  $clock = [System.Diagnostics.Stopwatch]::StartNew()
  while ($clock.Elapsed.TotalSeconds -lt $Seconds) {
    [System.Windows.Forms.Application]::DoEvents()
    $sameForeground = [FocusWindowProbe]::GetForegroundWindow() -eq $form.Handle
    $isPinned = [FocusWindowProbe]::Topmost($mini)
    $guardAboveMini = [FocusWindowProbe]::Above($form.Handle, $mini)
    $observations.Add(@{
      elapsedMs = $clock.ElapsedMilliseconds
      foregroundUnchanged = $sameForeground
      miniTopmost = $isPinned
      guardAboveMini = $guardAboveMini
    })
    if (-not $sameForeground) { throw 'A timer tick stole foreground focus.' }
    if ($isPinned -ne $expectedPinned) { throw 'The native always-on-top flag changed without a user action.' }
    if (-not $expectedPinned -and -not $guardAboveMini) { throw 'An unpinned timer raised itself above the foreground window.' }
    Start-Sleep -Milliseconds 100
  }
} finally {
  @{ pinned = $expectedPinned; observations = $observations.ToArray() } |
    ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 $Output
  $form.Close()
  $form.Dispose()
}
