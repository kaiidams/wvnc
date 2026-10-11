# Web VNC Viewer

This is a VNC Viewer written in WebAssembly and Rust.
This is my personal project to use QEMU over SSH tunnel with web browser.
It supports QEMU Audio Extension so that you can play audio from remote machine.

## Building

### Development

In development mode, Vite automatically serves and reloads frontend files and loads the debug WebAssembly artifact from `target/wasm32-unknown-unknown/debug/wvnc.wasm`.

1. Install frontend dependencies:
   ```bash
   pnpm install
   ```

2. Build the WebAssembly binary in debug mode:
   ```bash
   pnpm run rust-dev
   # or: cargo build --target wasm32-unknown-unknown --lib
   ```

3. Start the Vite development server:
   ```bash
   pnpm run dev
   ```

4. Open the viewer in your web browser (default: `http://localhost:5173/`).

### Production

For production deployment:

1. Build the optimized WebAssembly binary:
   ```bash
   pnpm run rust-rel
   # or: cargo build --release --target wasm32-unknown-unknown --lib
   ```

2. Copy the compiled WebAssembly binary into the `public/` directory so Vite packages it into the output bundle:
   ```bash
   cp target/wasm32-unknown-unknown/release/wvnc.wasm public/
   ```

3. Build the static production assets:
   ```bash
   pnpm run build
   ```
   The production files will be generated in `dist/`.

4. Preview the production build locally:
   ```bash
   pnpm run preview
   ```
   Or deploy `dist/` to any static web hosting provider (e.g., GitLab Pages, GitHub Pages, Nginx).

## How to Use

The viewer connects to a VNC server over WebSocket using URL query parameters.

### URL Query Parameters

| Parameter | Description | Default | Example |
| :--- | :--- | :--- | :--- |
| `c` | **(Required)** Target WebSocket host and port (`<host>:<port>` or `<host>:<display>`). | None | `?c=localhost:1` |
| `x` | Display scale factor (multiplier for framebuffer dimensions). | `1` | `?x=1.5` or `?x=2` |

**Example URL:**
```
http://localhost:5173/?c=127.0.0.1:0
```
(connects to WebSocket port 5700 on `127.0.0.1`)
or with explicit port and 1.5x scaling:
```
http://localhost:5173/?c=127.0.0.1:5700&x=1.5
```

---

### Example 1: Connecting to QEMU (with Audio Support)

QEMU includes built-in support for VNC over WebSocket as well as the QEMU Audio Extension.

1. Launch QEMU with a VNC WebSocket listener and audio redirection:
   ```bash
   qemu-system-x86_64 \
     -audiodev vnc,id=audio0 \
     -device intel-hda \
     -device hda-duplex,audiodev=audio0 \
     -vnc 127.0.0.1:0,websocket=5700,audiodev=audio0 \
     path/to/disk.img
   ```

2. If QEMU is running on a remote host, forward the WebSocket port via SSH tunnel:
   ```bash
   ssh -L 5700:127.0.0.1:5700 user@remote-server
   ```

3. Open the viewer in your browser:
   ```
   http://localhost:5173/?c=127.0.0.1:5700
   ```

---

### Example 2: Connecting to a Standard VNC Server (via websockify)

Standard VNC servers (e.g., TigerVNC, TightVNC, x11vnc) communicate via raw TCP (usually port 5900). Since web browsers can only initiate WebSocket connections, use [`websockify`](https://github.com/novnc/websockify) as a TCP-to-WebSocket proxy:

1. Start websockify proxying port 5700 to your VNC server on port 5900:
   ```bash
   websockify 5700 localhost:5900
   ```

2. Open the viewer pointing to websockify's WebSocket port:
   ```
   http://localhost:5173/?c=localhost:0
   ```
