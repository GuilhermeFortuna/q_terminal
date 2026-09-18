#ifndef Q_TERMINAL_CHART_CXX_H
#define Q_TERMINAL_CHART_CXX_H

#include <cstddef>
#include <cstdint>
#include <memory>

#include "rust/cxx.h"

class BarChartItem;
class BarFeed;
class BarSeries;
class QQmlApplicationEngine;

struct ProbeResult;
struct ProbeState;
struct ProbeVertex;
struct ViewportProbeResult;
struct ChartPaneProbeResult;

class ViewportProbe {
public:
    ViewportProbe();
    ~ViewportProbe();

    void set_bars_visible(int count);
    void set_price_margin(double margin);
    void update(int bar_count, double low, double high, int revision);
    ViewportProbeResult result() const;

private:
    struct Impl;
    std::unique_ptr<Impl> m_impl;
};

std::unique_ptr<ViewportProbe> make_viewport_probe();

class ChartPaneProbe {
public:
    ChartPaneProbe();
    ~ChartPaneProbe();

    void set_series(BarSeries* series);
    void set_feed(BarFeed* feed);
    void set_size(float width, float height);
    void set_bars_visible(int count);
    void set_price_margin(double margin);
    ChartPaneProbeResult result() const;

private:
    struct Impl;
    std::unique_ptr<Impl> m_impl;
};

std::unique_ptr<ChartPaneProbe> make_chart_pane_probe();

struct StatusStripProbeResult;
struct EmptyStateProbeResult;

class StatusStripProbe {
public:
    StatusStripProbe();
    ~StatusStripProbe();

    void set_feed(BarFeed* feed);
    StatusStripProbeResult result() const;

private:
    struct Impl;
    std::unique_ptr<Impl> m_impl;
};

std::unique_ptr<StatusStripProbe> make_status_strip_probe();

class EmptyStateProbe {
public:
    EmptyStateProbe();
    ~EmptyStateProbe();

    void set_feed(BarFeed* feed);
    EmptyStateProbeResult result() const;

private:
    struct Impl;
    std::unique_ptr<Impl> m_impl;
};

std::unique_ptr<EmptyStateProbe> make_empty_state_probe();

BarFeed* make_test_feed();
void feed_set_connection_state(BarFeed* feed, rust::Str state);
void feed_set_last_error(BarFeed* feed, rust::Str error);
void feed_set_stale(BarFeed* feed, bool stale);
void feed_set_data_age_ms(BarFeed* feed, std::int64_t ms);
void feed_set_live_only(BarFeed* feed, bool live_only);
void feed_set_history(BarFeed* feed, rust::Str source, std::int64_t shortfall);
void feed_set_bar_count(BarFeed* feed, std::int64_t count);

BarFeed* find_window_feed(QQmlApplicationEngine& engine);
void setup_window_feed(QQmlApplicationEngine& engine, BarFeed* feed);
void setup_window_auto_close(QQmlApplicationEngine& engine, int ms);

void feed_set_symbol(BarFeed* feed, rust::Str symbol);
void feed_set_timeframe(BarFeed* feed, rust::Str timeframe, std::int64_t timeframe_ms);
void feed_setup_and_load(BarFeed* feed, rust::Str api_base, rust::Str symbol, rust::Str timeframe);

void post_feed_stream_state(
    BarFeed* feed,
    rust::Str state,
    rust::Str last_error,
    std::int64_t applied,
    std::int64_t dropped,
    std::int64_t gaps_closed,
    std::int64_t resnapshots,
    std::int64_t rest_calls
);

void post_feed_completed_bar(
    BarFeed* feed,
    std::int64_t time,
    double open,
    double high,
    double low,
    double close
);

void post_feed_forming_bar(
    BarFeed* feed,
    std::int64_t time,
    double open,
    double high,
    double low,
    double close
);

void ensure_application();
int exec_application();
void process_events();
void register_bar_chart_types();

BarSeries* make_test_series(int bar_count);

ProbeResult chart_probe_sync(BarSeries* series, int first_bar, int last_bar, double low,
                             double high, float width_px, float height_px, ProbeState& state);

ProbeResult chart_probe_item_paint(BarChartItem* item, int frames);

BarChartItem* make_test_chart_item(BarSeries* series, int first_bar, int last_bar, double low,
                                   double high, float width_px, float height_px);

void chart_item_set_size(BarChartItem* item, float width_px, float height_px);

void chart_item_apply_view(BarChartItem* item, int first_bar, int last_bar, double low,
                           double high, float width_px, float height_px);

void chart_series_ingest_completed(BarSeries* series, std::int64_t time, double open, double high,
                                   double low, double close, double volume);

double chart_series_low(BarSeries* series);

double chart_series_high(BarSeries* series);

void chart_series_ingest_forming(BarSeries* series, std::int64_t time, double open, double high,
                                 double low, double close, double volume);

void chart_series_rebuild(BarSeries* series);

std::int64_t chart_series_geometry_revision(BarSeries* series);

int chart_series_vertex_len(BarSeries* series);

ProbeVertex chart_series_vertex_at(BarSeries* series, std::size_t index);

void ensure_test_app();

void reset_chart_probe_state();

int feed_bar_times_len(BarFeed* feed);
std::int64_t feed_bar_time_at(BarFeed* feed, int index);
int feed_vertex_len(BarFeed* feed);
ProbeVertex feed_vertex_at(BarFeed* feed, std::size_t index);
void feed_rebuild_geometry(BarFeed* feed, int first_bar, int last_bar, double low, double high, float width, float height);
rust::String feed_history_source(BarFeed* feed);
rust::String feed_history_error(BarFeed* feed);
bool feed_history_loading(BarFeed* feed);
std::int64_t feed_bar_count(BarFeed* feed);
std::int64_t feed_rest_calls(BarFeed* feed);

#endif // Q_TERMINAL_CHART_CXX_H

