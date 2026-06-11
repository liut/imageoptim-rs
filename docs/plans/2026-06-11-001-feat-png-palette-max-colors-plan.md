---
title: Add `--max-colors` for PNG lossy palette
type: feat
status: active
date: 2026-06-11
---

# Add `--max-colors` for PNG lossy palette

## Overview

Add a CLI flag that bounds the number of colors the imagequant palette
quantizer may produce on the `--lossy` PNG path. The current pipeline
hard-codes the imagequant default of 256 colors. The new flag exposes
that knob so users can trade image fidelity for smaller files without
touching the lossless path.

## Problem Frame

`--lossy` today always requests up to 256 palette entries from
imagequant. The imagequant crate exposes `Attributes::set_max_colors`
as the one-line way to cap that ceiling, but the surrounding plumbing
(CLI parsing, validation, test coverage) is what this plan covers.

Scope is intentionally tight: one new CLI flag, one new pipeline
parameter, one setter call. No new optimization stages, no quality
rework, no change to other formats.

## User-facing behavior

| Invocation | Result |
|---|---|
| `imageoptim --lossy foo.png` | Current behavior: up to 256 colors. |
| `imageoptim --lossy --max-colors 64 foo.png` | imagequant capped at 64 entries. |
| `imageoptim --max-colors 64 foo.png` | Error: `--max-colors` requires `--lossy`. Exit code 1. |
| `imageoptim --lossy --max-colors 1 foo.png` | Parse error from clap (range 2..=256). Exit code 1. |
| `imageoptim --lossy --max-colors 257 foo.png` | Parse error from clap (range 2..=256). Exit code 1. |
| `imageoptim --lossy --max-colors 2 foo.png` | Accepted; may produce severe banding. Safety contract still applies (output must be smaller and decode-valid, otherwise the file is skipped — not a failure). |
| `imageoptim --max-colors 64 foo.jpg` (non-PNG) | Cross-flag error fires before format dispatch (the check is on `--lossy`, not on format). Exit code 1. |

## Scope Boundaries

### In scope

