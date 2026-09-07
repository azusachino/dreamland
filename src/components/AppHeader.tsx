import { type FormEvent, type RefObject, useEffect, useRef } from "react";
import { Button, IconButton, InputAdornment, Menu, MenuItem, TextField } from "@mui/material";
import { Icon } from "./Icon";
import type { SiteDescriptor } from "../lib/ipc";

interface TagSuggestion {
  name: string;
  post_count: number | null;
}

interface AppHeaderProps {
  activeSite: SiteDescriptor | null;
  activeSiteId: string;
  sites: SiteDescriptor[];
  siteMenuAnchor: HTMLElement | null;
  onSiteMenuOpen: (anchor: HTMLElement) => void;
  onSiteMenuClose: () => void;
  onSelectSite: (site: SiteDescriptor) => void;
  onOpenSite: () => void;
  title: string;
  loading: boolean;
  canGoBack: boolean;
  canGoForward: boolean;
  onBack: () => void;
  onForward: () => void;
  onRefresh: () => void;
  onSettings: () => void;
  searchDraft: string;
  searchInput: RefObject<HTMLInputElement | null>;
  searchFocused: boolean;
  onSearchFocus: () => void;
  onSearchBlur: () => void;
  onSearchChange: (value: string) => void;
  onSearchSubmit: (event: FormEvent<HTMLFormElement>) => void;
  onClearSearch: () => void;
  onOpenAdvancedSearch: () => void;
  suggestions: TagSuggestion[];
  onChooseTag: (tag: string) => void;
}

export function AppHeader({ activeSite, activeSiteId, sites, siteMenuAnchor, onSiteMenuOpen, onSiteMenuClose, onSelectSite, onOpenSite, title, loading, canGoBack, canGoForward, onBack, onForward, onRefresh, onSettings, searchDraft, searchInput, searchFocused, onSearchFocus, onSearchBlur, onSearchChange, onSearchSubmit, onClearSearch, onOpenAdvancedSearch, suggestions, onChooseTag }: AppHeaderProps) {
  const headerRef = useRef<HTMLElement>(null);

  useEffect(() => {
    const header = headerRef.current;
    if (!header) return;
    const setHeight = () => {
      document.documentElement.style.setProperty("--app-header-height", `${header.offsetHeight}px`);
    };
    setHeight();
    const observer = new ResizeObserver(setHeight);
    observer.observe(header);
    return () => observer.disconnect();
  }, []);

  return (
    <header ref={headerRef} className="app-header shell-surface" data-tauri-drag-region="deep">
      <div className="header-identity">
        <div className="brand-lockup">
          <div className="brand-mark" aria-hidden="true">✦</div>
          <div>
            <p className="eyebrow">image board</p>
            <h1>Dreamland</h1>
          </div>
        </div>
      </div>
      <form className="search-bar" onSubmit={onSearchSubmit} role="search">
        <TextField
          className="search-field"
          inputRef={searchInput}
          aria-label="search tags"
          placeholder="Search by tags…"
          value={searchDraft}
          variant="outlined"
          fullWidth
          slotProps={{ input: { startAdornment: <InputAdornment position="start"><Icon name="search" /></InputAdornment> } }}
          onFocus={onSearchFocus}
          onBlur={onSearchBlur}
          onChange={(event) => onSearchChange(event.target.value)}
        />
        {searchDraft && (
          <IconButton className="clear-search" type="button" aria-label="clear search" onClick={onClearSearch}>
            <Icon name="close" />
          </IconButton>
        )}
        <Button className="search-advanced" type="button" variant="text" onClick={onOpenAdvancedSearch}>filters</Button>
        {searchFocused && suggestions.length > 0 && (
          <div className="suggestions" role="listbox">
            <p className="suggestion-heading">suggested tags</p>
            {suggestions.map((tag) => (
                <Button
                  key={tag.name}
                  className="suggestion"
                  type="button"
                variant="text"
                role="option"
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => onChooseTag(tag.name)}
              >
                <span>{tag.name}</span>
                {tag.post_count !== null && <small>{tag.post_count.toLocaleString()}</small>}
              </Button>
            ))}
          </div>
        )}
      </form>
      <div className="header-actions">
        <IconButton aria-label="back" title="back" disabled={!canGoBack} onClick={onBack}>
          <Icon name="back" />
        </IconButton>
        <IconButton aria-label="forward" title="forward" disabled={!canGoForward} onClick={onForward}>
          <Icon name="forward" />
        </IconButton>
        <div className="site-picker">
          <Button
            className="site-menu-trigger"
            variant="outlined"
            aria-label="choose image board site"
            aria-expanded={Boolean(siteMenuAnchor)}
            endIcon={<Icon name="chevron" />}
            onClick={(event) => onSiteMenuOpen(event.currentTarget)}
          >
            {activeSite?.name ?? "choose site"}
          </Button>
          <Menu
            anchorEl={siteMenuAnchor}
            open={Boolean(siteMenuAnchor)}
            onClose={onSiteMenuClose}
            slotProps={{ list: { "aria-label": "image board sites" } }}
          >
            {sites.map((site) => (
              <MenuItem
                key={site.id}
                selected={site.id === activeSiteId}
                disabled={!site.capabilities.browse}
                onClick={() => onSelectSite(site)}
                title={site.capabilities.browse ? `browse ${site.name}` : `${site.name} coming later`}
              >
                {site.name}{!site.capabilities.browse && " · later"}
              </MenuItem>
            ))}
            <MenuItem disabled={!activeSite} onClick={onOpenSite}>open current site</MenuItem>
          </Menu>
        </div>
        <IconButton aria-label={`refresh ${title.toLowerCase()}`} title={`refresh ${title.toLowerCase()}`} disabled={loading} onClick={onRefresh}>
          <Icon name="refresh" />
        </IconButton>
        <Button className="settings-button" variant="contained" startIcon={<Icon name="settings" />} onClick={onSettings}>settings</Button>
      </div>
    </header>
  );
}
