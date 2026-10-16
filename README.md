# GIFtorio

[GIFtorio](https://giftor.io) is a web application that converts animated GIFs
into Factorio blueprints. The resulting blueprint creates an animated display
using the game's circuit network and lamps. It requires no mods - works on
vanilla Factorio version 2.0 and the Space Age DLC.
![example nyan gif](https://github.com/colinchilds/giftorio/blob/main/web/assets/img/nyan.gif?raw=true)

## Features

- Convert animated GIFs to Factorio blueprints directly in your browser
- Automatically downscale GIFs to fit within signal limitations
- Configurable frame rate and image size
- Configurable power pole qualities to reduce dead pixels
- Attempts to maintain animation timing similar to the original GIF
- Supports different power pole qualities when using the Space Age DLC
- Supports using small, medium, and large power poles as well as substations

## Additional Features (in 2.0)

### Temporal Compression

Temporal compression samples frames at a fixed rate, storing pixels that didn't
change this sample in a **single** combinator for the sample (instead of being
duplicated across multiple combinators for each frame).

_N.B.: Temporal compression increases the total number of combinators required
in the blueprint._

Works better on longer videos with less movement. Size reduction varies from 1x
to 4x depending on the nature of the source video.

- For example, a sample window of 250 ms at 60fps will scan 15 frames at a time,
  storing any pixels that remained the same throughout those 15 frames in a
  single combinator.
- Window sizes (ms) can be tweaked for optimal compression.
- Longer windows could cache compress more pixels into a single combinator, but
  decreases the chances of finding such pixels that don't change.
- Smaller sizes increases the chances of finding non-changing pixels at a cost
  of reduced compression.

Temporal compression is a static compression technique, so it is possible to
seek to any frame at any point in time. This can greatly reduce blueprint sizes
(around x4), depending on the source GIF.

### Delta Compression

Delta compression stores the difference in raw pixel values between frames,
instead of the real frame data.

_N.B. Delta compression is volatile. Blueprints generated with delta compression
cannot be safely seeked, and must always be played from the beginning._

- Delta compression greatly reduces sizes (up to x10 smaller than uncompressed).
- Instead of storing the raw values of each frame, delta compression has each
  combinator store the difference required to obtain the desired pixel value.
- This means any pixels that don't change can be excluded entirely. This can
  give massive compression gains at a cost of not being able to seek safely, as
  a frame depends on state from all previous frames.

### Signal Sorting

In Factorio, signals come with a `type` and a `name` field. GIFtorio uses a
pre-compiled list of all available signals to store the pixel data.

However, some signals take up more space in JSON as their `type` and `name`
field values are longer. E.g. `{ "type": "entity", "name": "red-chest" }` is
much shorter than
`{ "type": "entity", "name": "small-demolisher-expanding-ash-cloud-29" }`.

Because of this, we can sort the signals by the length of their `type` and
`name` fields, and prioritise signals with shorter names and types first.
Splitting the video segments reduces the total number of unique signals
required, allowing us to use lots of e.g. 300 small-length signals instead of
1000 long ones.

### Wire Settings

Lamp wires can be configured to use red or green wires. Their connection
direction can also be configured (horizontal or vertical).

### Server-Side Optimisations

Most content is streamed from the source instead of being accumulated into one
buffer. This allows processing larger GIFs. GIFtorio can now support 13,000+
frame GIFs and a longest side up to 1,000 lamps.

- Includes a utility script that scans the byte-level frame data of the GIF /
  WebP, and can efficiently extract the total frame count and total duration.
- Video frames are streamed instead of accumulated into a single buffer, greatly
  reducing memory pressure. Note that for now, parallel processing is disabled.
- The output blueprint string is chunked and streamed directly to disk, instead
  of being serialised in one.
- JSON serialisation is chunked into groups of 1,000 entities, reducing memory
  pressure.
- Updated to use 13,341 available signals from Space Age (+80% more than old
  version, 3 signals reserved for internal calculations). Some signals don't
  show up at all in the Factorio GUI, but otherwise function as regular signals.

### Other

- Max size (longest side) increased to 1,080 - so big the entire screen cannot
  fit in min zoom in Editor mode
- Added support for resampling (`nearest`, `triangle`, `catrom`, `lanczos3`,
  `gaussian`).
- Settings are remembered between page reloads (using local storage).
- Automatically remembers the last uploaded file, and pre-populates the file
  upload. (Useful as a dev tool).
- Added direct JSON download mechanic for huge blueprints (>50 MB). Factorio has
  issues importing compressed blueprint strings if their sizes are very large,
  but can safely import JSON file (by dragging the file directly into Factorio;
  requires Factorio 2.0.25 or later).

## Prerequisites

For development:

- Rust (latest stable version)
- Node (latest LTS version)
- wasm-pack (`cargo install wasm-pack`)
- A web browser with WebAssembly support

## Development Setup

1. Clone this repository:

```bash
git clone https://github.com/colinchilds/giftorio.git
cd giftorio
```

2. Build the WebAssembly module:

```bash
wasm-pack build --target web --release
```

3. Run it with NPM:

```bash
npm install
npm start
```

4. Open your browser and navigate to `http://localhost:3000`

## Usage

1. Visit the website (or your local development server)
2. Upload your GIF file
3. Configure settings (frame rate, image size, etc.)
4. Click Generate
5. Copy the generated blueprint string
6. Import the blueprint string into Factorio

## How It Works

The application:

1. Uses WebAssembly (compiled from Rust) to process GIFs efficiently in the
   browser
2. Loads and downscales the input GIF to a manageable size
3. Converts each frame into a series of circuit network signals
4. Creates a blueprint containing:
   - A grid of substations to power the display
   - Constant combinators to store pixel data
   - Decider combinators to control frame timing
   - A grid of lamps to display the image
5. Outputs an encoded blueprint string compatible with Factorio

## Limitations

- Maximum image size is limited by available signals, but more realistically by
  in-game performance.
- Higher resolution images will require more in-game entities and may impact
  performance.
- Browser must support WebAssembly.
- Longer GIFs at high resolutions can be very large (5GB+).

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Credits

- [@colinchilds](https://github.com/colinchilds) for the original
  [GIFtorio](https://github.com/colinchilds/giftorio) project

## License

[MIT License](LICENSE)
