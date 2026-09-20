.pragma library

var commonStates = ["rest", "hover", "pressed", "focused", "selected", "disabled"]
var entries = [
    { name: "Panel", states: ["rest", "focused", "selected", "disabled"] },
    { name: "SectionHeader", states: ["rest", "focused", "disabled"] },
    { name: "Toolbar", states: ["rest", "focused", "disabled"] },
    { name: "AppButton", states: commonStates },
    { name: "IconButton", states: commonStates },
    { name: "StatusBadge", states: ["rest", "selected", "disabled", "degraded", "stale"] },
    { name: "ConnectionIndicator", states: ["rest", "disabled", "degraded", "stale"] },
    { name: "Metric", states: ["rest", "selected", "disabled", "degraded", "stale"] },
    { name: "DataTable", states: ["rest", "focused", "selected", "disabled"] },
    { name: "DataTableHeader", states: ["rest", "focused", "disabled"] },
    { name: "DataTableRow", states: commonStates },
    { name: "DataTableCell", states: ["rest", "selected", "disabled"] },
    { name: "TabBar", states: commonStates },
    { name: "SplitPane", states: ["rest", "hover", "pressed", "focused", "disabled"] },
    { name: "EmptyState", states: ["rest", "disabled", "degraded"] },
    { name: "AppDialog", states: ["rest", "focused", "disabled", "degraded"] },
    { name: "AppTextField", states: commonStates },
    { name: "AppComboBox", states: commonStates }
]
