# Test Script untuk Telkomsel Alarm Service (GitHub Issue #2)
# Usage: powershell -ExecutionPolicy Bypass -File .\test_alarm_service.ps1

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "  PENGUJIAN ENDPOINT ALARM ACTIVE SERVICE (BRAINCODE-BE)     " -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan

$baseUrl = "http://localhost:3000"

# 1. Uji Health Check
Write-Host "`n[1/3] Menguji Endpoint /health..." -ForegroundColor Yellow
try {
    $health = Invoke-RestMethod -Uri "$baseUrl/health" -Method Get -TimeoutSec 5
    Write-Host " Health Check Berhasil: $health" -ForegroundColor Green
} catch {
    Write-Host " Gagal terhubung ke $baseUrl. Pastikan backend 'cargo run' sudah berjalan!" -ForegroundColor Red
    exit 1
}

# 2. Uji Happy Path: POST /alarm_list_active (Issue #2 Acceptance Criteria)
Write-Host "`n[2/3] Menguji POST /alarm_list_active (Rentang Tanggal 2026-09-08 s/d 2026-09-14)..." -ForegroundColor Yellow
$bodyValid = @{
    start_date = "2026-09-08"
    end_date   = "2026-09-14"
} | ConvertTo-Json

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/alarm_list_active" -Method Post -Body $bodyValid -ContentType "application/json"
    Write-Host " Respons Diterima!" -ForegroundColor Green
    Write-Host "   - Status Code : $($response.status_code)" -ForegroundColor Cyan
    Write-Host "   - Total Alarm : $($response.total)" -ForegroundColor Cyan
    
    if ($response.result.Count -gt 0) {
        Write-Host "   - Sample Alarm Pertama:" -ForegroundColor Magenta
        $sample = $response.result[0]
        Write-Host "     * Identifier     : $($sample.identifier)"
        Write-Host "     * Alarm Name     : $($sample.alarmname)"
        Write-Host "     * Severity       : $($sample.severity)"
        Write-Host "     * Cleartime      : $($sample.cleartime) (Wajib 0 = Aktif)"
        Write-Host "     * Site Code      : $($sample.sitecode)"
        Write-Host "     * Node           : $($sample.node)"
        Write-Host "     * FirstOccurrence: $($sample.firstoccurrence)"
    } else {
        Write-Host "   - Info: Database belum terisi data atau tidak ada alarm di rentang tanggal tersebut." -ForegroundColor DarkYellow
    }
} catch {
    Write-Host " Error saat memanggil /alarm_list_active: $_" -ForegroundColor Red
}

# 3. Uji Validasi Error: Tanggal Tidak Valid (Negative Test)
Write-Host "`n[3/3] Menguji Validasi Error (start_date > end_date)..." -ForegroundColor Yellow
$bodyInvalid = @{
    start_date = "2026-09-20"
    end_date   = "2026-09-10"
} | ConvertTo-Json

try {
    $respErr = Invoke-WebRequest -Uri "$baseUrl/alarm_list_active" -Method Post -Body $bodyInvalid -ContentType "application/json"
    Write-Host " Harusnya gagal tetapi mengembalikan status $($respErr.StatusCode)" -ForegroundColor Red
} catch {
    Write-Host " Validasi Berhasil Menangkap Error! (Status 400 Bad Request sesuai harapan)" -ForegroundColor Green
}

Write-Host "`n============================================================" -ForegroundColor Cyan
Write-Host "  PENGUJIAN SELESAI                                          " -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan
