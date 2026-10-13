param([string]$Version='0.1.0-preview.1',[string]$Makensis='',[switch]$SkipBuild)
$ErrorActionPreference='Stop'
if($Version -notmatch '^\d+\.\d+\.\d+(-[a-zA-Z0-9.-]+)?$'){throw 'Invalid semantic version'}
$taskRoot=Split-Path -Parent $PSScriptRoot
Set-Location $taskRoot
New-Item -ItemType Directory -Path dist -Force | Out-Null
if(!$SkipBuild){
  cargo build --locked --profile dist -p trinedocs --bins
  if($LASTEXITCODE -ne 0){throw 'Cargo dist failed'}
}
if(!$Makensis){
  $vendor=Join-Path $taskRoot 'tools/vendor'
  New-Item -ItemType Directory -Path $vendor -Force | Out-Null
  $archive=Join-Path $vendor 'nsis-3.13.zip'
  $expected='ba63dffc4410ee89193e1cb5a41989991bd77c61068da17e3156d136b7b0b3d8'
  if(!(Test-Path $archive) -or (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected){
    Invoke-WebRequest -Uri 'https://sourceforge.net/projects/nsis/files/NSIS%203/3.13/nsis-3.13.zip/download' -OutFile $archive
    if((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected){
      $html=Get-Content -LiteralPath $archive -Raw
      $match=[regex]::Match($html,'http-equiv="refresh" content="[0-9]+; url=([^"]+)"')
      if(!$match.Success){throw 'NSIS official download link unavailable'}
      $uri=[Net.WebUtility]::HtmlDecode($match.Groups[1].Value)
      if(!([uri]$uri).Host.EndsWith('.sourceforge.net')){throw 'Unexpected NSIS download host'}
      Invoke-WebRequest -Uri $uri -OutFile $archive
    }
  }
  if((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected){throw 'NSIS SHA256 mismatch; no extraction or execution'}
  Expand-Archive -LiteralPath $archive -DestinationPath $vendor -Force
  $Makensis=Join-Path $vendor 'nsis-3.13/makensis.exe'
}
python -B tools/audit_dependencies.py --output dist/THIRD-PARTY-NOTICES.txt --check-age
if($LASTEXITCODE -ne 0){throw 'Dependency license audit failed'}
Copy-Item -LiteralPath target/dist/TrineDocs.exe,target/dist/trinedocs-mcp.exe -Destination dist -Force
Copy-Item -LiteralPath docs/audit/release-review.md -Destination dist/RELEASE-REVIEW.md -Force
$payload=Join-Path $taskRoot 'dist'
$setup=Join-Path $payload 'TrineDocs-Setup.exe'
& $Makensis /V3 /WX /INPUTCHARSET UTF8 "/DPAYLOAD=$payload" "/DOUTFILE=$setup" "/DVERSION=$Version" installer/trinedocs.nsi
if($LASTEXITCODE -ne 0){throw 'NSIS failed'}
$checksum=(Get-FileHash -LiteralPath $setup -Algorithm SHA256).Hash.ToLowerInvariant()
[IO.File]::WriteAllText((Join-Path $payload 'SHA256SUMS.txt'),"$checksum  TrineDocs-Setup.exe"+[Environment]::NewLine,[Text.UTF8Encoding]::new($false))
Get-Item -LiteralPath $setup,(Join-Path $payload 'TrineDocs.exe'),(Join-Path $payload 'trinedocs-mcp.exe') | Select-Object Name,Length
