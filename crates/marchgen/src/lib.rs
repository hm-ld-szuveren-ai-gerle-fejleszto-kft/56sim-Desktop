//! marchgen — endlessly regenerated procedural Soviet military marches.
//!
//! Pure `std`, fully deterministic from a single `u64` seed. One
//! [`MarchGenerator`] yields an endless stream of mono `f32` samples at
//! [`SAMPLE_RATE`]: a march of 38 bars 4/4 — 4-bar snare-roll intro,
//! A(8) A'(8) B(8, subdominant) A''(8), 2-bar cymbal outro — followed by
//! the next piece seeded `splitmix64(prev)`. All synthesis state lives in
//! fixed arrays; the render loop performs no heap allocation.

/// Sample rate of every stream produced by this crate, in Hz (mono).
pub const SAMPLE_RATE: u32 = 44_100;

/// Scales the march melodies are written in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    /// Aeolian minor: 0 2 3 5 7 8 10.
    NaturalMinor,
    /// Minor with a raised leading tone: 0 2 3 5 7 8 11.
    HarmonicMinor,
    /// Ionian major: 0 2 4 5 7 9 11.
    Major,
}

/// Bars per piece: intro(4) + A(8) + A'(8) + B(8) + A''(8) + outro(2).
const INTRO_BARS: usize = 4;
const SECTION_BARS: usize = 8;
const OUTRO_BARS: usize = 2;
const TOTAL_BARS: usize = INTRO_BARS + 4 * SECTION_BARS + OUTRO_BARS; // 38
const STEPS_PER_BAR: usize = 8; // 8th-note grid
const PIECE_STEPS: usize = TOTAL_BARS * STEPS_PER_BAR;

const MODES: [Mode; 3] = [Mode::NaturalMinor, Mode::HarmonicMinor, Mode::Major];
const BPMS: [u16; 4] = [96, 104, 112, 120];
const PROGRESSIONS_MINOR: [[usize; 4]; 3] = [
    [1, 7, 6, 5], // i – VII – VI – V
    [1, 4, 7, 1], // i – iv – VII – i
    [1, 5, 6, 5], // i – v – VI – V
];
const PROGRESSIONS_MAJOR: [[usize; 4]; 2] = [
    [1, 5, 6, 4], // I – V – vi – IV
    [1, 4, 5, 1], // I – IV – V – I
];

const SCALES: [&[u8; 7]; 3] = [
    &[0, 2, 3, 5, 7, 8, 10], // natural minor
    &[0, 2, 3, 5, 7, 8, 11], // harmonic minor
    &[0, 2, 4, 5, 7, 9, 11], // major
];

fn scale_semitones(mode: Mode) -> &'static [u8; 7] {
    match mode {
        Mode::NaturalMinor => SCALES[0],
        Mode::HarmonicMinor => SCALES[1],
        Mode::Major => SCALES[2],
    }
}

/// The compositional blueprint of one piece, fully derived from its seed.
#[derive(Clone, Debug, PartialEq)]
pub struct PiecePlan {
    /// One of 96, 104, 112, 120.
    pub bpm: u16,
    /// Chromatic pitch class of the tonic, `seed % 12`.
    pub key_root: usize,
    /// Melodic mode.
    pub mode: Mode,
    /// Scale degrees (1-based) of the four-bar chord cycle.
    pub progression: Vec<usize>,
    /// Bars of snare-roll intro (always 4).
    pub intro_bars: usize,
    /// Bars of cymbal outro (always 2).
    pub outro_bars: usize,
}

impl PiecePlan {
    /// Deterministically compose the blueprint for `seed`.
    pub fn from_seed(seed: u64) -> Self {
        let bpm = BPMS[(seed % 4) as usize];
        let key_root = (seed % 12) as usize;
        let mode = MODES[((seed / 4) % 3) as usize];
        let progression = if mode == Mode::Major {
            PROGRESSIONS_MAJOR[((seed / 12) % 2) as usize]
        } else {
            PROGRESSIONS_MINOR[((seed / 12) % 3) as usize]
        };
        PiecePlan {
            bpm,
            key_root,
            mode,
            progression: progression.to_vec(),
            intro_bars: INTRO_BARS,
            outro_bars: OUTRO_BARS,
        }
    }

