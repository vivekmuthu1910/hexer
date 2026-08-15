# Hexer architecture

Snapshot of how Hexer is structured today. Domain terms follow [`CONTEXT.md`](../../CONTEXT.md).

## Overview

Hexer is a Rust TUI binary viewer. An `App` shell owns one active **window** (File Picker or viewer), drives a ratatui draw / crossterm event loop, and switches windows on events. Binary interpretation lives in a pure **Grid** model; the viewer formats and paints Values from that model.

```
╭──────────────────────────────────────────────────────────────────╮
│                          hexer (bin)                             │
│                                                                  │
│   CLI args --> launch::resolve_launch --> App::from_launch       │
│                                                                  │
│   ╭──────────────────────────────────────────────────────────╮   │
│   │                          App                             │   │
│   │   Window: FilePicker │ HexViewer                         │   │
│   │   loop: draw(window) -> read event -> dispatch           │   │
│   ╰──────────────────┬───────────────────────┬───────────────╯   │
│                      │                       │                   │
│                      ▼                       ▼                   │
│            file_picker::           viewer::ViewerContainer       │
│            FilePickerState         (+ FileViewer, Grid)          │
╰──────────────────────┬───────────────────────┬───────────────────╯
                       │                       │
                       ╰───────────┬───────────╯
                                   │
                           ratatui + crossterm
                                   │
                                   ▼
                             terminal TTY
```

## Crate layout

Single binary crate (`hexer`). Modules under `src/`:

```
src/
├── main.rs              App shell, event loop, window switching
├── launch.rs            CLI path -> LaunchWindow (test seam)
├── file_picker.rs       File Picker UI + keys/mouse
├── utils/               path helpers, power-of-two column fit
╰── viewer/
    ├── mod.rs           ViewerContainer (chrome, options, keys)
    ├── file_viewer.rs   Grid paint + scroll state (ratatui widget)
    ├── grid.rs          Buffer -> Values (test seam)
    ╰── common_dt.rs     Data Type, Display Type, Endianness
```

│ Module │ Role │
│────────│──────│
│ `launch` │ Pure mapping: optional path -> File Picker vs viewer │
│ `file_picker` │ Browse directories; open a File │
│ `viewer` │ Viewer chrome, option controls, load File bytes │
│ `viewer::grid` │ Decode Buffer into addressable Values │
│ `viewer::file_viewer` │ Stateful widget: layout, format, scroll │
│ `utils` │ Shared helpers (path truncation, auto Width fit) │

## Startup and windows

```
                         optional path arg
                                 │
                                 ▼
                       resolve_launch(path)
                                 │
               ╭─────────────────┼─────────────────╮
               │                 │                 │
               ▼                 ▼                 ▼
          None/absent        directory           File
               │                 │                 │
               ▼                 ▼                 ▼
        FilePicker(cwd)   FilePicker(dir)    Viewer(file)
               │                 │                 │
               ╰─────────────────┼─────────────────╯
                                 │
                                 ▼
                            App.window
```

Runtime switching (unchanged after launch):

```
     File Picker                      Viewer
     ───────────                      ──────
          │                              │
          │      Enter on File           │
          ├─────────────────────────────▶│
          │                              │
          │◀──── Ctrl+F (parent dir) ────┤
          │                              │
     Quit (q/Esc)                   Quit (q/Esc)
```

## Viewer stack

`ViewerContainer` owns the File path, interpretation settings, and the `FileViewer` widget. On each frame it reads the File into a Buffer and renders chrome + Grid.

```
╭────────────────────────────────────────────────────────────────╮
│ ViewerContainer                                                │
│  file, data_type, display_type, endianness, action_mode        │
│                                                                │
│  render:                                                       │
│    ╭──────────────╮  ╭──────────────╮  ╭──────────────╮        │
│    │ File name    │  │ Search*      │  │ (spacer)     │        │
│    ╰──────────────╯  ╰──────────────╯  ╰──────────────╯        │
│                                                                │
│    ╭────────────────────╮  ╭─────────╮  ╭─────────────╮        │
│    │ Data Type buttons  │  │ Display │  │ Endianness  │        │
│    │ U8 I8 ... F64      │  │ Dec/Hex │  │ Little/Big  │        │
│    ╰────────────────────╯  ╰─────────╯  ╰─────────────╯        │
│                                                                │
│    ╭──────────────────────────────────────────────────────╮    │
│    │ FileViewer (stateful widget)                         │    │
│    │ Address gutter │ Values ... │ scrollbar              │    │
│    ╰──────────────────────────────────────────────────────╯    │
╰────────────────────────────────────────────────────────────────╯

* Search chrome is a non-functional stub.
```

