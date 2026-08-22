import { useEffect, useMemo, useRef, useState } from "react";
import { Button } from "@mui/material";
import * as THREE from "three";
import type { DownloadRecord } from "../lib/ipc";

const maxNodes = 24;
const maxPixelRatio = 1.5;

interface DownloadPlaygroundProps {
  records: DownloadRecord[];
  onOpenDownloads: () => void;
}

interface PlaygroundNode {
  id: string;
  mesh: THREE.Mesh<THREE.BufferGeometry, THREE.MeshBasicMaterial>;
  phase: number;
}

function isPlaygroundRecord(record: DownloadRecord): boolean {
  return record.status === "Queued" || record.status === "Running" || record.status === "Completed";
}

function progress(record: DownloadRecord): number {
  if (!record.total_bytes || record.total_bytes <= 0) return record.status === "Completed" ? 1 : 0.35;
  return Math.min(1, record.bytes_downloaded / record.total_bytes);
}

function nodeColor(record: DownloadRecord): number {
  if (record.status === "Completed") return 0xa7d2b7;
  if (record.status === "Running") return 0xb7cbd0;
  return 0x7f8b93;
}

function nodePosition(index: number, completed: boolean): THREE.Vector3 {
  if (completed) {
    const angle = index * 2.4;
    const radius = 1.15 + (index % 3) * 0.22;
    return new THREE.Vector3(Math.cos(angle) * radius, Math.sin(angle) * radius * 0.65, 0);
  }
  const columns = 5;
  return new THREE.Vector3((index % columns) * 1.15 - 2.3, Math.floor(index / columns) * 0.95 - 1.15, 0);
}

export default function DownloadPlayground({ records, onOpenDownloads }: DownloadPlaygroundProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const recordsRef = useRef<DownloadRecord[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [webglUnavailable, setWebglUnavailable] = useState(false);
  const visibleRecords = useMemo(
    () => records.filter(isPlaygroundRecord).slice(0, maxNodes),
    [records],
  );
  const sceneKey = visibleRecords.map((record) => `${record.id}:${record.status}`).join("|");
  recordsRef.current = visibleRecords;

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const targetCanvas = canvas;

    let renderer: THREE.WebGLRenderer;
    try {
      renderer = new THREE.WebGLRenderer({
        alpha: true,
        antialias: true,
        canvas: targetCanvas,
        powerPreference: "low-power",
      });
    } catch {
      setWebglUnavailable(true);
      return;
    }

    setWebglUnavailable(false);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, maxPixelRatio));
    renderer.setClearColor(0x000000, 0);

    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(35, 1, 0.1, 100);
    camera.position.z = 8;
    const group = new THREE.Group();
    scene.add(group);
    const nodes: PlaygroundNode[] = visibleRecords.map((record, index) => {
      const material = new THREE.MeshBasicMaterial({
        color: nodeColor(record),
        transparent: true,
        opacity: record.status === "Completed" ? 0.92 : 0.78,
      });
      const mesh = new THREE.Mesh(new THREE.IcosahedronGeometry(record.status === "Completed" ? 0.4 : 0.32, 1), material);
      mesh.position.copy(nodePosition(index, record.status === "Completed"));
      group.add(mesh);
      return { id: record.id, mesh, phase: index * 0.8 };
    });
    const raycaster = new THREE.Raycaster();
    const pointer = new THREE.Vector2();
    const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
    let visible = !document.hidden;

    function resize() {
      const width = Math.max(1, targetCanvas.clientWidth);
      const height = Math.max(1, targetCanvas.clientHeight);
      if (targetCanvas.width !== Math.floor(width * renderer.getPixelRatio()) || targetCanvas.height !== Math.floor(height * renderer.getPixelRatio())) {
        renderer.setSize(width, height, false);
        camera.aspect = width / height;
        camera.updateProjectionMatrix();
      }
    }

    function render(time = 0) {
      resize();
      const seconds = time * 0.001;
      const currentRecords = new Map(recordsRef.current.map((record) => [record.id, record]));
      for (const node of nodes) {
        const record = currentRecords.get(node.id);
        if (!record) continue;
        const amount = progress(record);
        node.mesh.scale.setScalar(0.75 + amount * 0.45);
        node.mesh.rotation.x = seconds * 0.18 + node.phase;
        node.mesh.rotation.y = seconds * 0.24 + node.phase;
        node.mesh.position.z = record.status === "Completed" ? 0 : Math.sin(seconds * 1.4 + node.phase) * 0.08;
      }
      renderer.render(scene, camera);
    }

    function setLoop() {
      if (!visible || reducedMotion.matches) {
        renderer.setAnimationLoop(null);
        render();
      } else {
        renderer.setAnimationLoop(render);
      }
    }

    function handleVisibility() {
      visible = !document.hidden;
      setLoop();
    }

    function handlePointerDown(event: PointerEvent) {
      const bounds = targetCanvas.getBoundingClientRect();
      pointer.x = ((event.clientX - bounds.left) / bounds.width) * 2 - 1;
      pointer.y = -((event.clientY - bounds.top) / bounds.height) * 2 + 1;
      raycaster.setFromCamera(pointer, camera);
      const hit = raycaster.intersectObjects(nodes.map((node) => node.mesh))[0];
      setSelectedId(hit ? nodes.find((node) => node.mesh === hit.object)?.id ?? null : null);
    }

    const resizeObserver = new ResizeObserver(resize);
    resizeObserver.observe(targetCanvas);
    document.addEventListener("visibilitychange", handleVisibility);
    targetCanvas.addEventListener("pointerdown", handlePointerDown);
    setLoop();

    return () => {
      renderer.setAnimationLoop(null);
      resizeObserver.disconnect();
      document.removeEventListener("visibilitychange", handleVisibility);
      targetCanvas.removeEventListener("pointerdown", handlePointerDown);
      for (const node of nodes) {
        node.mesh.geometry.dispose();
        node.mesh.material.dispose();
      }
      renderer.dispose();
    };
  }, [sceneKey]);

  const selected = visibleRecords.find((record) => record.id === selectedId);

  return (
    <section className="workspace-panel playground-panel shell-surface" aria-label="download constellation playground">
      <div className="inspector-heading">
        <div>
          <p className="eyebrow">experimental surface</p>
          <h2>download constellation</h2>
        </div>
        <Button variant="outlined" onClick={onOpenDownloads}>open downloads</Button>
      </div>
      <p className="helper-text">a small WebGL study: work grows as it progresses, then settles into a kept cluster.</p>
      <div className="playground-stage">
        {webglUnavailable ? (
          <div className="playground-fallback" role="status">
            <strong>WebGL is unavailable here.</strong>
            <span>The normal download panel remains fully available.</span>
            <Button variant="contained" onClick={onOpenDownloads}>view download state</Button>
          </div>
        ) : (
          <canvas ref={canvasRef} aria-label="download constellation; select a node to inspect its download" />
        )}
        {!webglUnavailable && visibleRecords.length === 0 && <div className="playground-empty">queue a download to give the constellation something to hold.</div>}
      </div>
      <div className="playground-caption" aria-live="polite">
        {selected ? `post #${selected.post_id} · ${selected.status.toLowerCase()} · select open downloads to manage it.` : `${visibleRecords.length} item${visibleRecords.length === 1 ? "" : "s"} represented · click a node to inspect its identity.`}
      </div>
    </section>
  );
}
