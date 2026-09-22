.pragma library

var commonStates = ["rest", "hover", "pressed", "focused", "selected", "disabled"]
var entries = [
    { name: "Panel", states: commonStates },
    { name: "SectionHeader", states: commonStates },
    { name: "Toolbar", states: commonStates },
    { name: "AppButton", states: commonStates },
    { name: "IconButton", states: commonStates },
    { name: "StatusBadge", states: ["rest", "selected", "disabled", "degraded", "stale"] },
    { name: "ConnectionIndicator", states: ["rest", "disabled", "degraded", "stale"] },
    { name: "Metric", states: ["rest", "selected", "disabled", "degraded", "stale"] },
    { name: "DataTable", states: commonStates },
    { name: "DataTableHeader", states: commonStates },
    { name: "DataTableRow", states: commonStates },
    { name: "DataTableCell", states: commonStates },
    { name: "TabBar", states: commonStates },
    { name: "SplitPane", states: ["rest", "hover", "pressed", "focused", "disabled"] },
    { name: "EmptyState", states: ["rest", "disabled", "degraded"] },
    { name: "AppDialog", states: ["rest", "focused", "disabled", "degraded"] },
    { name: "AppTextField", states: commonStates },
    { name: "AppComboBox", states: commonStates },
    { name: "ChartIdentity", states: ["rest", "switching", "loading", "stale", "disconnected"] },
    { name: "ChartTargetPicker", states: ["open", "manual", "following", "invalid", "no-data"] }
]
