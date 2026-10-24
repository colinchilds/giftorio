use crate::blueprint::models::{ComparatorString as CompStr, *};
use crate::blueprint::{
    combinator::*, constants::*, data::*, lamp::*, power_pole::*, timer::*, util::*,
};
use crate::image_processing::FrameData;
use crate::progress::set_progress;
use glam::{UVec2, dvec2, uvec2};
use std::{io, sync::Arc};
use wasm_bindgen::JsValue;

/// How many entities to encode in one go. Encoding more entities at once is faster, but uses more memory.
const ENCODE_CHUNK_SIZE: usize = 100;

/// Number of wires to encode per chunk. Since wires generate much less data,
/// we can batch more wires together for better speed.
const WIRE_ENCODE_CHUNK_SIZE: usize = 5000;

/// Data for blueprint generation.
pub struct BlueprintGenerator<'a> {
    /// Copy of the parsed arguments passed in from JS.
    args: &'a BlueprintArgs,
    /// Stores entities, wires, and next entity number.
    entity_data: EntityData,
    /// Stores the iterator that generates the next frame, for streaming output.
    /// May be None if only generating the lamp grid.
    frame_info: Option<FrameInfo<'a>>,
    /// Target dimensions of the image or video.
    dim: UVec2,
    /// Maximum number of lamp columns per group. Groups are always divided split vertically.
    max_cols_per_grp: u32,
    /// Total number of groups to generate. Each group is a vertical split of the lamps, with no data
    /// connections. Timing wires still connect each group.
    n_groups: u32,
    /// Keeps track of positions currently occupied by power poles, and used to request powerage for
    /// generated entities.
    occupied: PowerPoleOccupied,
    /// Whether to generate any combinators or not.
    should_generate_cbs: bool,
    /// List of unique signals to use for the blueprint.
    signals: Box<[Arc<SignalID>]>,
    state: BlueprintState,
}

/// Data only used when generating from an image or a video.
struct FrameInfo<'a> {
    curr_frame: image::DynamicImage,
    frame_data: FrameData<'a>,
    frames_per_cb: u32,
    group_data_combs: Vec<Vec<BlueprintEntity>>,
    n_buf_frames: u32,
    n_scaled_frames: u32,
    next_frame: Option<image::DynamicImage>,
    patch_states: Vec<GroupPatchState>,
    ticks_per_frame: u32,
    use_delta_comp: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum BlueprintState {
    Start,
    Entities,
    Data,
    Wires,
    Finished,
}

struct EntityData {
    ents: Vec<BlueprintEntity>,
    wires: Vec<Wire>,
    next_ent_n: u32,
}
struct GroupPatchState {
    chunk_i: u32,  // which *chunk* we’re patching (not frame)
    comb_i: usize, // Current combinator (we skip empty data)
    frame_buf: Vec<image::DynamicImage>,
    sig_buf: Vec<Vec<i32>>,
}

