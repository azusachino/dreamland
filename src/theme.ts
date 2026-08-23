import { createTheme } from "@mui/material/styles";

export type DreamlandPaletteMode = "light" | "dark";

export function createDreamlandTheme(mode: DreamlandPaletteMode) {
  const dark = mode === "dark";
  return createTheme({
    palette: {
      mode,
      primary: {
        main: dark ? "#b7cbd0" : "#52737b",
        contrastText: dark ? "#1d2a2e" : "#ffffff",
      },
      background: {
        default: dark ? "#111619" : "#f5f7f6",
        paper: dark ? "#192024" : "#ffffff",
      },
      text: {
        primary: dark ? "#e7ebee" : "#1d282d",
        secondary: dark ? "#aeb8bf" : "#5a6970",
      },
      divider: dark ? "#435158" : "#cbd5d5",
    },
    shape: { borderRadius: 16 },
    typography: {
      fontFamily: "Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, sans-serif",
      button: { fontWeight: 700, textTransform: "none" },
    },
    components: {
      MuiButton: {
        defaultProps: { disableElevation: true },
        styleOverrides: {
          root: {
            borderRadius: 999,
            minHeight: 42,
            paddingInline: 18,
            transition: "background-color var(--motion-fast) var(--ease-smooth-out), border-color var(--motion-fast) var(--ease-smooth-out), color var(--motion-fast) var(--ease-smooth-out), transform var(--motion-fast) var(--ease-smooth-out)",
            "&:active": { transform: "scale(0.98)" },
          },
        },
      },
      MuiIconButton: {
        styleOverrides: {
          root: {
            borderRadius: 999,
            transition: "background-color var(--motion-fast) var(--ease-smooth-out), color var(--motion-fast) var(--ease-smooth-out), transform var(--motion-fast) var(--ease-smooth-out)",
            "&:active": { transform: "scale(0.92)" },
          },
        },
      },
      MuiChip: {
        styleOverrides: {
          root: {
            color: dark ? "#e7ebee" : "#1d282d",
            borderRadius: 10,
            fontWeight: 600,
            transition: "background-color var(--motion-fast) var(--ease-smooth-out), border-color var(--motion-fast) var(--ease-smooth-out), transform var(--motion-fast) var(--ease-smooth-out)",
          },
        },
      },
      MuiAccordion: {
        styleOverrides: {
          root: { backgroundColor: "transparent", boxShadow: "none", "&::before": { display: "none" } },
        },
      },
      MuiTextField: {
        defaultProps: { size: "small" },
      },
      MuiOutlinedInput: {
        styleOverrides: {
          root: { color: dark ? "#e7ebee" : "#1d282d" },
          input: { color: dark ? "#e7ebee" : "#1d282d" },
        },
      },
      MuiInputLabel: {
        styleOverrides: {
          root: { color: dark ? "#aeb8bf" : "#5a6970" },
        },
      },
      MuiDialogTitle: {
        styleOverrides: {
          root: { color: dark ? "#e7ebee" : "#1d282d" },
        },
      },
      MuiDialogContent: {
        styleOverrides: {
          root: { color: dark ? "#e7ebee" : "#1d282d" },
        },
      },
      MuiSkeleton: {
        styleOverrides: {
          root: { backgroundColor: dark ? "rgba(183, 203, 208, 0.11)" : "rgba(82, 115, 123, 0.12)" },
        },
      },
    },
  });
}
