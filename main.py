import base64
import json
import sys
import zlib

from PIL import Image, ImageSequence


def encode_blueprint(bp_dict):
    """
    Encode a Python dictionary as a Factorio blueprint string.

    The blueprint string format is:
      <version><base64(zlib_deflate(JSON))>
    We use version "0" for compatibility.
    """
    try:
        # Convert the dictionary to a JSON string
        json_str = json.dumps(bp_dict)
        json_bytes = json_str.encode('utf-8')
        compressor = zlib.compressobj(level=9)
        compressed = compressor.compress(json_bytes) + compressor.flush()
        # Base64-encode the compressed bytes
        b64_encoded = base64.b64encode(compressed).decode('utf-8')
        # Prepend the version number ("0")
        return "0" + b64_encoded
    except Exception as e:
        print("Error encoding blueprint:", e)
        raise


def downscale_gif(input_path, max_size=30, output_path=None):
    """
    Downscale a GIF so that the longest side of each frame is at most max_size pixels.

    Parameters:
      - input_path: Path to the input GIF file.
      - max_size: Maximum size (in pixels) for the longest side of each frame.
      - output_path: Optional. If provided, the downscaled GIF will be saved here.

    Returns:
      - frames: A list of downscaled PIL Image objects (in RGB mode).
    """
    original_gif = Image.open(input_path)
    frames = []

    for frame in ImageSequence.Iterator(original_gif):
        # Convert frame to RGB (this discards palette information but gives us RGB values).
        frame = frame.convert("RGB")
        width, height = frame.size

        # Determine the scale factor: do not upscale if already small.
        scale_factor = min(max_size / width, max_size / height, 1)
        new_width = int(width * scale_factor)
        new_height = int(height * scale_factor)

        # Resize the frame with an appropriate resampling filter.
        frame_resized = frame.resize((new_width, new_height), resample=Image.BILINEAR)
        frames.append(frame_resized)

    # If an output path is provided, save the new GIF.
    if output_path:
        # Use the duration from the original GIF if available.
        duration = original_gif.info.get('duration', 100)
        frames[0].save(
            output_path,
            save_all=True,
            append_images=frames[1:],
            loop=0,
            duration=duration
        )

    return frames


def sample_frames(frames, target_fps=4):
    """
    Resample frames evenly to achieve the target FPS.

    Parameters:
      frames: List of PIL.Image objects (the frames from the GIF).
      original_frame_duration: Duration (in ms) each original frame is shown.
                               (Default is 100 ms, i.e. 10 FPS. TODO: try to get this from the image and remove this param)
      target_fps: Desired frames per second for the output (default is 4 for now, but we should be able to scale this up quite a bit
      and give users the option to change it so they can sacrifice framerate for blueprint size).

    Returns:
      A list of PIL.Image objects sampled at the target FPS.
    """
    total_frames = len(frames)

    # Calculate the total time in milliseconds for all frames.
    total_time_ms = 0
    for frame in frames:
        total_time_ms += frame.info['duration']

    # Calculate the average duration of a frame, since gifs can have variable frame durations.
    avg_frame_duration = total_time_ms / total_frames

    # Calculate how many frames we want in the output.
    target_total_frames = int(total_time_ms / 1000 * target_fps)

    sampled_frames = []
    for i in range(target_total_frames):
        target_time = i * (1000 / target_fps)
        # Determine the closest original frame index.
        orig_index = int(round(target_time / avg_frame_duration))
        if orig_index >= total_frames:
            orig_index = total_frames - 1
        sampled_frames.append(frames[orig_index])

    return sampled_frames


def rgb_to_int(r, g, b):
    return int((r << 16) + (g << 8) + b)


def load_signals(signals_path):
    """
    Load the list of available signals from a JSON file.
    The file should contain a JSON array of objects, for example:
      [{"name": "wooden-chest"}, {"name": "iron-chest"}, ...]
    """
    with open(signals_path, "r") as f:
        signals = json.load(f)

    # find and remove json object with name "signal-T" to not conflict with our timer signal. Everything else should be fair game
    for i, signal in enumerate(signals):
        if signal["name"] == "signal-T":
            del signals[i]
            break
    return signals


