import { Button, IconButton } from "@mui/material";
import type { SavedQuery } from "../lib/ipc";
import { Icon, type IconName } from "./Icon";

interface MainNavigationProps {
  view: string;
  savedQueries: SavedQuery[];
  selectedSavedQueryId: string | null;
  supportsPools: boolean;
  supportsFavorites: boolean;
  siteName: string;
  safeOnly: boolean;
  onChangeView: (view: "latest" | "popular" | "downloads" | "pools" | "favorites") => void;
  onOpenSavedQuery: (saved: SavedQuery) => void;
  onEditSavedQuery: (saved: SavedQuery) => void;
  onMoveSavedQuery: (saved: SavedQuery, direction: -1 | 1) => void;
  onToggleSavedPin: (saved: SavedQuery) => void;
  onDeleteSavedQuery: (saved: SavedQuery) => void;
  isRunnable: (saved: SavedQuery) => boolean;
  description: (saved: SavedQuery) => string;
}

function NavButton({ active, disabled, icon, label, onClick }: { active: boolean; disabled?: boolean; icon: IconName; label: string; onClick: () => void }) {
  return (
    <Button className={`nav-item${active ? " active" : ""}`} variant="text" disabled={disabled} onClick={onClick} role="tab" aria-selected={active}>
      <span className="nav-icon"><Icon name={icon} /></span>
      <span>{label}</span>
    </Button>
  );
}

export function MainNavigation({ view, savedQueries, selectedSavedQueryId, supportsPools, supportsFavorites, siteName, safeOnly, onChangeView, onOpenSavedQuery, onEditSavedQuery, onMoveSavedQuery, onToggleSavedPin, onDeleteSavedQuery, isRunnable, description }: MainNavigationProps) {
  return (
    <nav className="view-tabs shell-surface" aria-label="Dreamland sections" role="tablist">
      <NavButton active={view === "latest"} label="latest" icon="clock" onClick={() => onChangeView("latest")} />
      <NavButton active={view === "popular"} label="popular" icon="trend" onClick={() => onChangeView("popular")} />
      {savedQueries.map((saved, index) => {
        const previous = savedQueries[index - 1];
        const next = savedQueries[index + 1];
        const canMoveUp = Boolean(previous && previous.pinned === saved.pinned);
        const canMoveDown = Boolean(next && next.pinned === saved.pinned);
        const runnable = isRunnable(saved);
        return (
          <div className="saved-query-nav" key={saved.id}>
            <Button className={`nav-item${selectedSavedQueryId === saved.id ? " active" : ""}`} variant="text" role="tab" aria-selected={selectedSavedQueryId === saved.id} disabled={!runnable} onClick={() => onOpenSavedQuery(saved)} title={runnable ? description(saved) : `${description(saved)} (not supported yet)`}>
              <Icon name="search" />
              <span>{saved.name}</span>
              {!runnable && <small>unavailable</small>}
              {saved.pinned && runnable && <small>pinned</small>}
            </Button>
            <div className="saved-query-actions">
              <IconButton aria-label={`edit ${saved.name}`} title="edit query" disabled={!runnable} onClick={() => onEditSavedQuery(saved)}>✎</IconButton>
              <IconButton aria-label={`move ${saved.name} earlier`} title="move earlier" disabled={!canMoveUp} onClick={() => onMoveSavedQuery(saved, -1)}>↑</IconButton>
              <IconButton aria-label={`move ${saved.name} later`} title="move later" disabled={!canMoveDown} onClick={() => onMoveSavedQuery(saved, 1)}>↓</IconButton>
              <IconButton aria-label={`${saved.pinned ? "unpin" : "pin"} ${saved.name}`} onClick={() => onToggleSavedPin(saved)}>{saved.pinned ? "•" : "○"}</IconButton>
              <IconButton aria-label={`delete ${saved.name}`} onClick={() => onDeleteSavedQuery(saved)}>×</IconButton>
            </div>
          </div>
        );
      })}
      {supportsPools && <NavButton active={view === "pools"} label="pools" icon="book" onClick={() => onChangeView("pools")} />}
      {supportsFavorites && <NavButton active={view === "favorites"} label="favorites" icon="heart" onClick={() => onChangeView("favorites")} />}
      <NavButton active={view === "downloads"} label="downloads" icon="download" onClick={() => onChangeView("downloads")} />
      <span className="view-status">
        <span className="connection-dot" aria-hidden="true" />
        <span>{safeOnly ? `${siteName} · safe mode` : `${siteName} connected`}</span>
      </span>
    </nav>
  );
}
