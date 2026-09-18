pragma ComponentBehavior: Bound
import QtQuick
import qml

Window {
    id: window
    visible: true
    width: 1024
    height: 768
    title: "q_terminal"
    color: "#131722"

    property var feed: null

    BarFeed {
        id: defaultFeed
    }

    readonly property var activeFeed: window.feed ? window.feed : defaultFeed

    AppInfo {
        id: appInfo
        objectName: "appInfo"
    }

    function formatBarTime(timestamp) {
        if (!timestamp || timestamp <= 0) {
            return "--";
        }
        var ms = timestamp < 10000000000 ? timestamp * 1000 : timestamp;
        var d = new Date(ms);
        return d.toISOString().replace("T", " ").replace(".000Z", " UTC");
    }

    Rectangle {
        id: header
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        height: 42
        color: "#1e222d"
        z: 2

        Row {
            anchors.left: parent.left
            anchors.leftMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            spacing: 16

            Text {
                id: symbolText
                objectName: "headerSymbol"
                text: window.activeFeed ? window.activeFeed.symbol : ""
                color: "#d1d4dc"
                font.pixelSize: 15
                font.bold: true
                anchors.verticalCenter: parent.verticalCenter
            }

            Rectangle {
                width: 1
                height: 16
                color: "#2a2e39"
                anchors.verticalCenter: parent.verticalCenter
            }

            Text {
                id: timeframeText
                objectName: "headerTimeframe"
                text: window.activeFeed ? window.activeFeed.timeframe : ""
                color: "#787b86"
                font.pixelSize: 13
                font.bold: true
                anchors.verticalCenter: parent.verticalCenter
            }

            Rectangle {
                width: 1
                height: 16
                color: "#2a2e39"
                anchors.verticalCenter: parent.verticalCenter
            }

            Row {
                anchors.verticalCenter: parent.verticalCenter
                spacing: 6

                Text {
                    text: "Last:"
                    color: "#787b86"
                    font.pixelSize: 12
                    anchors.verticalCenter: parent.verticalCenter
                }

                Text {
                    id: priceText
                    objectName: "headerPrice"
                    text: window.activeFeed && window.activeFeed.last_price > 0
                        ? Number(window.activeFeed.last_price).toFixed(2)
                        : "--"
                    color: "#089981"
                    font.pixelSize: 14
                    font.bold: true
                    anchors.verticalCenter: parent.verticalCenter
                }
            }

            Rectangle {
                width: 1
                height: 16
                color: "#2a2e39"
                anchors.verticalCenter: parent.verticalCenter
            }

            Row {
                anchors.verticalCenter: parent.verticalCenter
                spacing: 6

                Text {
                    text: "Time:"
                    color: "#787b86"
                    font.pixelSize: 12
                    anchors.verticalCenter: parent.verticalCenter
                }

                Text {
                    id: lastTimeText
                    objectName: "headerLastTime"
                    text: window.formatBarTime(window.activeFeed ? window.activeFeed.last_time : 0)
                    color: "#d1d4dc"
                    font.pixelSize: 12
                    anchors.verticalCenter: parent.verticalCenter
                }
            }
        }

        Row {
            anchors.right: parent.right
            anchors.rightMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            spacing: 8

            Text {
                text: "v" + appInfo.app_version + " | core " + appInfo.core_version + " | " + appInfo.contracts_rev + " | " + appInfo.render_backend
                color: "#50535e"
                font.pixelSize: 11
                anchors.verticalCenter: parent.verticalCenter
            }
        }
    }

    ChartPane {
        id: chartPane
        objectName: "chartPane"
        anchors.top: header.bottom
        anchors.bottom: statusStrip.top
        anchors.left: parent.left
        anchors.right: parent.right
        feed: window.activeFeed
    }

    StatusStrip {
        id: statusStrip
        objectName: "statusStrip"
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        height: 26
        feed: window.activeFeed
    }
}