impl<'a> BlueprintGenerator<'a> {
    pub async fn new(
        data: Option<FrameData<'a>>,
        args: &'a BlueprintArgs,
    ) -> Result<Self, JsValue> {
        let signals = Self::get_signal_data(args);
        let gray_bits = args.grayscale_bits;
        let comb_comp = args.signal_compression;
        let time_comp_win = match comb_comp {
            Some(SignalCompression::Temporal { window }) => window,
            _ => 0,
        };
        let mut frame_info: Option<FrameInfo> = None;
        let dim: UVec2;

        if let Some(mut frame_data) = data {
            let n_scaled_frames = frame_data.total_frames() * if time_comp_win > 0 { 2 } else { 1 };
            let frames_per_cb = 32u32.checked_div(gray_bits).unwrap_or(1);
            let n_buf_frames = match time_comp_win {
                w if w > 0 => (w * args.target_fps).div_ceil(1000 * frames_per_cb),
                _ => 1,
            };
            dim = frame_data.dimensions();
            frame_info = Some(FrameInfo {
                curr_frame: frame_data.next().await?.unwrap(),
                frame_data,
                group_data_combs: Vec::new(),
                frames_per_cb,
                n_buf_frames,
                n_scaled_frames,
                next_frame: None,
                patch_states: Vec::new(),
                ticks_per_frame: 60 / args.target_fps,
                use_delta_comp: comb_comp.is_some_and(|c| c == SignalCompression::Delta),
            });
        } else {
            let w = args.custom_width.expect("Must have width or frame data.");
            let h = args.custom_height.expect("Must have height or frame data.");
            dim = uvec2(w, h);
        }
        let use_const_cbs = matches!(args.mode, Mode::Static { const_cb: true, .. });
        let max_lamp_cols_per_grp = match args.mode {
            Mode::Static { combs: false, .. } => dim.x,
            // Constant combinators can only hold up to 1000 signals
            _ => ((signals.len() as u32).min(if use_const_cbs { 255_000 } else { u32::MAX })
                / dim.y)
                .min(args.max_group_size.unwrap_or(dim.x)),
        };
        if max_lamp_cols_per_grp < 1 {
            return Err(JsValue::from_str(
                "Not enough signals for even one lamp column!",
            ));
        }
        let n_groups = dim.x.div_ceil(max_lamp_cols_per_grp);

        if let Some(frame_info) = frame_info.as_mut() {
            let states_iter = (0..n_groups).map(|_| GroupPatchState {
                chunk_i: 0,
                comb_i: 0,
                frame_buf: Vec::with_capacity(frame_info.frames_per_cb as usize),
                sig_buf: Vec::with_capacity(frame_info.n_buf_frames as usize),
            });
            frame_info.patch_states = states_iter.collect();
        }

        Ok(Self {
            args,
            frame_info,
            entity_data: const {
                EntityData {
                    ents: Vec::new(),
                    wires: Vec::new(),
                    next_ent_n: 0u32,
                }
            },
            dim,
            max_cols_per_grp: max_lamp_cols_per_grp,
            n_groups,
            occupied: PowerPoleOccupied::new(dvec2(0.0, -(args.display_margin_y as f64)), args)?,
            should_generate_cbs: !matches!(
                args.mode,
                Mode::Static { combs: false, .. } | Mode::Lamps | Mode::LampGrid
            ),
            signals,
            state: BlueprintState::Start,
        })
    }

    /// Calculates the required position for the given entity to counteract the rotation of the image.
    fn transform_entity(args: &BlueprintArgs, en: &mut BlueprintEntity) {
        let mut pos = en.position;
        pos = match args.flipped_axes {
            FlippedAxes::X => dvec2(-pos.x, pos.y),
            FlippedAxes::Y => dvec2(pos.x, -pos.y),
            FlippedAxes::Both => dvec2(-pos.x, -pos.y),
            _ => pos,
        };
        pos = match args.image_rotation {
            ImageRotation::Deg90 => dvec2(pos.y, -pos.x),
            ImageRotation::Deg180 => dvec2(-pos.x, -pos.y),
            ImageRotation::Deg270 => dvec2(-pos.y, pos.x),
            _ => pos,
        };

        en.position = pos;
        en.direction = en.direction.map(|v| {
            let mut v = v as i64;
            v += match args.image_rotation {
                ImageRotation::Deg90 => 12,
                ImageRotation::Deg180 => 8,
                ImageRotation::Deg270 => 4,
                _ => 0,
            };
            v += match args.flipped_axes {
                FlippedAxes::X if (v + 4) % 8 == 0 => 8,
                FlippedAxes::Y if v % 8 == 0 => 8,
                FlippedAxes::Both => 8,
                _ => 0,
            };
            (v % 16) as u32
        });
    }

    fn get_signal_data(args: &BlueprintArgs) -> Box<[Arc<SignalID>]> {
        let mut signals = PROTOTYPE_DATA.with_borrow(|v| {
            let signals = &v.as_ref().expect("No signal data loaded yet!").signals;
            if matches!(
                args.mode,
                Mode::LampGrid | Mode::Lamps | Mode::Static { timer: false, .. }
            ) {
                return signals.clone();
            }
            let blacklist_sigs = [
                Arc::clone(&args.timing_signals.f),
                Arc::clone(&args.timing_signals.s),
                Arc::clone(&args.timing_signals.t),
            ]; // Disallow timing signals from being used
            let filtered = signals.iter().filter(|&v| !blacklist_sigs.contains(v));
            filtered.cloned().collect::<Box<_>>()
        });
        match args.signal_sorting {
            // Only sort by name (compressor really likes this)
            SignalSorting::Compression => {
                signals.sort_by(|a, b| a.type_.cmp(&b.type_).then_with(|| a.name.cmp(&b.name)))
            }
            // Sort by total length of the type + name + quality (actual smallest JSON)
            SignalSorting::Json => signals.sort_by_key(|s| {
                s.quality.as_ref().map_or(0, |q| match q.as_ref() {
                    "normal" => -(",\"quality\":\"normal\"".len() as i64),
                    _ => q.to_string().len() as i64,
                }) + match s.type_.as_ref() {
                    "item" => -(",\"type\":\"item\"".len() as i64), // type defaults to "item" if not specified
                    _ => s.type_.len() as i64,
                } + s.name.len() as i64
            }),
            _ => {}
        };
        signals
    }

