import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { DspParams } from './DSPPanel';
import './SpatialPanel.css';

export const SpatialPanel: React.FC = () => {
  const [params, setParams] = useState<DspParams | null>(null);

  useEffect(() => {
    fetchParams();
  }, []);

  const fetchParams = async () => {
    try {
      const data: DspParams = await invoke('get_dsp_params');
      setParams(data);
    } catch (e) {
      // Ignore initial IPC error
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
    if (stage === 'spatial') updated.spatial_enabled = enabled;
    if (stage === 'reverb') updated.reverb_enabled = enabled;
    setParams(updated);
    try {
      await invoke('toggle_dsp_stage', { stage, enabled });
    } catch (e) {
      console.error('Failed to toggle stage:', e);
    }
  };

  const handleEnvironmentChange = async (env: string) => {
    if (!params) return;
    const updated = {
      ...params,
      reverb: { ...params.reverb, environment: env },
      reverb_enabled: env !== 'Off',
    };
    setParams(updated);
    try {
      await invoke('set_reverb_environment', { env });
    } catch (e) {
      console.error('Failed to set reverb environment:', e);
    }
  };

  if (!params) {
    return <div className="spatial-loading">Loading Spatial Engine...</div>;
  }

  const environments = ['Off', 'SmallRoom', 'ConcertHall', 'Cathedral', 'Cave'];
  const envLabels: Record<string, string> = {
    Off: '🔇 Off',
    SmallRoom: '🏠 Small Room',
    ConcertHall: '🎵 Concert Hall',
    Cathedral: '⛪ Cathedral',
    Cave: '🪨 Cave',
  };

  return (
    <div className="spatial-panel-container">
      <div className="spatial-header">
        <h2>Spatial Audio & Environment</h2>
        <span className="spatial-subtitle">
          Immersive Spatial Processing & Convolution Reverb
        </span>
      </div>

      <div className="spatial-grid">
        {/* Spatial Audio Controls */}
        <section className={`spatial-stage-card ${params.spatial_enabled ? 'active' : ''}`}>
          <div className="stage-card-header">
            <div className="stage-title">
              <span className="stage-number">6</span>
              <h3>Spatial Audio</h3>
            </div>
            <label className="toggle-switch">
              <input
                type="checkbox"
                checked={params.spatial_enabled}
                onChange={(e) => toggleStage('spatial', e.target.checked)}
              />
              <span className="slider-round" />
            </label>
          </div>

          <div className="control-group">
            <label>Stereo Width: {params.spatial.width.toFixed(1)}</label>
            <div className="width-labels">
              <span>Mono</span>
              <span>Original</span>
              <span>Wide</span>
            </div>
            <input
              type="range"
              min="0.0"
              max="2.0"
              step="0.05"
              value={params.spatial.width}
              onChange={(e) =>
                updateParams({
                  ...params,
                  spatial: { ...params.spatial, width: parseFloat(e.target.value) },
                })
              }
            />
          </div>

          <div className="control-group">
            <label>
              Crossfeed:{' '}
              {params.spatial.crossfeed_level > 0.01
                ? `${Math.round(params.spatial.crossfeed_level * 100)}%`
                : 'Off'}
            </label>
            <input
              type="range"
              min="0.0"
              max="1.0"
              step="0.05"
              value={params.spatial.crossfeed_level}
              onChange={(e) =>
                updateParams({
                  ...params,
                  spatial: {
                    ...params.spatial,
                    crossfeed_level: parseFloat(e.target.value),
                  },
                })
              }
            />
          </div>

          <div className="control-group hrtf-toggle">
            <label>HRTF Binaural</label>
            <label className="toggle-switch small">
              <input
                type="checkbox"
                checked={params.spatial.hrtf_enabled}
                onChange={(e) =>
                  updateParams({
                    ...params,
                    spatial: { ...params.spatial, hrtf_enabled: e.target.checked },
                  })
                }
              />
              <span className="slider-round" />
            </label>
          </div>
        </section>

        {/* Reverb Controls */}
        <section className={`spatial-stage-card ${params.reverb_enabled ? 'active' : ''}`}>
          <div className="stage-card-header">
            <div className="stage-title">
              <span className="stage-number">7</span>
              <h3>Convolution Reverb</h3>
            </div>
            <label className="toggle-switch">
              <input
                type="checkbox"
                checked={params.reverb_enabled}
                onChange={(e) => toggleStage('reverb', e.target.checked)}
              />
              <span className="slider-round" />
            </label>
          </div>

          <div className="control-group">
            <label>Environment</label>
            <div className="env-selector">
              {environments.map((env) => (
                <button
                  key={env}
                  className={`env-btn ${params.reverb.environment === env ? 'selected' : ''}`}
                  onClick={() => handleEnvironmentChange(env)}
                >
                  {envLabels[env] || env}
                </button>
              ))}
            </div>
          </div>

          <div className="control-group">
            <label>
              Wet/Dry Mix: {Math.round(params.reverb.wet_dry_mix * 100)}%
            </label>
            <div className="width-labels">
              <span>Dry</span>
              <span>Balanced</span>
              <span>Wet</span>
            </div>
            <input
              type="range"
              min="0.0"
              max="1.0"
              step="0.01"
              value={params.reverb.wet_dry_mix}
              onChange={(e) =>
                updateParams({
                  ...params,
                  reverb: {
                    ...params.reverb,
                    wet_dry_mix: parseFloat(e.target.value),
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
