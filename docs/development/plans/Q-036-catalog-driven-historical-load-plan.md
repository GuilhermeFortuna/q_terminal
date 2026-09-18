# Q-036 implementation plan: Catalog-driven historical load

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Specification:** [`../specs/Q-036-catalog-driven-historical-load-spec.md`](../specs/Q-036-catalog-driven-historical-load-spec.md)  
**Depends on:** Q-033, Q-035

## Current-system context

`contracts/catalog.rs` is the vendored `DatasetManifest`: `dataset_id`,
`subject`, `version`, `supersedes`, `state`, `published_at`,
`checksum_algorithm`, `files`, `arrow_schema`, `row_count`, `time_range`,
`tombstone`. Generation types `subject`, `files`, `time_range` and
`arrow_schema` as `serde_json::Value`, so this task destructures them itself
against `schema/catalog/dataset-manifest.schema.json`, where a file entry is
`{path, size_bytes, checksum}`, `path` is lake-relative and the schema's own
pattern already rejects leading separators and `..` segments, `state` is one of
`publishing | published | tombstoned | deleted`, and `checksum_algorithm` is one
of `sha256 | sha512 | blake3 | md5`.

`GET /api/v1/catalog/datasets?kind&symbol&timeframe` answers
`DatasetListResponse {root, datasets}` — `root` is the lake root and the only
base a path may be resolved against; the per-dataset route
`GET /api/v1/catalog/datasets/{dataset_id}` answers a manifest without a root.
Both answer `503` when the catalog is unavailable.
`q_backend/src/q_backend/market_data/catalog/repository.py:107` publishes
`checksum_algorithm="sha256"` and nothing else, which is the one algorithm
Q-033's `DigestAlgorithm` implements.

The fallback endpoint is `GET /api/v1/market/ohlcv/{symbol}` with `timeframe`
and `count` (default 500, maximum 5,000) or a `start`/`end` pair, answering a
JSON array of `OhlcvBarResponse`. There is no Arrow dataset endpoint anywhere in
`openapi.yaml`, which is why the spec's fallback is JSON and bounded at 5,000
bars.

From Q-033, `q-io` offers `read_bar_files(paths, range)`, `bar_file_rows(path)`,
`digest_file(path, algorithm)` and `DigestAlgorithm::parse`. From Q-034,
`LiveBarSeries::load_history` replaces the series atomically and refuses an
unordered batch. From Q-035, this repository has `Config`, a tokio runtime
thread, `reqwest`, the `BarFeed` QObject owning the series, and the rule that
deliveries reach the UI thread through a queued closure.

`BOUNDARY.md` §2 forbids a dataset catalog explorer in this repository, and §3
forbids starting any process. `make check` runs
`fmt-check lint build qml-lint test contracts-check` and the headless report.

The gap is that the terminal has a live tail and no history behind it.

## Interfaces produced

```rust
// src/history/manifest.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestFile { pub path: String, pub size_bytes: u64, pub checksum: String }

/// A published manifest, destructured and validated out of the vendored type.
#[derive(Debug, Clone, PartialEq)]
pub struct Dataset {
    pub id: String, pub version: i64, pub published_at: String,
    pub algorithm: DigestAlgorithm, pub files: Vec<ManifestFile>, pub row_count: usize,
}

pub enum ManifestError { NotPublished, BadPath { path: String }, UnsupportedDigest { name: String }, Shape { field: &'static str } }

/// Picks the current dataset: highest `version` with `state == "published"`.
pub fn select_current(datasets: &[DatasetManifest]) -> Result<Option<Dataset>, ManifestError>;

/// Joins a lake-relative path onto the catalog's root, refusing an absolute
/// path, a parent segment, or any result outside the root.
pub fn resolve(root: &Path, path: &str) -> Result<PathBuf, ManifestError>;

// src/history/verify.rs
pub enum Verdict { Verified, SizeMismatch { path: String }, DigestMismatch { path: String }, Missing { path: String } }

/// Sizes then digests every file. Remembered per (dataset_id, published_at) for
/// the session; a change in either re-verifies.
pub fn verify(dataset: &Dataset, root: &Path, cache: &mut VerifiedSet) -> Verdict;

// src/history/load.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source { Lake, Api, None }

pub struct Loaded { pub bars: BarColumns, pub source: Source, pub dataset: Option<Dataset>, pub shortfall: usize }

/// Reads the last `bars` rows of the dataset positionally, via `bar_file_rows`
/// and one `read_bar_files` call with a row range.
pub async fn load_from_lake(dataset: &Dataset, root: &Path, bars: usize) -> Result<BarColumns, LoadError>;

/// The JSON bars endpoint, capped by the API at 5,000.
pub async fn load_from_api(api: &ApiClient, symbol: &str, timeframe: &str, bars: usize)
    -> Result<(BarColumns, usize /* shortfall */), LoadError>;

/// Selection, verification, read, and the fallback policy in one place. The only
/// reasons it falls back are: catalog unreachable, no published dataset, file
/// missing, verification failed, unsupported digest.
pub async fn load(api: &ApiClient, config: &Config, bars: usize) -> Loaded;

// src/history/seam.rs
/// Drops history bars at or after `first_streamed_time`, so history and live
/// never duplicate or reorder a bar.
pub fn trim_to_seam(bars: BarColumns, first_streamed_time: Option<i64>) -> BarColumns;

// src/bridge/bar_feed.rs  (extended)
// Added QML-exposed properties:
//   history_loading (bool); history_progress (f64 in 0..=1);
//   history_source, history_dataset_id, history_published_at, history_error (QString);
//   history_bars, history_shortfall (i64)
// Added invokable:
//   load_history()   // refused, with a reason, while one is running
```