- New CLI flag `--max-colors <N>` on `imageoptim`.
- Range check at parse time: `2..=256`.
- Run-time cross-flag check: `--max-colors` requires `--lossy`.
- Plumbing the value through `cli::Args` → `pipeline::run` → `optimize_file` → `Optimizer::optimize` → `PngOptimizer::optimize_lossy`.
- Calling `Attributes::set_max_colors(N)` when the value is `Some(N)`. When `None`, the imagequant default of 256 is used (today's behavior).
- Tests: unit + integration.
- README sync (one paragraph + flag-table row + benchmark footnote).
- Plan deviations note in `docs/plans/2026-06-09-001-...` if the doc-comment / help-text drift is material (see U6).
- **No `Cargo.toml` changes are required.** `clap::value_parser` ships with the existing `clap = "4.5"` derive feature; `imagequant::Attributes::set_max_colors` ships with the existing `imagequant = "4.4"` dep. This plan adds zero dependencies.

### Out of scope

- A `--min-colors` lower bound. imagequant already enforces a minimum
  internally; exposing it as a separate flag is speculative.
- Changing the JPEG / WebP / GIF / SVG optimizers. None of them go
  through the imagequant palette path. They will receive the new
  `Optimizer` argument and ignore it (mirroring the existing
  precedent: `quality: Option<u8>` is already ignored by PNG).
- Auto-fallback when the requested palette size is "too small" for
  the image. The safety contract (smaller + decodes-valid) is the
  only fallback. Callers who want a different policy should pick a
  larger `N`.
- Exposing per-format advanced imagequant knobs (speed, dithering,
  posterization). The user asked for a single knob.

### Deferred to follow-up work

- README benchmark row showing the file-size effect at `N=16, 64, 256`
  for the existing `tests/example01.png`. Implementation may
  optionally capture one snapshot for the docs; not part of the
  feature itself.
- `--min-colors` if any user actually wants a lower bound.

## Key Technical Decisions

1. **Add `max_colors: Option<u8>` to the `Optimizer` trait**, not a
   separate `optimize_with_options` method or a configuration struct.
   The existing 4-arg trait already has precedent for a
   format-specific knob (`quality: Option<u8>`) that other formats
    ignore with `_quality`. Adding a 5th arg keeps the shape uniform
    and is the smallest, most direct change. ~14 invocation sites
    need updating (1 in `pipeline.rs` + 13 in tests); 5 impl
    signatures grow by one ignored arg; 1 trait declaration grows by
    one arg. None of them grow in complexity, they just gain an
    ignored parameter.

2. **Range check at parse time via `clap::value_parser!(u8).range(1..=256)`**,
   not a run-time `AppError`. This is the first `value_parser` use in
   the repo (current style for `--quality` and `--jobs` is to type
   the field as `Option<u8>` and trust the doc-string), but the
   run-time alternative would re-implement what clap already does
   well, and clap's error message is more informative. Range
   validation that clap can express stays in clap; the cross-flag
   check (which clap cannot express) goes to run-time.

3. **`--max-colors` requires `--lossy` is a run-time check** (new
   `AppError::MaxColorsRequiresLossy` variant). clap cannot express
   cross-flag dependencies. The error message names the missing
   flag, mirrors the style of `NoInput` / `NoMatches`.

4. **Default behavior unchanged when the flag is absent.** The
   `Option<u8>` is `None`, the PNG path does not call
   `set_max_colors`, and imagequant uses its 256-color default. This
   keeps the README benchmark (14.86% on `tests/example01.png`)
   valid without re-measurement.

5. **`Attr::set_max_colors` is called only when the user passed
   the flag.** Calling it unconditionally with the default value
   would be a no-op semantically, but the codebase has a clear
   "explicit opt-in" pattern for all other setters; respect it.

## Alternatives Considered

The plan as written picks the smallest-touch path. Three other
approaches were considered and dismissed; recording the dismissal
so future readers do not re-litigate.

1. **Refactor the `Optimizer` trait into an `OptimizeOptions`
   struct** to avoid the 5-arg method shape. Defensible at this
   scale (4 ignored-arg fields on non-PNG impls is genuinely
   awkward), but it is a non-trivial cross-cutting change that
   touches all five optimizers, the trait, every call site in the
   pipeline, and every test invocation — a much larger diff with
   risk of regression in unchanged code paths. The 5-arg trait
   plus the existing `#[allow(clippy::too_many_arguments)]` on
   `optimize_file` is the right line for this feature. The
   refactor is the natural follow-up if/when a sixth ignored
   argument is added.

2. **Name the flag `--palette-size` instead of `--max-colors`.**
   `--max-colors` mirrors the imagequant crate's `set_max_colors`
   API one-to-one, which is the implementer's reference and the
   most common terminology in the pngquant / imagequant ecosystem
   (`pngquant --colors=64` is the canonical CLI form too).
   `--palette-size` is slightly more user-facing but introduces a
   synonym that will puzzle anyone who has read the imagequant
   docs. Not worth the friction.

3. **Bundle into a `--png-quality` flag** that exposes
   `set_quality`, `set_max_colors`, `set_speed`, and a few other
   knobs as one combined value. Speculative; users have not asked
   for the other knobs. Building each knob behind its own flag
   (as this plan does for `max_colors`) is the right granularity,
   and a future `--png-quality` can be added as a higher-level
   shorthand if the demand materializes.

---

## System-Wide Impact

Three surfaces:

1. **CLI surface** — new flag. Affects end users directly.
   Documented in `--help` via the existing `///` doc comment
   convention. Documented in the README flag table.

2. **Trait / pipeline surface** — touches `src/optimize/mod.rs`,
   `src/optimize/{png,jpeg,gif,webp,svg}.rs`, and `src/pipeline.rs`.
   The 5th trait argument is a strict widening; the contract
   (per-format optimizers may ignore any arg) is preserved.

3. **Test surface** — 13 existing test invocations of
   `optimizer.optimize(...)` need a 5th argument. Plus new tests
   for the new behavior. Mechanical change, no test logic changes
   beyond the new ones.

No persistence, no migration, no concurrency, no format contract
changes, no new external command requirements. No CI changes (no
`.github/` directory exists in this repo today).

## Implementation Units

### U1. Add the CLI flag and range validation

**Goal:** Expose `--max-colors <N>` on `imageoptim`, range-validated
by clap at parse time.

**Requirements:** Range 2..=256 inclusive. Default `None` (no
behavior change). Long-form only (no `-c` short flag — `c` collides
with nothing, but symmetry with `--quality` `-q` and `--jobs` `-j`
suggests reserving the single-letter namespace). The lower bound
is 2, not 1, because `imagequant::Attributes::set_max_colors`
rejects `1` at run time; clap mirrors the lower bound so users
get a clear parse-time error instead of a deferred run-time
error.

**Dependencies:** none.

**Files:** `src/cli.rs`.

**Approach:** Add a new field to `Args` (clap derive) using
`#[arg(long, value_name = "N", value_parser =
clap::value_parser!(u8).range(2..=256))]`. The field type is
`Option<u8>`. The doc-comment (`///`) explains that the flag requires
`--lossy` and is silently a no-op for other formats — this is the
single source of truth for `--help` text.

**Test scenarios:**
- `imageoptim --help` shows the flag with the help text and the
  `2..=256` range implication.
- `imageoptim --max-colors 1 foo.png` exits non-zero with a clap
  error mentioning the range.
- `imageoptim --max-colors 0 foo.png` exits non-zero with a clap
  error mentioning the range.
- `imageoptim --max-colors 257 foo.png` exits non-zero with a clap
  error mentioning the range.
- `imageoptim --max-colors 64 foo.png` (no `--lossy`) is rejected
  with the cross-flag error (this is a U2 scenario but should
  exercise the flag's parse-time acceptance first).
- `imageoptim --max-colors 64 --lossy foo.png` parses successfully
  and proceeds to the optimization.

**Verification:** `cargo run -- --max-colors 0` exits 1 with
clap's standard "invalid value" error. `cargo run -- --max-colors 64
tests/example01.png --lossy` produces a real file-size result.
`cargo test` passes existing 34 tests.

### U2. Cross-flag validation in `pipeline::run`

**Goal:** Reject `--max-colors` when `--lossy` is not also set, with
a clear error message and exit code 1.

**Requirements:** The error message names the missing flag. The
error fires before any per-file work, so a corrupt `--max-colors
64` invocation on a real photo does no work and produces no `.bak`.

**Dependencies:** U1.

**Files:** `src/error.rs`, `src/pipeline.rs`.

**Approach:** Add an `AppError::MaxColorsRequiresLossy` variant
with a `#[error("--max-colors requires --lossy")]` `Display` impl.
At the top of `pipeline::run`, after the `NoInput` check and
before `collect_files`, branch on `args.max_colors.is_some() &&
!args.lossy` and return the new variant. `main.rs` already
translates `AppError` to exit code 1 via the existing `Err` arm,
so no main change is required.

**Test scenarios:**
- `imageoptim --max-colors 64 tests/example01.png` (no `--lossy`)
  exits 1, prints `error: --max-colors requires --lossy` on stderr.
  No file is modified, no `.bak` is created.
- `imageoptim --max-colors 64 --dry-run tests/example01.png` (no
  `--lossy`) — same outcome (the cross-flag check fires before
  `--dry-run` is even consulted).
- `imageoptim --max-colors 64 --lossy tests/example01.png` — no
  cross-flag error (the second invocation flows through).
- An existing test that uses `--lossy` without `--max-colors`
  (e.g., `tests/other_formats.rs:130-178`) still passes unchanged.

**Verification:** `cargo test` includes a new integration test in
`tests/max_colors.rs` that exercises the cross-flag rejection
end-to-end via the binary.

### U3. Plumb `max_colors` through the pipeline

**Goal:** Get the value from `args.max_colors` to the per-file
optimizer call without changing the control flow.

**Requirements:** Preserve the current behavior when `args.max_colors`
is `None`. Preserve the per-file error isolation (one bad file
shouldn't kill the run).

**Dependencies:** U1 (the value exists on `Args`), U2 (validation
has already run).

**Files:** `src/pipeline.rs`, `src/optimize/mod.rs`,
`src/optimize/{jpeg,gif,webp,svg}.rs`, plus the `Optimizer` trait
declaration in `src/optimize/mod.rs`.

**Approach:** Add a 5th parameter `max_colors: Option<u8>` to the
`Optimizer::optimize` trait method. Update all five impls: PNG
consumes it, the other four rename it to `_max_colors` to silence
the unused-arg warning. In `pipeline::run`, capture
`args.max_colors` into a local `let` (paralleling the existing
`lossy`, `quality`, `no_zopfli` captures) and pass it through
`optimize_file` (signature grows from 8 to 9 args — extend the
existing `#[allow(clippy::too_many_arguments)]` to cover 9, or
accept a clippy lint here; refactoring to a context struct is
out of scope for this plan and noted in Next Steps).
The final call site at `optimizer.optimize(&original, quality, lossy,
no_zopfli)` becomes `optimizer.optimize(&original, quality, lossy,
no_zopfli, max_colors)`.

**Test scenarios:** No new behavioral tests in this unit — it is
purely mechanical. The U1, U2, U4, and U5 tests cover correctness
end-to-end. Verify by running the full `cargo test` suite (34
existing tests must still pass with the 5-arg calls).

**Verification:** `cargo test --all` runs 34/34 (existing) plus the
new U5 tests once they exist.

### U4. Wire `set_max_colors` into the PNG lossy path

**Goal:** When `max_colors` is `Some(N)`, call
`imagequant::Attributes::set_max_colors(N)` before the existing
`set_quality` / `set_speed` chain. When `None`, do not call it
(preserving current behavior).

**Requirements:** Same outcome (size + decode-valid) as the
current pipeline when `None`. A smaller `N` produces a smaller or
equal output up to the point where the safety contract refuses
the file.

**Dependencies:** U3 (the value is on the trait).

**Files:** `src/optimize/png.rs`.

**Approach:** In `optimize_lossy(bytes, no_zopfli, max_colors)`,
between `Attributes::new()` and the existing `set_quality` call,
insert `if let Some(n) = max_colors { attr.set_max_colors(n as u32) }?;`
— using `?` matches the existing error-propagation style of the
other two setters. The `as u32` cast is needed because
`Attributes::set_max_colors` takes `u32`; the cast is value-
preserving because the clap range already constrains `n <= 256 <
u32::MAX`. Update the inline comment on the function
("Quantize to a palette of at most 256 colors.") to read "Quantize
to a palette of up to `max_colors` colors (default 256)." Update
the `--lossy` doc-comment in `src/cli.rs` to no longer claim "256
colors" as a fixed property.

**Test scenarios:** Unit-level coverage is the primary goal here.
- Direct call: `PngOptimizer.optimize(&bytes, None, true, false,
  Some(16))` produces a valid PNG that is smaller than `bytes`.
- Direct call: `PngOptimizer.optimize(&bytes, None, true, false,
  Some(256))` produces a result equivalent (byte-for-byte, or
  within `oxipng`'s deterministic output) to the `None` case —
  i.e., `set_max_colors(256)` is a no-op for imagequant.
- Direct call: `PngOptimizer.optimize(&bytes, None, true, false,
  Some(2))` either succeeds with a heavily banded result OR is
  skipped by the safety contract. Both are acceptable. Asserting
  "either smaller (Optimized) OR skipped" is the right
  observation.

**Verification:** `cargo test --test unit` and
`cargo test --test other_formats` both pass; the
`png_lossy_smaller_than_lossless` test still passes with the 5-arg
signature.

### U5. New integration tests

**Goal:** End-to-end coverage of the new flag via the binary.

**Requirements:** Each test in `tests/max_colors.rs` follows the
existing `tests/output_dir.rs` pattern (`Command::new(env!("CARGO_BIN_EXE_imageoptim"))`,
soft-skip when the fixture is missing, `tempfile::tempdir()` for
isolation).

**Dependencies:** U1, U2, U3, U4 (all feature work complete).

**Files:** `tests/max_colors.rs` (new).

**Approach:** Five tests:
1. `max_colors_reduces_output_at_small_n` — run
   `--lossy --max-colors 16` and `--lossy --max-colors 256` on the
   same fixture, assert `len(N=16) <= len(N=256)`.
2. `max_colors_256_matches_default` — run
   `--lossy --max-colors 256` and `--lossy` (no flag), assert
   `len(256) == len(default)`.
3. `max_colors_clamps_at_two` — `--max-colors 2` (smallest valid)
   either produces a smaller-and-decodable result (`Optimized`)
   or hits the safety contract (`Skipped`). The test asserts one
   of those two outcomes, matching the pattern in U4's test
   scenarios. (Strictly-smaller is too strong a claim for a 2-color
   palette against a real photo, where the index overhead can
   exceed the savings.)
4. `max_colors_without_lossy_errors` — covered in U2, listed here
   for completeness so the test file is self-contained.
5. `max_colors_above_range_rejected` — `--max-colors 999` is
   rejected at parse time (exit code != 0, no file modification).

**Test scenarios:** See Approach above. Each scenario names the
fixture, the args, and the expected outcome.

**Verification:** `cargo test --test max_colors` passes 5/5.
`cargo test --all` passes 39/39 (34 existing + 5 new).

### U6. Sync docs

**Goal:** Keep the user-facing docs and the plan-deviations note
honest.

**Requirements:** No new "256 colors" hard-coded claims in the
user-facing surface.

**Dependencies:** U1, U4, U5 (the doc strings to update depend on
the actual flag semantics, and the test names referenced in the
"Plan deviations" note depend on the test names that U5 locks in).

**Files:** `README.md`, `docs/plans/2026-06-09-001-feat-imageoptim-rust-cli-plan.md`.

**Approach:**
- `README.md` flag table: add the `--max-colors <N>` row.
- `README.md` `--lossy` section (around line 62): update "palette of
  up to 256 colors" to "palette of up to 256 colors by default, or
  any smaller bound via `--max-colors`".
- The benchmark table does not need a new row; the existing 14.86%
  number is the default-behavior measurement.
- Add a new "Plan deviations" line in the older plan document
  (alongside the existing three: GPL-3.0, jpeg-encoder fallback,
  gen-fixtures): a brief note that `--max-colors` was added after
  the original plan was written.

**Test scenarios:** Manual — render the README in a markdown viewer
or run `cargo run -- --help` and confirm the new flag appears.
No automated test.

**Verification:** The README renders without broken markup. The
plan deviations list has four entries. `cargo run -- --help`
shows `--max-colors <N>` in the help text.

## Sequencing and dependencies

U1 → U2 (cross-flag check needs the field) → U3 (plumbing) → U4
(wire it into imagequant) → U5 (tests) → U6 (docs).

A reviewer can read this plan top-to-bottom. An implementer should
execute the units in order. U3 and U4 can technically be done
together since they are a single touch (modify the trait, modify
the call site, modify the PNG impl), but separating them makes
the diff reviewable: U3 is the mechanical signature change, U4 is
the single new line of behavior.

## Risks

- **Risk: `clap::value_parser!(u8).range(...)` is the first use of
  `value_parser` in the repo.** Mitigation: the rest of the
  codebase already has a uniform clap derive style; introducing one
  call to a built-in clap combinator is low-risk. If the
  `clap::value_parser!` macro path is later found awkward, all
  other flags still work and a follow-up can convert the rest.
- **Risk: 13 existing test call sites all need a 5th `None`
  argument.** Mitigation: the change is mechanical, type-checker
  enforced, and the existing 34 tests serve as a regression
  boundary. If a test was missed, `cargo test` will not compile.
- **Risk: very small `N` produces a worse-than-lossless output
  that violates the "optimized < lossless" invariant in
  `tests/other_formats.rs:130-178`.** Mitigation: that test uses
  `lossy=true, max_colors=None` (the default), so the invariant
  holds unchanged. The new U5 tests use small `N` against the
  *lossless* baseline, which the safety contract handles (skip,
  not failure).
- **Risk: README benchmark claim "14.86% on a real photo" is
  invalidated by a code change.** Mitigation: the flag is a
  no-op when absent. The default 256 behavior is unchanged. The
  existing benchmark number remains accurate.

## Verification

After all units:

- `cargo test --all` runs 39/39 (34 existing + 5 new) and passes.
- `cargo clippy --all-targets` is clean.
- `cargo fmt --check` is clean.
- `cargo run --release -- --help` shows the new flag with the
  correct help text.
- `cargo run --release -- --lossy --max-colors 64
  tests/example01.png` produces a smaller output than `cargo run
  --release -- --lossy tests/example01.png` on the same input.
- `cargo run --release -- --max-colors 64 tests/example01.png`
  exits 1 with `error: --max-colors requires --lossy` on stderr.
- `cargo run --release -- --max-colors 0 tests/example01.png
  --lossy` exits 1 with clap's range error.
