pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Layouts
import qml

// Honest placement mode reporting and compositor rule export (Q-053).
Rectangle {
    id: root

    required property var workspace

    readonly property string mode: workspace.placement_mode
    readonly property bool compositor: root.mode === "compositor"
    readonly property var reports: JSON.parse(workspace.reports_json)

    visible: root.compositor || root.reports.length > 0
    implicitHeight: visible ? content.implicitHeight + Theme.spaceMd * 2 : 0
    color: root.compositor ? Theme.warningSurface : Theme.surfaceRaised
    radius: Theme.radiusMedium
    border.color: Theme.borderSubtle
    border.width: Theme.borderWidth

    ColumnLayout {
        id: content
        anchors.fill: parent
        anchors.margins: Theme.spaceMd
        spacing: Theme.spaceXs

        Text {
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
            visible: root.compositor
            text: "Placement: compositor — panels are restored; window positions follow Hyprland rules. Export rules and merge into ~/.config/hypr/windowrules.conf."
            color: Theme.warningText
            font.family: Theme.uiFont
            font.pixelSize: Theme.typeLabel
        }

        RowLayout {
            visible: root.compositor
            spacing: Theme.spaceSm
            AppButton {
                text: "Export compositor rules"
                onClicked: {
                    var path = workspace.export_compositor_rules();
                    if (path !== "") {
                        exportLabel.text = "Wrote " + path;
                    }
                }
            }
            Text {
                id: exportLabel
                Layout.fillWidth: true
                elide: Text.ElideRight
                color: Theme.textSecondary
                font.family: Theme.uiFont
                font.pixelSize: Theme.typeLabel
            }
        }

        Repeater {
            model: root.reports
            delegate: Text {
                required property var modelData
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                text: modelData.message
                color: Theme.textSecondary
                font.family: Theme.uiFont
                font.pixelSize: Theme.typeLabel
            }
        }
    }
}
