param([string]$Executable='target/debug/TrineDocs.exe',[string]$Output='target/qa-preview')
$ErrorActionPreference='Stop'
$taskRoot=Split-Path -Parent $PSScriptRoot
Set-Location $taskRoot
$Executable=[IO.Path]::GetFullPath($(if([IO.Path]::IsPathRooted($Executable)){$Executable}else{Join-Path $taskRoot $Executable}))
$Output=[IO.Path]::GetFullPath($(if([IO.Path]::IsPathRooted($Output)){$Output}else{Join-Path $taskRoot $Output}))
$targetRoot=[IO.Path]::GetFullPath((Join-Path $taskRoot 'target'))
if(!$Output.StartsWith($targetRoot+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'QA output must stay under target'}
if(Test-Path -LiteralPath $Output){throw 'Use a new output directory; old success results cannot be reused'}
if(!(Test-Path -LiteralPath $Executable)){throw 'Build the application first'}
New-Item -ItemType Directory -Path $Output -Force | Out-Null
$results=@()
foreach($mode in @('files','mcp','ui','ui-wide','ui-compact','ui-dark')){
  $path=Join-Path $Output $mode
  $profile=Join-Path $Output "$mode-profile"
  $temporary=Join-Path $Output "$mode-temp"
  New-Item -ItemType Directory -Path $profile,$temporary -Force | Out-Null
  $info=[Diagnostics.ProcessStartInfo]::new($Executable)
  $info.UseShellExecute=$false
  $info.CreateNoWindow=$true
  $info.Environment['APPDATA']=$profile
  $info.Environment['TEMP']=$temporary
  $info.Environment['TMP']=$temporary
  $flag=switch($mode){'files'{'--smoke-test'}'mcp'{'--mcp-smoke-test'}default{'--ui-smoke-test'}}
  $info.ArgumentList.Add($flag)
  $info.ArgumentList.Add($path)
  if($mode -in @('ui-wide','ui-compact','ui-dark')){$info.ArgumentList.Add("--$($mode.Replace('ui-','ui-smoke-'))")}
  $timer=[Diagnostics.Stopwatch]::StartNew()
  $process=[Diagnostics.Process]::Start($info)
  try{
    if(!$process.WaitForExit(60000)){$process.Kill($true);$process.WaitForExit();throw "$mode smoke timeout"}
    if($process.ExitCode -ne 0){throw "$mode smoke exit $($process.ExitCode)"}
  }finally{$process.Dispose();$timer.Stop()}
  $result=Get-Content -LiteralPath (Join-Path $path 'result.json') -Raw | ConvertFrom-Json
  if($result.status -ne 'passed'){throw "$mode smoke result failed"}
  $results+=[ordered]@{mode=$mode;status=$result.status;elapsed_seconds=[Math]::Round($timer.Elapsed.TotalSeconds,2)}
}
$summary=[ordered]@{status='passed';executable=$Executable;checks=$results}
[IO.File]::WriteAllText((Join-Path $Output 'result.json'),($summary | ConvertTo-Json -Depth 5),[Text.UTF8Encoding]::new($false))
$results | Format-Table -AutoSize
