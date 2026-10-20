@echo off
setlocal EnableExtensions
title Fallout New Vegas - Goodsprings World Builder
echo.
echo  FALLOUT: NEW VEGAS - NATIVE iPHONE WORLD BUILDER
echo  Source-only ByroRedux tool. Runs entirely on your PC.
echo  Multi-material world export (FNVWLD01 v3).
echo.
set "EXE=%~dp0falloutnv-ios-nif-extractor.exe"
if not exist "%EXE%" (
  echo ERROR: Keep this script in the same folder as falloutnv-ios-nif-extractor.exe
  pause
  exit /b 1
)
rem This handshake fails with any older v2 exporter, even if someone renamed
rem its output .fnvworld file to "-v3".
"%EXE%" --require-world-v3
if errorlevel 1 (
  echo ERROR: The extractor EXE is an old or incompatible version.
  echo Download the latest Windows exporter artifact and extract BOTH
  echo Build-Goodsprings.cmd and falloutnv-ios-nif-extractor.exe together.
  echo Do not copy the new CMD beside an old EXE.
  pause
  exit /b 1
)
set "DATA=%ProgramFiles(x86)%\Steam\steamapps\common\Fallout New Vegas\Data"
if not exist "%DATA%\FalloutNV.esm" (
  for %%D in (C D E F G H) do (
    if exist "%%D:\SteamLibrary\steamapps\common\Fallout New Vegas\Data\FalloutNV.esm" (
      set "DATA=%%D:\SteamLibrary\steamapps\common\Fallout New Vegas\Data"
    )
  )
)
if not exist "%DATA%\FalloutNV.esm" (
  echo Couldn't find the Steam New Vegas installation automatically.
  echo Enter the FULL path of Fallout New Vegas\Data ^(without quotation marks^).
  set /p "DATA=Game Data folder: "
)
if not exist "%DATA%\FalloutNV.esm" (
  echo ERROR: FalloutNV.esm does not exist at the selected folder.
  pause
  exit /b 1
)
if not exist "%DATA%\Fallout - Meshes.bsa" (
  echo ERROR: Meshes BSA missing. Verify local game files in Steam.
  pause
  exit /b 1
)
if not exist "%DATA%\Fallout - Textures.bsa" (
  echo ERROR: Textures BSA missing. Verify local game files in Steam.
  pause
  exit /b 1
)
set "OUTPUT=%USERPROFILE%\Desktop\GoodspringsSaloon-v3.fnvworld"
if exist "%OUTPUT%" (
  echo The output already exists at:
  echo "%OUTPUT%"
  echo Please move or rename it before creating a new version.
  pause
  exit /b 1
)
echo.
echo CELL: GSProspectorSaloonInterior
echo SOURCE: "%DATA%"
echo DESTINATION: "%OUTPUT%"
echo.
"%EXE%" --export-cell "%DATA%\FalloutNV.esm" "%DATA%\Fallout - Meshes.bsa" "%DATA%\Fallout - Textures.bsa" "%OUTPUT%" "GSProspectorSaloonInterior"
if errorlevel 1 (
  echo.
  echo EXPORT FAILED. Copy the exact error output if assistance is needed.
  pause
  exit /b 1
)
echo.
echo COMPLETE: A private, source-authored Goodsprings world was exported.
echo Copy GoodspringsSaloon-v3.fnvworld from Desktop to iPhone Files.
echo It will be stored in the app for offline, unplugged launches.
echo No original Bethesda files were uploaded anywhere.
pause
exit /b 0
