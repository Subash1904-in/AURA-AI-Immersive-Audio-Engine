import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './DSPPanel.css';

export interface EqBand {
  enabled: boolean;
  filter_type: 'LowShelf' | 'Peaking' | 'HighShelf';
  frequency: number;
  gain_db: number;
  q: number;
}

export interface EqParams {
  bands: EqBand[];
}

export interface BassEnhancerParams {
  cutoff_hz: number;
  drive: number;
  mix: number;
}

export interface CompressorParams {
  threshold_db: number;
  ratio: number;
  attack_ms: number;
  release_ms: number;
  knee_width_db: number;
}

export interface LoudnessParams {
  target_lufs: number;
  max_gain_db: number;
}

export interface LimiterParams {
  ceiling_db: number;
  release_ms: number;
}

export interface DspParams {
  eq_enabled: boolean;
  eq: EqParams;
  bass_enabled: boolean;
  bass: BassEnhancerParams;
  compressor_enabled: boolean;
  compressor: CompressorParams;
  loudness_enabled: boolean;
  loudness: LoudnessParams;
  limiter_enabled: boolean;
  limiter: LimiterParams;
}

export const DSPPanel: React.FC = () => {
  const [params, setParams] = useState<DspParams | null>(null);

  useEffect(() => {
    fetchParams();
  }, []);

  const fetchParams = async () => {
    try {
      const data: DspParams = await invoke('get_dsp_params');
      setParams(data);
    } catch (e) {
      // Ignore initial IPC error if audio thread is initializing
    }
  };

  const updateParams = async (newParams: DspParams) => {
    setParams(newParams);
    try {
      await invoke('set_dsp_params', { params: newParams });
    } catch (e) {
      console.error('Failed to set DSP params:', e);
    }
  };

  const toggleStage = async (stage: string, enabled: boolean) => {
    if (!params) return;
    const updated = { ...params };
    if (stage === 'eq') updated.eq_enabled = enabled;
    if (stage === 'bass') updated.bass_enabled = enabled;
    if (stage === 'compressor') updated.compressor_enabled = enabled;
    if (stage === 'loudness') updated.loudness_enabled = enabled;
    if (stage === 'limiter') updated.limiter_enabled = enabled;

    setParams(updated);
    try {
      await invoke('toggle_dsp_stage', { stage, enabled });
    } catch (e) {
      console.error('Failed to toggle stage:', e);
    }
  };

  if (!params) {
    return <div className="dsp-loading">Loading DSP Chain Engine...</div>;
  }

  return (
    <div className="dsp-panel-container">
      <div className="dsp-header">
        <h2>Real-Time Manual DSP Chain</h2>
        <span className="dsp-subtitle">5-Stage Sequential Audio Processing</span>
      </div>

      <div className="dsp-grid">
        {/* Stage 1: Parametric EQ */}
        <section className={`dsp-stage-card ${params.eq_enabled ? 'active' : ''}`}>
          <div className="stage-card-header">
            <div className="stage-title">
              <span className="stage-number">1</span>
              <h3>Parametric EQ (5 Bands)</h3>
            </div>
            <label className="toggle-switch">
              <input
                type="checkbox"
                checked={params.eq_enabled}
                onChange={(e) => toggleStage('eq', e.target.checked)}
              />
              <span className="slider-round" />
            </label>
          </div>

          <div className="eq-bands-grid">
            {params.eq.bands.map((band, idx) => (
              <div key={idx} className="eq-band-control">
                <span className="band-label">
                  B{idx + 1} ({Math.round(band.frequency)}Hz)
                </span>
                <input
                  type="range"
                  min="-12"
                  max="12"
                  step="0.5"
                  value={band.gain_db}
                  onChange={(e) => {
                    const newBands = [...params.eq.bands];
                    newBands[idx].gain_db = parseFloat(e.target.value);
                    updateParams({
                      ...params,
                      eq: { ...params.eq, bands: newBands },
                    });
                  }}
                />
                <span className="band-val">
                  {band.gain_db > 0 ? `+${band.gain_db}` : band.gain_db} dB
                </span>
              </div>
            ))}
          </div>
        </section>

        {/* Stage 2: Bass Enhancer */}
        <section className={`dsp-stage-card ${params.bass_enabled ? 'active' : ''}`}>
          <div className="stage-card-header">
            <div className="stage-title">
              <span className="stage-number">2</span>
              <h3>Bass Enhancer</h3>
            </div>
            <label className="toggle-switch">
              <input
                type="checkbox"
                checked={params.bass_enabled}
                onChange={(e) => toggleStage('bass', e.target.checked)}
              />
              <span className="slider-round" />
            </label>
          </div>

          <div className="control-group">
            <label>Cutoff Frequency: {params.bass.cutoff_hz} Hz</label>
            <input
              type="range"
              min="40"
              max="250"
              value={params.bass.cutoff_hz}
              onChange={(e) =>
                updateParams({
                  ...params,
                  bass: { ...params.bass, cutoff_hz: parseFloat(e.target.value) },
                })
              }
            />
          </div>

          <div className="control-group">
            <label>Harmonic Drive: {params.bass.drive.toFixed(1)}x</label>
            <input
              type="range"
              min="0.5"
              max="4.0"
              step="0.1"
              value={params.bass.drive}
              onChange={(e) =>
                updateParams({
                  ...params,
                  bass: { ...params.bass, drive: parseFloat(e.target.value) },
                })
              }
            />
          </div>

          <div className="control-group">
            <label>Wet Mix: {Math.round(params.bass.mix * 100)}%</label>
            <input
              type="range"
              min="0.0"
              max="1.0"
              step="0.05"
              value={params.bass.mix}
              onChange={(e) =>
                updateParams({
                  ...params,
                  bass: { ...params.bass, mix: parseFloat(e.target.value) },
                })
              }
            />
          </div>
        </section>

        {/* Stage 3: Compressor */}
        <section className={`dsp-stage-card ${params.compressor_enabled ? 'active' : ''}`}>
          <div className="stage-card-header">
            <div className="stage-title">
              <span className="stage-number">3</span>
              <h3>Soft-Knee Compressor</h3>
            </div>
            <label className="toggle-switch">
              <input
                type="checkbox"
                checked={params.compressor_enabled}
                onChange={(e) => toggleStage('compressor', e.target.checked)}
              />
              <span className="slider-round" />
            </label>
          </div>

          <div className="control-group">
            <label>Threshold: {params.compressor.threshold_db} dB</label>
            <input
              type="range"
              min="-40"
              max="0"
              value={params.compressor.threshold_db}
              onChange={(e) =>
                updateParams({
                  ...params,
                  compressor: {
                    ...params.compressor,
                    threshold_db: parseFloat(e.target.value),
                  },
                })
              }
            />
          </div>

          <div className="control-group">
            <label>Ratio: {params.compressor.ratio.toFixed(1)}:1</label>
            <input
              type="range"
              min="1"
              max="20"
              step="0.5"
              value={params.compressor.ratio}
              onChange={(e) =>
                updateParams({
                  ...params,
                  compressor: {
                    ...params.compressor,
                    ratio: parseFloat(e.target.value),
                  },
                })
              }
            />
          </div>

          <div className="control-group">
            <label>Attack: {params.compressor.attack_ms} ms</label>
            <input
              type="range"
              min="0.5"
              max="100"
              value={params.compressor.attack_ms}
              onChange={(e) =>
                updateParams({
                  ...params,
                  compressor: {
                    ...params.compressor,
                    attack_ms: parseFloat(e.target.value),
                  },
                })
              }
            />
          </div>
        </section>

        {/* Stage 4: Loudness Normalizer */}
        <section className={`dsp-stage-card ${params.loudness_enabled ? 'active' : ''}`}>
          <div className="stage-card-header">
            <div className="stage-title">
              <span className="stage-number">4</span>
              <h3>Loudness Normalizer (BS.1770)</h3>
            </div>
            <label className="toggle-switch">
              <input
                type="checkbox"
                checked={params.loudness_enabled}
                onChange={(e) => toggleStage('loudness', e.target.checked)}
              />
              <span className="slider-round" />
            </label>
          </div>

          <div className="control-group">
            <label>Target Loudness: {params.loudness.target_lufs} LUFS</label>
            <input
              type="range"
              min="-24"
              max="-10"
              value={params.loudness.target_lufs}
              onChange={(e) =>
                updateParams({
                  ...params,
                  loudness: {
                    ...params.loudness,
                    target_lufs: parseFloat(e.target.value),
                  },
                })
              }
            />
          </div>

          <div className="control-group">
            <label>Max Gain Adjustment: +{params.loudness.max_gain_db} dB</label>
            <input
              type="range"
              min="0"
              max="18"
              value={params.loudness.max_gain_db}
              onChange={(e) =>
                updateParams({
                  ...params,
                  loudness: {
                    ...params.loudness,
                    max_gain_db: parseFloat(e.target.value),
                  },
                })
              }
            />
          </div>
        </section>

        {/* Stage 5: Brick-Wall Limiter */}
        <section className={`dsp-stage-card ${params.limiter_enabled ? 'active' : ''}`}>
          <div className="stage-card-header">
            <div className="stage-title">
              <span className="stage-number">5</span>
              <h3>Brick-Wall Limiter</h3>
            </div>
            <label className="toggle-switch">
              <input
                type="checkbox"
                checked={params.limiter_enabled}
                onChange={(e) => toggleStage('limiter', e.target.checked)}
              />
              <span className="slider-round" />
            </label>
          </div>

          <div className="control-group">
            <label>Ceiling: {params.limiter.ceiling_db.toFixed(1)} dBFS</label>
            <input
              type="range"
              min="-3.0"
              max="0.0"
              step="0.1"
              value={params.limiter.ceiling_db}
              onChange={(e) =>
                updateParams({
                  ...params,
                  limiter: {
                    ...params.limiter,
                    ceiling_db: parseFloat(e.target.value),
                  },
                })
              }
            />
          </div>

          <div className="control-group">
            <label>Release Time: {params.limiter.release_ms} ms</label>
            <input
              type="range"
              min="5"
              max="200"
              value={params.limiter.release_ms}
              onChange={(e) =>
                updateParams({
                  ...params,
                  limiter: {
                    ...params.limiter,
                    release_ms: parseFloat(e.target.value),
                  },
                })
              }
            />
          </div>
        </section>
      </div>
    </div>
  );
};
