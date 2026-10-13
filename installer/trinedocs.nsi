Unicode true
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "FileFunc.nsh"
!include "WinVer.nsh"
!include "x64.nsh"
!include "nsDialogs.nsh"
!ifndef PAYLOAD
!error "Pass /DPAYLOAD=absolute payload directory"
!endif
!ifndef OUTFILE
!define OUTFILE "TrineDocs-Setup.exe"
!endif
!ifndef VERSION
!define VERSION "0.1.0-preview.1"
!endif
Name "TrineDocs ${VERSION} (unsigned preview)"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\Programs\TrineDocs"
RequestExecutionLevel user
SetCompressor /SOLID lzma
SetCompressorDictSize 32
SetDatablockOptimize on
ManifestDPIAware true
VIProductVersion "0.1.0.0"
VIAddVersionKey /LANG=1033 "ProductName" "TrineDocs"
VIAddVersionKey /LANG=1033 "FileDescription" "TrineDocs unsigned preview installer"
VIAddVersionKey /LANG=1033 "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=1033 "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=1033 "LegalCopyright" "TrineDocs contributors"
!define UNKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\TrineDocs"
!define CLASSES "Software\Classes"
!define MUI_ABORTWARNING
!define MUI_ICON "..\crates\trinedocs\assets\trinedocs.ico"
!define MUI_UNICON "..\crates\trinedocs\assets\trinedocs.ico"
!insertmacro MUI_PAGE_WELCOME
Page custom ScopePage ScopeLeave
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "Russian"
!insertmacro MUI_LANGUAGE "English"
Var Scope
Var Admin
Var ScopeBox
Var Dialog
Var Params
Var Option
Function .onInit
  SetRegView 64
  ReadRegStr $0 HKCU "${UNKEY}" "InstallLocation"
  ReadRegStr $1 HKLM "${UNKEY}" "InstallLocation"
  ${If} $0 != ""
  ${OrIf} $1 != ""
    MessageBox MB_OK|MB_ICONSTOP "Preview upgrades require uninstalling the existing TrineDocs first. Documents and APPDATA are kept." /SD IDOK
    Abort
  ${EndIf}
  ${IfNot} ${AtLeastWin10}
    MessageBox MB_OK|MB_ICONSTOP "Requires Windows 10/11 x64." /SD IDOK
    Abort
  ${EndIf}
  ${IfNot} ${RunningX64}
    MessageBox MB_OK|MB_ICONSTOP "Requires Windows x64." /SD IDOK
    Abort
  ${EndIf}
  StrCpy $Scope "current"
  SetShellVarContext current
  UserInfo::GetAccountType
  Pop $Admin
  ${GetParameters} $Params
  ${GetOptions} $Params "/ALLUSERS" $Option
  ${IfNot} ${Errors}
    ${If} $Admin != "Admin"
      MessageBox MB_OK|MB_ICONSTOP "For /ALLUSERS, run this installer as administrator." /SD IDOK
      Abort
    ${EndIf}
    StrCpy $Scope "all"
    SetShellVarContext all
    StrCpy $INSTDIR "$PROGRAMFILES64\TrineDocs"
  ${EndIf}
FunctionEnd
Function ScopePage
  nsDialogs::Create 1018
  Pop $Dialog
  ${If} $Dialog == error
    Abort
  ${EndIf}
  ${NSD_CreateLabel} 0 0 100% 38u "Неподписанная предварительная сборка. Полный паритет Office ещё не достигнут. По умолчанию установка для текущего пользователя без UAC."
  Pop $0
  ${NSD_CreateCheckbox} 0 48u 100% 20u "Для всех пользователей"
  Pop $ScopeBox
  ${If} $Scope == "all"
    ${NSD_Check} $ScopeBox
  ${EndIf}
  ${If} $Admin != "Admin"
    EnableWindow $ScopeBox 0
    ${NSD_CreateLabel} 0 78u 100% 30u "Для установки в Program Files запустите установщик от имени администратора."
    Pop $0
  ${EndIf}
  nsDialogs::Show
