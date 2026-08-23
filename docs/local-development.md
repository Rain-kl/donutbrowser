# Local Development

## First-time setup

Run these commands from the repository root:

```powershell
cd C:\code\donutbrowser
pnpm install
```

The local machine also needs the Rust toolchain and the Windows WebView2
runtime.

## Start the desktop app

Use this command for normal local development:

```powershell
cd C:\code\donutbrowser
pnpm tauri dev
```

The repository command wrapper automatically adds
`--no-watch --features self-hosted-browser-automation`. `--no-watch` avoids
repeated Tauri rebuilds caused by generated build files, while the feature
enables local API and MCP browser automation without requiring a hosted plan.
Next.js still serves the frontend on `http://localhost:12341`.

After changing Rust code, stop the command and run it again. Frontend changes
continue to use Next.js hot reload.

## Self-hosted automation default

Normal local development enables the authorized self-hosted automation feature
by default. The following commands are equivalent:

```powershell
cd C:\code\donutbrowser
pnpm tauri dev
pnpm tauri dev --no-watch --features self-hosted-browser-automation
```

The feature only changes the `browser_automation` entitlement. Cloud backup,
cloud proxy, Wayfern token, and other hosted-service checks keep their existing
behavior.

## Stop or restart

Press `Ctrl+C` in the terminal that started the app. To restart, run:

```powershell
pnpm tauri dev
```

## Local data and logs

- Development data: `%LOCALAPPDATA%\DonutBrowserDev`
- Production data: `%LOCALAPPDATA%\DonutBrowser`
- Development log: `%LOCALAPPDATA%\com.donutbrowser\logs\DonutBrowserDev.log`
- MCP server: `http://127.0.0.1:51080/mcp` when enabled

Development and production data are intentionally isolated.

## Build a release package

```powershell
pnpm tauri build
```

Release builds use the production data directory.

To package the authorized self-hosted automation variant:

```powershell
pnpm tauri build --features self-hosted-browser-automation
```
