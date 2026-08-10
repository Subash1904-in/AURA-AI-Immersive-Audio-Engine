import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './NLEqPanel.css';

interface NLEqResult {
  params: any;
  matched_phrases: string[];
}

export function NLEqPanel() {
  const [prompt, setPrompt] = useState('');
  const [availablePhrases, setAvailablePhrases] = useState<string[]>([
    'cinematic',
    'clearer vocals',
    'more bass',
    'vintage',
    'warmth',
    'bright',
    'spacey',
    'night mode',
  ]);
  const [matchedPhrases, setMatchedPhrases] = useState<string[]>([]);
  const [isNightMode, setIsNightMode] = useState(false);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);

  useEffect(() => {
    invoke<string[]>('get_nl_phrases')
      .then((phrases) => {
        if (phrases && phrases.length > 0) {
          setAvailablePhrases(phrases);
        }
      })
      .catch(() => {});
  }, []);

  const handleApplyPrompt = async (textToApply?: string) => {
    const query = textToApply || prompt;
    if (!query.trim()) return;

    try {
      const res = await invoke<NLEqResult>('apply_nl_prompt', { prompt: query });
      setMatchedPhrases(res.matched_phrases);
      if (res.params && res.params.is_night_mode !== undefined) {
        setIsNightMode(res.params.is_night_mode);
      }
      setStatusMessage(`Applied prompt: "${query}"`);
      setTimeout(() => setStatusMessage(null), 3000);
    } catch (err: any) {
      setStatusMessage(`Error: ${err.toString()}`);
    }
  };

  const handleChipClick = (phrase: string) => {
    setPrompt(phrase);
    handleApplyPrompt(phrase);
  };

  const handleToggleNightMode = async () => {
    const nextState = !isNightMode;
    try {
      const updatedParams = await invoke<any>('toggle_night_mode', { enabled: nextState });
      setIsNightMode(updatedParams.is_night_mode);
      setStatusMessage(nextState ? 'Night Mode Enabled' : 'Night Mode Disabled');
      setTimeout(() => setStatusMessage(null), 3000);
    } catch (err: any) {
      setStatusMessage(`Error: ${err.toString()}`);
    }
  };

  const handleSaveSettings = async () => {
    try {
      await invoke('save_settings');
      setStatusMessage('Settings saved to ~/.aura/config.json');
      setTimeout(() => setStatusMessage(null), 3000);
    } catch (err: any) {
      setStatusMessage(`Save Error: ${err.toString()}`);
    }
  };

  const handleResetSettings = async () => {
    try {
      const res = await invoke<any>('reset_settings');
      if (res && res.is_night_mode !== undefined) {
        setIsNightMode(res.is_night_mode);
      }
      setMatchedPhrases([]);
      setPrompt('');
      setStatusMessage('Settings reset to defaults');
      setTimeout(() => setStatusMessage(null), 3000);
    } catch (err: any) {
      setStatusMessage(`Reset Error: ${err.toString()}`);
    }
  };

  return (
    <div className="nl-eq-card">
      <div className="panel-header">
        <div className="title-row">
          <h3>Natural-Language Audio Control</h3>
          <span className="nl-badge">AI Assistant</span>
        </div>
        <div className="header-actions">
          <button
            className={`night-mode-toggle ${isNightMode ? 'active' : ''}`}
            onClick={handleToggleNightMode}
            title="Toggle Smart Night Mode"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M21 12.79A9 9 0 1111.21 3 7 7 0 0021 12.79z" />
            </svg>
            <span>{isNightMode ? 'Night Mode ON' : 'Night Mode'}</span>
          </button>
          <button className="save-btn" onClick={handleSaveSettings} title="Save preferences to config file">
            Save
          </button>
          <button className="reset-btn" onClick={handleResetSettings} title="Reset to defaults">
            Reset
          </button>
        </div>
      </div>

      <div className="nl-input-group">
        <input
          type="text"
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && handleApplyPrompt()}
          placeholder="Type e.g. 'more cinematic', 'clearer vocals', 'more bass'..."
          className="nl-prompt-input"
        />
        <button className="apply-prompt-btn" onClick={() => handleApplyPrompt()}>
          Apply
        </button>
      </div>

      {statusMessage && <div className="status-toast">{statusMessage}</div>}

      <div className="chips-section">
        <span className="chips-label">Quick Prompts:</span>
        <div className="chips-row">
          {availablePhrases.map((phrase) => (
            <button
              key={phrase}
              className={`preset-chip ${matchedPhrases.includes(phrase) ? 'matched' : ''}`}
              onClick={() => handleChipClick(phrase)}
            >
              {phrase}
            </button>
          ))}
        </div>
      </div>

      {matchedPhrases.length > 0 && (
        <div className="matched-tags-row">
          <span className="matched-label">Active Preset Modifiers:</span>
          {matchedPhrases.map((tag) => (
            <span key={tag} className="active-tag">
              ✓ {tag}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
