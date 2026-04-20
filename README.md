# Toolpath Viewer

A high-performance toolpath visualization application written in Rust, using OpenGL for rendering.

## Architecture

This project follows **Clean Architecture** principles with clear separation of concerns:

```
toolpath_viewer_rs/
├── src/
│   ├── main.rs                 # Application entry point
│   ├── domain/                 # Domain Layer (innermost)
│   │   ├── entities/           # Core business entities
│   │   │   ├── vector.rs       # Vector/polyline entity
│   │   │   ├── layer.rs        # Layer entity
│   │   │   ├── slice_stack.rs  # Stack of layers
│   │   │   └── toolpath.rs     # Complete toolpath
│   │   ├── value_objects/      # Immutable value types
│   │   │   ├── point.rs        # 2D/3D points
│   │   │   ├── color.rs        # RGBA colors
│   │   │   └── bounds.rs       # Bounding boxes
│   │   └── services/           # Domain services
│   │       └── geometry.rs     # Geometric calculations
│   │
│   ├── application/            # Application Layer
│   │   ├── ports/              # Interfaces (traits)
│   │   │   ├── file_port.rs    # File loading interface
│   │   │   ├── renderer_port.rs # Rendering interface
│   │   │   └── config_port.rs  # Configuration interface
│   │   ├── use_cases/          # Application logic
│   │   │   ├── load_toolpath.rs
│   │   │   ├── navigate_layers.rs
│   │   │   └── render_view.rs
│   │   └── dto/                # Data transfer objects
│   │
│   ├── infrastructure/         # Infrastructure Layer
│   │   ├── file_adapters/      # File format implementations
│   │   │   ├── cli_parser.rs   # CLI format parser
│   │   │   └── ilt_loader.rs   # ILT (compressed CLI) loader
│   │   ├── rendering/          # OpenGL rendering
│   │   │   ├── gl_renderer.rs  # Main renderer
│   │   │   ├── shader.rs       # Shader management
│   │   │   └── line_batch.rs   # VBO-based line rendering
│   │   └── config/             # Configuration management
│   │
│   └── presentation/           # Presentation Layer (outermost)
│       ├── app.rs              # Main application orchestration
│       ├── window.rs           # Window and event handling
│       └── input.rs            # Input state management
```

## Features

- **ILT/CLI File Support**: Load compressed (ILT) and uncompressed (CLI) toolpath files
- **Layer Navigation**: Navigate through layers using keyboard shortcuts
- **Efficient Rendering**: GPU-accelerated rendering using OpenGL VBOs
- **Pan & Zoom**: Interactive view manipulation
- **Display Toggles**: Show/hide boundaries, contours, hatches

## Dependencies

- `gl` - OpenGL bindings
- `glutin` - OpenGL context management
- `winit` - Cross-platform windowing
- `flate2` - Gzip decompression for ILT files
- `nalgebra` - Linear algebra operations

## Building

```bash
# Build release version
cargo build --release

# Run with a file
cargo run --release -- path/to/file.ilt
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| Up/W | Next layer |
| Down/S | Previous layer |
| Page Up | Jump forward 10 layers |
| Page Down | Jump backward 10 layers |
| Home | First layer |
| End | Last layer |
| F/R | Reset view (fit to content) |
| B | Toggle boundaries |
| C | Toggle contours |
| H | Toggle hatches |
| A | Toggle direction arrows |
| Ctrl+O | Open file |
| Ctrl+Q/Esc | Quit |

## Mouse Controls

| Action | Control |
|--------|---------|
| Pan | Middle mouse drag / Ctrl+Left drag |
| Zoom | Scroll wheel |

## File Formats

### CLI (Common Layer Interface)

ASCII format with commands:
- `$$LAYER/z` - Start layer at height z
- `$$POLYLINE/id,dir,n,x1,y1,...` - Polyline with n points
- `$$HATCHES/id,n,x1,y1,x2,y2,...` - Hatch lines
- `$$BOUNDARY/id,dir,n,x1,y1,...` - Part boundary

### ILT

Gzip-compressed CLI file.

## Clean Architecture Benefits

1. **Testability**: Domain and application layers can be tested without UI or file system
2. **Maintainability**: Clear boundaries between layers
3. **Flexibility**: Easy to swap implementations (e.g., different renderers)
4. **Independence**: Core business logic doesn't depend on frameworks

## Installer

The project includes an Inno Setup-based Windows installer with install, uninstall, and update support.

### Prerequisites

- [Inno Setup 6](https://jrsoftware.org/isdl.php) installed on the build machine

### Building the Installer

```powershell
# Build release binary + package installer in one step
.\installer\build_installer.ps1

# Skip rebuild, just repackage the installer
.\installer\build_installer.ps1 -SkipBuild

# Custom Inno Setup path
.\installer\build_installer.ps1 -InnoSetupPath "C:\Tools\InnoSetup\ISCC.exe"
```

The installer will be output to `installer/output/ToolpathViewer_Setup_<version>.exe`.

### Installer Features

- **Install**: Places the app in Program Files with Start Menu shortcuts
- **Uninstall**: Full removal via Windows Add/Remove Programs
- **Update**: Detects existing installation and offers to upgrade in-place
- **File associations**: Optional `.cli` and `.ilt` file association
- **Desktop shortcut**: Optional desktop icon

## License

MIT
