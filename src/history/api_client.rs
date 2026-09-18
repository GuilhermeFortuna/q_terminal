use serde::Deserialize;

use crate::history::catalog::DatasetManifest;

#[derive(Debug, Clone, Deserialize)]
pub struct DatasetListResponse {
    pub root: String,
    pub datasets: Vec<DatasetManifest>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct OhlcvBarResponse {
    timestamp: String,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: i64,
}

#[derive(Debug, Clone)]
pub struct ApiClient {
    client: reqwest::Client,
    base: String,
}

impl ApiClient {
    pub fn new(base: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base: base.trim_end_matches('/').to_string(),
        }
    }

    pub fn base(&self) -> &str {
        &self.base
    }

    pub async fn fetch_catalog(
        &self,
        symbol: &str,
        timeframe: &str,
    ) -> Result<DatasetListResponse, reqwest::Error> {
        let url = format!(
            "{}/api/v1/catalog/datasets?kind=bars&symbol={}&timeframe={}",
            self.base, symbol, timeframe
        );
        let response = self.client.get(url).send().await?;
        response.error_for_status()?.json().await
    }

    pub(crate) async fn fetch_ohlcv(
        &self,
        symbol: &str,
        timeframe: &str,
        count: usize,
    ) -> Result<Vec<OhlcvBarResponse>, reqwest::Error> {
        let capped = count.min(5_000);
        let url = format!(
            "{}/api/v1/market/ohlcv/{}?timeframe={}&count={}",
            self.base, symbol, timeframe, capped
        );
        let response = self.client.get(url).send().await?;
        response.error_for_status()?.json().await
    }
}

pub(crate) fn ohlcv_to_columns(
    bars: &[OhlcvBarResponse],
) -> Result<q_buffers::frame::BarColumns, String> {
    use q_buffers::frame::{BarColumns, TimeLabel};

    let mut time = Vec::with_capacity(bars.len());
    let mut open = Vec::with_capacity(bars.len());
    let mut high = Vec::with_capacity(bars.len());
    let mut low = Vec::with_capacity(bars.len());
    let mut close = Vec::with_capacity(bars.len());
    let mut volume = Vec::with_capacity(bars.len());

    for bar in bars {
        let micros = parse_timestamp_micros(&bar.timestamp)
            .map_err(|_| format!("malformed bar field `timestamp`: {}", bar.timestamp))?;
        time.push(micros);
        open.push(bar.open);
        high.push(bar.high);
        low.push(bar.low);
        close.push(bar.close);
        volume.push(bar.volume);
    }

    Ok(BarColumns {
        time,
        open,
        high,
        low,
        close,
        tick_volume: Some(volume),
        spread: None,
        real_volume: None,
        label: TimeLabel::Utc,
    })
}

fn parse_timestamp_micros(timestamp: &str) -> Result<i64, ()> {
    let trimmed = timestamp.trim();
    let without_tz = trimmed
        .split('+')
        .next()
        .and_then(|value| value.split('Z').next())
        .unwrap_or(trimmed);

    if without_tz.len() < 19 {
        return Err(());
    }

    let year = without_tz[0..4].parse::<i32>().map_err(|_| ())?;
    let month = without_tz[5..7].parse::<u32>().map_err(|_| ())?;
    let day = without_tz[8..10].parse::<u32>().map_err(|_| ())?;
    let hour = without_tz[11..13].parse::<u32>().map_err(|_| ())?;
    let minute = without_tz[14..16].parse::<u32>().map_err(|_| ())?;
    let second = without_tz[17..19].parse::<u32>().map_err(|_| ())?;

    let days = days_since_epoch(year, month, day)?;
    let secs = days as i64 * 86_400 + hour as i64 * 3_600 + minute as i64 * 60 + second as i64;

    let micros = if without_tz.len() > 20 && without_tz.as_bytes()[19] == b'.' {
        let frac = &without_tz[20..];
        let digits = frac.chars().take(6).collect::<String>();
        format!("{:0<6}", digits).parse::<i64>().map_err(|_| ())?
    } else {
        0
    };

    Ok(secs * 1_000_000 + micros)
}

fn days_since_epoch(year: i32, month: u32, day: u32) -> Result<u64, ()> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(());
    }
    let mut y = year as i64;
    let m = month as i64;
    let d = day as i64;
    y -= if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Ok((era * 146097 + doe - 719468) as u64)
}
