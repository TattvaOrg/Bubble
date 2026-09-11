set(BUBBLE_SOURCES
    main.cpp
    services/configmanager.cpp
    services/themeloader.cpp
    services/clipboardmanager.cpp
    services/fileoperations.cpp
    services/xdgtrash.cpp
    services/giotransferworker.cpp
    services/draghelper.cpp
    models/tabmodel.cpp
    models/tablistmodel.cpp
    models/bookmarkmodel.cpp
    models/filesystemmodel.cpp
    models/devicemodel.cpp
    models/recentfilesmodel.cpp
    models/searchresultsmodel.cpp
    models/searchproxymodel.cpp
    services/searchservice.cpp
    services/undomanager.cpp
    services/previewservice.cpp
    services/diskusageservice.cpp
    services/remoteaccessservice.cpp
    services/rcloneservice.cpp
    services/runtimefeaturesservice.cpp
    services/dependencychecker.cpp
    services/gitstatusservice.cpp
    services/sessionstate.cpp
    services/metadataextractor.cpp
    services/cryptoengine.cpp
    services/vaultdatabase.cpp
    services/vaultservice.cpp
    providers/thumbnailprovider.cpp
    providers/pdfpreviewprovider.cpp
)

qt_add_executable(bubble ${BUBBLE_SOURCES})
add_executable(hyprfm ALIAS bubble)

option(BUBBLE_ENABLE_PCH "Enable precompiled headers (faster builds)" ON)
option(BUBBLE_ENABLE_UNITY_BUILD "Enable unity builds (faster clean builds)" ON)

if(BUBBLE_ENABLE_PCH)
    target_precompile_headers(bubble PRIVATE
        <QtCore/QObject>
        <QtCore/QString>
        <QtCore/QStringList>
        <QtCore/QVariant>
        <QtCore/QDebug>
        <QtCore/QAbstractListModel>
        <QtCore/QHash>
        <QtCore/QFileInfo>
        <QtCore/QDir>
    )
endif()

if(BUBBLE_ENABLE_UNITY_BUILD)
    set_target_properties(bubble PROPERTIES
        UNITY_BUILD ON
        UNITY_BUILD_BATCH_SIZE 8
    )
    # Exclude files with anonymous-namespace symbol collisions from unity batches
    # (multiple files define local helpers like isRemoteUri/parentLocation)
    set_source_files_properties(
        models/bookmarkmodel.cpp
        models/tabmodel.cpp
        models/filesystemmodel.cpp
        services/fileoperations.cpp
        PROPERTIES SKIP_UNITY_BUILD_INCLUSION ON
    )
endif()

set_source_files_properties(qml/theme/Theme.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)

option(BUBBLE_ENABLE_QML_CACHEGEN "Enable Qt QML cache compilation (faster startup, slower builds)" OFF)

set(bubble_qml_module_args
    URI Bubble
    VERSION 1.0
)

if(NOT BUBBLE_ENABLE_QML_CACHEGEN)
    list(APPEND bubble_qml_module_args NO_CACHEGEN)
endif()

