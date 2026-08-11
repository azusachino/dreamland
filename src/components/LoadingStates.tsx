import { Skeleton } from "@mui/material";

function PostCardSkeleton() {
  return (
    <article className="card skeleton-card" aria-hidden="true">
      <div className="preview">
        <Skeleton className="skeleton-media" variant="rectangular" animation="wave" />
      </div>
      <div className="card-details">
        <div className="tag-list">
          <Skeleton variant="rounded" width="38%" height={28} animation="wave" />
          <Skeleton variant="rounded" width="30%" height={28} animation="wave" />
          <Skeleton variant="rounded" width="24%" height={28} animation="wave" />
        </div>
        <div className="card-footer">
          <Skeleton variant="text" width="42%" height={24} animation="wave" />
          <Skeleton variant="rounded" width={92} height={40} animation="wave" />
        </div>
      </div>
    </article>
  );
}

export function GallerySkeleton({ count = 12 }: { count?: number }) {
  return (
    <div className="gallery-grid" aria-label="loading posts" role="status">
      {Array.from({ length: count }, (_, index) => <PostCardSkeleton key={index} />)}
    </div>
  );
}

export function RowSkeleton({ count = 5 }: { count?: number }) {
  return (
    <div className="skeleton-rows" aria-label="loading content" role="status">
      {Array.from({ length: count }, (_, index) => (
        <Skeleton key={index} className="skeleton-row" variant="rounded" height={72} animation="wave" />
      ))}
    </div>
  );
}
