# KDEInstallDirs.cmake - Installation directory configuration for KDE projects
# Based on GNUInstallDirs but with KDE-specific additions and conventions

include(GNUInstallDirs)

# KDE-specific installation directories
# These build on top of the GNU standard directories

# Determine if we're on a Unix-like system
if(UNIX)
  set(APPLE_SUPPRESS_X11_WARNING ON)
endif()

# Base directories
if(NOT DEFINED KDE_INSTALL_USE_QT_SYS_PATHS)
  set(KDE_INSTALL_USE_QT_SYS_PATHS OFF CACHE BOOL "Install to the Qt system paths")
endif()

# Binary directory
if(NOT DEFINED KDE_INSTALL_BINDIR)
  if(WIN32)
    set(KDE_INSTALL_BINDIR "bin" CACHE PATH "User executables (bin)")
  else()
    set(KDE_INSTALL_BINDIR "${CMAKE_INSTALL_BINDIR}" CACHE PATH "User executables (bin)")
  endif()
endif()

# Library directory
if(NOT DEFINED KDE_INSTALL_LIBDIR)
  if(WIN32)
    set(KDE_INSTALL_LIBDIR "lib" CACHE PATH "Object code libraries (lib)")
  else()
    set(KDE_INSTALL_LIBDIR "${CMAKE_INSTALL_LIBDIR}" CACHE PATH "Object code libraries (lib)")
  endif()
endif()

# Library executable directory (internal executables, not for PATH)
if(NOT DEFINED KDE_INSTALL_LIBEXECDIR)
  if(WIN32)
    set(KDE_INSTALL_LIBEXECDIR "bin" CACHE PATH "Internal executables (libexec)")
  elseif(APPLE)
    set(KDE_INSTALL_LIBEXECDIR "${CMAKE_INSTALL_LIBEXECDIR}" CACHE PATH "Internal executables (libexec)")
  else()
    set(KDE_INSTALL_LIBEXECDIR "${CMAKE_INSTALL_LIBEXECDIR}/kf5" CACHE PATH "Internal executables (libexec)")
  endif()
endif()

# System configuration directory
if(NOT DEFINED KDE_INSTALL_SYSCONFDIR)
  if(WIN32)
    set(KDE_INSTALL_SYSCONFDIR "etc" CACHE PATH "System configuration (etc)")
  else()
    set(KDE_INSTALL_SYSCONFDIR "${CMAKE_INSTALL_FULL_SYSCONFDIR}" CACHE PATH "System configuration (etc)")
  endif()
endif()

# Configuration directory for KDE applications
if(NOT DEFINED KDE_INSTALL_CONFDIR)
  if(WIN32)
    set(KDE_INSTALL_CONFDIR "etc/xdg" CACHE PATH "Application configuration (etc/xdg)")
  elseif(APPLE)
    set(KDE_INSTALL_CONFDIR "/Library/Application Support" CACHE PATH "Application configuration")
  else()
    set(KDE_INSTALL_CONFDIR "${CMAKE_INSTALL_SYSCONFDIR}/xdg" CACHE PATH "Application configuration (etc/xdg)")
  endif()
endif()

# Data directory
if(NOT DEFINED KDE_INSTALL_DATADIR)
  if(WIN32)
    set(KDE_INSTALL_DATADIR "share" CACHE PATH "Read-only architecture-independent data (share)")
  else()
    set(KDE_INSTALL_DATADIR "${CMAKE_INSTALL_DATADIR}" CACHE PATH "Read-only architecture-independent data (share)")
  endif()
endif()

# Include directory
if(NOT DEFINED KDE_INSTALL_INCLUDEDIR)
  if(WIN32)
    set(KDE_INSTALL_INCLUDEDIR "include" CACHE PATH "C/C++ header files (include)")
  else()
    set(KDE_INSTALL_INCLUDEDIR "${CMAKE_INSTALL_INCLUDEDIR}" CACHE PATH "C/C++ header files (include)")
  endif()
endif()

# Documentation directory
if(NOT DEFINED KDE_INSTALL_DOCBUNDLEDIR)
  if(APPLE)
    set(KDE_INSTALL_DOCBUNDLEDIR "/Library/Documentation/Help" CACHE PATH "Documentation bundles")
  else()
    set(KDE_INSTALL_DOCBUNDLEDIR "${KDE_INSTALL_DATADIR}/doc/HTML" CACHE PATH "Documentation bundles")
  endif()
endif()

# Man pages directory
if(NOT DEFINED KDE_INSTALL_MANDIR)
  if(WIN32)
    set(KDE_INSTALL_MANDIR "man" CACHE PATH "Man documentation (man)")
  else()
    set(KDE_INSTALL_MANDIR "${CMAKE_INSTALL_MANDIR}" CACHE PATH "Man documentation (man)")
  endif()