list(APPEND bubble_qml_module_args
    QML_FILES
        qml/Main.qml
        qml/theme/Theme.qml
        qml/components/StatusBar.qml
        qml/components/OperationsBar.qml
        qml/components/Sidebar.qml
        qml/components/Breadcrumb.qml
        qml/components/Toolbar.qml
        qml/components/HoverRect.qml
        qml/components/SearchBar.qml
        qml/components/FilterPanel.qml
        qml/components/ContextMenu.qml
        qml/components/BulkRenameDialog.qml
        qml/components/ArchivePasswordDialog.qml
        qml/components/LockPasswordDialog.qml
        qml/components/LockOverlay.qml
        qml/components/RemoteConnectDialog.qml
        qml/components/KeyboardShortcutsDialog.qml
        qml/components/MissingDependenciesDialog.qml
        qml/components/SettingsPanel.qml
        qml/components/PreviewLoader.qml
        qml/components/QuickPreview.qml
        qml/components/Toast.qml
        qml/components/RubberBand.qml
        qml/views/FileViewContainer.qml
        qml/views/FileGridView.qml
        qml/views/FileDetailedView.qml
        qml/views/FileMillerView.qml
        qml/views/KineticWheelScroller.qml
        qml/icons/IconAlertCircle.qml
        qml/icons/IconAlignJustify.qml
        qml/icons/IconArrowRight.qml
        qml/icons/IconBattery.qml
        qml/icons/IconBell.qml
        qml/icons/IconBellOff.qml
        qml/icons/IconBluetooth.qml
        qml/icons/IconBluetoothConnected.qml
        qml/icons/IconBluetoothOff.qml
        qml/icons/IconBookmark.qml
        qml/icons/IconBookOpen.qml
        qml/icons/IconCalculator.qml
        qml/icons/IconCalendar.qml
        qml/icons/IconCamera.qml
        qml/icons/IconCheck.qml
        qml/icons/IconChevronDown.qml
        qml/icons/IconChevronLeft.qml
        qml/icons/IconChevronRight.qml
        qml/icons/IconChevronUp.qml
        qml/icons/IconCircle.qml
        qml/icons/IconClipboard.qml
        qml/icons/IconClock.qml
        qml/icons/IconCloud.qml
        qml/icons/IconCoffee.qml
        qml/icons/IconColumns.qml
        qml/icons/IconColumns3.qml
        qml/icons/IconCopy.qml
        qml/icons/IconCopyPath.qml
        qml/icons/IconDownload.qml
        qml/icons/IconEraser.qml
        qml/icons/IconEthernet.qml
        qml/icons/IconExternalLink.qml
        qml/icons/IconEye.qml
        qml/icons/IconEyeOff.qml
        qml/icons/IconFilePlus.qml
        qml/icons/IconFiles.qml
        qml/icons/IconFileText.qml
        qml/icons/IconFilter.qml
        qml/icons/IconFolder.qml
        qml/icons/IconFolderArchive.qml
        qml/icons/IconFolderPen.qml
        qml/icons/IconGitConflicted.qml
        qml/icons/IconGitDeleted.qml
        qml/icons/IconGitDirty.qml
        qml/icons/IconGitIgnored.qml
        qml/icons/IconGitModified.qml
        qml/icons/IconGitRenamed.qml
        qml/icons/IconGitStaged.qml
        qml/icons/IconGitUntracked.qml
        qml/icons/IconGlobe.qml
        qml/icons/IconGrid.qml
        qml/icons/IconHand.qml
        qml/icons/IconHardDrive.qml
        qml/icons/IconHardDriveOff.qml
        qml/icons/IconHighlighter.qml
        qml/icons/IconHome.qml
        qml/icons/IconImage.qml
        qml/icons/IconInfo.qml
        qml/icons/IconKeyboard.qml
        qml/icons/IconLayoutGrid.qml
        qml/icons/IconLink.qml
        qml/icons/IconList.qml
        qml/icons/IconLock.qml
        qml/icons/IconLockOpen.qml
        qml/icons/IconLogOut.qml
        qml/icons/IconMaximize.qml
        qml/icons/IconMic.qml
        qml/icons/IconMicOff.qml
        qml/icons/IconMinus.qml
        qml/icons/IconMonitor.qml
        qml/icons/IconMoon.qml
        qml/icons/IconMusic.qml
        qml/icons/IconPackageOpen.qml
        qml/icons/IconPalette.qml
        qml/icons/IconPanelLeft.qml
        qml/icons/IconPanelTop.qml
        qml/icons/IconPause.qml
        qml/icons/IconPen.qml
        qml/icons/IconPin.qml
        qml/icons/IconPinOff.qml
        qml/icons/IconPlay.qml
        qml/icons/IconPlus.qml
        qml/icons/IconPower.qml
        qml/icons/IconPrinter.qml
        qml/icons/IconRefreshCw.qml
        qml/icons/IconRocket.qml
        qml/icons/IconRotateCcw.qml
        qml/icons/IconRotateCw.qml
        qml/icons/IconScissors.qml
        qml/icons/IconSearch.qml
        qml/icons/IconSettings.qml
        qml/icons/IconSkipBack.qml
        qml/icons/IconSkipForward.qml
        qml/icons/IconSlidersH.qml
        qml/icons/IconSquare.qml
        qml/icons/IconSquareSplitHorizontal.qml
        qml/icons/IconStar.qml
        qml/icons/IconStrikethrough.qml
        qml/icons/IconSun.qml
        qml/icons/IconTerminal.qml
        qml/icons/IconTrash.qml
        qml/icons/IconTriangleAlert.qml
        qml/icons/IconType.qml
        qml/icons/IconUnderline.qml
        qml/icons/IconUndo.qml
        qml/icons/IconUnlink.qml
        qml/icons/IconUsb.qml
        qml/icons/IconUser.qml
        qml/icons/IconVideo.qml
        qml/icons/IconVolume.qml
        qml/icons/IconVolume1.qml
        qml/icons/IconVolume2.qml
        qml/icons/IconVolumeX.qml
        qml/icons/IconWifi.qml
        qml/icons/IconWifiOff.qml
        qml/icons/IconWifiSector.qml
        qml/icons/IconWifiStrength.qml
        qml/icons/IconX.qml
        qml/icons/IconZap.qml
        qml/icons/IconZoomIn.qml
        qml/icons/IconZoomOut.qml
)

