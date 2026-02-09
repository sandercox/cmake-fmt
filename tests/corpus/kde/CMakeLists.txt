# KDE-style CMakeLists.txt - Application with ECM (Extra CMake Modules)
# Demonstrates: ecm_setup_version, find_package chains, target_sources, KDE conventions

cmake_minimum_required(VERSION 3.16)

project(KDEApp VERSION 5.24.3)

# Find ECM (Extra CMake Modules) - KDE's CMake extension system
find_package(ECM 5.90.0 REQUIRED NO_MODULE)
set(CMAKE_MODULE_PATH ${ECM_MODULE_PATH} ${CMAKE_SOURCE_DIR}/cmake)

include(KDEInstallDirs)
include(KDECMakeSettings)
include(KDECompilerSettings NO_POLICY_SCOPE)
include(ECMSetupVersion)
include(ECMGenerateHeaders)
include(ECMQtDeclareLoggingCategory)
include(FeatureSummary)

# Qt and KDE Framework dependencies
set(QT_MIN_VERSION "5.15.2")
set(KF_MIN_VERSION "5.90.0")

find_package(Qt5 ${QT_MIN_VERSION} CONFIG REQUIRED COMPONENTS
  Core
  Widgets
  Gui
  Network
  DBus
  Xml
  Concurrent)

find_package(KF5 ${KF_MIN_VERSION} REQUIRED COMPONENTS
  CoreAddons
  I18n
  Config
  ConfigWidgets
  KIO
  XmlGui
  Notifications
  IconThemes
  WidgetsAddons
  WindowSystem
  Crash)

# Optional dependencies
find_package(KF5DocTools ${KF_MIN_VERSION})
set_package_properties(KF5DocTools PROPERTIES
  DESCRIPTION "Tools to generate documentation"
  TYPE OPTIONAL
  PURPOSE "Required to build the documentation")

find_package(KF5Activities ${KF_MIN_VERSION})
set_package_properties(KF5Activities PROPERTIES
  DESCRIPTION "Activities support"
  TYPE OPTIONAL
  PURPOSE "Enables activities integration")

# Version setup using ECM
ecm_setup_version(${PROJECT_VERSION}
  VARIABLE_PREFIX KDEAPP
  VERSION_HEADER "${CMAKE_CURRENT_BINARY_DIR}/kdeapp_version.h"
  PACKAGE_VERSION_FILE "${CMAKE_CURRENT_BINARY_DIR}/KDEAppConfigVersion.cmake"
  SOVERSION 5)

# Configure installation directories
set(KDEAPP_INSTALL_BINDIR "${KDE_INSTALL_BINDIR}")
set(KDEAPP_INSTALL_LIBDIR "${KDE_INSTALL_LIBDIR}")
set(KDEAPP_INSTALL_DATADIR "${KDE_INSTALL_DATADIR}/kdeapp")
set(KDEAPP_INSTALL_ICONDIR "${KDE_INSTALL_ICONDIR}")

# Build options
option(BUILD_TESTING "Build the automated tests" ON)
option(BUILD_COVERAGE "Build with code coverage support" OFF)

# Core library
add_library(kdeappcore SHARED)

target_sources(kdeappcore
  PRIVATE
    src/core/document.cpp
    src/core/settings.cpp
    src/core/plugin.cpp
    src/core/pluginmanager.cpp
    src/core/utils.cpp
  PUBLIC
    FILE_SET HEADERS
    BASE_DIRS src
    FILES
      src/core/document.h
      src/core/settings.h
      src/core/plugin.h
      src/core/pluginmanager.h
      src/core/utils.h)

target_include_directories(kdeappcore
  PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/src>
    $<BUILD_INTERFACE:${CMAKE_CURRENT_BINARY_DIR}>
    $<INSTALL_INTERFACE:${KDE_INSTALL_INCLUDEDIR}>)

target_link_libraries(kdeappcore
  PUBLIC
    Qt5::Core
    Qt5::Widgets
    KF5::CoreAddons
    KF5::ConfigCore
  PRIVATE
    Qt5::Network
    Qt5::DBus
    KF5::I18n
    KF5::ConfigWidgets
    KF5::XmlGui
    KF5::WidgetsAddons)

set_target_properties(kdeappcore PROPERTIES
  VERSION ${KDEAPP_VERSION}
  SOVERSION ${KDEAPP_SOVERSION}
  EXPORT_NAME Core)

# Generate logging categories
ecm_qt_declare_logging_category(kdeappcore
  HEADER kdeapp_debug.h
  IDENTIFIER KDEAPP
  CATEGORY_NAME org.kde.kdeapp
  DEFAULT_SEVERITY Warning
  DESCRIPTION "KDE Application")

# Main application executable
add_executable(kdeapp)

target_sources(kdeapp
  PRIVATE
    src/main.cpp
    src/mainwindow.cpp
    src/mainwindow.h
    src/ui/toolbar.cpp
    src/ui/toolbar.h
    src/ui/statusbar.cpp
    src/ui/statusbar.h
    src/ui/sidebar.cpp
    src/ui/sidebar.h
    src/widgets/customwidget.cpp
    src/widgets/customwidget.h)

# UI files for Qt Designer
qt5_wrap_ui(kdeapp_UI_SRCS
  src/ui/mainwindow.ui
  src/ui/settingsdialog.ui
  src/ui/aboutdialog.ui)

