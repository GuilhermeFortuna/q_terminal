import QtQuick
import qml

Text {
    id: root
    property string previewState: "rest"
    property bool numeric: false
    text: numeric ? "12345.67" : "Cell value"
    color: ControlState.foreground(previewState)
    font.family: numeric ? Theme.numericFontFamily : Theme.uiFont
    font.pixelSize: Theme.typeBodySmall
    font.features: numeric ? { "tnum": 1 } : {}
    verticalAlignment: Text.AlignVCenter; elide: Text.ElideRight
    width: Spacing.dialogWidth / 4; height: Theme.tableRowHeight
}
