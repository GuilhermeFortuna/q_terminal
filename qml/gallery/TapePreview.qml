import QtQuick
import qml

// Gallery-only: the real TapeView over a TapeModel bound to a deterministic fixture
// session, so the tape's degraded states (Q-082) are captured as they render live.
Rectangle {
    id: root
    property string previewState: "live"

    color: Theme.surfaceBase
    border.color: Theme.borderDefault
    border.width: Theme.borderWidth
    clip: true

    TapeModel {
        id: model
    }

    TapeView {
        anchors.fill: parent
        anchors.margins: Theme.borderWidth
        tape: model
    }

    Component.onCompleted: model.bind_fixture(root.previewState, 30)
}
