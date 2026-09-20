pragma Singleton
import QtQuick

QtObject {
    readonly property string uiFamily: "Inter"
    readonly property string numericFamily: "JetBrains Mono"
    readonly property string tabularFeatures: "tnum"

    readonly property int labelSmall: 9
    readonly property int label: 10
    readonly property int bodySmall: 11
    readonly property int body: 12
    readonly property int bodyLarge: 13
    readonly property int title: 16
    readonly property int heading: 20

    readonly property int weightRegular: Font.Normal
    readonly property int weightMedium: Font.Medium
    readonly property int weightBold: Font.Bold
}