## Implementation decisions

- **Selection is the highest published version, computed by the terminal from
  the list response, not asked for by name.** The catalog has no "current"
  route, and `supersedes` chains would need a traversal that a version
  comparison already gives. Refusing every non-published state is what keeps a
  half-written `publishing` dataset from being read.

- **`resolve` re-checks what the schema already constrains.** The manifest's
  path pattern rejects absolute and `..` paths, but the terminal receives the
  manifest over the network from a service that could be misconfigured, and
  §4.7's guarantee is the reader's, not the publisher's. The check is
  canonicalisation-free — a prefix test on normalised components — so it does
  not depend on the path existing.

- **Verification is size-then-digest, and one failure fails the dataset.** A
  partially verified dataset has no honest rendering: the bars that failed are
  exactly the ones whose absence would be invisible on a chart. §4.7's tombstone
  grace period means a vanished file signals a lifecycle race, which the
  fallback handles correctly and a partial read would not.

- **Verification results are remembered by `(dataset_id, published_at)` for the
  session only.** Datasets are immutable once published, so re-digesting on
  every reload would spend seconds proving what cannot have changed; keying on
  the published time as well means a republished identifier cannot reuse a
  verdict, and holding it only in memory means a corrupted file is caught again
  after a restart.

- **The read is positional, through `bar_file_rows` then one ranged
  `read_bar_files`.** The catalog selects files for a window, but the slice
  wants "the last N bars", and a time filter in the terminal would duplicate
  selection logic §4.7 assigns to the backend. Summing footer row counts is
  cheap and turns the request into the row range Q-033 already implements.

- **The seam is resolved by dropping history, never streamed bars.** A streamed
  bar has been applied to the series and may already be drawn; removing it would
  make the chart flicker backwards. History is the redundant copy, so it yields.

- **History is loaded before the stream's first apply, and the load path holds
  the series' history slot until it finishes.** `LiveBarSeries::load_history`
  replaces the series wholesale, so a load completing after live bars had been
  applied would erase them. Ordering the two is simpler and more honest than
  merging them, and matches §4.2's snapshot-then-delta shape.

- **Fallback triggers are an explicit closed set, and slowness is not one.**
  A timeout-triggered fallback would mean a healthy lake read on a busy machine
  silently degrades to 5,000 JSON bars, and the surface would show a truncated
  chart with no fault to point at.

- **Progress is reported in bars intended, not bytes or files.** It is the only
  unit the surface can render honestly, and it stays monotonic across the
  verify, read and apply phases by weighting them once at the start.

## Ordered implementation

- [x] 1. Work on the branch `Q-036-catalog-driven-historical-load` in
   `q_terminal`, created from `development` by `./work start`. Confirm Q-035 has
   merged, that `CONTRACTS_REV` is unchanged, and that
   `env -u WAYLAND_DISPLAY -u DISPLAY make check` passes before changing
   anything.