    /// Wall-clock length of one piece in seconds.
    pub fn duration_secs(&self) -> f64 {
        (TOTAL_BARS * 4) as f64 * 60.0 / self.bpm as f64
    }

    /// Scale degree of the chord sounding in `bar` of the piece.
    fn chord_degree(&self, bar: usize) -> usize {
        if bar < INTRO_BARS || bar >= TOTAL_BARS - OUTRO_BARS {
            return 1; // intro is percussion-only; outro rests on the tonic
        }
        let melody_bar = bar - INTRO_BARS;
        let section = melody_bar / SECTION_BARS; // 0 A, 1 A', 2 B, 3 A''
        let local = melody_bar % SECTION_BARS;
        if local >= SECTION_BARS - 2 {
            return 5; // cadence: two bars of the dominant at each section end
        }
        let offset = if section == 2 { 2 } else { 0 }; // B starts off-centre
        self.progression[(local + offset) % self.progression.len()]
    }

    fn root_midi(&self, degree: usize) -> u8 {
        let semi = i32::from(scale_semitones(self.mode)[(degree - 1) % 7]);
        (48 + (40 + self.key_root as i32 + semi - 48).rem_euclid(12)) as u8 // C2..B2
    }

    fn melody_midi(&self, degree: usize) -> u8 {
        let semi = i32::from(scale_semitones(self.mode)[(degree - 1) % 7]);
        (67 + (67 + self.key_root as i32 + semi - 67).rem_euclid(12)) as u8 // G4..F#5
    }
}

/// splitmix64: the piece-to-piece and per-piece random stream.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    fn range(&mut self, lo: usize, hi: usize) -> usize {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() as usize) % (hi - lo)
    }
}

fn next_piece_seed(prev: u64) -> u64 {
    let n = SplitMix64(prev).next_u64();
    if n == 0 {
        u64::MAX / 3
    } else {
        n
    }
}

// ---------------------------------------------------------------------------
// Synthesis
// ---------------------------------------------------------------------------

const VOICE_SILENT: u8 = 0;
const VOICE_TRUMPET: u8 = 1;
const VOICE_TUBA: u8 = 2;
const VOICE_PICCOLO: u8 = 3;
const VOICE_BASS_DRUM: u8 = 4;
const VOICE_SNARE: u8 = 5;
const VOICE_CYMBAL: u8 = 6;
const VOICE_TROMBONE: u8 = 7;

/// Fixed decay in seconds per voice kind; 0 = use the trigger's `hold`.
const VOICE_DECAY: [f32; 8] = [0.0, 0.0, 0.0, 0.40, 0.20, 0.030, 1.5, 0.0];

const MAX_VOICES: usize = 32;
const NOISE_TABLE_SIZE: usize = 4096;

/// One scratch synthesiser voice. Plain data, lives in a fixed pool.
#[derive(Clone, Copy)]
struct Voice {
    kind: u8,
    phase: f32,
    phase_step: f32,
    amp: f32,
    decay: f32,
    elapsed: f32,
    a1: f32,
    z1: f32,
    z2: f32,
    lp_x1: f32,
    noise_idx: usize,
}

const SILENT_VOICE: Voice = Voice {
    kind: VOICE_SILENT,
    phase: 0.0,
    phase_step: 0.0,
    amp: 0.0,
    decay: 0.0,
    elapsed: 0.0,
    a1: 0.0,
    z1: 0.0,
    z2: 0.0,
    lp_x1: 0.0,
    noise_idx: 0,
};

const SQUARE_MIX: f32 = 0.55;
const MASTER_GAIN: f32 = 0.55;
const DOWNBEAT_GAIN: f32 = 1.189_207_1; // +1.5 dB

const G_TRUMPET: f32 = 0.15;
const G_TUBA_ROOT: f32 = 0.20;
const G_TUBA_FIFTH: f32 = 0.14;
const G_TROMBONE: f32 = 0.11;
const G_PICCOLO: f32 = 0.07;
const G_SNARE: f32 = 0.09;
const G_SNARE_ACCENT: f32 = 0.135;
const G_SNARE_ROLL: f32 = 0.055;
const G_SNARE_ROLL_ACCENT: f32 = 0.09;
const G_BASS_DRUM: f32 = 0.45;
const G_CYMBAL: f32 = 0.11;

