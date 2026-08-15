# Hexer

A TUI that visualizes a binary Buffer as a typed Grid. General-purpose, especially useful for raw images.

## Language

### Opening

**Buffer**:
The byte sequence under inspection.
_Avoid_: Blob, content, payload

**File**:
A path on disk that loads into a Buffer. A directory path opens the File Picker instead of the viewer. May be given as a CLI argument (absent → File Picker at the current working directory).
_Avoid_: Document, resource

**File Picker**:
The UI for browsing directories and choosing a File to open as a Buffer.
_Avoid_: File browser, open dialog

### Grid layout

**Grid**:
The two-dimensional layout of Values from a Buffer (rows × columns).
_Avoid_: Dump, canvas, table, hex view

**Value**:
One typed unit in the Grid, interpreted from a fixed number of bytes according to the Data Type.
_Avoid_: Element, sample, cell, word

**Width**:
How many Values make one logical Grid row. Shared by both View Modes; drives Height. In Binary, Width may be automatic (fit the terminal) until the user pins an explicit Width. In Image, Width is required for a meaningful Grid.
_Avoid_: Image width (as a separate mechanism), columns (when meaning Viewport Width)

**Stride**:
The number of Values from the start of one Grid row to the start of the next. When Stride is larger than Width, the extra Values are Padding. Counted in Values, not bytes. Defaults to Width.
_Avoid_: Pitch, row length (ambiguous with Width)

**Padding**:
The Stride−Width Values after each row's visible Width. Omitted from the Grid by default; may be shown optionally.
_Avoid_: Gap, gutter, alignment bytes (when speaking in Values)

**Height**:
The number of rows in the Grid, derived from Buffer length once Width (and optional Stride) are known — not set directly.
_Avoid_: Image height (as a separate user knob), row count (when meaning viewport rows)

**Viewport Width**:
How many Grid columns are visible in the current view (terminal fit and horizontal scroll) — not the Buffer's logical Width.
_Avoid_: Width, visible width (unqualified)

### Interpretation

**Data Type**:
The interpretation rule for a Value (signed/unsigned integer or float, and its byte width).
_Avoid_: Value type, element type, mode

**Display Type**:
How integer Values are shown: Decimal or Hex. Float Values use scientific notation regardless of Display Type.
_Avoid_: Format, radix, HexaDecimal

**Endianness**:
Byte order used to decode multi-byte Values from the Buffer: Little or Big.
_Avoid_: Byte order (as a different concept), host endianness (when meaning the user setting)

### View Mode

**View Mode**:
Toggle that changes Grid chrome and which layout knobs feel primary — not a separate rendering engine. Variants: Binary (default) and Image.
_Avoid_: Layout mode, profile, viewer mode

**Binary**:
View Mode focused on general Buffer inspection. Row gutter shows Address. Width may be automatic; Stride defaults to Width. Status shows the Cursor's Address (byte offset of the focused Value).
_Avoid_: Hex mode, dump mode

**Image**:
View Mode focused on raw-image-style layout. Headers show Row and Column indices; status also shows the Cursor's (Row, Column). Width is required; Width/Stride are the primary layout knobs.
_Avoid_: Picture mode, canvas mode

**Address**:
Byte offset of a Grid position within the Buffer. In Binary View Mode, the row gutter shows the Address of each row start; status shows the Address of the Cursor's Value.
_Avoid_: Index (when meaning Value index), offset (unqualified)

**Row**:
Zero-based vertical index of a Grid line (Image headers and Cursor).
_Avoid_: Line, Y (unless used informally)

**Column**:
Zero-based horizontal index of a Value within a Grid row (Image headers and Cursor).
_Avoid_: X (unless used informally)

**Cursor**:
The currently focused Grid Value. Binary status shows its Address; Image status shows its (Row, Column).
_Avoid_: Selection, focus, caret
