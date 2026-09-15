import QtQuick
import Bubble

// Toast notification container – anchored bottom-right of its parent.
// Usage: toast.show("message", "info"|"error"|"success")
Item {
    id: root
    Accessible.role: Accessible.AlertMessage
    Accessible.name: "Notifications"

    // Size to the column of toasts
    implicitWidth: toastColumn.implicitWidth
    implicitHeight: toastColumn.implicitHeight

    z: 200

    Component { id: toastIconError; IconAlertCircle { size: 16; color: Theme.error } }
    Component { id: toastIconSuccess; IconCheck { size: 16; color: Theme.success } }
    Component { id: toastIconInfo; IconInfo { size: 16; color: Theme.accent } }

    ListModel {
        id: toastModel
    }

    Column {
        id: toastColumn
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        spacing: 8

        Repeater {
            model: toastModel

            delegate: Item {
                id: toastItem
                implicitWidth: toastRect.implicitWidth
                implicitHeight: toastRect.implicitHeight
                opacity: 1.0

                required property string message
                required property string toastType
                required property int toastIndex

                Rectangle {
                    id: toastRect
                    implicitWidth: Math.max(240, toastRow.implicitWidth + 32)
                    implicitHeight: toastRow.implicitHeight + 20
                    radius: Theme.radiusMedium
                    color: Theme.overlayBackground

                    border.width: 2
                    border.color: toastItem.toastType === "error"   ? Theme.error
                                : toastItem.toastType === "success" ? Theme.success
                                : Theme.accent

                    Row {
                        id: toastRow
                        anchors.centerIn: parent
                        spacing: 8

                        Loader {
                            anchors.verticalCenter: parent.verticalCenter
                            sourceComponent: {
                                if (toastItem.toastType === "error") return toastIconError
                                if (toastItem.toastType === "success") return toastIconSuccess
                                return toastIconInfo
                            }
                        }

                        Text {
                            id: toastText
                            anchors.verticalCenter: parent.verticalCenter
                            textFormat: Text.PlainText
                            text: toastItem.message
                            color: Theme.text
                            font.pointSize: Theme.fontNormal
                            wrapMode: Text.WordWrap
                            width: Math.min(implicitWidth, 320)
                        }
                    }
                }

                Timer {
                    id: dismissTimer
                    // Errors stay longer: a config parse error at startup is easy to miss in 3 s.
                    interval: toastItem.toastType === "error" ? 8000 : 3000
                    running: true
                    onTriggered: fadeOut.start()
                }

                SequentialAnimation {
                    id: fadeOut
                    NumberAnimation {
                        target: toastItem
                        property: "opacity"
                        to: 0
                        duration: 300
                        easing.type: Theme.animEasingExit; easing.bezierCurve: Theme.animBezierCurve
                    }
                    ScriptAction {
                        script: {
                            // Find and remove by toastIndex
                            for (var i = 0; i < toastModel.count; i++) {
                                if (toastModel.get(i).toastIndex === toastItem.toastIndex) {
                                    toastModel.remove(i)
                                    break
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Counter to give each toast a unique id
    property int _nextIndex: 0

    function show(message, type) {
        toastModel.append({
            message:    message,
            toastType:  type || "info",
            toastIndex: _nextIndex
        })
        _nextIndex++
    }
}
