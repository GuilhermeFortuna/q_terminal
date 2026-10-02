.pragma library

// Steps a study's editable parameter one notch up (+1) or down (-1) through the feed, which
// validates it. Price studies step their period; the trade rate steps its window by a second
// and large prints step their threshold by a tenth. Delta and cumulative delta have no
// parameter to edit.
function editable(study) {
    return !study.volume || study.kind === "trade_rate" || study.kind === "large_prints";
}

function step(feed, study, direction) {
    if (!feed || !editable(study)) {
        return false;
    }
    if (!study.volume) {
        var period = Math.max(1, study.period + direction);
        return period === study.period ? false : feed.update_study(study.id, period, study.source, study.num_std);
    }
    var windowMs = study.window_ms;
    var threshold = study.large_print_threshold;
    if (study.kind === "trade_rate") {
        windowMs = Math.max(1000, Math.min(300000, windowMs + direction * 1000));
    } else {
        threshold = Math.max(1, threshold + direction * Math.max(1, Math.round(threshold / 10)));
    }
    return feed.update_volume_study(study.id, windowMs, threshold);
}
