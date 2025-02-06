import base64
import json
import math
import sys
import zlib

from PIL import Image, ImageSequence


def encode_blueprint(bp_dict):
    """
    Encode a Python dictionary as a Factorio blueprint string.
    Blueprint string format: <version><base64(zlib_deflate(JSON))>
    """
    try:
        json_str = json.dumps(bp_dict)
        json_bytes = json_str.encode('utf-8')
        compressor = zlib.compressobj(level=9)
        compressed = compressor.compress(json_bytes) + compressor.flush()
        b64_encoded = base64.b64encode(compressed).decode('utf-8')
        # Prepend the version number ("0")
        return "0" + b64_encoded
    except Exception as e:
        print("Error encoding blueprint:", e)
        raise


def downscale_gif(input_path, max_size=30, output_path=None):
    """
    Downscale a GIF so that the longest side of each frame is at most max_size pixels.
    Returns a list of downscaled PIL Image objects in RGB mode.
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
        frame_resized = frame.resize((new_width, new_height), resample=Image.BILINEAR)
        # Save per-frame duration in info (if needed later)
        frame_resized.info['duration'] = frame.info.get('duration', 100)
        frames.append(frame_resized)

    # If an output path is provided, save the new GIF.
    if output_path:
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
      target_fps: Desired frames per second for the output (default is 4 for now, but we should be able to scale this up quite a bit
      and give users the option to change it so they can sacrifice framerate for blueprint size).

    Returns:
      A list of PIL.Image objects sampled at the target FPS.
    """
    total_frames = len(frames)
    total_time_ms = 0
    for frame in frames:
        total_time_ms += frame.info['duration']

    # Calculate the average duration of a frame, since gifs can have variable frame durations.
    avg_frame_duration = total_time_ms / total_frames
    target_total_frames = int(total_time_ms / 1000 * target_fps)

    sampled_frames = []
    for i in range(target_total_frames):
        target_time = i * (1000 / target_fps)
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
    Removes any entry for "signal-T" (to avoid conflict with our timer signal).
    """
    with open(signals_path, "r") as f:
        signals = json.load(f)
    for i, signal in enumerate(signals):
        if signal["name"] == "signal-T":
            del signals[i]
            break
    return signals


def frame_to_filters(frame, signals_subset):
    """
    Given a PIL image frame and a list of available signals (for this group),
    convert the pixel values into a list of filter dictionaries.
    Each pixel is paired with a signal in row‑major order.
    Raises a ValueError if the number of pixels exceeds the available signals.
    """
    width, height = frame.size
    pixels = list(frame.getdata())
    num_pixels = width * height

    if num_pixels > len(signals_subset):
        raise ValueError(f"Frame pixel count ({num_pixels}) exceeds available signals ({len(signals_subset)}).")

    filters = []
    for i, (pixel, signal) in enumerate(zip(pixels, signals_subset), start=1):
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
    """
    Generate the timer combinators and wires.
    Returns a tuple: (list of timer entities, list of timer wires)
    """
    entities = [
        {
            "entity_number": 1,
            "name": "constant-combinator",
            "position": {"x": -2.5, "y": -4.0},
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
            "position": {"x": -1.5, "y": -4.0},
            "direction": 4,
            "control_behavior": {
                "decider_conditions": {
                    "conditions": [
                        {
                            "first_signal": {"type": "virtual", "name": "signal-T"},
                            "second_signal": {"type": "virtual", "name": "signal-S"},
                            "comparator": "<"
                        }
                    ],
                    "outputs": [
                        {"signal": {"type": "virtual", "name": "signal-T"}}
                    ]
                }
            }
        },
        {
            "entity_number": 3,
            "name": "arithmetic-combinator",
            "position": {"x": -1.5, "y": -3.0},
            "direction": 12,
            "control_behavior": {
                "arithmetic_conditions": {
                    "first_signal": {"type": "virtual", "name": "signal-T"},
                    "second_constant": 1,
                    "operation": "+",
                    "output_signal": {"type": "virtual", "name": "signal-T"}
                }
            }
        }
    ]

    wires = []
    wires.append([1, 1, 2, 1])  # constant to decider in
    wires.append([2, 2, 3, 4])  # decider in to arithmetic out
    wires.append([2, 4, 3, 2])  # arithmetic in to decider out
    return entities, wires


def generate_substations(coverage, full_width, full_height, start_entity_number):
    """
    Generate substations to power the entire blueprint.
    Returns (substation_entities, substation_wires, next_entity_number).
    """
    substation_entities = []
    substation_wires = []
    current_entity = start_entity_number

    num_substations_width = math.ceil((full_width - ((coverage - 2) / 2)) / coverage) + 1
    num_substations_height = math.ceil((full_height - ((coverage - 2) / 2)) / coverage) + 1
    start_x = -1
    start_y = -1

    for i in range(num_substations_height):
        for j in range(num_substations_width):
            substation = {
                "entity_number": current_entity,
                "name": "substation",
                "position": {"x": start_x + j * coverage, "y": start_y + i * coverage}
            }
            substation_entities.append(substation)

            # add a wire to the left and up if there is a substation there
            if i > 0:
                substation_wires.append([current_entity, 5, current_entity - num_substations_width, 5])
            if j > 0:
                substation_wires.append([current_entity, 5, current_entity - 1, 5])

            current_entity += 1


    return substation_entities, substation_wires, current_entity



def generate_frame_combinators(frames_filters,
                               ticks_per_frame=15,
                               base_entity_number=1,
                               base_constant_x=0.5,
                               base_decider_x=1.5,
                               base_y=-3.0):
    """
    For a given group (set of frames) build combinator pairs.
    Each frame gets:
      - A constant combinator holding the frame’s pixel data.
      - A decider combinator activated when signal-T is within a specific tick range.
    Returns: (list of new combinator entities, list of wires, next available entity number)
    """
    new_entities = []
    wires = []
    current_entity_number = base_entity_number
    first_decider = True

    # Build combinator pairs (constant and decider) for each frame.
    for i, filters in enumerate(frames_filters):
        current_y = base_y - i
        constant_entity = {
            "entity_number": current_entity_number,
            "name": "constant-combinator",
            "position": {"x": base_constant_x, "y": current_y},
            "direction": 4,
            "control_behavior": {
                "sections": {
                    "sections": [
                        {"index": 1, "filters": filters}
                    ]
                }
            }
        }
        current_entity_number += 1

        lower_bound = i * ticks_per_frame
        upper_bound = (i + 1) * ticks_per_frame

        decider_entity = {
            "entity_number": current_entity_number,
            "name": "decider-combinator",
            "position": {"x": base_decider_x, "y": current_y},
            "direction": 4,
            "control_behavior": {
                "decider_conditions": {
                    "conditions": [
                        {
                            "first_signal": {"type": "virtual", "name": "signal-T"},
                            "constant": lower_bound,
                            "comparator": ">="
                        },
                        {
                            "first_signal": {"type": "virtual", "name": "signal-T"},
                            "constant": upper_bound,
                            "comparator": "<",
                            "compare_type": "and"
                        }
                    ],
                    "outputs": [
                        {"signal": {"type": "virtual", "name": "signal-everything"}}
                    ]
                }
            }
        }
        current_entity_number += 1

        new_entities.append(constant_entity)
        new_entities.append(decider_entity)
        # Wire constant (circuit 1) to decider (circuit 1).
        wires.append([constant_entity["entity_number"], 1,
                      decider_entity["entity_number"], 1])

        # If not the very first decider, add wires from the previous decider.
        if not first_decider:
            previous_decider_id = decider_entity["entity_number"] - 2
            wires.append([previous_decider_id, 2, decider_entity["entity_number"], 2])
            wires.append([previous_decider_id, 3, decider_entity["entity_number"], 3])
        first_decider = False

    return new_entities, wires, current_entity_number


def generate_lamps(lamp_signals, grid_width, grid_height,
                   start_entity_number, start_x=0, start_y=0):
    """
    Generate a grid of lamp entities and wiring.
    Returns (lamp_entities, lamp_wires, next_entity_number).
    """
    lamp_entities = []
    lamp_wires = []
    current_entity = start_entity_number

    # Create lamp entities in row‑major order.
    for r in range(grid_height):
        for c in range(grid_width):
            index = r * grid_width + c
            lamp = {
                "entity_number": current_entity,
                "name": "small-lamp",
                "position": {"x": start_x + c, "y": start_y + r},
                "control_behavior": {
                    "use_colors": True,
                    "rgb_signal": lamp_signals[index],
                    "color_mode": 2
                },
                "always_on": True
            }
            lamp_entities.append(lamp)
            current_entity += 1

    # Horizontal wiring on top row.
    for c in range(grid_width - 1):
        source = lamp_entities[c]
        dest = lamp_entities[c + 1]
        lamp_wires.append([source["entity_number"], 1,
                           dest["entity_number"], 1])
    # Vertical wiring.
    for c in range(grid_width):
        for r in range(grid_height - 1):
            source = lamp_entities[r * grid_width + c]
            dest = lamp_entities[(r + 1) * grid_width + c]
            lamp_wires.append([source["entity_number"], 1,
                               dest["entity_number"], 1])
    return lamp_entities, lamp_wires, current_entity


def update_full_blueprint(target_fps, sampled_frames, signals):
    """
    Build a new blueprint using multiple groups.
    We break the full image (and frames) into vertical chunks (groups)
    such that each group uses at most len(signals) signals.
    The constant and decider combinators (which take 3 vertical spaces)
    are shifted to the right for each new group.
    """
    blueprint = empty_blueprint()
    full_width, full_height = sampled_frames[0].size

    # Determine how many columns we can have per group.
    max_columns_per_group = len(signals) // full_height
    if max_columns_per_group < 1:
        raise ValueError("Not enough signals for even one column of lamps!")
    num_groups = math.ceil(full_width / max_columns_per_group)
    ticks_per_frame = 60.0 / target_fps
    total_frames = len(sampled_frames)
    stop = int(total_frames * ticks_per_frame)

    # Create timer (only once)
    timer_entities, timer_wires = generate_timer(stop=stop)

    all_entities = timer_entities.copy()
    all_wires = timer_wires.copy()

    next_entity = max(e["entity_number"] for e in timer_entities) + 1
    previous_first_decider_entity = None

    # Place substations
    substation_coverage = 18
    substation_entities, substation_wires, next_entity = generate_substations(substation_coverage, full_width, full_height, next_entity)
    all_entities.extend(substation_entities)
    all_wires.extend(substation_wires)

    # Process each group (vertical stripe).
    for group_index in range(num_groups):
        group_left = group_index * max_columns_per_group
        group_right = min((group_index + 1) * max_columns_per_group, full_width)
        group_width = group_right - group_left

        # For each sampled frame, crop the group and generate filters.
        group_frames_filters = []
        for frame in sampled_frames:
            group_frame = frame.crop((group_left, 0, group_right, full_height))
            # Use only as many signals as needed for this group.
            signals_subset = signals[:group_width * full_height]
            filters = frame_to_filters(group_frame, signals_subset)
            group_frames_filters.append(filters)

        # Compute horizontal offset for this group.
        group_offset_x = group_index * max_columns_per_group

        # Generate combinators for this group.
        group_combinators, group_comb_wires, next_entity = generate_frame_combinators(
            frames_filters=group_frames_filters,
            ticks_per_frame=ticks_per_frame,
            base_entity_number=next_entity,
            base_constant_x=group_offset_x + 0.5,  # shift by group offset
            base_decider_x=group_offset_x + 1.5,
            base_y=-3.0
        )

        # Generate lamps for this group.
        # Use the first group_width * full_height signals.
        group_lamp_signals = signals[:group_width * full_height]
        group_lamps, group_lamp_wires, next_entity = generate_lamps(
            lamp_signals=group_lamp_signals,
            grid_width=group_width,
            grid_height=full_height,
            start_entity_number=next_entity,
            start_x=group_offset_x,
            start_y=0
        )

        # Wire this group’s first decider to the first lamp.
        first_decider_entity = group_combinators[1]["entity_number"]
        first_lamp_entity = group_lamps[0]["entity_number"]
        group_comb_wires.append([first_lamp_entity, 1, first_decider_entity, 3])
        if group_index == 0:
            group_comb_wires.append([2, 4, first_decider_entity, 2])  # decider of timer to first decider of frames

        # TODO: connect the first decider of this group to the first decider of the last group. Input to input on the green wire
        if previous_first_decider_entity:
            group_comb_wires.append([first_decider_entity, 2, previous_first_decider_entity, 2])
        previous_first_decider_entity = first_decider_entity

        # Merge group entities and wires.
        all_entities.extend(group_combinators)
        all_entities.extend(group_lamps)
        all_wires.extend(group_comb_wires)
        all_wires.extend(group_lamp_wires)

    blueprint["blueprint"]["entities"] = all_entities
    blueprint["blueprint"]["wires"] = all_wires

    return blueprint


def empty_blueprint():
    return {
        "blueprint": {
            "icons": [
                {"signal": {"name": "decider-combinator"}, "index": 1}
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

    # Downscale and sample the GIF.
    downscaled_frames = downscale_gif("input.gif", max_size=30)
    sampled_frames = sample_frames(downscaled_frames, target_fps=target_fps)

    # Check that the image’s height is not zero.
    if not sampled_frames:
        print("No frames sampled!")
        sys.exit(1)

    # Build the blueprint using the multi‐group approach.
    updated_blueprint = update_full_blueprint(target_fps, sampled_frames, signals)

    # Uncomment below to save the blueprint JSON for debugging.
    # with open("blueprint.json", "w") as f:
    #     json.dump(updated_blueprint, f, indent=2)

    print(encode_blueprint(updated_blueprint))


if __name__ == "__main__":
    main()