qt_add_qml_module(bubble ${bubble_qml_module_args})

# Make Quill module discoverable at runtime
set_target_properties(bubble PROPERTIES
    QT_QML_IMPORT_PATH "${CMAKE_CURRENT_SOURCE_DIR}/qml"
)

target_link_libraries(bubble PRIVATE
    Qt6::Core
    Qt6::Gui
    Qt6::Qml
    Qt6::Quick
    Qt6::QuickControls2
    Qt6::DBus
    Qt6::Svg
    Qt6::Network
    Qt6::Concurrent
    Qt6::Sql
    OpenSSL::Crypto
    PkgConfig::ARGON2
    PkgConfig::GIO
)

if(KF6WindowSystem_FOUND)
    target_link_libraries(bubble PRIVATE KF6::WindowSystem)
endif()

target_include_directories(bubble PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
    ${CMAKE_CURRENT_SOURCE_DIR}/third_party
)

# Standalone helper targets mapped to cargo_bubble
add_custom_target(bubble-vault-destroy DEPENDS cargo_bubble)
add_custom_target(bubble-vault-helper DEPENDS cargo_bubble)

set(BUBBLE_DATA_DIR "${CMAKE_SOURCE_DIR}" CACHE STRING "Runtime data directory for themes and QML")

target_compile_definitions(bubble PRIVATE
    BUBBLE_SOURCE_DIR="${CMAKE_SOURCE_DIR}"
    BUBBLE_DATA_DIR="${BUBBLE_DATA_DIR}"
    BUBBLE_VERSION="${PROJECT_VERSION}"
    HYPRFM_SOURCE_DIR="${CMAKE_SOURCE_DIR}"
    HYPRFM_DATA_DIR="${BUBBLE_DATA_DIR}"
    HYPRFM_VERSION="${PROJECT_VERSION}"
)

if(KF6WindowSystem_FOUND)
    target_compile_definitions(bubble PRIVATE
        BUBBLE_HAS_KWINDOWSYSTEM=1
        HYPRFM_HAS_KWINDOWSYSTEM=1
    )
endif()

install(TARGETS bubble DESTINATION bin)
install(PROGRAMS
    "${CMAKE_SOURCE_DIR}/target/release/bubble-vault-helper"
    "${CMAKE_SOURCE_DIR}/target/release/bubble-vault-destroy"
    DESTINATION bin
)
install(FILES "${CMAKE_SOURCE_DIR}/dist/io.github.soyeb_jim285.Bubble.desktop"
    DESTINATION share/applications)
install(FILES "${CMAKE_SOURCE_DIR}/dist/io.github.soyeb_jim285.Bubble.svg"
    DESTINATION share/icons/hicolor/scalable/apps)
install(FILES "${CMAKE_SOURCE_DIR}/dist/io.github.soyeb_jim285.Bubble.metainfo.xml"
    DESTINATION share/metainfo)
install(FILES "${CMAKE_SOURCE_DIR}/dist/bubble-cleanup.hook"
    DESTINATION share/libalpm/hooks)
install(FILES "${CMAKE_SOURCE_DIR}/dist/org.bubble.vault.policy"
    DESTINATION share/polkit-1/actions)

