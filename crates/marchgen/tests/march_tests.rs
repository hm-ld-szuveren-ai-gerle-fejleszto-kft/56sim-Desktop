//! March generator test suite (ported invariants from plan Step 5).

use marchgen::{MarchGenerator, Mode, PiecePlan, SAMPLE_RATE};

const BPMS: [u16; 4] = [96, 104, 112, 120];
/// Bars per piece: intro(4) + A + A' + B + A'' (4x8) + outro(2).
const TOTAL_BARS: usize = 38;

/// Exact sample length of one piece: the generator walks an 8th-note grid
/// of `floor(44100 * 60 / bpm / 2)` samples over 38 * 8 steps.
fn piece_samples(bpm: u16) -> usize {
    let step = (SAMPLE_RATE as u64 * 60 / bpm as u64 / 2) as usize;
    step * TOTAL_BARS * 8
}

fn take(n: usize, seed: u64) -> Vec<f32> {
    MarchGenerator::new(seed).take(n).collect()
}

fn rms(window: &[f32]) -> f32 {
    let acc: f64 = window.iter().map(|&s| (s as f64) * (s as f64)).sum();
    ((acc / window.len() as f64).sqrt()) as f32
}

#[test]
fn same_seed_yields_byte_equal_first_two_seconds() {
    let n = 2 * SAMPLE_RATE as usize;
    let a = take(n, 0xDEAD_BEEF_CAFE_1234);
    let b = take(n, 0xDEAD_BEEF_CAFE_1234);
    assert_eq!(a.len(), n);
    assert_eq!(a, b, "two generators on the same seed must be bit-identical");
}

#[test]
fn bpm_and_duration_across_many_seeds() {
    let mut min = f64::MAX;
    let mut max: f64 = 0.0;
    for seed in 0..64u64 {
        let plan = PiecePlan::from_seed(seed);
        assert!(
            BPMS.contains(&plan.bpm),
            "seed {seed}: bpm {} not in {BPMS:?}",
            plan.bpm
        );
        assert_eq!(plan.key_root, (seed % 12) as usize);
        assert_eq!(plan.intro_bars, 4);
        assert_eq!(plan.outro_bars, 2);
        assert_eq!(plan.progression.len(), 4);
        for &deg in &plan.progression {
            assert!((1..=7).contains(&deg), "seed {seed}: degree {deg} out of range");
        }
        let d = plan.duration_secs();
        assert!((45.0..=120.0).contains(&d), "seed {seed}: duration {d} s out of [45, 120]");
        min = min.min(d);
        max = max.max(d);
    }
    // Every bpm bucket is hit within the first 16 seeds (seed % 4).
    let seen: Vec<u16> = (0..16u64).map(|s| PiecePlan::from_seed(s).bpm).collect();
    for bpm in BPMS {
        assert!(seen.contains(&bpm), "bpm {bpm} never selected");
    }
    println!("observed piece duration range over 64 seeds: {min:.3} s .. {max:.3} s");
}

#[test]
fn mode_selects_matching_progression_family() {
    // A major-mode plan must cadence through the major degrees; a minor plan
    // through minor ones. Check the three minor tables and two major tables
    // are reachable and mode-consistent.
    let mut modes = Vec::new();
    for seed in 0..120u64 {
        let plan = PiecePlan::from_seed(seed);
        modes.push(plan.mode);
        if plan.mode == Mode::Major {
            assert!(
                plan.progression == vec![1, 5, 6, 4] || plan.progression == vec![1, 4, 5, 1],
                "seed {seed}: unexpected major progression {:?}",
                plan.progression
            );
        } else {
            assert!(
                plan.progression == vec![1, 7, 6, 5]
                    || plan.progression == vec![1, 4, 7, 1]
                    || plan.progression == vec![1, 5, 6, 5],
                "seed {seed}: unexpected minor progression {:?}",
                plan.progression
            );
        }
    }
    assert!(modes.contains(&Mode::NaturalMinor));
    assert!(modes.contains(&Mode::HarmonicMinor));
    assert!(modes.contains(&Mode::Major));
}

