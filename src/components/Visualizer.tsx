import { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import * as THREE from 'three';
import './Visualizer.css';

interface VisualizerData {
  magnitudes: number[];
  is_beat: boolean;
  beat_boost: number;
  rms_energy: number;
}

export function Visualizer() {
  const [isEnabled, setIsEnabled] = useState(false);
  const [mode, setMode] = useState<'circular' | 'particles'>('circular');
  const [isFullscreen, setIsFullscreen] = useState(false);

  const canvasRef = useRef<HTMLCanvasElement>(null);
  const dataRef = useRef<VisualizerData>({
    magnitudes: Array(64).fill(0),
    is_beat: false,
    beat_boost: 0,
    rms_energy: 0,
  });

  const rendererRef = useRef<THREE.WebGLRenderer | null>(null);
  const sceneRef = useRef<THREE.Scene | null>(null);
  const cameraRef = useRef<THREE.PerspectiveCamera | null>(null);
  const animationFrameRef = useRef<number | null>(null);

  // References for meshes
  const circularGroupRef = useRef<THREE.Group | null>(null);
  const particlesRef = useRef<{
    mesh: THREE.Mesh;
    direction: THREE.Vector3;
    home: THREE.Vector3;
    randOffset: number;
  }[]>([]);

  // Listen to visualizer state changes
  useEffect(() => {
    invoke('set_visualizer_active', { active: isEnabled }).catch(() => {});
  }, [isEnabled]);

  // Tauri Event Listener for visualizer-data
  useEffect(() => {
    if (!isEnabled) return;

    const unsubscribePromise = listen<VisualizerData>('visualizer-data', (event) => {
      dataRef.current = event.payload;
    });

    return () => {
      unsubscribePromise.then((unsub) => unsub());
    };
  }, [isEnabled]);

  // Set up Three.js scene
  useEffect(() => {
    if (!isEnabled || !canvasRef.current) {
      cleanupThree();
      return;
    }

    const width = canvasRef.current.clientWidth;
    const height = canvasRef.current.clientHeight;

    const scene = new THREE.Scene();
    scene.background = null; // transparent background so CSS gradient shows through
    sceneRef.current = scene;

    const camera = new THREE.PerspectiveCamera(60, width / height, 0.1, 100);
    camera.position.set(0, 5, 12);
    camera.lookAt(0, 0, 0);
    cameraRef.current = camera;

    const renderer = new THREE.WebGLRenderer({
      canvas: canvasRef.current,
      antialias: true,
      alpha: true,
    });
    renderer.setSize(width, height, false);
    rendererRef.current = renderer;

    // Add lighting
    const ambientLight = new THREE.AmbientLight(0xffffff, 0.6);
    scene.add(ambientLight);

    const pointLight = new THREE.PointLight(0x00ffff, 2, 50);
    pointLight.position.set(0, 10, 5);
    scene.add(pointLight);

    const dirLight = new THREE.DirectionalLight(0xff00ff, 1.5);
    dirLight.position.set(5, 5, -5);
    scene.add(dirLight);

    // Build Circular Analyzer Mode
    const circularGroup = new THREE.Group();
    scene.add(circularGroup);
    circularGroupRef.current = circularGroup;

    const barCount = 64;
    const radius = 4;
    const barWidth = 0.25;
    const geometry = new THREE.BoxGeometry(barWidth, 1, barWidth);

    for (let i = 0; i < barCount; i++) {
      const angle = (i / barCount) * Math.PI * 2;
      const x = Math.cos(angle) * radius;
      const z = Math.sin(angle) * radius;

      const material = new THREE.MeshStandardMaterial({
        color: new THREE.Color(),
        roughness: 0.3,
        metalness: 0.1,
      });

      const bar = new THREE.Mesh(geometry, material);
      bar.position.set(x, 0, z);
      bar.rotation.y = -angle + Math.PI / 2;
      circularGroup.add(bar);
    }

    // Build Particle Field Mode
    const particles: typeof particlesRef.current = [];
    const particleCount = 150;
    const particleGeometry = new THREE.SphereGeometry(0.12, 8, 8);

    for (let i = 0; i < particleCount; i++) {
      const theta = Math.random() * Math.PI * 2;
      const phi = Math.acos(Math.random() * 2 - 1);
      const r = 3 + Math.random() * 4;

      const x = r * Math.sin(phi) * Math.cos(theta);
      const y = r * Math.sin(phi) * Math.sin(theta);
      const z = r * Math.cos(phi);

      const home = new THREE.Vector3(x, y, z);
      const direction = home.clone().normalize();

      const colorVal = Math.random();
      const material = new THREE.MeshStandardMaterial({
        color: new THREE.Color().setHSL(colorVal, 0.9, 0.6),
        roughness: 0.2,
      });

      const mesh = new THREE.Mesh(particleGeometry, material);
      mesh.position.copy(home);
      scene.add(mesh);

      particles.push({
        mesh,
        direction,
        home,
        randOffset: Math.random() * 100,
      });
    }
    particlesRef.current = particles;

    // Handle resizing
    const handleResize = () => {
      if (!canvasRef.current || !rendererRef.current || !cameraRef.current) return;
      const w = canvasRef.current.clientWidth;
      const h = canvasRef.current.clientHeight;
      cameraRef.current.aspect = w / h;
      cameraRef.current.updateProjectionMatrix();
      rendererRef.current.setSize(w, h, false);
    };
    window.addEventListener('resize', handleResize);

    // Animation Loop
    let time = 0;
    const animate = () => {
      animationFrameRef.current = requestAnimationFrame(animate);
      time += 0.01;

      const data = dataRef.current;
      const mags = data.magnitudes;
      const beatBoost = data.beat_boost;
      const rms = data.rms_energy;
      const isBeat = data.is_beat;

      if (mode === 'circular') {
        if (circularGroupRef.current) circularGroupRef.current.visible = true;
        particles.forEach((p) => {
          p.mesh.visible = false;
        });

        if (circularGroupRef.current) {
          circularGroupRef.current.rotation.y += 0.003 + beatBoost * 0.04;
          circularGroupRef.current.rotation.x = Math.sin(time * 0.5) * 0.15;
          const targetScale = 1.0 + beatBoost * 0.2;
          circularGroupRef.current.scale.setScalar(
            circularGroupRef.current.scale.x + (targetScale - circularGroupRef.current.scale.x) * 0.25
          );

          circularGroupRef.current.children.forEach((child, idx) => {
            const bar = child as THREE.Mesh;
            const mag = mags[idx] || 0;
            const targetHeight = 0.2 + mag * 4.5;
            bar.scale.y += (targetHeight - bar.scale.y) * 0.2;
            bar.position.y = bar.scale.y / 2 - 0.5;

            const mat = bar.material as THREE.MeshStandardMaterial;
            const hue = (idx / barCount) * 0.75 + time * 0.05;
            mat.color.setHSL(hue % 1.0, 0.9, 0.5 + mag * 0.25);
            mat.emissive.setHSL(hue % 1.0, 0.9, mag * 0.4);
          });
        }
      } else {
        if (circularGroupRef.current) circularGroupRef.current.visible = false;
        particles.forEach((p) => {
          p.mesh.visible = true;
        });

        scene.rotation.y += 0.002 + beatBoost * 0.01;
        scene.rotation.x = Math.sin(time * 0.3) * 0.1;

        particles.forEach((p, idx) => {
          const bin = idx % 64;
          const mag = mags[bin] || 0;

          const floatOffset = Math.sin(time * 1.5 + p.randOffset) * 0.06 * (1.0 + rms * 2.0);
          const targetPos = p.home.clone().addScaledVector(p.direction, floatOffset + mag * 3.5);

          if (isBeat) {
            targetPos.addScaledVector(p.direction, beatBoost * 2.0);
          }

          p.mesh.position.lerp(targetPos, 0.15);

          const targetScale = 0.4 + mag * 3.0;
          p.mesh.scale.setScalar(
            p.mesh.scale.x + (targetScale - p.mesh.scale.x) * 0.2
          );

          const mat = p.mesh.material as THREE.MeshStandardMaterial;
          mat.emissive.setRGB(mag * 0.8, rms * 0.2, beatBoost * 0.5);
        });
      }

      if (rendererRef.current && sceneRef.current && cameraRef.current) {
        rendererRef.current.render(sceneRef.current, cameraRef.current);
      }
    };

    animate();

    return () => {
      window.removeEventListener('resize', handleResize);
      cleanupThree();
    };
  }, [isEnabled, mode]);

  const cleanupThree = () => {
    if (animationFrameRef.current !== null) {
      cancelAnimationFrame(animationFrameRef.current);
      animationFrameRef.current = null;
    }
    if (rendererRef.current) {
      rendererRef.current.dispose();
      rendererRef.current = null;
    }
    if (sceneRef.current) {
      sceneRef.current.traverse((object) => {
        if (object instanceof THREE.Mesh) {
          if (object.geometry) object.geometry.dispose();
          if (Array.isArray(object.material)) {
            object.material.forEach((mat) => mat.dispose());
          } else if (object.material) {
            object.material.dispose();
          }
        }
      });
      sceneRef.current = null;
    }
    cameraRef.current = null;
    circularGroupRef.current = null;
    particlesRef.current = [];
  };

  const handleToggleFullscreen = () => {
    if (!canvasRef.current) return;
    setIsFullscreen(!isFullscreen);
  };

  useEffect(() => {
    const handleResize = () => {
      if (!canvasRef.current || !rendererRef.current || !cameraRef.current) return;
      const w = canvasRef.current.clientWidth;
      const h = canvasRef.current.clientHeight;
      cameraRef.current.aspect = w / h;
      cameraRef.current.updateProjectionMatrix();
      rendererRef.current.setSize(w, h, false);
    };
    
    // Trigger resize layout update
    const timer = setTimeout(handleResize, 100);
    return () => clearTimeout(timer);
  }, [isFullscreen, isEnabled]);

  return (
    <div className={`visualizer-card ${isFullscreen ? 'fullscreen' : ''}`}>
      <div className="panel-header">
        <div className="title-row">
          <h3>3D Immersive Visualizer</h3>
          <span className="visualizer-badge">WebGL</span>
        </div>
        <div className="controls-row">
          <label className="switch-container">
            <input
              type="checkbox"
              checked={isEnabled}
              onChange={(e) => setIsEnabled(e.target.checked)}
            />
            <span className="slider-round" />
            <span className="switch-label">{isEnabled ? 'Active' : 'Disabled'}</span>
          </label>
        </div>
      </div>

      {isEnabled ? (
        <div className="visualizer-stage-wrapper">
          <canvas ref={canvasRef} className="visualizer-canvas" />

          <div className="visualizer-overlay">
            <div className="mode-selector">
              <button
                className={mode === 'circular' ? 'active' : ''}
                onClick={() => setMode('circular')}
              >
                Spectrum Ring
              </button>
              <button
                className={mode === 'particles' ? 'active' : ''}
                onClick={() => setMode('particles')}
              >
                Particle Field
              </button>
            </div>
            <button className="fullscreen-btn" onClick={handleToggleFullscreen} aria-label="Toggle Fullscreen">
              {isFullscreen ? (
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <path d="M4 14h6v6m0-6l-7 7M20 10h-6V4m0 6l7-7" />
                </svg>
              ) : (
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <path d="M8 3H5a2 2 0 00-2 2v3m18 0V5a2 2 0 00-2-2h-3m0 18h3a2 2 0 002-2v-3M3 16v3a2 2 0 002 2h3" />
                </svg>
              )}
            </button>
          </div>
        </div>
      ) : (
        <div className="visualizer-placeholder">
          <div className="visualizer-placeholder-icon">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M2 10v3M6 6v11M10 3v18M14 8v7M18 5v13M22 10v3" />
            </svg>
          </div>
          <p>Visualizer is currently disabled. Toggle it active to render 3D beat-synced visuals.</p>
        </div>
      )}
    </div>
  );
}
