# GIFtorio

GIFtorio is a Python script that converts animated GIFs into Factorio blueprints. The resulting blueprint creates an animated display using the game's circuit network and lamp display system.

## Description

This tool takes an animated GIF and converts it into a Factorio blueprint string that you can import into the game. The blueprint creates a setup of combinators and lamps that will display your GIF as an animated pixel display in-game.

## Features

- Converts animated GIFs to Factorio blueprints
- Automatically downscales GIFs to fit within signal limitations
- Supports both base game and Space Exploration DLC signal sets
- Configurable frame rate and image size
- Maintains animation timing similar to the original GIF

## Prerequisites

- Python 3.9 or higher
- Poetry (for installing dependencies)
- A copy of Factorio (to use the generated blueprints)

## Installation

1. Clone this repository:

```bash
git clone https://github.com/colinchilds/giftorio.git
cd giftorio
```

2. Install required dependencies:

```bash
poetry install
```

## Usage

1. Place your input GIF in the same directory as the script and name it `input.gif`
2. Run the script:

```bash
poetry run python main.py
```

3. The script will output a Factorio blueprint string that you can copy and import into the game

## How It Works

The script:
1. Loads and downscales the input GIF to a manageable size
2. Converts each frame into a series of circuit network signals
3. Creates a blueprint containing:
   - Constant combinators to store pixel data
   - Decider combinators to control frame timing
   - A grid of lamps to display the image
4. Outputs an encoded blueprint string compatible with Factorio

## Limitations

- Maximum image size is limited by available signals (currently limited to 30 pixels, but will be expanded soon to approximately 700 pixels for base game, more with Space Age)
- Frame rate is currently optimized for 4 FPS but can be adjusted soon
- Higher resolution images will require more in-game entities and may impact performance

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

[MIT License](LICENSE)