FunctionEnd
Function ScopeLeave
  ${NSD_GetState} $ScopeBox $0
  ${If} $0 == ${BST_CHECKED}
    StrCpy $Scope "all"
    SetShellVarContext all
    StrCpy $INSTDIR "$PROGRAMFILES64\TrineDocs"
  ${Else}
    StrCpy $Scope "current"
    SetShellVarContext current
    StrCpy $INSTDIR "$LOCALAPPDATA\Programs\TrineDocs"
  ${EndIf}
FunctionEnd
!macro OpenWith EXT PROG
  WriteRegNone SHCTX "${CLASSES}\.${EXT}\OpenWithProgids" "${PROG}"
  WriteRegNone SHCTX "${CLASSES}\Applications\TrineDocs.exe\SupportedTypes" ".${EXT}"
!macroend
!macro Native EXT PROG LABEL
  ClearErrors
  ReadRegStr $0 SHCTX "${CLASSES}\.${EXT}" ""
  ${If} ${Errors}
    WriteRegDWORD SHCTX "${UNKEY}\AssociationBackup" ".${EXT}.present" 0
  ${Else}
    WriteRegDWORD SHCTX "${UNKEY}\AssociationBackup" ".${EXT}.present" 1
  ${EndIf}
  WriteRegStr SHCTX "${UNKEY}\AssociationBackup" ".${EXT}" "$0"
  WriteRegStr SHCTX "${CLASSES}\.${EXT}" "" "${PROG}"
  WriteRegStr SHCTX "${CLASSES}\${PROG}" "" "${LABEL}"
  WriteRegStr SHCTX "${CLASSES}\${PROG}\DefaultIcon" "" '"$INSTDIR\TrineDocs.exe",0'
  WriteRegStr SHCTX "${CLASSES}\${PROG}\shell\open\command" "" '"$INSTDIR\TrineDocs.exe" "%1"'
  !insertmacro OpenWith "${EXT}" "${PROG}"
