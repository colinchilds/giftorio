# GIFtorio

GIFtorio is a web application that converts animated GIFs into Factorio blueprints. The resulting blueprint creates an animated display using the game's circuit network and lamp display system.

## Description

This tool takes an animated GIF and converts it into a Factorio blueprint string that you can import into the game. The blueprint creates a setup of combinators and lamps that will display your GIF as an animated pixel display in-game.

## Features

- Converts animated GIFs to Factorio blueprints directly in your browser
- Automatically downscales GIFs to fit within signal limitations
- Configurable frame rate and image size
- Configurable substation quality to reduce dead pixels
- Maintains animation timing similar to the original GIF
- Supports different substation qualities when using Space Exploration

## Prerequisites

For development:
- Rust (latest stable version)
- wasm-pack (`cargo install wasm-pack`)
- A web browser with WebAssembly support
- Python 3.x (for local development server)

## Development Setup

1. Clone this repository:

```bash
git clone https://github.com/colinchilds/giftorio.git
cd giftorio
```

2. Build the WebAssembly module:

```bash
wasm-pack build --target web --release --out-dir web/pkg
```

3. Start a local development server:

```bash
cd web
python -m http.server 3000
```

4. Open your browser and navigate to `http://localhost:3000`

## Usage

1. Visit the website (or your local development server)
2. Upload your GIF file
3. Configure settings:
   - Frame rate (default: 15 FPS)
   - Maximum size (default: 100 pixels)
   - Toggle Space Exploration mode
   - Select substation quality (if Space Exploration is enabled)
4. Click Submit
5. Copy the generated blueprint string
6. Import the blueprint string into Factorio

## How It Works

The application:
1. Uses WebAssembly (compiled from Rust) to process GIFs efficiently in the browser
2. Loads and downscales the input GIF to a manageable size
3. Converts each frame into a series of circuit network signals
4. Creates a blueprint containing:
   - Constant combinators to store pixel data
   - Decider combinators to control frame timing
   - A grid of lamps to display the image
5. Outputs an encoded blueprint string compatible with Factorio

## Limitations

- Maximum image size is limited by available signals
- Higher resolution images will require more in-game entities and may impact performance
- Browser must support WebAssembly

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

[MIT License](LICENSE)