    pub fn done(&mut self) -> bool {
        self.state == BlueprintState::Finished
    }

    pub async fn next_chunk(&mut self, writer: &mut dyn io::Write) -> Result<bool, JsValue> {
        let old_state = self.state.clone();
        match old_state {
            BlueprintState::Start => {
                write_to(writer, b"{\"blueprint\":{")?;

                let bp_color = match self.args.grayscale_bits {
                    0 => "#8888ff",
                    _ => "#888888",
                };
                let name = self.args.name.clone();
                let name = name.map(|n| format!("{n} ({}x{})", self.dim.x, self.dim.y));
                let bp = BlueprintInner {
                    description: None,
                    icons: Some(vec![Icon {
                        signal: Arc::from(SignalID {
                            type_: Arc::from("item"),
                            name: Arc::clone(&LAMP),
                            quality: None,
                        }),
                        index: 1,
                    }]),
                    item: BLUEPRINT.to_string(),
                    label: Some(match self.args.mode {
                        Mode::Full => name.unwrap_or("GIF".to_owned()),
                        Mode::Lamps => format!("{}x{} Lamps", self.dim.x, self.dim.y),
                        Mode::LampGrid => format!("{}x{} Lamp Grid", self.dim.x, self.dim.y),
                        Mode::Static { .. } => self.args.name.clone().unwrap_or("Image".to_owned()),
                    }),
                    label_color: Color::from_hex(bp_color),
                    version: BLUEPRINT_VERSION,
                    entities: Vec::new(),    // don't serialize
                    wires: Some(Vec::new()), // don't serialize
                };

                write_json_trimmed_to(writer, &bp)?;
                self.state = BlueprintState::Entities;
            }
            BlueprintState::Entities => self.place_entities(writer).await?,
            BlueprintState::Data => {
                if self.should_generate_cbs {
                    self.populate_data(writer).await?;
                } else {
                    self.state = BlueprintState::Wires;
                }
            }
            BlueprintState::Wires => self.make_wires(writer).await?,
            BlueprintState::Finished => return Ok(false),
        }

        // Properly delimit the JSON stream before the next chunks
        if old_state != self.state {
            match self.state {
                BlueprintState::Entities => write_to(writer, b",\"entities\":[")?,
                BlueprintState::Wires => {
                    // Set description of constant combinator
                    if true
                        && let Some(incrementer) = self.entity_data.ents.first()
                        && incrementer.has_tag("incrementer".to_owned())
                    {
                        let desc = &self.get_description();
                        self.entity_data.ents[0] = incrementer.clone().with_description(desc);
                    }

                    for en in self.entity_data.ents.iter() {
                        self.occupied.request(en.position); // Request power poles to power entities
                    }
                    let (ents, wires, _) = self
                        .occupied
                        .generate_power_poles(self.entity_data.next_ent_n, self.args);
                    self.entity_data.ents.extend(ents);
                    self.entity_data.wires.extend(wires);

                    for en in self.entity_data.ents.iter_mut() {
                        Self::transform_entity(self.args, en);
                    }

                    // Write remaining entities. Data combinators were NOT included in self.entity_data.ents
                    for ents in self.entity_data.ents.chunks(ENCODE_CHUNK_SIZE) {
                        write_to(writer, b",")?;
                        write_json_trimmed_to(writer, &ents)?;
                    }

                    write_to(writer, b"],\"wires\":[")?;
                }
                BlueprintState::Finished => {
                    write_to(writer, b"]")?;
                    write_to(writer, b",\"description\":\"")?;
                    write_json_trimmed_to(writer, &self.get_description())?;
                    write_to(writer, b"\"")?;
                    write_to(writer, b"}}")?;
                }
                _ => {}
            }
        }
        Ok(true)
    }

