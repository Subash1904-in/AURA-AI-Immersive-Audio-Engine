import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { NLEqPanel } from './NLEqPanel';
import '@testing-library/jest-dom';

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, args: any) => mockInvoke(cmd, args),
}));

describe('NLEqPanel Component', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_nl_phrases') {
        return Promise.resolve([
          'cinematic',
          'clearer vocals',
          'more bass',
          'vintage',
          'warmth',
          'bright',
          'spacey',
          'night mode',
        ]);
      }
      if (cmd === 'apply_nl_prompt') {
        return Promise.resolve({
          params: { is_night_mode: false },
          matched_phrases: ['cinematic'],
        });
      }
      if (cmd === 'toggle_night_mode') {
        return Promise.resolve({ is_night_mode: true });
      }
      if (cmd === 'save_settings') {
        return Promise.resolve();
      }
      if (cmd === 'reset_settings') {
        return Promise.resolve({ is_night_mode: false });
      }
      return Promise.resolve({});
    });
  });

  it('renders input field, buttons, and preset chips', async () => {
    render(<NLEqPanel />);
    expect(screen.getByText(/Natural-Language Audio Control/i)).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/more cinematic/i)).toBeInTheDocument();
    expect(screen.getByText(/Apply/i)).toBeInTheDocument();
    expect(screen.getByTitle(/Toggle Smart Night Mode/i)).toBeInTheDocument();

    await waitFor(() => {
      expect(screen.getByText(/cinematic/i)).toBeInTheDocument();
      expect(screen.getByText(/more bass/i)).toBeInTheDocument();
    });
  });

  it('triggers apply_nl_prompt on chip click', async () => {
    render(<NLEqPanel />);
    await waitFor(() => expect(screen.getByText(/cinematic/i)).toBeInTheDocument());

    const chip = screen.getByText(/cinematic/i);
    fireEvent.click(chip);

    expect(mockInvoke).toHaveBeenCalledWith('apply_nl_prompt', { prompt: 'cinematic' });
  });

  it('toggles night mode when clicking Night Mode button', async () => {
    render(<NLEqPanel />);
    const nightBtn = screen.getByTitle(/Toggle Smart Night Mode/i);
    fireEvent.click(nightBtn);

    expect(mockInvoke).toHaveBeenCalledWith('toggle_night_mode', { enabled: true });
  });

  it('calls save_settings on Save button click', async () => {
    render(<NLEqPanel />);
    const saveBtn = screen.getByText(/Save/i);
    fireEvent.click(saveBtn);

    expect(mockInvoke).toHaveBeenCalledWith('save_settings', undefined);
  });
});
