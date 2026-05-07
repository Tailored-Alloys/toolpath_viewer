# 3MF Laser Toolpath Support

## Overview

The Toolpath Viewer supports loading and visualizing **3MF files** containing the
[3MF Laser Toolpath Extension](https://github.com/3MFConsortium/spec_lasertoolpath/blob/master/3MF%20Laser%20Toolpath%20Extension.md).

A `.3mf` file is an OPC (ZIP) package that may contain one or more toolpath
resources, each describing layer-by-layer laser scan geometry with associated
process parameters (power, speed, focus, etc.).

## Supported Features

| Feature | Status |
|---|---|
| Planar toolpaths (loop, polyline, hatch) | ✅ Supported |
| Toolpath profiles (laser power, speed) | ✅ Supported |
| Multiple toolpath resources per file | ✅ Supported (one tab per resource) |
| Unit factor scaling (device units → mm) | ✅ Supported |
| Arbitrary namespace prefixes (`tp:`, `t:`, etc.) | ✅ Supported |
| 3-axis / 6-axis toolpaths | ❌ Not yet supported |
| Binary encoding extension | ❌ Not yet supported |
| Profile modifiers (e/f/g/h override factors) | ❌ Not yet supported |
| Custom metadata visualization | ❌ Not yet supported |

## How It Works

### File Structure

A `.3mf` file is a ZIP archive with the following relevant structure:

```
my_build.3mf (ZIP)
├── 3D/
│   └── 3dmodel.model          ← Main model XML with toolpath resources
├── Toolpath/
│   ├── layer1.xml             ← Per-layer geometry (segments)
│   ├── layer2.xml
│   └── ...
├── [Content_Types].xml
└── _rels/
    └── .rels
```

### Parsing Pipeline

1. **Open ZIP** — The `.3mf` file is opened as a standard ZIP archive using the
   `zip` crate (already used for ILT files).

2. **Parse Model** — `3D/3dmodel.model` is parsed to extract `<toolpathresource>`
   elements. Each resource contains:
   - `unitfactor` — scaling factor from device units to millimeters
   - `<toolpathprofiles>` — laser process parameters (power, speed, focus)
   - `<toolpathlayers>` — references to per-layer XML files with Z heights

3. **Extract Layer XMLs** — All referenced layer XML files are read from the ZIP
   (sequential extraction since `ZipArchive` is not `Send`).

4. **Parse Layers in Parallel** — Each layer XML is parsed concurrently using
   `rayon`. The parser handles:
   - `<profiles>` — local profile ID → resource profile UUID mapping
   - `<segments>` — geometry segments:
     - `type="loop"` → closed `Contour` vector
     - `type="polyline"` → open `Contour` vector
     - `type="hatch"` → individual `Hatch` vectors (one per `<hatch>` child)
   - Coordinates are scaled by `unitfactor` to convert to millimeters

5. **Build Toolpath** — Layers are sorted by Z height and assembled into a
   `SliceStack` → `Toolpath`, reusing the existing domain model.

### Multi-Resource Files

A single `.3mf` file may contain multiple `<toolpathresource>` elements (e.g.,
different build configurations or part-specific toolpaths). Each resource is
loaded as a **separate tab** in the viewer, allowing independent navigation.

The `FileLoader` trait's `load_all()` method enables this — it returns a
`Vec<(String, Toolpath)>` where each entry becomes its own `FileEntry` with a
dedicated tab, layer slider, and camera state.

### Segment Type Mapping

| 3MF Segment Type | Domain VectorType | Behavior |
|---|---|---|
| `loop` | `Contour` | Closed polygon (auto-closes if last ≠ first) |
| `polyline` | `Contour` | Open polyline |
| `hatch` | `Hatch` | Individual line segments (one per `<hatch>` child) |

### Profile Parameter Mapping

| 3MF Profile Attribute | Domain VectorParameter | Unit |
|---|---|---|
| `laserpower` | `power` | W (watts) |
| `laserspeed` | `speed` | mm/s |

These are mapped directly to `VectorParameters`, enabling the existing parameter
visualization modes (color by power, color by speed) to work with 3MF data.

## Architecture

### New Files

| File | Purpose |
|---|---|
| `src/infrastructure/file_adapters/threemf_loader.rs` | 3MF parser and `FileLoader` implementation |

### Modified Files

| File | Change |
|---|---|
| `Cargo.toml` | Added `quick-xml` dependency for XML parsing |
| `src/infrastructure/file_adapters/mod.rs` | Registered `threemf_loader` module |
| `src/application/ports/file_port.rs` | Added `load_all()` default method to `FileLoader` trait |
| `src/application/use_cases/load_toolpath.rs` | Added `execute_multi()` method |
| `src/presentation/app.rs` | Registered `ThreemfLoader`, updated file dialog filter, use `execute_multi()` in background loader |

### Design Decisions

- **Namespace-agnostic parsing** — The parser strips XML namespace prefixes
  (e.g., `tp:`, `t:`) and matches on local element names only. This handles
  real-world 3MF files that use varying prefixes.

- **Parallel layer parsing** — Layer XMLs are extracted sequentially from the ZIP
  (required by `ZipArchive`'s API), then parsed in parallel with `rayon`, matching
  the existing ILT loader's architecture.

- **Reuse existing domain model** — 3MF data is mapped directly to the existing
  `Layer`, `Vector`, `SliceStack`, and `Toolpath` entities. No new domain types
  are introduced, ensuring all existing visualization and navigation features
  work out of the box.

- **`load_all()` trait method** — Added as a default method on `FileLoader` so
  existing loaders (ILT/CLI) continue to work unchanged while the 3MF loader can
  return multiple toolpath resources.

## Testing

### Unit Tests

The `threemf_loader.rs` module includes unit tests for:

- Local name extraction (namespace prefix stripping)
- Model XML parsing (single resource, both `tp:` and `t:` prefixes)
- Layer XML parsing (loop, polyline, hatch segments)
- Unit factor coordinate scaling
- Profile parameter mapping
- Integration tests loading the sample files

### Sample Files

| File | Description |
|---|---|
| `example/dummy.toolpath.3mf` | Simple test file: 5 layers, loop + hatch segments, 3 profiles |
| `example/helix.3mf` | Large production file: ~350 layers, polyline segments |

### Running Tests

```bash
cargo test --lib threemf
```

## Future Enhancements

- **Binary encoding** — Support the binary layer encoding extension for faster
  loading of large production files.
- **3-axis / 6-axis toolpaths** — Extend the parser and renderer to handle
  `polyline3d` and `point6d` segments.
- **Profile modifiers** — Visualize per-segment modifier factors (e/f/g/h)
  as additional parameter coloring modes.
- **Custom metadata** — Surface vendor-specific metadata (e.g., machine model,
  compensation mode) in the UI's file info panel.
- **OPC relationship resolution** — Follow `.rels` files instead of relying on
  hardcoded `3D/3dmodel.model` path for full spec compliance.
