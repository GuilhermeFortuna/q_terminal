# Q-036: Catalog-driven historical load

**Status:** authoritative in the [Q project board](https://github.com/users/GuilhermeFortuna/projects/2)  
**Project direction:** [`q_contracts/docs/system-architecture.md` §4.7, §8.1, §10 Phase 3](https://github.com/GuilhermeFortuna/q_contracts/blob/a9724767015905f1e9cd5d98ba492080910db4f2/docs/system-architecture.md#47-persistence-boundary-and-direct-parquet-access)  
**Depends on:** Q-033, Q-035  
**Implementation plan:** [`../plans/Q-036-catalog-driven-historical-load-plan.md`](../plans/Q-036-catalog-driven-historical-load-plan.md)

## Purpose

A live stream starting at the moment of connection is not a chart. The slice
also loads the symbol's recent history, and §4.7 fixes exactly how: the terminal
asks the API for a dataset, receives a manifest, verifies checksums on first
open, and reads the listed files with `q_core` — never listing a directory,
never guessing a path, and falling back to fetching the data from the API when
the manifest is gone or a file fails its digest. The catalog has served
manifests since Q-017 and `q_core` has read Parquet since Q-033, but nothing in
the terminal asks. This task adds the load, its verification, and its fallback,
and hands the result to the same series the stream feeds.

## Requirements

### Selecting a dataset

- The terminal asks the catalog for the bars datasets of its configured symbol
  and timeframe and uses the current published one: the highest version whose
  state is published. A dataset in any other state is never read.
- The lake root the catalog reports is the only base a file path is resolved
  against. A manifest path is used as given, relative to that root.
- A path that escapes the root, that is absolute, or that contains a parent
  segment, is refused before any file is opened, naming the path.
- If the subject has no published dataset, the load reports that plainly and the
  chart shows live bars alone rather than an error.

### Verification

- Before a dataset's files are read for the first time, each is verified: its
  size matches the manifest and its digest under the manifest's algorithm
  matches the manifest's. A mismatch, a missing file, or an algorithm the
  terminal cannot compute, all fail the dataset as a whole.
- A verified dataset is not re-verified on a later load in the same session
  unless its identifier or its published time changed.
- Verification never rewrites, repairs, or deletes a lake file. The terminal
  reads the lake and writes nothing into it.

### Reading

- The load reads the most recent bars of the dataset up to a configured bar
  count, using positional reads so that a large dataset does not have to be read
  whole.
- Bars load into the same series the stream feeds, in time order, before any
  streamed bar is applied, so that history and live data never interleave.
- A bar in the history whose time is at or after the first streamed bar the
  client has applied is dropped in favour of the streamed one, so the seam
  never duplicates or reorders a bar.
- The load runs off the UI thread and reports progress as a proportion of the
  bars it intends to load.

### Fallback

- When the catalog is unreachable, the subject has no manifest, a file is
  missing, or verification fails, the terminal fetches the same bars from the
  API's bars endpoint instead, and marks the loaded history as fetched rather
  than read from the lake.
- The fallback's bar count is whatever that endpoint allows; when it is smaller
  than the configured count, the shortfall is reported rather than hidden.
- The fallback is used only for the reasons above. A slow local read is not one
  of them.
- When neither path yields bars, the chart shows live bars alone, and the reason
  is reported.

### State exposed

- QML sees, as properties that notify on change: whether history is loading, the
  progress, the source of the loaded history drawn from a closed set covering
  lake, API and none, the number of bars loaded, the dataset identifier and
  published time when the source is the lake, and the last failure's reason.
- A load can be requested again; a second request while one is running is
  refused rather than queued, and says so.

### Cost

- Verifying and loading 200,000 bars from the lake completes within five seconds
  on the target machine, and its memory stays bounded by the series plus one
  file's row group.
- The load never blocks the UI thread for longer than one frame at a time.

### Boundaries preserved

- The terminal reads local Parquet only through the catalog. It never lists a
  lake directory, never constructs a path from a symbol and timeframe, and never
  reads a file the manifest does not list.
- Decoding, digesting and reading are `q_core`'s; this task decides what to read
  and what a failure means.
- The terminal starts nothing: an unreachable API is reported and retried, never
  launched.
- Existing checks pass unchanged, including the headless report and the
  contracts check.

## Constraints and non-goals

- **No dataset browser.** Choosing among datasets, inspecting them, or managing
  them is a research surface, and `BOUNDARY.md` forbids it here. The terminal
  resolves one subject.
- **No writes, no caching of lake files.** Verification results may be
  remembered for the session; bytes are not copied anywhere.
- **No Arrow-over-HTTP fallback.** §4.7 describes the fallback as Arrow, but the
  API serves bars as JSON today and no Arrow dataset endpoint exists. The
  fallback uses the JSON endpoint, and adding an Arrow one is a `q_backend`
  decision taken if this path's cost proves to matter, with the measurement
  from this task as its evidence.
- **No tick history.** Bars only, as in the rest of the slice.
- **No time-range selection or paging in the UI.** The load is "the last N
  bars". Panning further back is a later surface with its own task.
- **No chart rendering.** Q-037 draws what this task loads.
- **No manifest schema change.** The terminal consumes what the catalog
  publishes today.

## Acceptance criteria

### Agent-verifiable

1. Selection tests state the rules as cases: among three versions the highest
   published one is chosen; a tombstoned or publishing dataset is never chosen;
   no published dataset reports the empty case without an error; an absolute
   path, a parent segment and a path escaping the root are each refused naming
   the path, before any open.
2. Verification tests state the rules as cases: a matching size and digest
   passes; a byte flipped in a file fails the dataset; a size mismatch fails; a
   missing file fails; an unsupported algorithm fails; a second load of the same
   dataset does not re-digest; a changed identifier or published time does.
3. Seam tests: history loads before any streamed bar applies; a history bar at
   or after the first applied streamed bar is dropped; the resulting series is
   strictly ascending with no duplicate time.
4. Fallback tests, against a fake API: an unreachable catalog, an empty catalog,
   a missing file and a failed digest each fall back and mark the source; a
   short fallback reports the shortfall; both paths failing leaves the chart
   live-only with a reason.
5. A test asserts the load runs off the UI thread, reports monotonic progress,
   and that a second concurrent request is refused with a reason.
6. A measurement test reports wall time and peak memory for a 200,000-bar
   verified load from a fixture lake.
7. The full validation suite passes: `env -u WAYLAND_DISPLAY -u DISPLAY make check`.

### Human-verifiable

1. Against the real lake and the running API, the configured symbol's history
   loads; the dataset identifier, bar count, verification time and total load
   time are reported, and the first and last bar times are checked against the
   same range read through the backend.
   Command: `./research` in the workspace, then `cd q_terminal && make run`
2. A lake file is corrupted in a scratch copy of the dataset and the terminal is
   confirmed to fail the dataset, fall back to the API, and say so.
3. The catalog is made unreachable and the terminal is confirmed to fall back
   and report it, with the fallback's bar count and the shortfall against the
   configured count.
