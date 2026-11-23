# Match the actual WebView2 Runtime, not the separately updated Edge browser.
# Microsoft: learn.microsoft.com/microsoft-edge/webview2/how-to/webdriver
$ErrorActionPreference = 'Stop'
if (-not $IsWindows) { throw 'Run this script on Windows.' }
$registryPaths = @(
  'HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
  'HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
  'HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',
  'HKCU:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
)
$runtimeVersion = $null
foreach ($registryPath in $registryPaths) {
  $candidate = Get-ItemPropertyValue -Path $registryPath -Name pv -ErrorAction SilentlyContinue
  if ($candidate -match '^\d+\.\d+\.\d+\.\d+$' -and $candidate -ne '0.0.0.0') {
    $runtimeVersion = $candidate
    break
  }
}
if (-not $runtimeVersion) { throw 'The runner has no WebView2 Runtime.' }
$focusDriverDirectory = Join-Path $env:RUNNER_TEMP 'focus-webdriver'
New-Item -ItemType Directory -Path $focusDriverDirectory -Force | Out-Null
$archive = Join-Path $focusDriverDirectory 'driver.zip'
$uri = "https://msedgedriver.microsoft.com/$runtimeVersion/edgedriver_win64.zip"
Invoke-WebRequest -Uri $uri -OutFile $archive -MaximumRetryCount 3 -RetryIntervalSec 2
Expand-Archive -LiteralPath $archive -DestinationPath $focusDriverDirectory -Force
$driver = Join-Path $focusDriverDirectory 'msedgedriver.exe'
$driverVersion = (& $driver --version) -replace '^.*?(\d+\.\d+\.\d+\.\d+).*$', '$1'
$driverBuild = ($driverVersion -split '\.')[0..2] -join '.'
$runtimeBuild = ($runtimeVersion -split '\.')[0..2] -join '.'
if ($driverBuild -ne $runtimeBuild) {
  throw "WebView2 ($runtimeVersion) and Edge Driver ($driverVersion) do not match."
}
$signature = Get-AuthenticodeSignature -FilePath $driver
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'Microsoft') {
  throw 'The downloaded Edge Driver does not have a valid Microsoft signature.'
}
$evidence = Join-Path $PWD 'native-evidence/windows'
New-Item -ItemType Directory -Path $evidence -Force | Out-Null
@{
  webview2 = $runtimeVersion
  driver = $driverVersion
  sha256 = (Get-FileHash $driver -Algorithm SHA256).Hash.ToLowerInvariant()
  signature = [string]$signature.Status
} | ConvertTo-Json | Set-Content -Encoding utf8 (Join-Path $evidence 'driver-version.json')
if ($env:GITHUB_PATH) { $focusDriverDirectory | Out-File -FilePath $env:GITHUB_PATH -Encoding utf8 -Append }
if ($env:GITHUB_ENV) { "FOCUS_NATIVE_DRIVER=$driver" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append }
Write-Host "Matched Microsoft WebView2 $runtimeVersion with Edge Driver $driverVersion."