!macroend
Section "TrineDocs" Main
  ${If} ${FileExists} "$INSTDIR\TrineDocs.exe"
  ${OrIf} ${FileExists} "$INSTDIR\trinedocs-mcp.exe"
  ${OrIf} ${FileExists} "$INSTDIR\Uninstall.exe"
  ${OrIf} ${FileExists} "$INSTDIR\THIRD-PARTY-NOTICES.txt"
  ${OrIf} ${FileExists} "$INSTDIR\RELEASE-REVIEW.md"
    MessageBox MB_OK|MB_ICONSTOP "Destination contains application files. Choose an empty application directory." /SD IDOK
    Abort
  ${EndIf}
  SetOutPath "$INSTDIR"
  ClearErrors
  File "${PAYLOAD}\TrineDocs.exe"
  File "${PAYLOAD}\trinedocs-mcp.exe"
  File "${PAYLOAD}\THIRD-PARTY-NOTICES.txt"
  File "${PAYLOAD}\RELEASE-REVIEW.md"
  ${If} ${Errors}
    Delete "$INSTDIR\TrineDocs.exe"
    Delete "$INSTDIR\trinedocs-mcp.exe"
    Delete "$INSTDIR\THIRD-PARTY-NOTICES.txt"
    Delete "$INSTDIR\RELEASE-REVIEW.md"
    RMDir "$INSTDIR"
    MessageBox MB_OK|MB_ICONSTOP "Cannot write application files. Retry in a writable empty application directory." /SD IDOK
    Abort
  ${EndIf}
  ClearErrors
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  ${If} ${Errors}
    Delete "$INSTDIR\TrineDocs.exe"
    Delete "$INSTDIR\trinedocs-mcp.exe"
    Delete "$INSTDIR\THIRD-PARTY-NOTICES.txt"
    Delete "$INSTDIR\RELEASE-REVIEW.md"
    RMDir "$INSTDIR"
    MessageBox MB_OK|MB_ICONSTOP "Could not write the uninstaller." /SD IDOK
    Abort
  ${EndIf}
  WriteRegStr SHCTX "${UNKEY}" "DisplayName" "TrineDocs ${VERSION} (unsigned preview)"
  WriteRegStr SHCTX "${UNKEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr SHCTX "${UNKEY}" "Publisher" "TrineDocs contributors"
  WriteRegStr SHCTX "${UNKEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr SHCTX "${UNKEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr SHCTX "${UNKEY}" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'
  WriteRegStr SHCTX "${UNKEY}" "DisplayIcon" "$INSTDIR\TrineDocs.exe,0"
  WriteRegStr SHCTX "${UNKEY}" "InstallScope" "$Scope"
  WriteRegDWORD SHCTX "${UNKEY}" "NoModify" 1
  WriteRegDWORD SHCTX "${UNKEY}" "NoRepair" 1
  WriteRegStr SHCTX "${CLASSES}\TrineDocs.Office" "" "TrineDocs office document"
  WriteRegStr SHCTX "${CLASSES}\TrineDocs.Office\DefaultIcon" "" '"$INSTDIR\TrineDocs.exe",0'
  WriteRegStr SHCTX "${CLASSES}\TrineDocs.Office\shell\open\command" "" '"$INSTDIR\TrineDocs.exe" "%1"'
  WriteRegStr SHCTX "${CLASSES}\Applications\TrineDocs.exe\shell\open\command" "" '"$INSTDIR\TrineDocs.exe" "%1"'
  !insertmacro OpenWith "docx" "TrineDocs.Office"
  !insertmacro OpenWith "dotx" "TrineDocs.Office"
  !insertmacro OpenWith "docm" "TrineDocs.Office"
  !insertmacro OpenWith "xlsx" "TrineDocs.Office"
  !insertmacro OpenWith "xlsm" "TrineDocs.Office"
  !insertmacro OpenWith "xltx" "TrineDocs.Office"
  !insertmacro OpenWith "ods" "TrineDocs.Office"
  !insertmacro OpenWith "pptx" "TrineDocs.Office"
  !insertmacro OpenWith "ppsx" "TrineDocs.Office"
  !insertmacro OpenWith "potx" "TrineDocs.Office"
  !insertmacro OpenWith "csv" "TrineDocs.Office"
  !insertmacro OpenWith "tsv" "TrineDocs.Office"
  !insertmacro OpenWith "txt" "TrineDocs.Office"
  !insertmacro OpenWith "md" "TrineDocs.Office"
  !insertmacro Native "trinedoc" "TrineDocs.Doc" "TrineDocs document"
  !insertmacro Native "trinesheet" "TrineDocs.Sheet" "TrineDocs spreadsheet"
  !insertmacro Native "trineslide" "TrineDocs.Slide" "TrineDocs presentation"
  CreateDirectory "$SMPROGRAMS\TrineDocs"
  CreateShortcut "$SMPROGRAMS\TrineDocs\TrineDocs.lnk" "$INSTDIR\TrineDocs.exe"
  CreateShortcut "$SMPROGRAMS\TrineDocs\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
  CreateShortcut "$DESKTOP\TrineDocs.lnk" "$INSTDIR\TrineDocs.exe"
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
SectionEnd
Function un.onInit
  SetRegView 64
  SetShellVarContext current
  ReadRegStr $0 HKCU "${UNKEY}" "InstallLocation"
  ${If} $0 != $INSTDIR
    ReadRegStr $0 HKLM "${UNKEY}" "InstallLocation"
    ${If} $0 == $INSTDIR
      UserInfo::GetAccountType
      Pop $Admin
      ${If} $Admin != "Admin"
        MessageBox MB_OK|MB_ICONSTOP "Run the uninstaller as administrator." /SD IDOK
        Abort
      ${EndIf}
      SetShellVarContext all
    ${Else}
      MessageBox MB_OK|MB_ICONSTOP "Install registration does not match this directory." /SD IDOK
      Abort
    ${EndIf}
  ${EndIf}
FunctionEnd
!macro UnOpenWith EXT PROG
  DeleteRegValue SHCTX "${CLASSES}\.${EXT}\OpenWithProgids" "${PROG}"
  DeleteRegKey /ifempty SHCTX "${CLASSES}\.${EXT}\OpenWithProgids"
!macroend
!macro UnNative EXT PROG
  ReadRegStr $0 SHCTX "${CLASSES}\.${EXT}" ""
  ${If} $0 == "${PROG}"
    ReadRegStr $1 SHCTX "${UNKEY}\AssociationBackup" ".${EXT}"
    ReadRegDWORD $2 SHCTX "${UNKEY}\AssociationBackup" ".${EXT}.present"
    ${If} $2 == 0
      DeleteRegValue SHCTX "${CLASSES}\.${EXT}" ""
    ${Else}
      WriteRegStr SHCTX "${CLASSES}\.${EXT}" "" "$1"
    ${EndIf}
  ${EndIf}
  !insertmacro UnOpenWith "${EXT}" "${PROG}"
  DeleteRegKey /ifempty SHCTX "${CLASSES}\.${EXT}"
  DeleteRegKey SHCTX "${CLASSES}\${PROG}"
!macroend
Section "Uninstall"
  ${If} ${FileExists} "$INSTDIR\TrineDocs.exe"
    System::Call 'kernel32::CreateFileW(w "$INSTDIR\TrineDocs.exe", i 0x40000000, i 0, p 0, i 3, i 0, p 0) p.r0'
    ${If} $0 == -1
      MessageBox MB_OK|MB_ICONSTOP "Close TrineDocs before uninstall." /SD IDOK
      Abort
    ${EndIf}
    System::Call 'kernel32::CloseHandle(p r0)'
  ${EndIf}
  ${If} ${FileExists} "$INSTDIR\trinedocs-mcp.exe"
    System::Call 'kernel32::CreateFileW(w "$INSTDIR\trinedocs-mcp.exe", i 0x40000000, i 0, p 0, i 3, i 0, p 0) p.r0'
    ${If} $0 == -1
      MessageBox MB_OK|MB_ICONSTOP "Close MCP clients before uninstall." /SD IDOK
      Abort
    ${EndIf}
    System::Call 'kernel32::CloseHandle(p r0)'
  ${EndIf}
  Delete "$INSTDIR\TrineDocs.exe"
  Delete "$INSTDIR\trinedocs-mcp.exe"
  ${If} ${FileExists} "$INSTDIR\TrineDocs.exe"
  ${OrIf} ${FileExists} "$INSTDIR\trinedocs-mcp.exe"
    MessageBox MB_OK|MB_ICONSTOP "Close TrineDocs and MCP clients, then retry uninstall." /SD IDOK
    Abort
  ${EndIf}
  Delete "$INSTDIR\THIRD-PARTY-NOTICES.txt"
  Delete "$INSTDIR\RELEASE-REVIEW.md"
  Delete "$SMPROGRAMS\TrineDocs\TrineDocs.lnk"
  Delete "$SMPROGRAMS\TrineDocs\Uninstall.lnk"
  RMDir "$SMPROGRAMS\TrineDocs"
  Delete "$DESKTOP\TrineDocs.lnk"
  !insertmacro UnOpenWith "docx" "TrineDocs.Office"
  !insertmacro UnOpenWith "dotx" "TrineDocs.Office"
  !insertmacro UnOpenWith "docm" "TrineDocs.Office"
  !insertmacro UnOpenWith "xlsx" "TrineDocs.Office"
  !insertmacro UnOpenWith "xlsm" "TrineDocs.Office"
  !insertmacro UnOpenWith "xltx" "TrineDocs.Office"
  !insertmacro UnOpenWith "ods" "TrineDocs.Office"
  !insertmacro UnOpenWith "pptx" "TrineDocs.Office"
  !insertmacro UnOpenWith "ppsx" "TrineDocs.Office"
  !insertmacro UnOpenWith "potx" "TrineDocs.Office"
  !insertmacro UnOpenWith "csv" "TrineDocs.Office"
  !insertmacro UnOpenWith "tsv" "TrineDocs.Office"
  !insertmacro UnOpenWith "txt" "TrineDocs.Office"
  !insertmacro UnOpenWith "md" "TrineDocs.Office"
  !insertmacro UnNative "trinedoc" "TrineDocs.Doc"
  !insertmacro UnNative "trinesheet" "TrineDocs.Sheet"
  !insertmacro UnNative "trineslide" "TrineDocs.Slide"
  DeleteRegKey SHCTX "${CLASSES}\TrineDocs.Office"
  DeleteRegKey SHCTX "${CLASSES}\Applications\TrineDocs.exe"
  DeleteRegKey SHCTX "${UNKEY}"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
SectionEnd
