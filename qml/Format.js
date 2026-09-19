.pragma library

// Format.js - Pure formatting helpers for Q operations views.
// Crucial constraint: NO arithmetic on money or quantities.
// Values are formatted directly from backend strings/numbers, never derived.

function formatIsoTime(isoStr) {
    if (!isoStr || isoStr === "" || isoStr === "null") {
        return "--";
    }
    try {
        var d = new Date(isoStr);
        if (isNaN(d.getTime())) {
            return String(isoStr);
        }
        var pad = function(n) { return n < 10 ? "0" + n : n; };
        var y = d.getUTCFullYear();
        var m = pad(d.getUTCMonth() + 1);
        var day = pad(d.getUTCDate());
        var hh = pad(d.getUTCHours());
        var mm = pad(d.getUTCMinutes());
        var ss = pad(d.getUTCSeconds());
        return y + "-" + m + "-" + day + " " + hh + ":" + mm + ":" + ss + " UTC";
    } catch (e) {
        return String(isoStr);
    }
}

function formatShortTime(isoStr) {
    if (!isoStr || isoStr === "" || isoStr === "null") {
        return "--";
    }
    try {
        var d = new Date(isoStr);
        if (isNaN(d.getTime())) {
            return String(isoStr);
        }
        var pad = function(n) { return n < 10 ? "0" + n : n; };
        var hh = pad(d.getUTCHours());
        var mm = pad(d.getUTCMinutes());
        var ss = pad(d.getUTCSeconds());
        return hh + ":" + mm + ":" + ss;
    } catch (e) {
        return String(isoStr);
    }
}

function formatAge(seconds) {
    if (seconds === undefined || seconds === null || isNaN(seconds) || seconds < 0) {
        return "--";
    }
    if (seconds < 1.0) {
        return (Math.round(seconds * 10) / 10).toFixed(1) + "s";
    }
    if (seconds < 60) {
        return Math.floor(seconds) + "s";
    }
    var m = Math.floor(seconds / 60);
    var s = Math.floor(seconds % 60);
    if (m < 60) {
        return m + "m " + s + "s";
    }
    var h = Math.floor(m / 60);
    m = m % 60;
    return h + "h " + m + "m";
}

function truncateId(id, len) {
    if (!id || id === "") {
        return "--";
    }
    var str = String(id);
    var maxLen = len || 8;
    if (str.length <= maxLen) {
        return str;
    }
    return str.substring(0, maxLen) + "…";
}

function formatDecimal(val) {
    if (val === undefined || val === null || val === "" || val === "null") {
        return "--";
    }
    return String(val);
}