const TWO_PI: f32 = core::f32::consts::PI * 2.0;
const ATTACK_SECS: f32 = 0.02;

fn midi_hz(midi: u8) -> f32 {
    440.0 * 2.0f32.powf((f32::from(midi) - 69.0) / 12.0)
}

/// Endless procedural march stream: an `Iterator<Item = f32>` of mono
/// samples at [`SAMPLE_RATE`], one piece after the other, every piece fully
/// determined by `splitmix64` iterating from the seed given to [`Self::new`].
pub struct MarchGenerator {
    piece_seed: u64,
    plan: PiecePlan,
    grid: [u8; PIECE_STEPS],
    piccolo_bars: [bool; TOTAL_BARS],
    voices: [Voice; MAX_VOICES],
    noise: [f32; NOISE_TABLE_SIZE],
    step: usize,
    sample_in_step: usize,
    step_len: usize,
    any_voice_on: bool,
}

impl MarchGenerator {
    /// Start an endless march stream whose first piece is composed from
    /// `seed`; subsequent pieces use `splitmix64(prev)` as their seed.
    pub fn new(seed: u64) -> Self {
        let mut gen = MarchGenerator {
            piece_seed: seed,
            plan: PiecePlan::from_seed(seed),
            grid: [0; PIECE_STEPS],
            piccolo_bars: [false; TOTAL_BARS],
            voices: [SILENT_VOICE; MAX_VOICES],
            noise: [0.0; NOISE_TABLE_SIZE],
            step: 0,
            sample_in_step: 0,
            step_len: 0,
            any_voice_on: false,
        };
        gen.setup_piece(seed);
        gen
    }

    /// The blueprint of the piece currently being rendered.
    pub fn plan(&self) -> &PiecePlan {
        &self.plan
    }

    // -- piece setup --------------------------------------------------------

    fn setup_piece(&mut self, seed: u64) {
        self.piece_seed = seed;
        self.plan = PiecePlan::from_seed(seed);
        let mut rng = SplitMix64(seed ^ 0xA5A5_5A5A_1234_5678);
        for slot in self.noise.iter_mut() {
            *slot = rng.next_f64() as f32 * 2.0 - 1.0;
        }
        for bar in 0..TOTAL_BARS {
            self.piccolo_bars[bar] = rng.next_f64() < 0.25;
        }
        self.build_melody(&mut rng);
        // 8th-note in samples; integer division keeps the stream deterministic.
        self.step_len = (SAMPLE_RATE as u64 * 60 / self.plan.bpm as u64 / 2) as usize;
        self.step = 0;
        self.sample_in_step = 0;
    }

    /// Lay the four 8-bar melody sections on the 8th-note grid: A = motif,
    /// A' = identical restatement, B = second motif transposed to the
    /// subdominant, A'' = motif again.
    fn build_melody(&mut self, rng: &mut SplitMix64) {
        self.grid = [0; PIECE_STEPS];
        let a = self.motif(rng);
        let b = self.motif(rng);
        self.fill_section(INTRO_BARS, &a, 0); // A
        self.fill_section(INTRO_BARS + SECTION_BARS, &a, 0); // A'
        self.fill_section(INTRO_BARS + 2 * SECTION_BARS, &b, 5); // B, subdominant
        self.fill_section(INTRO_BARS + 3 * SECTION_BARS, &a, 0); // A''
    }

    /// A 4–8-note chord-tone walk: degrees (1-based), note count, per-note
    /// slot durations (1 = an eighth-note tap).
    fn motif(&self, rng: &mut SplitMix64) -> ([usize; 8], usize, [u8; 8]) {
        let n = rng.range(4, 9);
        let mut degrees = [1usize; 8];
        let mut durs = [1u8; 8];
        for i in 0..n {
            let degree = self.plan.chord_degree(INTRO_BARS + i / 4);
            let chord = [(degree - 1) % 7, (degree + 1) % 7, (degree + 3) % 7];
            degrees[i] = if i % 4 == 0 {
                chord[0] + 1 // chord root on each phrase head
            } else {
                chord[rng.range(0, 3)] + 1
            };
            durs[i] = if rng.next_f64() < 0.4 {
                1
            } else if rng.next_f64() < 0.55 {
                2
            } else {
                4
            };
        }
        (degrees, n, durs)
    }

