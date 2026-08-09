import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { Visualizer } from './Visualizer';
import '@testing-library/jest-dom';

// Mock tauri core/event
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(() => Promise.resolve()),
}));

let mockListenCallback: (event: any) => void = () => {};
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn((_eventName, cb) => {
    mockListenCallback = cb;
    return Promise.resolve(() => {});
  }),
}));

// Mock Three.js
vi.mock('three', () => {
  const Vector3 = class {
    x = 0; y = 0; z = 0;
    constructor(x = 0, y = 0, z = 0) {
      this.x = x; this.y = y; this.z = z;
    }
    set(x: number, y: number, z: number) {
      this.x = x; this.y = y; this.z = z;
      return this;
    }
    clone() { return new Vector3(this.x, this.y, this.z); }
    normalize() { return this; }
    addScaledVector() { return this; }
    lerp() { return this; }
    copy() { return this; }
  };

  const Color = class {
    setHSL() { return this; }
    setRGB() { return this; }
  };

  const MeshStandardMaterial = class {
    color = new Color();
    emissive = new Color();
    emissiveMap = null;
    roughness = 0.5;
    metalness = 0.5;
    dispose() {}
    setRGB() {}
  };

  return {
    Scene: class {
      add() {}
      remove() {}
      traverse(cb: any) {
        cb(new (class {
          geometry = { dispose: () => {} };
          material = new MeshStandardMaterial();
        })());
      }
      rotation = { x: 0, y: 0 };
    },
    PerspectiveCamera: class {
      position = { set: () => {} };
      lookAt() {}
      aspect = 1;
      updateProjectionMatrix() {}
    },
    WebGLRenderer: class {
      setSize() {}
      dispose() {}
      render() {}
      domElement = document.createElement('canvas');
    },
    AmbientLight: class {},
    PointLight: class {
      position = { set: () => {} };
    },
    DirectionalLight: class {
      position = { set: () => {} };
    },
    Group: class {
      add() {}
      children = Array(64).fill(null).map(() => ({
        scale: { x: 1, y: 1, z: 1 },
        position: { x: 0, y: 0, z: 0, set: () => {} },
        material: new MeshStandardMaterial(),
      }));
      scale = { x: 1, y: 1, z: 1, setScalar: () => {} };
      rotation = { x: 0, y: 0, z: 0 };
    },
    BoxGeometry: class {},
    SphereGeometry: class {},
    Mesh: class {
      position = new Vector3();
      scale = { x: 1, y: 1, z: 1, setScalar: () => {} };
      rotation = { x: 0, y: 0, z: 0 };
      material: any;
      geometry: any;
      visible = true;
      constructor(_geo: any, _mat: any) {
        this.geometry = _geo;
        this.material = _mat || new MeshStandardMaterial();
      }
    },
    Vector3,
    Color,
    MeshStandardMaterial,
  };
});

// Mock requestAnimationFrame and cancelAnimationFrame globally on window
(window as any).requestAnimationFrame = (cb: any) => setTimeout(cb, 16) as any;
(window as any).cancelAnimationFrame = (id: any) => clearTimeout(id);

describe('3D Immersive Visualizer', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders placeholder when disabled by default', () => {
    render(<Visualizer />);
    expect(screen.getByText(/Visualizer is currently disabled/i)).toBeInTheDocument();
  });

  it('initializes Three.js and listens to events when toggled active', async () => {
    const { container } = render(<Visualizer />);
    const checkbox = screen.getByRole('checkbox');
    expect(checkbox).not.toBeChecked();

    fireEvent.click(checkbox);
    expect(checkbox).toBeChecked();

    expect(screen.getByText(/Spectrum Ring/i)).toBeInTheDocument();
    expect(screen.getByText(/Particle Field/i)).toBeInTheDocument();
    
    const canvas = container.querySelector('canvas');
    expect(canvas).toBeInTheDocument();
  });

  it('supports mode switching between Spectrum Ring and Particle Field', () => {
    render(<Visualizer />);
    const checkbox = screen.getByRole('checkbox');
    fireEvent.click(checkbox);

    const ringButton = screen.getByText(/Spectrum Ring/i);
    const particleButton = screen.getByText(/Particle Field/i);

    expect(ringButton).toHaveClass('active');
    expect(particleButton).not.toHaveClass('active');

    fireEvent.click(particleButton);
    expect(particleButton).toHaveClass('active');
    expect(ringButton).not.toHaveClass('active');
  });

  it('updates ref data when receiving Tauri event payload', () => {
    render(<Visualizer />);
    const checkbox = screen.getByRole('checkbox');
    fireEvent.click(checkbox);

    const mockPayload = {
      magnitudes: Array(64).fill(0.8),
      is_beat: true,
      beat_boost: 0.9,
      rms_energy: 0.7,
    };

    mockListenCallback({ payload: mockPayload });

    expect(screen.getByText(/Spectrum Ring/i)).toBeInTheDocument();
  });
});
