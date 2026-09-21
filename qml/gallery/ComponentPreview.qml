import QtQuick
import qml

Item {
    id: root
    required property string componentName
    property string previewState: "rest"

    Loader {
        id: previewLoader
        anchors.left: parent.left; anchors.verticalCenter: parent.verticalCenter
        width: Math.min(implicitWidth, parent.width); height: parent.height
        source: "../components/" + root.componentName + ".qml"
        onLoaded: {
            if (item && item.hasOwnProperty("previewState")) item.previewState = root.previewState;
            if (item && item.hasOwnProperty("text")) item.text = root.componentName;
            if (item && item.hasOwnProperty("model")) item.model = ["Paper", "Live"];
        }
    }
}