### Decode -> paint pipeline

```
                         File on disk
                              │
                              │  fs::read
                              ▼
                        Buffer (bytes)
                              │
                              │  Grid::from_buffer(
                              │    buffer, Data Type,
                              │    Endianness, Width)
                              ▼
                 Grid { width, values: Vec<Value> }
                              │
                              │  format Value
                              │  (Display Type /
                              │   scientific for floats)
                              ▼
                 ratatui cells (FileViewer::render)
```

Current layout assumptions inside the Grid:

- **Width** = visible column count used for indexing (auto-fit from terminal via `calc_cols` / power-of-two)
- **Stride** = Width (no Padding yet)
- **Endianness** = Little (default) or Big; applied for multi-byte ints/floats; U8/I8 ignore it
- **Height** = floor(value_count / Width) for scrolling; Values remain flat-addressable

```
  Buffer bytes                    Data Type + Endianness
  ───────────────────             ───────────────────────────
  [ aa bb cc dd ... ]  ───────▶   Value::U16(0xbbaa), ... (Little)
                       ───────▶   Value::U16(0xaabb), ... (Big)

  Width = 4

  values:  V0  V1  V2  V3  V4  V5  V6  V7  ...
           ╰─── row 0 ────╯  ╰─── row 1 ────╯
```

## Event flow

```
                   crossterm::event::read
                              │
                              ▼
                             App
                              │
                    ╭─────────┴─────────╮
                    │    Key / Mouse    │
                    ╰─────────┬─────────╯
                              │
                        match window
                              │
              ╭───────────────┴───────────────╮
              │                               │
              ▼                               ▼
         FilePickerState                 ViewerContainer
           .handle_key                     .handle_key
           .handle_mouse                      │
              │                               │
              │                               ├─ Normal
              │                               │     options, scroll,
              │                               │     quit, Ctrl+F
              │                               ├─ SelectDataType
              │                               │     Ctrl+T chord
              │                               │
              ▼                               ▼
         FilePickerEvent                 ViewerContainerEvent
           Quit │ Poll │                   Quit │ Poll │
           SelectedFile(path)              SelectFile(dir)
              │                               │
              ╰───────────────┬───────────────╯
                              │
                              ▼
                App updates Window / running
```

Mouse is handled for the File Picker only (debug builds).

## Test seams

Preferred test surfaces (no ratatui frame snapshots):

```
  ╭──────────────────────────────────────────╮
  │ launch::resolve_launch                   │
  │   path cases -> LaunchWindow             │
  ╰──────────────────────────────────────────╯

  ╭──────────────────────────────────────────╮
  │ viewer::grid::Grid::from_buffer          │
  │   decode + Width/Height                  │
  │   Endianness Little/Big                  │
  ╰──────────────────────────────────────────╯
```

UI widgets and the App loop are exercised manually / by construction from those seams.

## Dependencies (runtime)

│ Crate │ Use │
│-------│-----│
│ `ratatui` │ Terminal widgets and layout │
│ `crossterm` │ Input events, mouse capture │
│ `color-eyre` │ Error reporting │
│ `num-traits` │ Float formatting helpers │
│ `tracing`* │ Debug-build logging to `tracing.log` │

\* tracing stack is compiled for debug builds.

## Known gaps (domain vs code)

Documented in the domain model / parent spec but not fully wired yet:

│ Concept │ Status │
│--------│--------│
│ View Mode (Binary \│ Image) │ Not implemented; chrome is Binary-style only │
│ User-set Width / Stride / Padding │ Auto Width only; Stride = Width │
│ Viewport Width vs logical Width │ Horizontal scroll APIs exist; logical Width not pinned │
│ Cursor + Address-as-byte-offset │ Row label is still Value-index style (`row * Width`) │
│ CLI viewer option flags │ Path only (`launch`); flags are future work │
│ Search │ UI stub only │

Architecture intent going forward: keep a **single Grid engine**; View Mode and layout knobs change inputs/chrome, not a second renderer.
