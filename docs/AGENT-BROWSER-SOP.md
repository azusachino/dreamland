# agent-browser SOP

Use `agent-browser` for repeatable browser-level checks of Dreamland's Vite
surface. Treat every browser session as a resource: it owns a headless Chrome
process, and abandoned sessions can keep a GPU process busy after the test is
over.

## Required lifecycle

Use one named session per task. Never use the unnamed shared session.

```bash
export AGENT_BROWSER_SESSION="$(bunx agent-browser session id --scope worktree --prefix dreamland)"
export AGENT_BROWSER_IDLE_TIMEOUT_MS=900000

trap 'bunx agent-browser close --session "$AGENT_BROWSER_SESSION" >/dev/null 2>&1 || true' EXIT INT TERM

bunx agent-browser open 'http://127.0.0.1:1420/#/downloads?demo=1'
bunx agent-browser snapshot -i
```

Keep the same session for the complete check. Re-snapshot after navigation or
any action that changes the page; refs are invalid after a re-render.

## Resource rules

- Default to one browser session and one tab per task.
- Do not create parallel sessions unless the test explicitly needs isolation.
- Reuse the existing Vite server; do not start a second dev server on another
  port without first checking what is already running.
- Set an idle timeout for exploratory work so an interrupted task can recover
  its browser automatically.
- Close the named session as soon as the check is complete; do not leave it for
  the next task.
- Do not use `close --all` as routine cleanup. It can close another task's
  browser; use it only after `session list` confirms every listed session is
  owned by this task.

## Cleanup and audit

The `trap` is the normal cleanup path, but run the audit explicitly before
reporting a browser check complete:

```bash
bunx agent-browser close --session "$AGENT_BROWSER_SESSION"
bunx agent-browser session list
ps -axo pid,ppid,%cpu,etime,command | rg 'agent-browser|Google Chrome.*headless' || true
```

Expected result: the named session is gone and no Chrome process created by the
task remains. If other sessions are listed, leave them alone and report them;
do not terminate them speculatively. If CPU remains high, take a fresh whole
system sample with `top -l 3 -n 15 -o cpu` before attributing it to Dreamland.

## Completion evidence

Record the session-isolated command, route, interaction, and cleanup result in
the relevant verification receipt. A browser interaction is not fully verified
until both the UI assertion and resource cleanup pass.
