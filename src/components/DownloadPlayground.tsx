import { useEffect, useMemo, useRef, useState } from "react";
import { Button } from "@mui/material";
import {
  BufferGeometry,
  CanvasTexture,
  Group,
  LineBasicMaterial,
  LineSegments,
  Mesh,
  MeshBasicMaterial,
  PerspectiveCamera,
  PlaneGeometry,
  Raycaster,
  RingGeometry,
  Scene,
  SRGBColorSpace,
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
  texture: CanvasTexture;
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

function roundedRect(context: CanvasRenderingContext2D, x: number, y: number, width: number, height: number, radius: number) {
  const right = x + width;
  const bottom = y + height;
  context.beginPath();
  context.moveTo(x + radius, y);
  context.lineTo(right - radius, y);
  context.quadraticCurveTo(right, y, right, y + radius);
  context.lineTo(right, bottom - radius);
  context.quadraticCurveTo(right, bottom, right - radius, bottom);
  context.lineTo(x + radius, bottom);
  context.quadraticCurveTo(x, bottom, x, bottom - radius);
  context.lineTo(x, y + radius);
  context.quadraticCurveTo(x, y, x + radius, y);
  context.closePath();
}

function createNodeTexture(record: DownloadRecord): CanvasTexture {
  const textureCanvas = document.createElement("canvas");
  textureCanvas.width = 256;
  textureCanvas.height = 176;
  const context = textureCanvas.getContext("2d");
  if (!context) return new CanvasTexture(textureCanvas);

  const seed = [...record.post_id].reduce((value, character) => value + character.charCodeAt(0), 0);
  const accent = ["#f1c7a8", "#b9d9d0", "#c8c3ed", "#e6c9dc"][seed % 4];
  const background = record.status === "Completed" ? "#17332f" : record.status === "Running" ? "#1d3038" : "#27303b";
  const label = record.status === "Completed" ? "downloaded" : record.status === "Running" ? "in motion" : "waiting";

  context.fillStyle = background;
  context.fillRect(0, 0, textureCanvas.width, textureCanvas.height);
  context.globalAlpha = 0.16;
  context.fillStyle = accent;
  context.beginPath();
  context.arc(202, 42, 48 + (seed % 3) * 10, 0, Math.PI * 2);
  context.fill();
  context.globalAlpha = 1;

  roundedRect(context, 16, 16, 224, 144, 18);
  context.strokeStyle = accent;
  context.lineWidth = 2;
  context.globalAlpha = 0.6;
  context.stroke();
  context.globalAlpha = 1;
  context.fillStyle = accent;
  context.beginPath();
  context.arc(48, 56, 16, 0, Math.PI * 2);
  context.fill();
  context.fillStyle = background;
  context.beginPath();
  context.arc(48, 56, 6, 0, Math.PI * 2);
  context.fill();

  context.fillStyle = "#f5f3ed";
  context.font = "700 22px system-ui, sans-serif";
  context.fillText(label, 76, 54);
  context.fillStyle = accent;
  context.font = "500 14px system-ui, sans-serif";
  context.globalAlpha = 0.87;
  context.fillText(`post ${record.post_id}`, 28, 104);
  context.globalAlpha = 1;

  roundedRect(context, 28, 124, 200, 10, 5);
  context.fillStyle = "rgba(255, 255, 255, 0.12)";
  context.fill();
  if (record.total_bytes && record.total_bytes > 0) {
    roundedRect(context, 28, 124, 200 * progress(record), 10, 5);
    context.fillStyle = accent;
    context.fill();
  } else {
    context.fillStyle = accent;
    for (let index = 0; index < 3; index += 1) {
      context.beginPath();
      context.arc(38 + index * 13, 129, 3, 0, Math.PI * 2);
      context.fill();
    }
  }

  const texture = new CanvasTexture(textureCanvas);
  texture.colorSpace = SRGBColorSpace;
  return texture;
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
  const stageRef = useRef<HTMLDivElement>(null);
  const renderRef = useRef<(() => void) | null>(null);
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

  function selectNode(id: string | null) {
    selectedIdRef.current = id;
    setSelectedId(id);
    renderRef.current?.();
  }

  useEffect(() => {
    const canvas = canvasRef.current;
    const stage = stageRef.current;
    if (!canvas || !stage) return;
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
      const texture = createNodeTexture(record);
      const material = new MeshBasicMaterial({
        map: texture,
        transparent: true,
        opacity: record.status === "Completed" ? 0.92 : 0.78,
      });
      const haloMaterial = new MeshBasicMaterial({
        color: nodeColor(record),
        transparent: true,
        opacity: 0.2,
      });
      const mesh = new Mesh(new PlaneGeometry(1.5, 1.03), material);
      const halo = new Mesh(new RingGeometry(0.68, 0.74, 32), haloMaterial);
      mesh.position.copy(nodePosition(index, record.status === "Completed"));
      halo.position.copy(mesh.position);
      halo.position.z -= 0.08;
      group.add(halo);
      group.add(mesh);
      return { id: record.id, mesh, halo, texture, phase: index * 0.8 };
    });
    const linkGeometry = new BufferGeometry().setFromPoints(
      nodes.flatMap((node) => [
        new Vector3(0, 0, -0.12),
        new Vector3(node.mesh.position.x, node.mesh.position.y, -0.12),
      ]),
    );
    const linkMaterial = new LineBasicMaterial({ color: 0xb7cbd0, transparent: true, opacity: 0.18 });
    const links = new LineSegments(linkGeometry, linkMaterial);
    group.add(links);
    const raycaster = new Raycaster();
    const pointer = new Vector2();
    const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
    const hasActiveRecords = visibleRecords.some((record) => record.status === "Queued" || record.status === "Running");
    const frameInterval = 1000 / 30;
    let lastFrame = -Infinity;
    let visible = !document.hidden;
    let disposed = false;
    let resizeObserver: ResizeObserver | null = null;
    let intersectionObserver: IntersectionObserver | null = null;
    let stageVisible = true;

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
      if (disposed) return;
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
        const scale = (0.86 + amount * 0.24) * (selected ? 1.12 : 1);
        node.mesh.scale.setScalar(scale);
        node.mesh.material.opacity = selected ? 1 : record.status === "Completed" ? 0.92 : 0.78;
        node.halo.scale.setScalar((1.1 + amount * 0.25) * (selected ? 1.15 : 1));
        node.halo.material.opacity = selected ? 0.42 : 0.2;
        node.mesh.rotation.x = Math.sin(seconds * 0.45 + node.phase) * 0.045;
        node.mesh.rotation.y = Math.cos(seconds * 0.38 + node.phase) * 0.08;
        node.mesh.rotation.z = Math.sin(seconds * 0.25 + node.phase) * 0.035;
        node.mesh.position.z = record.status === "Completed" ? 0 : Math.sin(seconds * 1.4 + node.phase) * 0.08;
        node.halo.position.z = node.mesh.position.z - 0.03;
      }
      renderer.render(scene, camera);
    }

    function setLoop() {
      if (disposed) return;
      if (!visible || !stageVisible || reducedMotion.matches || !hasActiveRecords) {
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

    function handleIntersection(entries: IntersectionObserverEntry[]) {
      stageVisible = entries[0]?.isIntersecting ?? true;
      setLoop();
    }

    function handlePointerDown(event: PointerEvent) {
      const bounds = targetCanvas.getBoundingClientRect();
      pointer.x = ((event.clientX - bounds.left) / bounds.width) * 2 - 1;
      pointer.y = -((event.clientY - bounds.top) / bounds.height) * 2 + 1;
      raycaster.setFromCamera(pointer, camera);
      const hit = raycaster.intersectObjects(nodes.map((node) => node.mesh))[0];
      const nextId = hit ? nodes.find((node) => node.mesh === hit.object)?.id ?? null : null;
      selectNode(nextId);
    }

    function handleContextLost(event: Event) {
      event.preventDefault();
      dispose();
      setWebglUnavailable(true);
    }

    function dispose() {
      if (disposed) return;
      disposed = true;
      renderer.setAnimationLoop(null);
      resizeObserver?.disconnect();
      intersectionObserver?.disconnect();
      document.removeEventListener("visibilitychange", handleVisibility);
      reducedMotion.removeEventListener("change", handleMotionPreference);
      targetCanvas.removeEventListener("pointerdown", handlePointerDown);
      targetCanvas.removeEventListener("webglcontextlost", handleContextLost);
      renderRef.current = null;
      for (const node of nodes) {
        node.mesh.geometry.dispose();
        node.mesh.material.dispose();
        node.texture.dispose();
        node.halo.geometry.dispose();
        node.halo.material.dispose();
      }
      links.geometry.dispose();
      links.material.dispose();
      renderer.dispose();
    }

    resizeObserver = new ResizeObserver(() => {
      const previousWidth = targetCanvas.width;
      const previousHeight = targetCanvas.height;
      resize();
      if (targetCanvas.width !== previousWidth || targetCanvas.height !== previousHeight) {
        render(lastFrame + frameInterval);
      }
    });
    resizeObserver.observe(targetCanvas);
    if (typeof IntersectionObserver !== "undefined") {
      intersectionObserver = new IntersectionObserver(handleIntersection, { threshold: 0.01 });
      intersectionObserver.observe(stage);
    }
    document.addEventListener("visibilitychange", handleVisibility);
    reducedMotion.addEventListener("change", handleMotionPreference);
    targetCanvas.addEventListener("pointerdown", handlePointerDown);
    targetCanvas.addEventListener("webglcontextlost", handleContextLost);
    renderRef.current = () => render(lastFrame + frameInterval);
    setLoop();

    return () => {
      dispose();
    };
  }, [sceneKey]);

  const selected = visibleRecords.find((record) => record.id === selectedId);
  const activeCount = visibleRecords.filter((record) => record.status === "Queued" || record.status === "Running").length;
  const downloadedCount = visibleRecords.filter((record) => record.status === "Completed").length;

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
            selectNode(null);
          }}>{demoMode ? "use live downloads" : "preview example"}</Button>
          <Button variant="outlined" onClick={onOpenDownloads}>open downloads</Button>
        </div>
      </div>
      <p className="helper-text">{demoMode ? "preview data only — this does not enter the queue or history." : "a small WebGL study: work grows as it progresses, then settles into a downloaded cluster."}</p>
      <div ref={stageRef} className="playground-stage">
        <div className="playground-stage-note" aria-hidden="true">
          <span>{demoMode ? "preview orbit" : "live orbit"}</span>
          <strong>{activeCount > 0 ? `${activeCount} in motion` : downloadedCount > 0 ? "everything downloaded" : "waiting for work"}</strong>
        </div>
        {webglUnavailable ? (
          <div className="playground-fallback" role="status">
            <strong>WebGL is unavailable here.</strong>
            <span>The normal download panel remains fully available.</span>
            <Button variant="contained" onClick={onOpenDownloads}>view download state</Button>
          </div>
        ) : (
          <canvas ref={canvasRef} aria-hidden="true" />
        )}
        {!webglUnavailable && visibleRecords.length === 0 && <div className="playground-empty">queue a download to give the constellation something to hold.</div>}
      </div>
      <div className="playground-legend" aria-label="constellation legend">
        <span><i className="playground-dot is-running" /> working</span>
        <span><i className="playground-dot is-queued" /> queued</span>
        <span><i className="playground-dot is-completed" /> downloaded</span>
        <span className="playground-legend-hint">size shows progress</span>
      </div>
      {visibleRecords.length > 0 && <div className="playground-node-list" aria-label="downloads represented by the constellation">
        {visibleRecords.map((record) => {
          const selected = selectedId === record.id;
          const amount = progress(record);
          const progressText = record.total_bytes && record.total_bytes > 0
            ? `${Math.round(amount * 100)}%`
            : record.status === "Completed" ? "downloaded" : "unknown size";
          return (
            <button
              key={record.id}
              type="button"
              className={`playground-node${selected ? " is-selected" : ""}`}
              aria-pressed={selected}
              onClick={() => selectNode(record.id)}
            >
              <span className={`playground-dot is-${record.status.toLowerCase() === "completed" ? "completed" : record.status.toLowerCase() === "running" ? "running" : "queued"}`} aria-hidden="true" />
              <span className="playground-node-copy">
                <strong>{demoMode ? `preview ${record.post_id}` : `post #${record.post_id}`}</strong>
                <small>{record.status.toLowerCase()} · {progressText}</small>
              </span>
            </button>
          );
        })}
      </div>}
      <div className="playground-caption" aria-live="polite">
        {selected ? <>
          {`${demoMode ? "preview" : `post #${selected.post_id}`} · ${selected.status.toLowerCase()} · ${demoMode ? "example data only." : "ready to manage."}`}
          {!demoMode && <Button variant="text" size="small" onClick={onOpenDownloads}>manage download</Button>}
        </> : `${visibleRecords.length} item${visibleRecords.length === 1 ? "" : "s"} represented · select a node to inspect its identity.`}
      </div>
    </section>
  );
}