def frame_to_filters(frame, signals):
    """
    Given a PIL image frame and a list of available signals (loaded from signals.json),
    convert the pixel values into a list of filter dictionaries.

    The image is assumed to have its pixels in row‑major order matching the order
    of signals. Each filter dictionary has:
      - an "index" (starting at 1),
      - the "name" from the signal,
      - a comparator (always "="),
      - the "count" (the decimal color value),
      - and a "quality" (set to "normal").

    Raises an error if the number of pixels in the frame is greater than the available signals.
    """
    width, height = frame.size
    pixels = list(frame.getdata())
    num_pixels = width * height

    if num_pixels > len(signals):
        raise ValueError(f"Frame pixel count ({num_pixels}) exceeds available signals ({len(signals)}).")

    filters = []
    # Iterate in row-major order; index the filters starting at 1 (this is what shows up in the blueprint, which uses 1-based indexes).
    for i, (pixel, signal) in enumerate(zip(pixels, signals), start=1):
        # Convert the pixel (R, G, B) into a single decimal value.
        value = rgb_to_int(*pixel)
        filter_dict = {
            "index": i,
            "comparator": "=",
            "count": value,
            "quality": "normal"
        }
        filter_dict.update(signal)
        filters.append(filter_dict)
    return filters


def generate_timer(stop):
    entities = [
        {
            "entity_number": 1,
            "name": "constant-combinator",
            "position": {
                "x": -2.5,
                "y": -4.0
            },
            "direction": 4,
            "control_behavior": {
                "sections": {
                    "sections": [
                        {
                            "index": 1,
                            "filters": [
                                {
                                    "index": 1,
                                    "type": "virtual",
                                    "name": "signal-T",
                                    "quality": "normal",
                                    "comparator": "=",
                                    "count": 1
                                },
                                {
                                    "index": 2,
                                    "type": "virtual",
                                    "name": "signal-S",
                                    "quality": "normal",
                                    "comparator": "=",
                                    "count": stop
                                }
                            ]
                        }
                    ]
                }
            }
        },
        {
            "entity_number": 2,
            "name": "decider-combinator",
            "position": {
                "x": -1.5,
                "y": -4.0
            },
            "direction": 4,
            "control_behavior": {
                "decider_conditions": {
                    "conditions": [
                        {
                            "first_signal": {
                                "type": "virtual",
                                "name": "signal-T"
                            },
                            "second_signal": {
                                "type": "virtual",
                                "name": "signal-S"
                            },
                            "comparator": "<"
                        }
                    ],
                    "outputs": [
                        {
                            "signal": {
                                "type": "virtual",
                                "name": "signal-T"
                            }
                        }
                    ]
                }
            }
        },
        {
            "entity_number": 3,
            "name": "arithmetic-combinator",
            "position": {
                "x": -1.5,
                "y": -3.0
            },
            "direction": 12,
            "control_behavior": {
                "arithmetic_conditions": {
                    "first_signal": {
                        "type": "virtual",
                        "name": "signal-T"
                    },
                    "second_constant": 1,
                    "operation": "+",
                    "output_signal": {
                        "type": "virtual",
                        "name": "signal-T"
                    }
                }
            }
        }
    ]

    # Kinda hacky, but we need to just connect the timer to the combinators and the combinators to the image for now
    wires = []
    wires.append([1, 1, 2, 1])  # constant to decider in
    wires.append([2, 2, 3, 4])  # decider in to arithmetic out
    wires.append([2, 4, 3, 2])  # arithmetic in to decider out
    wires.append([2, 4, 5, 2])  # decider of timer to first decider of frames
    return entities, wires