endif()

# Info pages directory
if(NOT DEFINED KDE_INSTALL_INFODIR)
  if(WIN32)
    set(KDE_INSTALL_INFODIR "info" CACHE PATH "Info documentation (info)")
  else()
    set(KDE_INSTALL_INFODIR "${CMAKE_INSTALL_INFODIR}" CACHE PATH "Info documentation (info)")
  endif()
endif()

# Locale directory (translations)
if(NOT DEFINED KDE_INSTALL_LOCALEDIR)
  if(WIN32)
    set(KDE_INSTALL_LOCALEDIR "share/locale" CACHE PATH "Locale-dependent data (locale)")
  elseif(APPLE)
    set(KDE_INSTALL_LOCALEDIR "${KDE_INSTALL_DATADIR}/locale" CACHE PATH "Locale-dependent data (locale)")
  else()
    set(KDE_INSTALL_LOCALEDIR "${CMAKE_INSTALL_LOCALEDIR}" CACHE PATH "Locale-dependent data (locale)")
  endif()
endif()

# Qt plugin directory
if(NOT DEFINED KDE_INSTALL_QTPLUGINDIR)
  if(KDE_INSTALL_USE_QT_SYS_PATHS)
    # Use Qt's plugin directory
    query_qmake(qt_plugins_dir QT_INSTALL_PLUGINS)
    set(KDE_INSTALL_QTPLUGINDIR "${qt_plugins_dir}" CACHE PATH "Qt plugins")
  else()
    if(WIN32)
      set(KDE_INSTALL_QTPLUGINDIR "plugins" CACHE PATH "Qt plugins")
    else()
      set(KDE_INSTALL_QTPLUGINDIR "${KDE_INSTALL_LIBDIR}/plugins" CACHE PATH "Qt plugins")
    endif()
  endif()
endif()

# Generic plugin directory
if(NOT DEFINED KDE_INSTALL_PLUGINDIR)
  set(KDE_INSTALL_PLUGINDIR "${KDE_INSTALL_QTPLUGINDIR}" CACHE PATH "Plugin directory")
endif()

# QML imports directory
if(NOT DEFINED KDE_INSTALL_QMLDIR)
  if(KDE_INSTALL_USE_QT_SYS_PATHS)
    query_qmake(qt_qml_dir QT_INSTALL_QML)
    set(KDE_INSTALL_QMLDIR "${qt_qml_dir}" CACHE PATH "QML imports")
  else()
    if(WIN32)
      set(KDE_INSTALL_QMLDIR "qml" CACHE PATH "QML imports")
    else()
      set(KDE_INSTALL_QMLDIR "${KDE_INSTALL_LIBDIR}/qml" CACHE PATH "QML imports")
    endif()
  endif()
endif()

# Desktop files directory
if(NOT DEFINED KDE_INSTALL_APPDIR)
  set(KDE_INSTALL_APPDIR "${KDE_INSTALL_DATADIR}/applications" CACHE PATH "Application desktop files")
endif()

# AppStream metadata directory
if(NOT DEFINED KDE_INSTALL_METAINFODIR)
  set(KDE_INSTALL_METAINFODIR "${KDE_INSTALL_DATADIR}/metainfo" CACHE PATH "AppStream metadata")
endif()

# Icon directory
if(NOT DEFINED KDE_INSTALL_ICONDIR)
  set(KDE_INSTALL_ICONDIR "${KDE_INSTALL_DATADIR}/icons" CACHE PATH "Icons")
endif()

# Sound files directory
if(NOT DEFINED KDE_INSTALL_SOUNDDIR)
  set(KDE_INSTALL_SOUNDDIR "${KDE_INSTALL_DATADIR}/sounds" CACHE PATH "Sound files")
endif()

# Wallpapers directory
if(NOT DEFINED KDE_INSTALL_WALLPAPERDIR)
  set(KDE_INSTALL_WALLPAPERDIR "${KDE_INSTALL_DATADIR}/wallpapers" CACHE PATH "Wallpapers")
endif()

# D-Bus interfaces directory
if(NOT DEFINED KDE_INSTALL_DBUSINTERFACEDIR)
  set(KDE_INSTALL_DBUSINTERFACEDIR "${KDE_INSTALL_DATADIR}/dbus-1/interfaces" CACHE PATH "D-Bus interfaces")
endif()

# D-Bus system services directory
if(NOT DEFINED KDE_INSTALL_DBUSSYSTEMSERVICEDIR)
  if(WIN32)
    set(KDE_INSTALL_DBUSSYSTEMSERVICEDIR "share/dbus-1/system-services" CACHE PATH "D-Bus system services")
  else()
    set(KDE_INSTALL_DBUSSYSTEMSERVICEDIR "${CMAKE_INSTALL_FULL_DATADIR}/dbus-1/system-services" CACHE PATH "D-Bus system services")
  endif()
