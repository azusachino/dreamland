import { useCallback, useEffect, useRef } from "react";
import { Button, Skeleton } from "@mui/material";

interface LoadMoreProps {
  autoLoad?: boolean;
  hasNext: boolean;
  loading: boolean;
  onLoadMore: () => void;
}

export function LoadMore({ autoLoad = true, hasNext, loading, onLoadMore }: LoadMoreProps) {
  const sentinel = useRef<HTMLDivElement>(null);
  const state = useRef({ autoLoad, hasNext, loading, onLoadMore });
  const requestInFlight = useRef(false);
  const previousLoading = useRef(loading);

  useEffect(() => {
    state.current = { autoLoad, hasNext, loading, onLoadMore };
    if (previousLoading.current && !loading) requestInFlight.current = false;
    previousLoading.current = loading;
  }, [autoLoad, hasNext, loading, onLoadMore]);

  const requestMore = useCallback(() => {
    if (!state.current.hasNext || state.current.loading || requestInFlight.current) return;
    requestInFlight.current = true;
    state.current.onLoadMore();
  }, []);

  useEffect(() => {
    if (!autoLoad || !sentinel.current) return;
    const element = sentinel.current;
    const tryLoad = () => {
      if (!state.current.autoLoad || !state.current.hasNext || state.current.loading) return;
      const bounds = element.getBoundingClientRect();
      if (bounds.top <= window.innerHeight + 1600) requestMore();
    };
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0]?.isIntersecting) requestMore();
      },
      { root: null, rootMargin: "1600px 0px" },
    );
    observer.observe(element);
    window.addEventListener("scroll", tryLoad, { passive: true });
    document.addEventListener("scroll", tryLoad, { passive: true, capture: true });
    window.addEventListener("resize", tryLoad);
    tryLoad();
    return () => {
      observer.disconnect();
      window.removeEventListener("scroll", tryLoad);
      document.removeEventListener("scroll", tryLoad, true);
      window.removeEventListener("resize", tryLoad);
    };
  }, [autoLoad, requestMore]);

  if (!hasNext && !loading) return <div className="load-more-end">you’ve reached the end.</div>;
  return (
    <div className="load-more" ref={sentinel}>
      {loading ? <Skeleton variant="rounded" width={148} height={44} animation="wave" /> : <Button variant="outlined" onClick={requestMore}>load more</Button>}
    </div>
  );
}
