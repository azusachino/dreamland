import { useEffect, useMemo, useRef, useState } from "react";
import { Button } from "@mui/material";
import {
  BufferGeometry,
  Group,
  IcosahedronGeometry,
  Mesh,
  MeshBasicMaterial,
  PerspectiveCamera,
  Raycaster,
  RingGeometry,
  Scene,
  Vector2,
  Vector3,
  WebGLRenderer,
} from "three";
import type { DownloadRecord, Post } from "../lib/ipc";

const maxNodes = 24;
const maxPixelRatio = 1.5;

function demoRecord(id: string, postId: string, status: "Queued" | "Running" | "Completed", bytesDownloaded: number, totalBytes: number | null): DownloadRecord {
  const metadata: Post = {
    post: { site: "demo", id: postId },
    tags: ["dreamland_preview"],
    author: "demo",
    creator_id: null,
    md5: null,
    source: null,
    parent_id: null,
    has_children: false,
    created_at: null,
    width: 1200,
    height: 900,
    rating: "Safe",
    score: null,
    preview_url: null,
    sample_url: null,
    full_url: null,
    file_size: totalBytes,
  };
  return {
    id,
    site: "demo",
    post_id: postId,
    variant: "Sample",
    status,
    target_path: null,
    error: null,
    attempts: status === "Queued" ? 0 : 1,
    bytes_downloaded: bytesDownloaded,
    total_bytes: totalBytes,
    created_at_ms: 0,
    updated_at_ms: 0,
    metadata,
  };
}

const demoRecords: DownloadRecord[] = [
  demoRecord("demo-running", "running", "Running", 4_800_000, 12_000_000),
  demoRecord("demo-queued", "queued", "Queued", 0, null),
  demoRecord("demo-kept-a", "kept-a", "Completed", 9_000_000, 9_000_000),
  demoRecord("demo-kept-b", "kept-b", "Completed", 7_500_000, 7_500_000),
];

interface DownloadPlaygroundProps {
  records: DownloadRecord[];
  onOpenDownloads: () => void;
}

