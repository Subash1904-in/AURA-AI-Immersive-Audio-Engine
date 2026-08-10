import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { AnalysisPanel } from './components/AnalysisPanel';
import { StemPanel } from './components/StemPanel';
import { DSPPanel } from './components/DSPPanel';
import { SpatialPanel } from './components/SpatialPanel';
import { Visualizer } from './components/Visualizer';
import { NLEqPanel } from './components/NLEqPanel';
import './App.css';

interface TrackInfo {
  file_path: string;
  title: string;
  duration_ms: number;
  sample_rate: number;
  channels: number;
}

interface PlaybackStateInfo {
  is_playing: boolean;
  current_position_ms: number;
  duration_ms: number;
  track: TrackInfo | null;
}

interface ToastMessage {
  id: string;
  type: 'error' | 'info' | 'success';
  message: string;
}

export function App() {
  const [track, setTrack] = useState<TrackInfo | null>(null);
  const [isPlaying, setIsPlaying] = useState<boolean>(false);
  const [positionMs, setPositionMs] = useState<number>(0);
  const [durationMs, setDurationMs] = useState<number>(0);
  const [isSeeking, setIsSeeking] = useState<boolean>(false);
  const [seekValue, setSeekValue] = useState<number>(0);
  const [toasts, setToasts] = useState<ToastMessage[]>([]);

  const showToast = (message: string, type: 'error' | 'info' | 'success' = 'error') => {
    const id = Math.random().toString(36).substring(2, 9);
    setToasts((prev) => [...prev, { id, type, message }]);
    setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, 5000);
  };

  const removeToast = (id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  };

  // Poll position every 100ms
  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const state: PlaybackStateInfo = await invoke('get_position');
        setIsPlaying(state.is_playing);
        if (!isSeeking) {
          setPositionMs(state.current_position_ms);
          setSeekValue(state.current_position_ms);
        }
        if (state.duration_ms > 0) {
          setDurationMs(state.duration_ms);
        }
        if (state.track) {
          setTrack(state.track);
        }
      } catch (err: any) {
        // Silently ignore IPC polling error
      }
    }, 100);

    return () => clearInterval(interval);
  }, [isSeeking]);

  const handleOpenFile = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: 'Audio Files',
            extensions: ['mp3', 'flac', 'wav', 'm4a', 'aac', 'ogg'],
          },
        ],
      });

      if (selected && typeof selected === 'string') {
        const loadedTrack: TrackInfo = await invoke('load_file', { path: selected });
        setTrack(loadedTrack);
        setPositionMs(0);
        setSeekValue(0);
        setDurationMs(loadedTrack.duration_ms);
        // Automatically start playing on file load
        await invoke('play');
        setIsPlaying(true);
        showToast(`Loaded: ${loadedTrack.title}`, 'success');
      }
    } catch (err: any) {
      showToast(typeof err === 'string' ? err : err?.message || 'Failed to load file', 'error');
    }
  };

  const handlePlayPause = async () => {
    if (!track) {
      showToast('Please open an audio file first', 'info');
      return;
    }

    try {
      if (isPlaying) {
        await invoke('pause');
        setIsPlaying(false);
      } else {
        await invoke('play');
        setIsPlaying(true);
      }
    } catch (err: any) {
      showToast(typeof err === 'string' ? err : err?.message || 'Playback error', 'error');
    }
  };

  const handleSeekChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setSeekValue(Number(e.target.value));
  };

  const handleSeekMouseDown = () => {
    setIsSeeking(true);
  };

  const handleSeekMouseUp = async (e: React.SyntheticEvent) => {
    setIsSeeking(false);
    const targetMs = Number((e.target as HTMLInputElement).value);
    setPositionMs(targetMs);
    try {
      await invoke('seek', { ms: targetMs });
    } catch (err: any) {
      showToast(typeof err === 'string' ? err : err?.message || 'Seek failed', 'error');
    }
  };

  const formatTime = (ms: number) => {
    const totalSeconds = Math.floor(ms / 1000);
    const mins = Math.floor(totalSeconds / 60);
    const secs = totalSeconds % 60;
    return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
  };

  return (
    <div className="aura-container">
      {/* Background Glow */}
      <div className="glow-bg" />

      {/* App Header */}
      <header className="aura-header">
        <div className="brand">
          <div className="logo-icon">
            <div className="bar bar-1" />
            <div className="bar bar-2" />
            <div className="bar bar-3" />
          </div>
          <div className="brand-text">
            <h1>AURA</h1>
            <span className="subtitle">AI Immersive Audio Engine</span>
          </div>
        </div>
        <button className="open-btn" onClick={handleOpenFile}>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M9 13h6m-3-3v6m-9 1V7a2 2 0 012-2h6l2 2h6a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2z" />
          </svg>
          Open Audio File
        </button>
      </header>

      {/* Main Track Display */}
      <main className="aura-content">
        <div className="track-card">
          <div className="artwork-placeholder">
            <div className={`disc ${isPlaying ? 'spinning' : ''}`}>
              <div className="disc-center" />
            </div>
          </div>

          <div className="track-details">
            <h2 className="track-title">{track ? track.title : 'No Audio File Loaded'}</h2>
            <div className="track-meta">
              {track ? (
                <>
                  <span className="badge">{track.sample_rate} Hz</span>
                  <span className="badge">{track.channels === 1 ? 'Mono' : 'Stereo'}</span>
                  <span className="track-path" title={track.file_path}>
                    {track.file_path}
                  </span>
                </>
              ) : (
                <span className="placeholder-text">
                  Select an MP3, FLAC, or WAV file to begin playback.
                </span>
              )}
            </div>
          </div>
        </div>

        {/* Transport & Control Section */}
        <section className="player-controls">
          <div className="seek-bar-container">
            <span className="time-display">{formatTime(isSeeking ? seekValue : positionMs)}</span>
            <input
              type="range"
              min={0}
              max={durationMs || 100}
              value={isSeeking ? seekValue : positionMs}
              onChange={handleSeekChange}
              onMouseDown={handleSeekMouseDown}
              onMouseUp={handleSeekMouseUp}
              onTouchStart={handleSeekMouseDown}
              onTouchEnd={handleSeekMouseUp}
              className="seek-slider"
              disabled={!track}
            />
            <span className="time-display">{formatTime(durationMs)}</span>
          </div>

          <div className="button-row">
            <button
              className={`play-btn ${isPlaying ? 'playing' : ''}`}
              onClick={handlePlayPause}
              disabled={!track}
              aria-label={isPlaying ? 'Pause' : 'Play'}
            >
              {isPlaying ? (
                <svg viewBox="0 0 24 24" fill="currentColor">
                  <rect x="6" y="4" width="4" height="16" rx="1" />
                  <rect x="14" y="4" width="4" height="16" rx="1" />
                </svg>
              ) : (
                <svg viewBox="0 0 24 24" fill="currentColor">
                  <path d="M8 5v14l11-7z" />
                </svg>
              )}
            </button>
          </div>
        </section>

        {/* 3D Visualizer Panel */}
        <Visualizer />

        {/* Natural-Language EQ & Persistence Panel */}
        <NLEqPanel />

        {/* AI Analysis & Adaptive Presets Panel */}
        <AnalysisPanel />

        {/* AI Source Separation Panel */}
        <StemPanel />

        {/* Manual DSP Chain Control Panel */}
        <DSPPanel />

        {/* Spatial Audio & Environment Panel */}
        <SpatialPanel />
      </main>

      {/* Toast Notification Container */}
      <div className="toast-container">
        {toasts.map((toast) => (
          <div key={toast.id} className={`toast toast-${toast.type}`}>
            <span className="toast-message">{toast.message}</span>
            <button className="toast-close" onClick={() => removeToast(toast.id)}>
              ✕
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}

export default App;
