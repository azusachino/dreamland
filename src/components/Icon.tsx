import AccessTimeRounded from "@mui/icons-material/AccessTimeRounded";
import ArrowBackRounded from "@mui/icons-material/ArrowBackRounded";
import ArrowForwardRounded from "@mui/icons-material/ArrowForwardRounded";
import CheckRounded from "@mui/icons-material/CheckRounded";
import CloseRounded from "@mui/icons-material/CloseRounded";
import DownloadRounded from "@mui/icons-material/DownloadRounded";
import ExpandMoreRounded from "@mui/icons-material/ExpandMoreRounded";
import FavoriteRounded from "@mui/icons-material/FavoriteRounded";
import FavoriteBorderRounded from "@mui/icons-material/FavoriteBorderRounded";
import MenuBookRounded from "@mui/icons-material/MenuBookRounded";
import RefreshRounded from "@mui/icons-material/RefreshRounded";
import SearchRounded from "@mui/icons-material/SearchRounded";
import SettingsRounded from "@mui/icons-material/SettingsRounded";
import TrendingUpRounded from "@mui/icons-material/TrendingUpRounded";

export type IconName = "clock" | "trend" | "download" | "book" | "heart" | "heartFilled" | "search" | "refresh" | "settings" | "close" | "back" | "forward" | "check" | "chevron";

export function Icon({ name }: { name: IconName }) {
  const icons = {
    clock: AccessTimeRounded,
    trend: TrendingUpRounded,
    download: DownloadRounded,
    book: MenuBookRounded,
    heart: FavoriteBorderRounded,
    heartFilled: FavoriteRounded,
    search: SearchRounded,
    refresh: RefreshRounded,
    settings: SettingsRounded,
    close: CloseRounded,
    back: ArrowBackRounded,
    forward: ArrowForwardRounded,
    check: CheckRounded,
    chevron: ExpandMoreRounded,
  } as const;
  const Component = icons[name];
  return <Component className="icon" aria-hidden="true" />;
}
