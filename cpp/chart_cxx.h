#ifndef Q_TERMINAL_CHART_CXX_H
#define Q_TERMINAL_CHART_CXX_H

#include <cstddef>
#include <cstdint>

#include <memory>

class BarChartItem;
class BarFeed;
class BarSeries;

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

#endif // Q_TERMINAL_CHART_CXX_H