    /// Gets the description to attach to the blueprint.
    fn get_description(&self) -> String {
        // Note: we use the `var` font for bold text as the word "var" is short and takes up less space.

        let name = self.args.name.clone();
        let info = self.frame_info.as_ref();
        let dim = self.dim;

        let mut output: Vec<String> = Vec::new();
        let name = match self.args.mode {
            Mode::LampGrid => format!("{}x{} Lamp Grid", dim.x, dim.y),
            Mode::Lamps => format!("{}x{} Lamps", dim.x, dim.y),
            _ => name.unwrap_or("Blueprint".to_owned()),
        };

        output.extend([
            format!("\n[img=item.small-lamp]  [font=compi]{name}[/font]"),
            "".to_owned(),
            format!("\n[font=var]Dimensions[/font]:    {}x{}", dim.x, dim.y),
        ]);

        if let Some(fd) = info.map(|x| &x.frame_data) {
            let duration = format_duration(fd.curr_duration_ms());
            let color_format = match self.args.grayscale_bits {
                0 => "Full Color (RGB8)",
                1 => "Black & White",
                4 => "4-bit Grayscale (16 colors)",
                8 => "8-bit Grayscale (256 colors)",
                x => &format!("{x}"),
            };
            let n_frames = fd.curr_total_frames();
            let fps = self.args.target_fps;

            output.push(format!("\n[font=var]Color Format[/font]:   {color_format}"));
            if let Some(compression) = self.args.signal_compression {
                let compression = match compression {
                    SignalCompression::Delta => "Delta",
                    SignalCompression::Temporal { window } => &format!("Temporal ({window}ms)"),
                };
                output.push(format!("\n[font=var]Compression[/font]:  {compression}"));
            }
            output.extend([
                format!("\n[font=var]Duration[/font]:   {duration}"),
                format!("\n[font=var]Frames[/font]:     {n_frames} ({fps} FPS)"),
            ]);
        }

        let footer = "Generated using GIFtorio.";
        output.push(format!("\n\n[font=default-small-semibold]{footer}[/font]"));

        output.join("")
    }