#[test]
fn no_nan_or_inf_over_three_consecutive_pieces() {
    // seed % 4 == 3 => bpm 120 => shortest pieces, quickest test.
    let seed = 3u64;
    let per = piece_samples(120);
    let samples = take(3 * per + 4096, seed);
    for (i, &s) in samples.iter().enumerate() {
        assert!(s.is_finite(), "sample {i} is not finite: {s}");
        assert!((-1.0..=1.0).contains(&s), "sample {i} out of range: {s}");
    }
}

#[test]
fn rms_of_five_second_windows_in_band() {
    let seed = 7u64; // bpm 120, major-ish territory
    let starts_s = [0.0f64, 2.0, 20.0, 45.0, 70.0, 90.0]; // last spans a piece seam
    let end = (starts_s.iter().cloned().fold(0.0f64, f64::max) + 5.0) * SAMPLE_RATE as f64;
    let samples = take(end as usize + 1, seed);
    for start in starts_s {
        let a = (start * SAMPLE_RATE as f64) as usize;
        let b = a + 5 * SAMPLE_RATE as usize;
        let r = rms(&samples[a..b]);
        assert!(
            r > 0.005 && r < 0.45,
            "window {start}..{} s: RMS {r} out of (0.005, 0.45)",
            start + 5.0
        );
        println!("RMS {start:>5.1}..{:.1} s = {r:.4}", start + 5.0);
    }
}

#[test]
fn seam_continuity_max_delta_small() {
    // Sweeping more than two piece boundaries catches every seam: no sample
    // jump may exceed 0.6.
    let seed = 11u64; // bpm 120
    let per = piece_samples(120);
    let samples = take(2 * per + 8192, seed);
    let mut max_delta = 0.0f32;
    for w in samples.windows(2) {
        let d = (w[1] - w[0]).abs();
        if d > max_delta {
            max_delta = d;
        }
    }
    assert!(
        max_delta < 0.6,
        "max |delta| {max_delta} at (or between) piece seams exceeds 0.6"
    );
}

#[test]
fn plan_from_generator_matches_piece_plan_from_seed() {
    for seed in [0u64, 1, 42, 1956, u64::MAX] {
        let gen = MarchGenerator::new(seed);
        assert_eq!(*gen.plan(), PiecePlan::from_seed(seed));
    }
}

#[cfg(feature = "dump")]
#[test]
fn render_wav_writes_valid_16bit_mono_header_and_size() {
    let mut id = std::process::id();
    id = id.wrapping_mul(2654435761) ^ 0x56_53_49_4D;
    let path = std::env::temp_dir().join(format!("marchgen-dump-{id:08x}.wav"));
    marchgen::render_wav(1956, 1, &path).expect("render_wav failed");
    let bytes = std::fs::read(&path).expect("read back wav");
    let _ = std::fs::remove_file(&path);

    let n_samples = SAMPLE_RATE as usize; // 1 second
    assert_eq!(bytes.len(), 44 + n_samples * 2, "file size mismatch");
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    assert_eq!(&bytes[12..16], b"fmt ");
    assert_eq!(&bytes[16..20], &16u32.to_le_bytes(), "fmt chunk size");
    assert_eq!(&bytes[20..22], &1u16.to_le_bytes(), "PCM format");
    assert_eq!(&bytes[22..24], &1u16.to_le_bytes(), "mono");
    assert_eq!(&bytes[24..28], &SAMPLE_RATE.to_le_bytes(), "sample rate");
    assert_eq!(&bytes[28..32], &(SAMPLE_RATE * 2).to_le_bytes(), "byte rate");
    assert_eq!(&bytes[32..34], &2u16.to_le_bytes(), "block align");
    assert_eq!(&bytes[34..36], &16u16.to_le_bytes(), "bits per sample");
    assert_eq!(&bytes[36..40], b"data");
    assert_eq!(&bytes[40..44], &((n_samples * 2) as u32).to_le_bytes(), "data size");
    // RIFF size = 36 + data
    assert_eq!(&bytes[4..8], &(36u32 + (n_samples * 2) as u32).to_le_bytes());
    // Not all-silence (sanity, cheap slice scan).
    assert!(
        bytes[44..].chunks(2).any(|p| p != [0, 0]),
        "rendered WAV is pure silence"
    );
}
