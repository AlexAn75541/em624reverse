# Windows PowerShell script to find EM624 Mouse VID/PID and HID details
Write-Host "=== Scanning for EM624 / Gaming Mouse HID Devices ===" -ForegroundColor Cyan

$devices = Get-PnpDevice -Class "HIDClass", "Mouse" -Status "OK" | Where-Object {
    $_.FriendlyName -match "Mouse|Gaming|EM624" -or $_.Description -match "Mouse|HID"
}

foreach ($dev in $devices) {
    $hwId = ($dev | Get-PnpDeviceProperty -KeyName "DEVPKEY_Device_HardwareIds").Data
    $match = [regex]::Match($hwId, "VID_([0-9A-Fa-f]{4})&PID_([0-9A-Fa-f]{4})")
    
    if ($match.Success) {
        $vendorId = $match.Groups[1].Value
        $productId = $match.Groups[2].Value
        Write-Host "Found: $($dev.FriendlyName) [$($dev.InstanceId)]" -ForegroundColor Green
        Write-Host "  -> VID: 0x$vendorId | PID: 0x$productId`n" -ForegroundColor Yellow
    }
}

Write-Host "=== Done. Copy VID and PID for em624 CLI configuration ===" -ForegroundColor Cyan