    fn fill_section(&mut self, start_bar: usize, motif: &([usize; 8], usize, [u8; 8]), transpose: i32) {
        let (degrees, n, durs) = *motif;
        let base = start_bar * STEPS_PER_BAR;
        for phrase in 0..4 {
            let pstart = base + phrase * 16;
            let pend = (pstart + 16).min(PIECE_STEPS);
            let mut pos = pstart;
            let mut prev = i32::from(self.plan.melody_midi(degrees[0])) + transpose;
            for i in 0..n {
                let mut pitch = i32::from(self.plan.melody_midi(degrees[i]));
                while pitch < prev - 16 {
                    pitch += 12;
                }
                while pitch > prev + 9 {
                    pitch -= 12;
                }
                prev = pitch + transpose;
                if pos >= pend {
                    break;
                }
                let note_end = (pos + durs[i] as usize).min(pend);
                while pos < note_end {
                    self.grid[pos] = prev as u8;
                    pos += 1;
                }
            }
            while pos < pend {
                self.grid[pos] = prev as u8; // eighth-note taps of the last note
                pos += 1;
            }
        }
    }

    // -- triggering ---------------------------------------------------------

    fn alloc(&mut self) -> usize {
        for i in 0..MAX_VOICES {
            if self.voices[i].kind == VOICE_SILENT {
                self.voices[i] = SILENT_VOICE;
                return i;
            }
        }
        let mut oldest = 0;
        let mut lowest = f32::MAX;
        for i in 0..MAX_VOICES {
            let a = self.voices[i].amp
                * (-self.voices[i].elapsed / (self.voices[i].decay + 1e-6)).exp();
            if a < lowest {
                lowest = a;
                oldest = i;
            }
        }
        self.voices[oldest] = SILENT_VOICE;
        oldest
    }

    fn trigger(&mut self, kind: u8, freq: f32, amp: f32, hold_secs: f32) {
        let tonal = matches!(
            kind,
            VOICE_TRUMPET | VOICE_TUBA | VOICE_PICCOLO | VOICE_TROMBONE
        );
        if amp <= 0.0 || (tonal && freq <= 0.0) {
            return;
        }
        let decay = VOICE_DECAY[kind as usize];
        let decay = if decay > 0.0 { decay } else { hold_secs };
        let i = self.alloc();
        let a1 = {
            let rc = 1.0 - (-1.0 / (TWO_PI * 2200.0 / SAMPLE_RATE as f32)).exp();
            rc / (1.0 - rc) / 0.7
        };
        let v = &mut self.voices[i];
        v.kind = kind;
        v.phase = 0.0;
        v.phase_step = TWO_PI * freq / SAMPLE_RATE as f32;
        v.amp = amp;
        v.decay = decay.max(1e-4);
        v.elapsed = 0.0;
        v.a1 = a1;
        v.z1 = 0.0;
        v.z2 = 0.0;
        v.lp_x1 = 0.0;
        v.noise_idx = (kind as usize)
            .wrapping_mul(0x9E37_79B1)
            .wrapping_add(self.piece_seed as usize);
        self.any_voice_on = true;
    }

