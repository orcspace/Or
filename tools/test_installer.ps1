param([string]$Setup='dist/TrineDocs-Setup.exe',[string]$Destination='',[string]$Output='target/installer-smoke',[switch]$SkipUI)
$ErrorActionPreference='Stop'
$taskRoot=Split-Path -Parent $PSScriptRoot
Set-Location $taskRoot
$Setup=[IO.Path]::GetFullPath($(if([IO.Path]::IsPathRooted($Setup)){$Setup}else{Join-Path $taskRoot $Setup}))
$Output=[IO.Path]::GetFullPath($(if([IO.Path]::IsPathRooted($Output)){$Output}else{Join-Path $taskRoot $Output}))
if(Test-Path $Output){throw 'Use a new QA output directory; previous results must not be reused'}
if(!$Destination){$Destination=Join-Path $Output 'installed'}
$Destination=[IO.Path]::GetFullPath($Destination)
if(!$Destination.StartsWith($Output+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'Test install destination must stay under Output'}
if(Test-Path $Destination){throw 'Use a new empty install test destination'}
$unkey='HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\TrineDocs'
if(Test-Path $unkey){throw 'Existing TrineDocs installation must not be touched by this test'}
if(Test-Path 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\TrineDocs'){throw 'Existing machine installation must not be touched'}
foreach($name in 'TrineDocs.Doc','TrineDocs.Sheet','TrineDocs.Slide','TrineDocs.Office','Applications\TrineDocs.exe'){foreach($hive in 'HKCU','HKLM'){if(Test-Path "$($hive):\Software\Classes\$name"){throw "Existing application association must not be touched: $name"}}}
foreach($ext in '.docx','.dotx','.docm','.xlsx','.xlsm','.xltx','.ods','.pptx','.ppsx','.potx','.csv','.tsv','.txt','.md','.trinedoc','.trinesheet','.trineslide'){
  $program=switch($ext){'.trinedoc'{'TrineDocs.Doc'}'.trinesheet'{'TrineDocs.Sheet'}'.trineslide'{'TrineDocs.Slide'}default{'TrineDocs.Office'}}
  foreach($hive in 'HKCU','HKLM'){
    $key="$($hive):\Software\Classes\$ext\OpenWithProgids"
    if(Test-Path $key){
      $value=Get-Item -LiteralPath $key
      if($value.GetValueNames() -contains $program){throw "Existing OpenWith association must not be touched: $ext"}
    }
  }
}
foreach($path in @((Join-Path ([Environment]::GetFolderPath('Desktop')) 'TrineDocs.lnk'),(Join-Path ([Environment]::GetFolderPath('StartMenu')) 'Programs/TrineDocs'))){if(Test-Path $path){throw "Existing shortcut must not be touched: $path"}}
function Invoke-App([string]$Executable,[string[]]$Arguments){
  $info=[Diagnostics.ProcessStartInfo]::new($Executable)
  $info.UseShellExecute=$false
  $info.CreateNoWindow=$true
  foreach($argument in $Arguments){$info.ArgumentList.Add($argument)}
  $process=[Diagnostics.Process]::Start($info)
  if(!$process.WaitForExit(60000)){$process.Kill($true);$process.WaitForExit();$process.Dispose();throw 'Application smoke timeout'}
  $code=$process.ExitCode
  $process.Dispose()
  if($code -ne 0){throw "Application exit $code"}
}
function Invoke-NSIS([string]$Executable,[string]$RawArguments,[switch]$ExpectFailure){
  $info=[Diagnostics.ProcessStartInfo]::new($Executable)
  $info.UseShellExecute=$false
  $info.CreateNoWindow=$true
  $info.Arguments=$RawArguments
  $process=[Diagnostics.Process]::Start($info)
  if(!$process.WaitForExit(60000)){$process.Kill($true);$process.WaitForExit();$process.Dispose();throw 'Installer process timeout'}
  $code=$process.ExitCode
  $process.Dispose()
  if($ExpectFailure){if($code -eq 0){throw "Installer unexpectedly succeeded"}}elseif($code -ne 0){throw "Installer process exit $code"}
}
$exts='.docx','.xlsx','.pptx','.csv','.txt','.md','.trinedoc','.trinesheet','.trineslide'
$before=@{}
foreach($ext in $exts){$before[$ext]=(Get-ItemProperty -LiteralPath "HKCU:\Software\Classes\$ext" -Name '(default)' -ErrorAction SilentlyContinue).'(default)'}
New-Item -ItemType Directory -Path $Output -Force | Out-Null
$exe=Join-Path $Destination 'TrineDocs.exe'
$uninstaller=Join-Path $Destination 'Uninstall.exe'
$priorAppData=$env:APPDATA
$env:APPDATA=Join-Path $Output 'appdata'
try{
Invoke-NSIS -Executable $Setup -RawArguments "/S /D=$Destination"
$exe=Join-Path $Destination 'TrineDocs.exe'
$uninstaller=Join-Path $Destination 'Uninstall.exe'
if(!(Test-Path $exe) -or !(Test-Path $uninstaller)){throw 'Installer payload missing'}
Invoke-NSIS -Executable $Setup -RawArguments "/S /D=$Destination" -ExpectFailure
$temporaryUninstaller=Join-Path $Output 'uninstall-test.exe'
Copy-Item -LiteralPath $uninstaller -Destination $temporaryUninstaller
$lockedPayload=[IO.File]::Open($exe,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::None)
try{
  Invoke-NSIS -Executable $temporaryUninstaller -RawArguments "/S _?=$Destination" -ExpectFailure
  if(!(Test-Path $exe) -or !(Test-Path (Join-Path $Destination 'trinedocs-mcp.exe')) -or !(Test-Path $unkey)){throw 'Blocked uninstall changed payload or registration'}
}finally{$lockedPayload.Dispose()}

  foreach($ext in $exts[0..5]){$after=(Get-ItemProperty -LiteralPath "HKCU:\Software\Classes\$ext" -Name '(default)' -ErrorAction SilentlyContinue).'(default)';if($after -ne $before[$ext]){throw "Office default association changed: $ext"}}
  $modes=@('files','mcp')
  if(!$SkipUI){$modes+='ui'}
  foreach($mode in $modes){
    $flag=switch($mode){'files'{'--smoke-test'}'mcp'{'--mcp-smoke-test'}'ui'{'--ui-smoke-test'}}
    $path=Join-Path $Output $mode
    Invoke-App -Executable $exe -Arguments @($flag,$path)
    $result=Get-Content -LiteralPath (Join-Path $path 'result.json') -Raw | ConvertFrom-Json
    if($result.status -ne 'passed'){throw "$mode smoke result failed"}
  }
}finally{
  try{
  $registered=(Get-ItemProperty -LiteralPath $unkey -Name 'InstallLocation' -ErrorAction SilentlyContinue).InstallLocation
  if($registered -eq $Destination -and (Test-Path $uninstaller)){
    $temporaryUninstaller=Join-Path $Output 'uninstall-test.exe'
    Copy-Item -LiteralPath $uninstaller -Destination $temporaryUninstaller
    Invoke-NSIS -Executable $temporaryUninstaller -RawArguments "/S _?=$Destination"
    Remove-Item -LiteralPath $temporaryUninstaller
  }
  }finally{$env:APPDATA=$priorAppData}
}
if(Test-Path $exe){throw 'Uninstaller left main executable'}
if(Test-Path $uninstaller){throw 'Uninstaller left its installed executable'}
if(Test-Path $unkey){throw 'Uninstaller left registration'}
foreach($ext in $exts){$after=(Get-ItemProperty -LiteralPath "HKCU:\Software\Classes\$ext" -Name '(default)' -ErrorAction SilentlyContinue).'(default)';if($after -ne $before[$ext]){throw "Association not restored: $ext"}}
$summary=[ordered]@{status='passed';installation='per-user';file_smoke=$true;mcp_smoke=$true;ui_smoke=(!$SkipUI);office_defaults_preserved=$true;associations_restored=$true;uninstall=$true;duplicate_install_rejected=$true;locked_uninstall_preserves_payload=$true}
[IO.File]::WriteAllText((Join-Path $Output 'result.json'),($summary|ConvertTo-Json -Compress),[Text.UTF8Encoding]::new($false))