def generate_frame_combinators(frames_filters,
                               ticks_per_frame=15,
                               base_entity_number=1,
                               base_constant_x=0.5,
                               base_decider_x=1.5,
                               base_y=-3.0):
    """
    Rebuild the blueprint's combinator entities and wires based on frames_filters.

    For each frame:
      - Create a constant combinator that holds that frame's pixel data.
      - Create a decider combinator that outputs its signals when:
            signal-T >= lower_bound AND signal-T < upper_bound,
        where lower_bound = frame_index * 15 and upper_bound = (frame_index + 1) * 15.
      - Immediately create a red-wire connection (circuit 1) from the constant to the decider.
      - If this decider is not the first (i.e. its entity ID > 2), assume the previous decider's ID is
        current_decider_id - 2 and add:
            * A green-wire connection on circuit 2 from the previous decider to the current decider.
            * A red-wire connection on circuit 3 from the previous decider to the current decider.

    Returns the new entities added, the wires we connected, and the next available entity number.
    """
    new_entities = []
    wires = []
    current_entity_number = base_entity_number
    first_decider = True

    # Build combinator pairs (constant and decider) for each frame.
    for i, filters in enumerate(frames_filters):
        current_y = base_y - i

        # Create the constant combinator.
        constant_entity = {
            "entity_number": current_entity_number,
            "name": "constant-combinator",
            "position": {
                "x": base_constant_x,
                "y": current_y
            },
            "direction": 4,
            "control_behavior": {
                "sections": {
                    "sections": [
                        {
                            "index": 1,
                            "filters": filters
                        }
                    ]
                }
            }
        }
        current_entity_number += 1

        lower_bound = i * ticks_per_frame
        upper_bound = (i + 1) * ticks_per_frame

        # Create the decider combinator.
        decider_entity = {
            "entity_number": current_entity_number,
            "name": "decider-combinator",
            "position": {
                "x": base_decider_x,
                "y": current_y
            },
            "direction": 4,
            "control_behavior": {
                "decider_conditions": {
                    "conditions": [
                        {
                            "first_signal": {
                                "type": "virtual",
                                "name": "signal-T"
                            },
                            "constant": lower_bound,
                            "comparator": ">="
                        },
                        {
                            "first_signal": {
                                "type": "virtual",
                                "name": "signal-T"
                            },
                            "constant": upper_bound,
                            "comparator": "<",
                            "compare_type": "and"
                        }
                    ],
                    "outputs": [
                        {
                            "signal": {
                                "type": "virtual",
                                "name": "signal-everything"
                            }
                        }
                    ]
                }
            }
        }
        current_entity_number += 1

        # Append combinator pair.
        new_entities.append(constant_entity)
        new_entities.append(decider_entity)

        # Add red-wire connection from constant to decider (both on circuit 1).
        wires.append([constant_entity["entity_number"], 1,
                      decider_entity["entity_number"], 1])

        # If not the very first decider, add wires from the previous decider.
        if not first_decider:
            # Previous decider assumed to be current decider's ID - 2.
            previous_decider_id = decider_entity["entity_number"] - 2
            # Wire from previous decider's circuit 2 to current decider's circuit 2 (green wire).
            wires.append([previous_decider_id, 2, decider_entity["entity_number"], 2])
            # Wire from previous decider's circuit 3 to current decider's circuit 3 (red wire).
            wires.append([previous_decider_id, 3, decider_entity["entity_number"], 3])
        first_decider = False

    return new_entities, wires, current_entity_number