    fn on_step(&mut self) {
        let step = self.step;
        let bar = step / STEPS_PER_BAR;
        let s = step % STEPS_PER_BAR;
        let is_intro = bar < INTRO_BARS;
        let is_outro = bar >= TOTAL_BARS - OUTRO_BARS;
        let downbeat = s == 0;
        let g = if downbeat { DOWNBEAT_GAIN } else { 1.0 };
        let step_secs = self.step_len as f32 / SAMPLE_RATE as f32;

        // Snare: filtered noise on all eighths, beats 1 & 3 accented,
        // rolls in the intro and outro.
        if is_intro || is_outro {
            if s % 2 == 0 {
                let a = if s == 0 { G_SNARE_ROLL_ACCENT } else { G_SNARE_ROLL };
                self.trigger(VOICE_SNARE, 0.0, a * g, 0.030);
            }
        } else {
            let a = match s {
                0 | 4 => G_SNARE_ACCENT,
                2 | 6 => G_SNARE,
                _ => G_SNARE * 0.6,
            };
            self.trigger(VOICE_SNARE, 0.0, a * g, 0.030);
        }

        // Bass drum: 55 Hz decaying sine + click on beats 1 & 3 (the dut).
        if s == 0 || s == 4 {
            self.trigger(VOICE_BASS_DRUM, 55.0, G_BASS_DRUM * g, 0.0);
        }

        if is_intro {
            return; // the intro is snare roll + dut only
        }

        let degree = self.plan.chord_degree(bar);
        let root = self.plan.root_midi(degree);
        let tuba_secs = 2.0 * step_secs;
        if s == 0 || s == 4 {
            self.trigger(VOICE_TUBA, midi_hz(root), G_TUBA_ROOT * g, tuba_secs);
        } else if s == 2 || s == 6 {
            self.trigger(VOICE_TUBA, midi_hz(root + 7), G_TUBA_FIFTH, tuba_secs);
        }
        if s == 2 || s == 6 {
            // Trombone chord stabs on beats 2 & 4.
            self.trigger(VOICE_TROMBONE, midi_hz(root + 12), G_TROMBONE, 0.22);
            self.trigger(VOICE_TROMBONE, midi_hz(root + 16), G_TROMBONE * 0.85, 0.22);
            self.trigger(VOICE_TROMBONE, midi_hz(root + 19), G_TROMBONE * 0.8, 0.22);
        }
        if !is_outro {
            let pitch = self.grid[step];
            if pitch > 0 {
                self.trigger(VOICE_TRUMPET, midi_hz(pitch), G_TRUMPET, 1.25 * step_secs);
            }
            if (s == 4 || s == 6) && self.piccolo_bars[bar] && pitch > 0 {
                self.trigger(VOICE_PICCOLO, midi_hz(pitch + 24), G_PICCOLO, 0.0);
            }
        }
        if is_outro && s == 0 {
            self.trigger(VOICE_CYMBAL, 0.0, G_CYMBAL * g, 0.0);
        }
    }

    // -- rendering ----------------------------------------------------------

