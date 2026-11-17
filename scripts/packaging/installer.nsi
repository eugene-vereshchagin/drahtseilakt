; Drahtseilakt Windows installer (NSIS 3), made by scripts/ci-windows-installer.sh, which passes VERSION, ARCH, OUTFILE,
; HAVE_TOOLBOX (0|1) and the generated file lists FILES and UNFILES.
; Per-user by default: no administrator rights, %LOCALAPPDATA%\Programs\Drahtseilakt.
Unicode true
!include "MUI2.nsh"
!include "LogicLib.nsh"

Name "Drahtseilakt"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\Programs\Drahtseilakt"
InstallDirRegKey HKCU "Software\Drahtseilakt" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
BrandingText "Drahtseilakt ${VERSION} (${ARCH})"

!define UNKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Drahtseilakt"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\drahtseilakt.exe"
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Drahtseilakt"
  SetOverwrite on
  !include "${FILES}"
  SetOutPath "$INSTDIR"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateDirectory "$SMPROGRAMS\Drahtseilakt"
  CreateShortcut "$SMPROGRAMS\Drahtseilakt\Drahtseilakt.lnk" "$INSTDIR\drahtseilakt.exe"
  !if ${HAVE_TOOLBOX} == 1
    CreateShortcut "$SMPROGRAMS\Drahtseilakt\Drahtseilakt Toolbox.lnk" "$INSTDIR\drahtseilakt-toolbox.exe"
  !endif
  CreateShortcut "$SMPROGRAMS\Drahtseilakt\Uninstall Drahtseilakt.lnk" "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\Drahtseilakt" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "${UNKEY}" "DisplayName" "Drahtseilakt"
  WriteRegStr HKCU "${UNKEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNKEY}" "Publisher" "Veresk"
  WriteRegStr HKCU "${UNKEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNKEY}" "DisplayIcon" "$INSTDIR\drahtseilakt.exe"
  WriteRegStr HKCU "${UNKEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegDWORD HKCU "${UNKEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNKEY}" "NoRepair" 1
SectionEnd

; Removes exactly the files the installer wrote (a generated list), then the folders it made if they are empty.
; The player's data (%APPDATA%\drahtseilakt: settings, replays, imported levels) is never touched.
Section "Uninstall"
  !include "${UNFILES}"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\Drahtseilakt\Drahtseilakt.lnk"
  Delete "$SMPROGRAMS\Drahtseilakt\Drahtseilakt Toolbox.lnk"
  Delete "$SMPROGRAMS\Drahtseilakt\Uninstall Drahtseilakt.lnk"
  RMDir "$SMPROGRAMS\Drahtseilakt"
  DeleteRegKey HKCU "${UNKEY}"
  DeleteRegKey HKCU "Software\Drahtseilakt"
SectionEnd