interface PlaygroundNode {
  id: string;
  mesh: Mesh<BufferGeometry, MeshBasicMaterial>;
  halo: Mesh<BufferGeometry, MeshBasicMaterial>;
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

function nodePosition(index: number, completed: boolean): Vector3 {
  if (completed) {
    const angle = index * 2.4;
    const radius = 1.15 + (index % 3) * 0.22;
    return new Vector3(Math.cos(angle) * radius, Math.sin(angle) * radius * 0.65, 0);
  }
  const columns = 5;
  return new Vector3((index % columns) * 1.15 - 2.3, Math.floor(index / columns) * 0.95 - 1.15, 0);
}

export default function DownloadPlayground({ records, onOpenDownloads }: DownloadPlaygroundProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const recordsRef = useRef<DownloadRecord[]>([]);
  const selectedIdRef = useRef<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [demoMode, setDemoMode] = useState(() => window.location.hash.includes("demo=1"));
  const [webglUnavailable, setWebglUnavailable] = useState(false);
  const sourceRecords = demoMode ? demoRecords : records;
  const visibleRecords = useMemo(
    () => sourceRecords.filter(isPlaygroundRecord).slice(0, maxNodes),
    [sourceRecords],
  );
  const sceneKey = visibleRecords.map((record) => `${record.id}:${record.status}`).join("|");
  recordsRef.current = visibleRecords;
  selectedIdRef.current = selectedId;

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const targetCanvas = canvas;

    let renderer: WebGLRenderer;
    try {
      renderer = new WebGLRenderer({
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

    const scene = new Scene();
    const camera = new PerspectiveCamera(35, 1, 0.1, 100);
    camera.position.z = 8;
    const group = new Group();
    scene.add(group);
    const nodes: PlaygroundNode[] = visibleRecords.map((record, index) => {
      const material = new MeshBasicMaterial({
        color: nodeColor(record),
        transparent: true,
        opacity: record.status === "Completed" ? 0.92 : 0.78,
      });
      const haloMaterial = new MeshBasicMaterial({
        color: nodeColor(record),
        transparent: true,
        opacity: 0.2,
      });
      const mesh = new Mesh(new IcosahedronGeometry(record.status === "Completed" ? 0.4 : 0.32, 1), material);
      const halo = new Mesh(new RingGeometry(0.44, 0.5, 32), haloMaterial);
      mesh.position.copy(nodePosition(index, record.status === "Completed"));
      halo.position.copy(mesh.position);
      halo.position.z -= 0.03;
      group.add(halo);
      group.add(mesh);
      return { id: record.id, mesh, halo, phase: index * 0.8 };
    });
    const raycaster = new Raycaster();
    const pointer = new Vector2();
    const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
    const hasActiveRecords = visibleRecords.some((record) => record.status === "Queued" || record.status === "Running");
    const frameInterval = 1000 / 30;
    let lastFrame = -Infinity;
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
      if (time - lastFrame < frameInterval) return;
      lastFrame = time;
      resize();
      const seconds = time * 0.001;
      const currentRecords = new Map(recordsRef.current.map((record) => [record.id, record]));
      for (const node of nodes) {
        const record = currentRecords.get(node.id);
        if (!record) continue;
        const amount = progress(record);
        const selected = selectedIdRef.current === node.id;
        const scale = (0.75 + amount * 0.45) * (selected ? 1.2 : 1);
        node.mesh.scale.setScalar(scale);
        node.mesh.material.opacity = selected ? 1 : record.status === "Completed" ? 0.92 : 0.78;
        node.halo.scale.setScalar((1.1 + amount * 0.25) * (selected ? 1.15 : 1));
        node.halo.material.opacity = selected ? 0.42 : 0.2;
        node.mesh.rotation.x = seconds * 0.18 + node.phase;
        node.mesh.rotation.y = seconds * 0.24 + node.phase;
        node.mesh.position.z = record.status === "Completed" ? 0 : Math.sin(seconds * 1.4 + node.phase) * 0.08;
        node.halo.position.z = node.mesh.position.z - 0.03;
      }
      renderer.render(scene, camera);
    }

    function setLoop() {
      if (!visible || reducedMotion.matches || !hasActiveRecords) {
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

    function handleMotionPreference() {
      setLoop();
    }

    function handlePointerDown(event: PointerEvent) {
      const bounds = targetCanvas.getBoundingClientRect();
      pointer.x = ((event.clientX - bounds.left) / bounds.width) * 2 - 1;
      pointer.y = -((event.clientY - bounds.top) / bounds.height) * 2 + 1;
      raycaster.setFromCamera(pointer, camera);
      const hit = raycaster.intersectObjects(nodes.map((node) => node.mesh))[0];
      const nextId = hit ? nodes.find((node) => node.mesh === hit.object)?.id ?? null : null;
      selectedIdRef.current = nextId;
      setSelectedId(nextId);
      render(lastFrame + frameInterval);
    }

    const resizeObserver = new ResizeObserver(() => {
      const previousWidth = targetCanvas.width;
      const previousHeight = targetCanvas.height;
      resize();
      if (targetCanvas.width !== previousWidth || targetCanvas.height !== previousHeight) {
        render(lastFrame + frameInterval);
      }
    });
    resizeObserver.observe(targetCanvas);
    document.addEventListener("visibilitychange", handleVisibility);
    reducedMotion.addEventListener("change", handleMotionPreference);
    targetCanvas.addEventListener("pointerdown", handlePointerDown);
    setLoop();

    return () => {
      renderer.setAnimationLoop(null);
      resizeObserver.disconnect();
      document.removeEventListener("visibilitychange", handleVisibility);
      reducedMotion.removeEventListener("change", handleMotionPreference);
      targetCanvas.removeEventListener("pointerdown", handlePointerDown);
      for (const node of nodes) {
        node.mesh.geometry.dispose();
        node.mesh.material.dispose();
        node.halo.geometry.dispose();
        node.halo.material.dispose();
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
        <div className="playground-actions">
          <Button variant="text" size="small" onClick={() => {
            setDemoMode((current) => !current);
            setSelectedId(null);
          }}>{demoMode ? "use live downloads" : "preview example"}</Button>
          <Button variant="outlined" onClick={onOpenDownloads}>open downloads</Button>
        </div>
      </div>
      <p className="helper-text">{demoMode ? "preview data only — this does not enter the queue or history." : "a small WebGL study: work grows as it progresses, then settles into a kept cluster."}</p>
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
      <div className="playground-legend" aria-label="constellation legend">
        <span><i className="playground-dot is-running" /> working</span>
        <span><i className="playground-dot is-queued" /> queued</span>
        <span><i className="playground-dot is-completed" /> kept</span>
        <span className="playground-legend-hint">size shows progress</span>
      </div>
      <div className="playground-caption" aria-live="polite">
        {selected ? `${demoMode ? "preview" : `post #${selected.post_id}`} · ${selected.status.toLowerCase()} · ${demoMode ? "example data only." : "select open downloads to manage it."}` : `${visibleRecords.length} item${visibleRecords.length === 1 ? "" : "s"} represented · click a node to inspect its identity.`}
      </div>
    </section>
  );
}