    async fn place_entities(&mut self, writer: &mut dyn io::Write) -> Result<(), JsValue> {
        let args = self.args;
        let info = &mut self.frame_info;
        let signals = &mut self.signals;
        let ent_data = &mut self.entity_data;

        if self.should_generate_cbs && !matches!(args.mode, Mode::Static { timer: false, .. }) {
            let info = info.as_mut().unwrap();
            let stop = info.frame_data.total_frames() * info.ticks_per_frame;
            let (mut ents, wires) =
                generate_timer(stop, info.ticks_per_frame, info.frames_per_cb, args);
            ent_data.next_ent_n = ents.iter().map(|e| e.entity_number).max().unwrap_or(0);
            ent_data.next_ent_n += 1;

            for en in ents.iter_mut() {
                en.position.y -= args.display_margin_y as f64;
            }

            ent_data.ents.extend(ents);
            ent_data.wires.extend(wires);
        }
        let mut prev_top_right_lamp_ent_n: Option<u32> = None;
        let n_groups = self.n_groups;

        for group_i in 0..n_groups {
            let is_static = matches!(args.mode, Mode::Static { .. });
            let thresh = if is_static { 0.60 } else { 0.10 };
            set_progress(
                0.00,
                thresh,
                (group_i + 1) as f64 / n_groups as f64,
                &format!("Generating lamps (group {} / {n_groups})", group_i + 1),
            );

            let grp_left = group_i * self.max_cols_per_grp;
            let grp_width = (self.max_cols_per_grp).min(self.dim.x - grp_left);
            let (mut cb_in_ent_n, mut cb_out_ent_n) = (0, 0);
            let (
                ObjectGeneratorOutput {
                    entities: mut grp_lamps,
                    wires: mut grp_lamp_wires,
                    next_ent_n: new_next_ent_n,
                },
                io_ents_n,
            ) = generate_lamps(
                signals,
                uvec2(grp_width, self.dim.y),
                matches!(self.args.mode, Mode::Static { combs: false, .. }).then(
                    || info.as_ref().unwrap().curr_frame.clone(), // Pass image data to lamp generation
                ),
                &mut self.occupied,
                ent_data.next_ent_n,
                dvec2(grp_left as f64, 0.0),
                args,
            );
            let (top_left_lamp_ent_n, top_right_lamp_ent_n) = io_ents_n;
            ent_data.next_ent_n = new_next_ent_n;

            if self.should_generate_cbs {
                let info = info.as_mut().unwrap();
                let GenerateCombinatorsOutput {
                    data_cbs:
                        ObjectGeneratorOutput {
                            entities: data_combs,
                            wires,
                            next_ent_n: new_base_ent_n,
                        },
                    other_ents,
                    in_ent_n,
                    out_ent_n,
                } = generate_combinators(
                    info.n_scaled_frames.div_ceil(info.frames_per_cb),
                    &mut self.occupied,
                    ent_data.next_ent_n,
                    dvec2(grp_left as f64, -(args.display_margin_y as f64)),
                    self.max_cols_per_grp,
                    args,
                )?;
                ent_data.next_ent_n = new_base_ent_n;
                if group_i == 0 {
                    ent_data.wires.push([3, WIRE_OUT_R, in_ent_n, WIRE_R]); // Connect timer
                }
                info.group_data_combs.push(data_combs);
                ent_data.ents.extend(other_ents);
                ent_data.wires.extend(wires);

                (cb_in_ent_n, cb_out_ent_n) = (in_ent_n, out_ent_n);
            }

            if self.should_generate_cbs {
                let wires = &mut ent_data.wires;
                if let Some(top_left_lamp_ent_n) = top_left_lamp_ent_n {
                    if !matches!(args.mode, Mode::Static { timer: false, .. }) {
                        // Timing wire
                        wires.push([top_left_lamp_ent_n, WIRE_R, cb_in_ent_n, WIRE_R]);
                    }

                    // Data wire
                    if args.grayscale_bits == 0
                        && matches!(args.mode, Mode::Static { const_cb: true, .. })
                    {
                        wires.push([top_left_lamp_ent_n, WIRE_G, cb_out_ent_n, WIRE_G]);
                    } else {
                        wires.push([top_left_lamp_ent_n, WIRE_G, cb_out_ent_n, WIRE_OUT_G]);
                    }
                }
            }

            // Connect previous lamps together
            if let (Some(prev_right), Some(prev_left)) =
                (prev_top_right_lamp_ent_n, top_left_lamp_ent_n)
            {
                grp_lamp_wires.push([prev_left, WIRE_R, prev_right, WIRE_R]);
            }
            prev_top_right_lamp_ent_n = top_right_lamp_ent_n;

            for lamp in grp_lamps.iter() {
                self.occupied.request(lamp.position); // Request powerage for lamps
            }
            if group_i > 0 {
                write_to(writer, b",")?;
            }

            for en in grp_lamps.iter_mut() {
                Self::transform_entity(self.args, en);
            }

            write_json_trimmed_to(writer, &grp_lamps)?;
            ent_data.wires.extend(grp_lamp_wires);
        }

        ent_data.ents.sort_by_key(|e| e.entity_number);
        self.state = BlueprintState::Data;

        Ok(())
    }

