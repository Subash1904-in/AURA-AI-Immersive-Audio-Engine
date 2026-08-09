import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import './StemPanel.css';

export interface StemParams {
  vocals_gain: number;
  vocals_mute: boolean;
  drums_gain: number;
  drums_mute: boolean;
  bass_gain: number;
  bass_mute: boolean;
  other_gain: number;
  other_mute: boolean;
  stems_active: boolean;
  stems_ready: boolean;
}

interface ProgressPayload {
  job_id: string;
  hash: string;
  progress: number;
  cache_hit: boolean;
  status: 'processing' | 'completed' | 'error';
  message: string;
}

export const StemPanel: React.FC = () => {
  const [params, setParams] = useState<StemParams | null>(null);
  const [progress, setProgress] = useState<number>(0);
  const [status, setStatus] = useState<'idle' | 'processing' | 'completed' | 'error'>('idle');
  const [cacheHit, setCacheHit] = useState<boolean>(false);
  const [message, setMessage] = useState<string>('');

  useEffect(() => {
    fetchParams();

    // Poll parameters occasionally to keep UI in sync
    const interval = setInterval(fetchParams, 300);

    let unlisten: (() => void) | null = null;
    const setupListener = async () => {
      try {
        unlisten = await listen<ProgressPayload>('separation-progress', (event) => {
          const payload = event.payload;
          setProgress(payload.progress);
          setCacheHit(payload.cache_hit);
          setStatus(payload.status);
          setMessage(payload.message);

          if (payload.status === 'completed') {
            fetchParams();
          }
        });
      } catch (err) {
        console.error('Failed to setup separation progress listener:', err);
      }
    };

    setupListener();

    return () => {
      clearInterval(interval);
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  const fetchParams = async () => {
    try {
      const dsp: any = await invoke('get_dsp_params');
      setParams({
        vocals_gain: dsp.vocals_gain,
        vocals_mute: dsp.vocals_mute,
        drums_gain: dsp.drums_gain,
        drums_mute: dsp.drums_mute,
        bass_gain: dsp.bass_gain,
        bass_mute: dsp.bass_mute,
        other_gain: dsp.other_gain,
        other_mute: dsp.other_mute,
        stems_active: dsp.stems_active,
        stems_ready: dsp.stems_ready,
      });

      // If stems are ready and we are idle/processing, set completed
      if (dsp.stems_ready && status === 'idle') {
        setStatus('completed');
        setCacheHit(true);
      }
    } catch (e) {
      // Ignore
    }
  };

  const updateParam = async (key: keyof StemParams, value: any) => {
    if (!params) return;
    const updated = { ...params, [key]: value };
    setParams(updated);

    try {
      if (key === 'stems_active') {
        await invoke('set_stems_active', { active: value });
      } else if (key.endsWith('_gain')) {
        const stem = key.replace('_gain', '');
        await invoke('set_stem_gain', { stem, gain: value });
      } else if (key.endsWith('_mute')) {
        const stem = key.replace('_mute', '');
        await invoke('set_stem_mute', { stem, mute: value });
      }
    } catch (err) {
      console.error('Failed to update stem parameter:', err);
    }
  };

  if (!params) {
    return <div className="stem-loading">Initializing Stem Mixer State...</div>;
  }

  const stemsList = [
    { key: 'vocals', label: 'Vocals', icon: '🎤', color: '#e040fb' },
    { key: 'drums', label: 'Drums', icon: '🥁', color: '#ff5252' },
    { key: 'bass', label: 'Bass', icon: '🎸', color: '#7c4dff' },
    { key: 'other', label: 'Other', icon: '🎹', color: '#448aff' },
  ];

  return (
    <div className="stem-panel-container">
      <div className="stem-header">
        <div className="header-left">
          <h2>AI Stem Separation & Real-time Mixer</h2>
          <span className="stem-subtitle">
            Demix audio tracks into independent vocal, drum, bass, and instrumental channels
          </span>
        </div>
        <div className="badge-group">
          {params.stems_ready && (
            <span className={`badge ${cacheHit ? 'cache-hit' : 'processed'}`}>
              {cacheHit ? '⚡ Cached Stems' : '⚙️ Separated'}
            </span>
          )}
          <span className={`status-badge ${status}`}>
            {status === 'processing' && `Processing (${Math.round(progress * 100)}%)`}
            {status === 'completed' && 'Ready'}
            {status === 'error' && 'Inference Failed'}
            {status === 'idle' && 'No Track Loaded'}
          </span>
        </div>
      </div>

      {status === 'processing' && (
        <div className="separation-progress-card">
          <div className="progress-top">
            <span className="spinner">⌛</span>
            <span className="progress-msg">{message || 'Splitting audio frequencies...'}</span>
            <span className="progress-pct">{Math.round(progress * 100)}%</span>
          </div>
          <div className="progress-bar-bg">
            <div className="progress-bar-fill" style={{ width: `${progress * 100}%` }} />
          </div>
        </div>
      )}

      {status === 'error' && (
        <div className="error-card">
          <span className="error-icon">❌</span>
          <span className="error-msg">{message || 'Separation failed. Playing original track.'}</span>
        </div>
      )}

      <div className="mixer-mode-toggle-card">
        <div className="mode-toggle-left">
          <span className="mixer-icon">🎛️</span>
          <div className="mode-toggle-text">
            <h4>Activate Stem Mixer</h4>
            <p>Seamlessly transition between original stereo mix and 4-stem mixing</p>
          </div>
        </div>
        <label className="toggle-switch large">
          <input
            type="checkbox"
            checked={params.stems_active}
            disabled={!params.stems_ready}
            onChange={(e) => updateParam('stems_active', e.target.checked)}
          />
          <span className="slider-round" />
        </label>
      </div>

      <div className={`stems-mixer-grid ${params.stems_active ? 'mixer-active' : 'mixer-disabled'}`}>
        {stemsList.map((stem) => {
          const gainKey = `${stem.key}_gain` as keyof StemParams;
          const muteKey = `${stem.key}_mute` as keyof StemParams;
          const gain = params[gainKey] as number;
          const mute = params[muteKey] as boolean;

          return (
            <div key={stem.key} className={`stem-channel-strip ${mute ? 'muted' : ''}`} style={{ '--stem-color': stem.color } as React.CSSProperties}>
              <div className="stem-info">
                <span className="stem-icon">{stem.icon}</span>
                <span className="stem-name">{stem.label}</span>
              </div>

              <div className="fader-container">
                <input
                  type="range"
                  min="0.0"
                  max="1.0"
                  step="0.01"
                  value={gain}
                  disabled={!params.stems_active}
                  onChange={(e) => updateParam(gainKey, parseFloat(e.target.value))}
                />
                <span className="gain-value">{Math.round(gain * 100)}%</span>
              </div>

              <button
                className={`mute-btn ${mute ? 'active-mute' : ''}`}
                disabled={!params.stems_active}
                onClick={() => updateParam(muteKey, !mute)}
              >
                MUTE
              </button>
            </div>
          );
        })}
      </div>
    </div>
  );
};
