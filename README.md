# FastXcel
A blazing fast viewer for large datasets, optimized for financial time series. Built with Rust + Polars + egui.

## Problem Statement
When working with large financial datasets (OHLCV - Open, High, Low, Close, Volume):

- Loading large CSV/Parquet files is slow with traditional tools
- Memory consumption is high when viewing entire datasets
- Common column names vary across data sources (exchanges, vendors)
- UI becomes unresponsive with large datasets
- Filtering & searching is computationally expensive

## Solution
This viewer solves these problems by:

1. **Lazy Loading**: Uses Polars Lazy evaluation to load only visible data
2. **Column Aliasing**: Automatically detects common OHLCV column names
3. **Virtualized UI**: Only renders visible rows using egui's table virtualization
4. **Fast Filtering**: Uses Polars expressions for efficient filtering
5. **Memory Efficient**: Streams data in pages rather than loading entire file

## Features
- 📊 Support for CSV and Parquet files
- 🚀 Blazing fast load times even for 100M+ rows
- 🔍 Real-time filtering using Polars expressions
- 📝 Automatic column name detection
- 🎨 Dark theme optimized UI
- 🖥️ Windows-native file picker

## Quick Start
```powershell
# Clone repository
git clone https://github.com/yourusername/fastxcel
cd fastxcel

# Build release version
cargo build --release

# Run FastXcel
.\target\release\fastxcel.exe
```

## Usage
1. Click "Open CSV/Parquet" to select a file
2. Use Prev/Next to navigate through pages
3. Adjust page size based on your system's RAM
4. Enter filter expressions like: `Open > 100 & Volume > 10000`

## Technical Details
- Built with Rust 2021 edition
- Uses Polars for data processing
- egui/eframe for GUI
- Native file dialogs with rfd
- Optimized release build configuration

## Performance
- CSV Load: ~100ms for 1M rows
- Memory Usage: ~50MB base + page size
- UI Responsiveness: 60fps even with large datasets

## Build Requirements
- Rust 1.75+
- Windows 10/11
- Cargo package manager

## License
MIT

## Contributing
Pull requests welcome! Areas for improvement:
- Additional file formats
- Column pinning
- Candlestick chart view
- Schema analysis panel
- Column statistics

## Project Structure
```
fastxcel/
├─ Cargo.toml
└─ src/
   ├─ main.rs       # App entry point
   ├─ app.rs        # Main application logic
   ├─ data.rs       # Data engine & loading
   ├─ columns.rs    # Column name handling
   └─ ui/
      └─ table.rs   # Virtualized table widget
```