    async fn populate_data(&mut self, writer: &mut dyn io::Write) -> Result<(), JsValue> {
        let info = self.frame_info.as_mut().unwrap();

        let group_data_combs = &mut info.group_data_combs;
        let frame_data = &mut info.frame_data;
        let signals = &mut self.signals;

        let frames_per_cb = info.frames_per_cb;
        let gray_bits = self.args.grayscale_bits;
        let max_cols_per_grp = self.max_cols_per_grp;
        let n_buf_frames = info.n_buf_frames;
        let n_groups = self.n_groups;
        let ticks_per_frame = info.ticks_per_frame;
        let use_delta_comp = info.use_delta_comp;
        let ticks_per_grp = ticks_per_frame * frames_per_cb;
        let mode = self.args.mode;
        let sig_t = self.args.timing_signals.t.clone();

        let mk_cb = |start: u32, end: u32, outputs: Vec<DeciderCombinatorOutput>| {
            if matches!(mode, Mode::Static { const_cb: true, .. }) {
                let mut sections: Vec<LogisticSection> = Vec::new();
                for (i, f) in outputs.chunks(1000).enumerate() {
                    let filters = f.iter().enumerate().map(|(j, o)| BlueprintLogisticFilter {
                        index: j as u16 + 1,
                        type_: Arc::clone(&o.signal.type_),
                        name: Arc::clone(&o.signal.name),
                        quality: o.signal.quality.clone(),
                        comparator: Some(ComparatorString::Eq),
                        count: o.constant,
                    });
                    let filters = filters.collect::<Vec<_>>();
                    let index = i as u8 + 1;

                    sections.push(LogisticSection { index, filters });
                }
                let sections = LogisticSections { sections };
                ControlBehavior::ConstantCombinator { sections }
            } else {
                ControlBehavior::decider(DeciderCombinatorParameters {
                    conditions: vec![
                        DeciderCombinatorCondition::new(sig_t.clone(), start as i32, CompStr::Ge),
                        DeciderCombinatorCondition::new(sig_t.clone(), end as i32, CompStr::Lt)
                            .with_compare_type(CompareTypeString::And),
                    ],
                    outputs,
                })
            }
        };
        let mut output_ents: Vec<BlueprintEntity> = Vec::with_capacity(ENCODE_CHUNK_SIZE);

        loop {
            info.next_frame = frame_data.next().await?;
            let frame = &info.curr_frame;
            let is_last_frame = info.next_frame.is_none();

            for group_i in 0..n_groups {
                if matches!(self.args.mode, Mode::Static { const_cb: true, .. }) {
                    let thresh = 0.60;
                    set_progress(
                        thresh,
                        0.90,
                        (group_i + 1) as f64 / n_groups as f64,
                        &format!("Generating data (group {} / {n_groups})", group_i + 1),
                    );
                }
                let state = &mut info.patch_states[group_i as usize]; // Individual state for each group of lamps
                let data_combs = &group_data_combs[group_i as usize];
                let frame_sigs: Vec<i32>;

                let grp_left = group_i * max_cols_per_grp;
                let grp_right = ((group_i + 1) * max_cols_per_grp).min(self.dim.x);

                let img = frame.crop_imm(grp_left, 0, grp_right - grp_left, self.dim.y);
                let target_outputs_len = (img.width() * img.height()) as usize;

                if gray_bits > 0 {
                    state.frame_buf.push(img);
                    if state.frame_buf.len() < frames_per_cb as usize && !is_last_frame {
                        continue; // wait until we have a full chunk for this group
                    }
                    frame_sigs = grayscale_frames_to_outputs(&state.frame_buf, gray_bits)?;
                    state.frame_buf.clear();
                } else {
                    frame_sigs = color_frame_to_outputs(&img)?;
                }

                if frame_sigs.len() != target_outputs_len {
                    return Err(JsValue::from_str(&format!(
                        "Outputs length ({}) does not match frame size ({target_outputs_len}).",
                        frame_sigs.len(),
                    )));
                }

                if use_delta_comp && state.sig_buf.is_empty() {
                    state.sig_buf.push(vec![0i32; target_outputs_len]); // Populate first frame with zeros
                } else if !use_delta_comp {
                    state.sig_buf.push(frame_sigs.clone()); // If we're using delta compression we only compare aginst the previous frame
                }

                if state.sig_buf.len() < n_buf_frames as usize && !is_last_frame {
                    continue; // Accumulate until we have `compression_level` outputs,
                }

                if use_delta_comp {
                    // Delta compression branch
                    let prev_outputs = &state.sig_buf[0];
                    let mut target_outputs = Vec::with_capacity(target_outputs_len);

                    for i in 0..target_outputs_len {
                        let v = frame_sigs[i].wrapping_sub(prev_outputs[i]); // Uses overflow to reach any target value.
                        if v != 0 {
                            let signal = Arc::clone(&signals[i]);
                            target_outputs.push(DeciderCombinatorOutput::new(signal, Some(v)));
                        }
                    }
                    if !target_outputs.is_empty() {
                        let en = &data_combs[state.comb_i];
                        let range = uvec2(1, 2) + (ticks_per_grp * state.chunk_i);
                        let cb = mk_cb(range.x, range.y, target_outputs);
                        output_ents.push(en.clone().with_control_behavior(cb));
                        state.sig_buf[0] = frame_sigs; // cache absolute output
                        state.comb_i += 1;
                    }
                } else {
                    // Temporal compression branch
                    let mut changed_mask: Vec<bool> = vec![false; target_outputs_len]; // to find what changed
                    let outputs_0 = &state.sig_buf[0];
                    let base_chunk_i = state.chunk_i * n_buf_frames;

                    // If we cached more than 1 frame, we can apply temporal compression
                    if state.sig_buf.len() > 1 {
                        for comb_outputs in state.sig_buf.iter().skip(1) {
                            for i in 0..target_outputs_len {
                                if !changed_mask[i] && (comb_outputs[i] != outputs_0[i]) {
                                    changed_mask[i] = true;
                                }
                            }
                        }
                    } else {
                        changed_mask.fill(true);
                    }

                    // Handle changed pixels
                    for (cb_i, cb_outputs) in state.sig_buf.iter().enumerate() {
                        let target_i = base_chunk_i + cb_i as u32;
                        let mut target_outputs = Vec::with_capacity(signals.len());

                        // Accumulate only the changed outputs
                        for (i, &v) in cb_outputs.iter().enumerate() {
                            if changed_mask[i] && v != 0 {
                                let signal = Arc::clone(&signals[i]);
                                target_outputs.push(DeciderCombinatorOutput::new(signal, Some(v)));
                            }
                        }
                        if !target_outputs.is_empty() {
                            let en = &data_combs[state.comb_i];
                            let range = (uvec2(0, 1) + target_i) * ticks_per_grp;
                            let cb = mk_cb(range.x, range.y, target_outputs);
                            output_ents.push(en.clone().with_control_behavior(cb));
                            state.comb_i += 1;
                        }
                    }

                    // Handle unchanged pixels
                    if state.sig_buf.len() > 1 {
                        let mut target_outputs = Vec::with_capacity(signals.len());
                        for i in 0..target_outputs_len {
                            if !changed_mask[i] && outputs_0[i] != 0 {
                                let sig = Arc::clone(&signals[i]);
                                target_outputs
                                    .push(DeciderCombinatorOutput::new(sig, Some(outputs_0[i])));
                            }
                        }
                        if !target_outputs.is_empty() {
                            let e = &data_combs[state.comb_i];
                            let range = ticks_per_grp
                                * uvec2(base_chunk_i, base_chunk_i + state.sig_buf.len() as u32);
                            let cb = mk_cb(range.x, range.y, target_outputs);
                            output_ents.push(e.clone().with_control_behavior(cb));
                            state.comb_i += 1;
                        }
                    }
                    state.sig_buf.clear();
                }
                state.chunk_i += 1;
            }

            // Note: Always advance before checking if we have max entities
            if info.next_frame.is_some() {
                info.curr_frame = info.next_frame.take().unwrap();
            } else {
                self.state = BlueprintState::Wires;
                break;
            }
            if output_ents.len() >= ENCODE_CHUNK_SIZE {
                break;
            }
        }

        if !output_ents.is_empty() {
            for en in output_ents.iter_mut() {
                self.occupied.request(en.position); // Request powerage for generated combinators
                Self::transform_entity(self.args, en);
            }
            write_to(writer, b",")?;
            write_json_trimmed_to(writer, &output_ents)?; // Remove braces
        }

        Ok(())
    }

    async fn make_wires(&mut self, writer: &mut dyn io::Write) -> Result<(), JsValue> {
        let all_ents = &mut self.entity_data.ents;
        let all_wires = &mut self.entity_data.wires;
        let n_wires = all_wires.len();

        // Swap all wires if requested (default uses red wires, so swap all for green).
        // Circuit network filters are already handled.
        if !self.args.prefer_green_wires {
            invert_wires(all_ents, all_wires);
        }
        for (i, wires) in all_wires.chunks(WIRE_ENCODE_CHUNK_SIZE).enumerate() {
            let display_i = (i + 1) * WIRE_ENCODE_CHUNK_SIZE;
            set_progress(
                0.90,
                1.00,
                display_i as f64 / n_wires as f64,
                &format!("Generating wires ({display_i} / {n_wires})..."),
            );
            if i > 0 {
                write_to(writer, b",")?;
            }
            write_json_trimmed_to(writer, wires)?;
        }
        self.state = BlueprintState::Finished;

        Ok(())
    }
}