    fn render_sample(&mut self) -> f32 {
        if self.step >= PIECE_STEPS {
            self.setup_piece(next_piece_seed(self.piece_seed));
        }
        if self.sample_in_step == 0 {
            self.on_step();
        }
        let mut out = 0.0f32;
        if self.any_voice_on {
            let mut any = false;
            for i in 0..MAX_VOICES {
                let v = &mut self.voices[i];
                if v.kind == VOICE_SILENT {
                    continue;
                }
                let mut env = (-(v.elapsed / v.decay)).exp();
                if matches!(
                    v.kind,
                    VOICE_TRUMPET | VOICE_TUBA | VOICE_PICCOLO | VOICE_TROMBONE
                ) {
                    env *= (v.elapsed / ATTACK_SECS).min(1.0);
                }
                v.elapsed += 1.0 / SAMPLE_RATE as f32;
                if env < 1e-4 {
                    v.kind = VOICE_SILENT;
                    continue;
                }
                any = true;
                let raw = match v.kind {
                    VOICE_TRUMPET => {
                        // square + low-pass 2.2 kHz Q0.7, vibrato 5 Hz 0.3 %
                        let f = v.phase_step * (1.0 + 0.003 * (TWO_PI * 5.0 * v.elapsed).sin());
                        v.phase += f;
                        if v.phase >= TWO_PI {
                            v.phase -= TWO_PI;
                        }
                        let square = if v.phase < core::f32::consts::PI { 1.0 } else { -1.0 };
                        v.z1 += v.a1 * (square - v.z1);
                        v.z2 += v.a1 * (v.z1 - v.z2);
                        SQUARE_MIX * square + (1.0 - SQUARE_MIX) * v.z2
                    }
                    VOICE_TUBA | VOICE_PICCOLO | VOICE_TROMBONE => {
                        // sine + 3rd/5th harmonics
                        v.phase += v.phase_step;
                        if v.phase >= TWO_PI {
                            v.phase -= TWO_PI;
                        }
                        let s1 = v.phase.sin();
                        let s3 = s1 * (3.0 - 4.0 * s1 * s1);
                        let s1sq = s1 * s1;
                        let s5 = s1 * (16.0 * s1sq * s1sq - 20.0 * s1sq + 5.0);
                        s1 + 0.30 * s3 + 0.15 * s5
                    }
                    VOICE_BASS_DRUM => {
                        v.phase += v.phase_step;
                        if v.phase >= TWO_PI {
                            v.phase -= TWO_PI;
                        }
                        let click = if v.elapsed < 0.002 {
                            self.noise[v.noise_idx & (NOISE_TABLE_SIZE - 1)]
                        } else {
                            0.0
                        };
                        v.noise_idx = v.noise_idx.wrapping_add(1);
                        v.phase.sin() + click
                    }
                    VOICE_SNARE | VOICE_CYMBAL => {
                        let n = self.noise[v.noise_idx & (NOISE_TABLE_SIZE - 1)];
                        v.noise_idx = v.noise_idx.wrapping_add(1);
                        let coeff = if v.kind == VOICE_CYMBAL { 0.06 } else { 0.35 };
                        let lp = v.lp_x1 + coeff * (n - v.lp_x1); // one-pole LP
                        v.lp_x1 = lp;
                        n - lp // complement = high-passed noise
                    }
                    _ => 0.0,
                };
                out += v.amp * env * raw;
            }
            self.any_voice_on = any;
        }
        self.sample_in_step += 1;
        if self.sample_in_step >= self.step_len {
            self.sample_in_step = 0;
            self.step += 1;
        }
        (MASTER_GAIN * out).tanh()
    }
}

impl Iterator for MarchGenerator {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        Some(self.render_sample())
    }
}

// ---------------------------------------------------------------------------
// WAV dump (debug flag `--dump-march`)
// ---------------------------------------------------------------------------

#[cfg(feature = "dump")]
fn wav_header(data_bytes: u32) -> [u8; 44] {
    let mut h = [0u8; 44];
    h[0..4].copy_from_slice(b"RIFF");
    h[4..8].copy_from_slice(&(36 + data_bytes).to_le_bytes());
    h[8..12].copy_from_slice(b"WAVE");
    h[12..16].copy_from_slice(b"fmt ");
    h[16..20].copy_from_slice(&16u32.to_le_bytes());
    h[20..22].copy_from_slice(&1u16.to_le_bytes()); // PCM
    h[22..24].copy_from_slice(&1u16.to_le_bytes()); // mono
    h[24..28].copy_from_slice(&SAMPLE_RATE.to_le_bytes());
    h[28..32].copy_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    h[32..34].copy_from_slice(&2u16.to_le_bytes());
    h[34..36].copy_from_slice(&16u16.to_le_bytes());
    h[36..40].copy_from_slice(b"data");
    h[40..44].copy_from_slice(&data_bytes.to_le_bytes());
    h
}

/// Render `seconds` of the endless march stream for `seed` to a 16-bit PCM
/// mono WAV file at `path` (hand-rolled header, no dependencies).
#[cfg(feature = "dump")]
pub fn render_wav(seed: u64, seconds: u32, path: &std::path::Path) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let n = seconds as usize * SAMPLE_RATE as usize;
    let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);
    file.write_all(&wav_header((n * 2) as u32))?;
    let mut chunk = vec![0u8; 8192 * 2];
    let mut gen = MarchGenerator::new(seed);
    let mut done = 0;
    while done < n {
        let take = core::cmp::min(8192, n - done);
        for i in 0..take {
            let s = gen.next().unwrap_or(0.0).clamp(-1.0, 1.0);
            let q = (s * i16::MAX as f32).round() as i16;
            chunk[2 * i..2 * i + 2].copy_from_slice(&q.to_le_bytes());
        }
        file.write_all(&chunk[..take * 2])?;
        done += take;
    }
    file.flush()
}