endif()

# D-Bus session services directory
if(NOT DEFINED KDE_INSTALL_DBUSSESSIONSERVICEDIR)
  if(WIN32)
    set(KDE_INSTALL_DBUSSESSIONSERVICEDIR "share/dbus-1/services" CACHE PATH "D-Bus session services")
  else()
    set(KDE_INSTALL_DBUSSESSIONSERVICEDIR "${CMAKE_INSTALL_FULL_DATADIR}/dbus-1/services" CACHE PATH "D-Bus session services")
  endif()
endif()

# KDE services directory
if(NOT DEFINED KDE_INSTALL_KSERVICES5DIR)
  set(KDE_INSTALL_KSERVICES5DIR "${KDE_INSTALL_DATADIR}/kservices5" CACHE PATH "KDE services")
endif()

# KDE service types directory
if(NOT DEFINED KDE_INSTALL_KSERVICETYPES5DIR)
  set(KDE_INSTALL_KSERVICETYPES5DIR "${KDE_INSTALL_DATADIR}/kservicetypes5" CACHE PATH "KDE service types")
endif()

# CMake packages directory
if(NOT DEFINED KDE_INSTALL_CMAKEPACKAGEDIR)
  set(KDE_INSTALL_CMAKEPACKAGEDIR "${KDE_INSTALL_LIBDIR}/cmake" CACHE PATH "CMake packages")
endif()

# Autocomplete scripts directory
if(NOT DEFINED KDE_INSTALL_AUTOCOMPLDIR)
  set(KDE_INSTALL_AUTOCOMPLDIR "${KDE_INSTALL_DATADIR}/bash-completion/completions" CACHE PATH "Bash completion scripts")
endif()

# systemd unit files directory
if(NOT DEFINED KDE_INSTALL_SYSTEMDUSERUNITDIR)
  if(UNIX AND NOT APPLE AND NOT WIN32)
    set(KDE_INSTALL_SYSTEMDUSERUNITDIR "${CMAKE_INSTALL_PREFIX}/lib/systemd/user" CACHE PATH "systemd user unit files")
  endif()
endif()

# systemd system unit files directory
if(NOT DEFINED KDE_INSTALL_SYSTEMDUNITDIR)
  if(UNIX AND NOT APPLE AND NOT WIN32)
    set(KDE_INSTALL_SYSTEMDUNITDIR "/lib/systemd/system" CACHE PATH "systemd system unit files")
  endif()
endif()

# Helper function to query qmake for paths
function(query_qmake varname property)
  if(NOT DEFINED ${varname})
    if(TARGET Qt5::qmake)
      get_target_property(qmake_executable Qt5::qmake LOCATION)
      execute_process(
        COMMAND "${qmake_executable}" -query "${property}"
        RESULT_VARIABLE return_code
        OUTPUT_VARIABLE output
        OUTPUT_STRIP_TRAILING_WHITESPACE)

      if(return_code EQUAL 0)
        file(TO_CMAKE_PATH "${output}" output_path)
        set(${varname} "${output_path}" PARENT_SCOPE)
      endif()
    endif()
  endif()
endfunction()

# Mark all variables as advanced
mark_as_advanced(
  KDE_INSTALL_BINDIR
  KDE_INSTALL_LIBDIR
  KDE_INSTALL_LIBEXECDIR
  KDE_INSTALL_SYSCONFDIR
  KDE_INSTALL_CONFDIR
  KDE_INSTALL_DATADIR
  KDE_INSTALL_INCLUDEDIR
  KDE_INSTALL_DOCBUNDLEDIR
  KDE_INSTALL_MANDIR
  KDE_INSTALL_INFODIR
  KDE_INSTALL_LOCALEDIR
  KDE_INSTALL_QTPLUGINDIR
  KDE_INSTALL_PLUGINDIR
  KDE_INSTALL_QMLDIR
  KDE_INSTALL_APPDIR
  KDE_INSTALL_METAINFODIR
  KDE_INSTALL_ICONDIR
  KDE_INSTALL_SOUNDDIR
  KDE_INSTALL_WALLPAPERDIR
  KDE_INSTALL_DBUSINTERFACEDIR
  KDE_INSTALL_DBUSSYSTEMSERVICEDIR
  KDE_INSTALL_DBUSSESSIONSERVICEDIR
  KDE_INSTALL_KSERVICES5DIR
  KDE_INSTALL_KSERVICETYPES5DIR
  KDE_INSTALL_CMAKEPACKAGEDIR
  KDE_INSTALL_AUTOCOMPLDIR
  KDE_INSTALL_SYSTEMDUSERUNITDIR
  KDE_INSTALL_SYSTEMDUNITDIR)
