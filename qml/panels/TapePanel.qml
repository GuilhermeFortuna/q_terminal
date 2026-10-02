import QtQuick
import qml

// The trade tape panel (Q-082): the session's prints beside the live chart. Each instance
// owns its display filters and reads the one shared trade feed.
PanelFrame {
    id: root
    kind: "tape"

    readonly property alias tape: tapeModel
    function filterJson() {
        return view.filterJson();
    }
    function restoreFilter(json) {
        view.restoreFilter(json);
    }

    TapeModel {
        id: tapeModel
        objectName: "tapeModel"
        Component.onCompleted: tapeModel.bind()
    }

    TapeView {
        id: view
        objectName: "tapeView"
        anchors.fill: parent
        tape: tapeModel
        onFilterEdited: root.shell.scheduleAutosave()
    }
}
