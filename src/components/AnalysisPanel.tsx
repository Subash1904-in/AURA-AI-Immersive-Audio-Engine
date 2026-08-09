import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { DspParams } from './DSPPanel';
import './AnalysisPanel.css';

export interface AnalysisStateInfo {
  bpm: number;
  genre: string;
  mood_valence: number;
  mood_energy: number;
  is_beat: boolean;
  active_preset: string;
  is_auto_mode: boolean;
  is_onnx_loaded: boolean;
}

export const AnalysisPanel: React.FC = () => {
  const [analysisState, setAnalysisState] = useState<AnalysisStateInfo | null>(null);
  const [dspParams, setDspParams] = useState<DspParams | null>(null);

  useEffect(() => {
    const interval = setInterval(() => {
      fetchAnalysisState();
      fetchDspParams();
    }, 150);

    return () => clearInterval(interval);
  }, []);

  const fetchAnalysisState = async () => {
    try {
      const data: AnalysisStateInfo = await invoke('get_analysis_state');
      setAnalysisState(data);
    } catch (e) {
      // Ignore IPC errors before playback starts
    }
  };

  const fetchDspParams = async () => {
    try {
      const params: DspParams = await invoke('get_dsp_params');
      setDspParams(params);
    } catch (e) {
      // Ignore IPC errors
    }
  };

  const handleToggleAutoMode = async (enabled: boolean) => {
    try {
      await invoke('toggle_auto_mode', { enabled });
      fetchAnalysisState();
      fetchDspParams();
    } catch (e) {
      console.error('Failed to toggle Auto mode:', e);
    }
  };

  const handleToggleBeatModulation = async (enabled: boolean) => {
    try {
      await invoke('toggle_beat_modulation', { enabled });
      fetchDspParams();
    } catch (e) {
      console.error('Failed to toggle Beat Modulation:', e);
    }
  };

  if (!analysisState || !dspParams) {
    return <div className="analysis-loading">Initializing AI Analysis Engine...</div>;
  }

  const genreColors: Record<string, string> = {
    Rock: '#ff5252',
    EDM: '#7c4dff',
    Classical: '#448aff',
    Podcast: '#00e676',
    Lofi: '#ffab40',
    Pop: '#e040fb',
    Unknown: '#90a4ae',
  };

  const activeGenreColor = genreColors[analysisState.genre] || '#90a4ae';

  return (
    <div className="analysis-panel-container">
      <div className="analysis-header">
        <div className="header-left">
          <h2>AI Intelligence & Adaptive Presets</h2>
          <span className="analysis-subtitle">
            Real-time Tempo, Feature Extraction & Data-driven DSP Presets
          </span>
        </div>
        <div className="engine-badge">
          <span
            className={`status-dot ${analysisState.is_onnx_loaded ? 'onnx' : 'heuristic'}`}
          />
          <span className="badge-text">
            {analysisState.is_onnx_loaded ? 'ONNX Active' : 'Heuristic Classifier'}
          </span>
        </div>
      </div>

      <div className="analysis-grid">
        {/* Main AI Mode Toggle Card */}
        <div className="analysis-card mode-card">
          <div className="card-top">
            <div className="mode-title-group">
              <span className="ai-icon">✨</span>
              <h3>Adaptive Listening Mode</h3>
            </div>
            <label className="toggle-switch large">
              <input
                type="checkbox"
                checked={dspParams.is_auto_mode}
                onChange={(e) => handleToggleAutoMode(e.target.checked)}
              />
              <span className="slider-round" />
            </label>
          </div>

          <div className="mode-status">
            <span className="mode-label">CURRENT MODE</span>
            <span className={`mode-value ${dspParams.is_auto_mode ? 'auto' : 'manual'}`}>
              {dspParams.is_auto_mode ? '✨ Auto (AI-Driven)' : '🎛️ Manual Override'}
            </span>
          </div>

          <div className="beat-mod-toggle-row">
            <div className="beat-mod-info">
              <span className="mod-title">Beat-Driven Modulation</span>
              <span className="mod-sub">Transient bass & width boost on beat onsets</span>
            </div>
            <label className="toggle-switch small">
              <input
                type="checkbox"
                checked={dspParams.beat_modulation_enabled}
                onChange={(e) => handleToggleBeatModulation(e.target.checked)}
              />
              <span className="slider-round" />
            </label>
          </div>
        </div>

        {/* Real-time Analysis Readout Card */}
        <div className="analysis-card readout-card">
          {/* BPM & Beat Indicator */}
          <div className="bpm-section">
            <div className="bpm-display">
              <span className="bpm-number">
                {Math.round(analysisState.bpm || 120)}
              </span>
              <span className="bpm-unit">BPM</span>
            </div>
            <div className="beat-indicator-container">
              <div
                className={`beat-pulse ${analysisState.is_beat ? 'pulse-active' : ''}`}
              />
              <span className="beat-text">
                {analysisState.is_beat ? 'BEAT ONSET' : 'TEMPO TRACKING'}
              </span>
            </div>
          </div>

          {/* Genre Badge */}
          <div className="genre-section">
            <span className="section-title">DETECTED GENRE</span>
            <div
              className="genre-badge"
              style={{
                borderColor: activeGenreColor,
                backgroundColor: `${activeGenreColor}18`,
                color: activeGenreColor,
              }}
            >
              {analysisState.genre}
            </div>
          </div>

          {/* Mood Meters */}
          <div className="mood-section">
            <div className="mood-bar-group">
              <div className="mood-label-row">
                <span>Valence (Bright/Chill)</span>
                <span>{Math.round(analysisState.mood_valence * 100)}%</span>
              </div>
              <div className="mood-bar-bg">
                <div
                  className="mood-bar-fill valence"
                  style={{ width: `${analysisState.mood_valence * 100}%` }}
                />
              </div>
            </div>

            <div className="mood-bar-group">
              <div className="mood-label-row">
                <span>Energy (Intensity)</span>
                <span>{Math.round(analysisState.mood_energy * 100)}%</span>
              </div>
              <div className="mood-bar-bg">
                <div
                  className="mood-bar-fill energy"
                  style={{ width: `${analysisState.mood_energy * 100}%` }}
                />
              </div>
            </div>
          </div>

          {/* Active Preset Indicator */}
          <div className="active-preset-banner">
            <span>ACTIVE DSP PRESET: </span>
            <strong>{dspParams.active_preset || 'Manual'}</strong>
          </div>
        </div>
      </div>
    </div>
  );
};