- [x] 2. Write failing tests in `history/manifest.rs`: among versions 1, 2 and 3
   with states published, published and tombstoned, version 2 is chosen; an
   all-tombstoned list gives `None`; a `publishing` dataset is never chosen; a
   manifest missing a required field gives `Shape` naming it; `blake3` gives
   `UnsupportedDigest`; `resolve` refuses `/etc/passwd`, `../../etc/passwd` and
   `a/../../b`, and accepts `bars/WINFUT/M1/part-0.parquet`. Implement. Confirm
   they pass. Commit.
- [x] 3. Write failing tests in `history/verify.rs` over a temporary lake: a
   correct dataset verifies; one flipped byte gives `DigestMismatch` naming the
   file; a truncated file gives `SizeMismatch`; a deleted file gives `Missing`;
   a second verify of the same dataset digests nothing; a changed
   `published_at` digests again. Implement. Confirm they pass. Commit.
- [x] 4. Write failing tests in `history/load.rs` for the lake path: the last
   1,000 rows of a three-file dataset are exactly the last 1,000 in listed
   order; a request larger than the dataset returns all of it; an empty dataset
   returns nothing. Implement `load_from_lake` over `q-io`. Confirm they pass.
   Commit.
- [x] 5. Write failing tests for the API path against a fake server: 500 bars
   parse into ascending columns; a request for 10,000 returns 5,000 with a
   shortfall of 5,000; a malformed bar is an error naming the field. Implement
   `load_from_api`. Confirm they pass. Commit.
- [x] 6. Write failing tests for the fallback policy: catalog `503`, empty
   catalog, missing file, digest mismatch and unsupported algorithm each fall
   back and set `Source::Api`; both paths failing gives `Source::None` with a
   reason; a successful lake read never calls the API. Implement `load`.
   Confirm they pass. Commit.
- [x] 7. Write failing tests in `history/seam.rs`: with a first streamed time,
   history at or after it is dropped; with none, history passes through; the
   result is strictly ascending. Implement. Confirm they pass. Commit.
- [x] 8. Extend `BarFeed` with the history properties and `load_history()`.
   Write failing tests: progress is monotonic and ends at 1.0; a second
   concurrent call is refused with a reason; the load runs off the UI thread;
   the series holds history before the first streamed apply. Implement. Confirm
   they pass. Commit.
- [x] 9. Add `bench-history-load` to the `Makefile`: verify and load 200,000
   bars from a generated fixture lake, reporting verify time, read time, total
   time and peak RSS. Commit.
- [x] 10. Run `env -u WAYLAND_DISPLAY -u DISPLAY make check`. Fix, re-run,
   commit.
- [ ] 11. **Human:** against the real lake and API, load the configured symbol's
   history; corrupt a file in a scratch copy and confirm the fallback; make the
   catalog unreachable and confirm the fallback and the reported shortfall.

## Validation

- **Unit:** selection and path resolution; verification verdicts and the
  session cache; positional lake reads; API parsing and shortfall; the fallback
  policy's closed trigger set; the seam.
- **Integration:** `BarFeed` end to end over a fixture lake and a fake API,
  including the history-then-live ordering.
- **Regression:** Q-035's protocol and fault tests unchanged; the headless
  report; `make contracts-check`; `qml-lint`.
- **Threading:** the load off the UI thread; monotonic progress; concurrent
  request refused.
- **Measurement:** verify, read and total time plus peak RSS at 200,000 bars.
- **Manual:** real lake load; corrupted file; unreachable catalog.

```bash
cd /home/gui/projects/q/q_terminal
env -u WAYLAND_DISPLAY -u DISPLAY make check
cargo test --test history_load
cargo test --test history_fallback
make bench-history-load

# human (step 11)
cd /home/gui/projects/q && ./research
cd /home/gui/projects/q/q_terminal && make run
```

## Handoff

Report the dataset identifier, version, file count, row count and algorithm used
in the real load, with verify, read and total times and peak RSS. Report the
first and last bar times against the backend's read of the same range. Report
what the corrupted file and the unreachable catalog produced, including the
fallback's bar count and shortfall, and give the measured JSON fallback cost as
the evidence for or against a later Arrow dataset endpoint in `q_backend`.
State explicitly that no directory was listed, nothing was written to the lake,
and no dataset browsing surface landed.