def generate_lamps(lamp_signals, grid_width, grid_height,
                   start_entity_number, start_x=0, start_y=0):
    """
    Generate a grid of lamp entities and wiring for a lamp grid.

    - lamp_signals: list of available signals (one per lamp) in row-major order.
    - grid_width: number of lamps horizontally.
    - grid_height: number of lamps vertically.
    - start_entity_number: the first entity number to assign.
    - start_x, start_y: position of the top-left lamp.

    Wiring:
      - A single red-wire connection (circuit 1) runs horizontally across the top row.
      - In each column, a red-wire connection (circuit 1) runs vertically.

    Returns a tuple: (lamp_entities, lamp_wires, next_entity_number).
    """
    lamp_entities = []
    lamp_wires = []
    current_entity = start_entity_number

    # Create lamp entities in row-major order.
    for r in range(grid_height):
        for c in range(grid_width):
            index = r * grid_width + c
            lamp = {
                "entity_number": current_entity,
                "name": "small-lamp",
                "position": {
                    "x": start_x + c,
                    "y": start_y + r
                },
                "control_behavior": {
                    "use_colors": True,
                    "rgb_signal": lamp_signals[index],
                    "color_mode": 2
                },
                "always_on": True
            }
            lamp_entities.append(lamp)
            current_entity += 1

    # Horizontal wiring: Connect adjacent lamps in the top row.
    for c in range(grid_width - 1):
        source = lamp_entities[c]  # top row: r == 0, so index c.
        dest = lamp_entities[c + 1]
        lamp_wires.append([source["entity_number"], 1,
                           dest["entity_number"], 1])
    # Vertical wiring: For each column, connect lamps vertically.
    for c in range(grid_width):
        for r in range(grid_height - 1):
            source = lamp_entities[r * grid_width + c]
            dest = lamp_entities[(r + 1) * grid_width + c]
            lamp_wires.append([source["entity_number"], 1,
                               dest["entity_number"], 1])

    return lamp_entities, lamp_wires, current_entity


def update_full_blueprint(target_fps, frames_filters, lamp_signals,
                          lamp_grid_width, lamp_grid_height):
    """
    Build a new blueprint from scratch using frames_filters for the combinators
    and lamp_signals for a lamp grid. This function discards any existing entities.

    The resulting blueprint will have:
      - Combinator entities and their wires (built first).
      - Lamp entities and their wires (appended afterward).

    Returns the updated blueprint dictionary.
    """
    blueprint = empty_blueprint()

    ticks_per_frame = 60.0 / target_fps
    stop = int(len(frames_filters) * ticks_per_frame)
    timer_entites, timer_wires = generate_timer(stop=stop)

    combinator_entities, combinator_wires, next_entity = generate_frame_combinators(
        frames_filters, ticks_per_frame=ticks_per_frame, base_entity_number=4
    )

    lamp_entities, lamp_wires, final_entity = generate_lamps(
        lamp_signals, lamp_grid_width, lamp_grid_height,
        start_entity_number=next_entity
    )

    # Hacky, remove when we have a better way to do this
    combinator_wires.append([5, 3, next_entity, 1])

    blueprint["blueprint"]["entities"] = timer_entites + combinator_entities + lamp_entities
    blueprint["blueprint"]["wires"] = timer_wires + combinator_wires + lamp_wires

    return blueprint


def empty_blueprint():
    return {
        "blueprint": {
            "icons": [
                {
                    "signal": {
                        "name": "decider-combinator"
                    },
                    "index": 1
                }
            ],
            "entities": [],
            "wires": [],
            "item": "blueprint",
            "version": 562949955518464
        }
    }


def main():
    # JSON file listing available signals. We'll want to eventually ask the user if they want a blueprint for the base game or the DLC.
    # The DLC has more signals, allowing for larger images
    signals_path = "signals.json"
    signals = load_signals(signals_path)
    target_fps = 4

    # We'll want to eventually parameterize the max size with a limit. We should theoretically be able to get up to ~700 pixels tall in the base game
    # and a few thousand with the DLC (including quality)
    downscaled_frames = downscale_gif("input.gif", max_size=30)

    sampled_frames = sample_frames(downscaled_frames, target_fps=target_fps)

    # For each sampled frame, convert its pixels into filters. Make sure that the number
    # of pixels (width * height) does not exceed the number of available signals.
    frames_filters = []
    for frame in sampled_frames:
        try:
            filters = frame_to_filters(frame, signals)
        except ValueError as e:
            print("Error processing frame:", e)
            sys.exit(1)
        frames_filters.append(filters)

    width, height = sampled_frames[0].size
    updated_blueprint = update_full_blueprint(target_fps, frames_filters, signals, width, height)

    # Save the updated blueprint for debugging
    # with open("blueprint.json", "w") as f:
    #     json.dump(updated_blueprint, f, indent=2)

    print(encode_blueprint(updated_blueprint))


if __name__ == "__main__":
    main()
