; FocusNook is local-first. Uninstalling the executable must not erase the
; encrypted vault: users may reinstall later or need support-assisted recovery.
!macro NSIS_HOOK_PREUNINSTALL
  StrCpy $DeleteAppDataCheckboxState 0
!macroend
