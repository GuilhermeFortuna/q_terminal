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
    property bool detailsVisible: false

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

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spaceSm
            Text {
                Layout.fillWidth: true
                elide: Text.ElideRight
                text: root.compositor ? "Placement: compositor rules available" : "Workspace placement adjusted"
                color: root.compositor ? Theme.warningText : Theme.textSecondary
                font.family: Theme.uiFont
                font.pixelSize: Theme.typeLabel
            }
            AppButton {
                text: root.detailsVisible ? "Hide details" : "Placement details"
                implicitWidth: Spacing.size140
                Accessible.name: root.detailsVisible ? "Hide placement details" : "Show placement details"
                onClicked: root.detailsVisible = !root.detailsVisible
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            visible: root.detailsVisible
            spacing: Theme.spaceXs

            Text {
                Layout.fillWidth: true
                visible: root.compositor
                wrapMode: Text.WordWrap
                text: "Panels restore in Q Terminal. The compositor, not this application, places windows; export rules only for installation in your compositor configuration."
                color: Theme.textSecondary
                font.family: Theme.uiFont
                font.pixelSize: Theme.typeLabel
            }

            AppButton {
                visible: root.compositor
                text: "Export compositor rules"
                implicitWidth: Spacing.size160
                onClicked: {
                    var path = workspace.export_compositor_rules();
                    if (path !== "") exportLabel.text = "Exported " + path;
                }
            }

            Text {
                id: exportLabel
                Layout.fillWidth: true
                visible: exportLabel.text !== ""
                elide: Text.ElideRight
                color: Theme.textSecondary
                font.family: Theme.uiFont
                font.pixelSize: Theme.typeLabel
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
}
