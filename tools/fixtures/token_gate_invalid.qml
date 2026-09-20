import QtQuick

Rectangle {
    width: 24
    color: "#ff0000"
    property string session_pnl: "1.00"
    property string doubled: session_pnl * 2
    Text { font.pixelSize: 12 }
}