target_sources(kdeapp PRIVATE ${kdeapp_UI_SRCS})

# Resources
qt5_add_resources(kdeapp_RESOURCES
  resources/kdeapp.qrc)

target_sources(kdeapp PRIVATE ${kdeapp_RESOURCES})

target_link_libraries(kdeapp
  PRIVATE
    kdeappcore
    Qt5::Widgets
    Qt5::Gui
    KF5::CoreAddons
    KF5::I18n
    KF5::XmlGui
    KF5::Notifications
    KF5::IconThemes
    KF5::WidgetsAddons
    KF5::WindowSystem
    KF5::Crash)

# Optional features
if(KF5Activities_FOUND)
  target_link_libraries(kdeapp PRIVATE KF5::Activities)
  target_compile_definitions(kdeapp PRIVATE HAVE_ACTIVITIES)
endif()

# Plugin system
add_library(kdeapp_plugin SHARED)

target_sources(kdeapp_plugin
  PRIVATE
    src/plugins/pluginbase.cpp
    src/plugins/pluginbase.h
    src/plugins/plugininterface.h)

target_link_libraries(kdeapp_plugin
  PUBLIC
    kdeappcore
    Qt5::Core
  PRIVATE
    KF5::CoreAddons)

set_target_properties(kdeapp_plugin PROPERTIES
  VERSION ${KDEAPP_VERSION}
  SOVERSION ${KDEAPP_SOVERSION}
  EXPORT_NAME Plugin)

# Example plugin
add_library(exampleplugin MODULE)

target_sources(exampleplugin
  PRIVATE
    src/plugins/examples/exampleplugin.cpp
    src/plugins/examples/exampleplugin.h)

target_link_libraries(exampleplugin
  PRIVATE
    kdeapp_plugin
    Qt5::Widgets
    KF5::I18n)

# Installation
install(TARGETS kdeappcore
  EXPORT KDEAppTargets
  LIBRARY DESTINATION ${KDE_INSTALL_LIBDIR}
    COMPONENT runtime
  ARCHIVE DESTINATION ${KDE_INSTALL_LIBDIR}
    COMPONENT devel
  INCLUDES DESTINATION ${KDE_INSTALL_INCLUDEDIR})

install(TARGETS kdeapp
  RUNTIME DESTINATION ${KDEAPP_INSTALL_BINDIR}
    COMPONENT runtime)

install(TARGETS kdeapp_plugin
  EXPORT KDEAppTargets
  LIBRARY DESTINATION ${KDE_INSTALL_LIBDIR}
    COMPONENT runtime
  ARCHIVE DESTINATION ${KDE_INSTALL_LIBDIR}
    COMPONENT devel)

install(TARGETS exampleplugin
  LIBRARY DESTINATION ${KDE_INSTALL_PLUGINDIR}/kdeapp
    COMPONENT plugins)

# Install desktop file and metadata
install(FILES org.kde.kdeapp.desktop
  DESTINATION ${KDE_INSTALL_APPDIR}
  COMPONENT runtime)

install(FILES org.kde.kdeapp.appdata.xml
  DESTINATION ${KDE_INSTALL_METAINFODIR}
  COMPONENT runtime)

# Install icons
ecm_install_icons(ICONS
  icons/16-apps-kdeapp.png
  icons/32-apps-kdeapp.png
  icons/48-apps-kdeapp.png
  icons/64-apps-kdeapp.png
  icons/128-apps-kdeapp.png
  DESTINATION ${KDEAPP_INSTALL_ICONDIR}
  THEME hicolor)

# Configuration files
install(FILES kdeapprc
  DESTINATION ${KDE_INSTALL_CONFDIR}
  COMPONENT config)

# Documentation
if(KF5DocTools_FOUND)
  kdoctools_create_handbook(doc/index.docbook
    INSTALL_DESTINATION ${KDE_INSTALL_DOCBUNDLEDIR}/kdeapp
    SUBDIR kdeapp)
endif()

# CMake config files for downstream projects
include(CMakePackageConfigHelpers)

configure_package_config_file(
  "${CMAKE_CURRENT_SOURCE_DIR}/KDEAppConfig.cmake.in"
  "${CMAKE_CURRENT_BINARY_DIR}/KDEAppConfig.cmake"
  INSTALL_DESTINATION ${KDE_INSTALL_CMAKEPACKAGEDIR}/KDEApp)

install(FILES
  "${CMAKE_CURRENT_BINARY_DIR}/KDEAppConfig.cmake"
  "${CMAKE_CURRENT_BINARY_DIR}/KDEAppConfigVersion.cmake"
  DESTINATION ${KDE_INSTALL_CMAKEPACKAGEDIR}/KDEApp
  COMPONENT devel)

install(EXPORT KDEAppTargets
  FILE KDEAppTargets.cmake
  NAMESPACE KDEApp::
  DESTINATION ${KDE_INSTALL_CMAKEPACKAGEDIR}/KDEApp
  COMPONENT devel)

# Testing
if(BUILD_TESTING)
  add_subdirectory(autotests)
  add_subdirectory(tests)
endif()

# Feature summary - prints what was found/not found
feature_summary(WHAT ALL INCLUDE_QUIET_PACKAGES FATAL_ON_MISSING_REQUIRED_PACKAGES)
