use crate::audio::dsp::params::DspParams;

/// Applies Smart Night Mode parameters over existing DSP parameters.
///
/// Features:
/// (a) Reduces high-frequency harshness via EQ (high shelf -5.0dB at 10kHz).
/// (b) Gentle mid-frequency vocal boost (+2.5dB at 1kHz).
/// (c) Tightens compressor/limiter to prevent sudden loud peaks (threshold -18dB, ratio 6.0, attack 5ms, release 80ms, limiter ceiling -0.5dB).
/// (d) Keeps bass audible at lower overall levels (bass enhancer drive 1.8, mix 0.4, sub-bass low shelf -2.0dB).
pub fn apply_night_mode(params: &mut DspParams, enabled: bool) {
    params.is_night_mode = enabled;

    if enabled {
        params.active_preset = "Night Mode".to_string();

        // (a) & (b) EQ adjustments
        params.eq_enabled = true;
        params.eq.bands[0].gain_db = -2.0; // Dampen sub-bass thump through walls
        params.eq.bands[2].gain_db = 2.5; // Gentle vocal presence boost
        params.eq.bands[4].gain_db = -5.0; // Reduce high-frequency harshness

        // (c) Dynamic peak control
        params.compressor_enabled = true;
        params.compressor.threshold_db = -18.0;
        params.compressor.ratio = 6.0;
        params.compressor.attack_ms = 5.0;
        params.compressor.release_ms = 80.0;
        params.compressor.knee_width_db = 6.0;

        params.limiter_enabled = true;
        params.limiter.ceiling_db = -0.5;
        params.limiter.release_ms = 40.0;

        // (d) Audible bass harmonics at lower overall level
        params.bass_enabled = true;
        params.bass.cutoff_hz = 100.0;
        params.bass.drive = 1.8;
        params.bass.mix = 0.40;
    } else {
        params.active_preset = "Manual".to_string();
    }
}
