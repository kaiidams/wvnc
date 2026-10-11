import './style.css'
import { FrameBufferComponent } from './framebuffer.ts';
import { AudioSink } from './audio.ts';

function parseHost(target: string | null): string | null {
  if (!target) {
    return null;
  }
  target = target.trim();
  if (!target) {
    return null;
  }

  const index = target.lastIndexOf(":");
  if (index === -1) return target;
  const host = target.slice(0, index)
  const port = target.slice(index + 1);
  if (port.indexOf("]") !== -1) {
    // [....:xxxx] without port number
    return target;
  }

  const portNum = parseInt(port);
  if (portNum < 100) {
    return `${host}:${portNum + 5700}`;
  }

  return target;
}

function getArguments(): { host: string | null; scale: number } {
  const params = new URLSearchParams(window.location.search);
  const host = parseHost(params.get("c"));
  const x = params.get("x");
  const scale = x !== null && !isNaN(parseFloat(x)) ? parseFloat(x) : 1;
  return { host, scale };
}
const { host, scale } = getArguments();


function disableDefaultActions() {
  function cancel(event: any) {
    event.preventDefault();
  }
  document.addEventListener("keydown", cancel, false);
  document.addEventListener("keyup", cancel, false);
  document.addEventListener("keydown", cancel, true);
  document.addEventListener("keyup", cancel, true);
}

disableDefaultActions();

let pointerRelative = false;
let prevX = 0, prevY = 0;

const container = document.querySelector<HTMLDivElement>('#app')!;
const framebuffer = new FrameBufferComponent(container, {
  width: 640,
  height: 480,
  scale: scale,
  fractionalScale: true,
});
framebuffer.focus();

const audioSink = new AudioSink();
audioSink.setFormat();

const log_levels = [
  null,
  console.error,
  console.warn,
  console.info,
  console.debug,
  null,
];

const beepAudio = new Audio("beep.wav");

function debug_print(level: number, s: number, len: number) {
  try {
    const log_func = log_levels[level];
    if (log_func) {
      let bytes = memory.buffer.slice(s, s + len);
      let t = decoder.decode(bytes);
      // log_func(t);
      console.log(t);
    }
  } catch (e) {
    console.error(e);
  }
}

function set_title(ptr: number, len: number) {
  try {
    let bytes = memory.buffer.slice(ptr, ptr + len);
    const dataArray = new Uint8Array(bytes);
    const text = decoder.decode(dataArray);
    framebuffer.setTitle(text);
    return true;
  } catch {
    return false;
  }
}

function resize(width: number, height: number) {
  try {
    console.debug(`resize ${width} ${height}`);
    framebuffer.resize(width, height);
    return true;
  } catch {
    return false;
  }
}

function fill_rect(pixel: number, x: number, y: number, width: number, height: number) {
  try {
    console.debug(`fillRect ${pixel} ${x} ${y} ${width} ${height}`);
    framebuffer.fillRect(pixel, x, y, width, height);
    return true;
  } catch {
    return false;
  }
}

function update_rect(data: number, x: number, y: number, width: number, height: number) {
  try {
    console.debug(`updateRect ${x},${y} ${width}x${height}`);
    let bytes = memory.buffer.slice(data, data + width * height * 4);
    const dataArray = new Uint8ClampedArray(bytes);
    framebuffer.updateRect(dataArray, x, y, width, height);
    return true;
  } catch (e) {
    console.error(e);
    return false;
  }
}

function copy_rect(sx: number, sy: number, dx: number, dy: number, width: number, height: number) {
  try {
    console.debug(`copyRect ${sx},${sy} ${dx},${dy} ${width}x${height}`);
    framebuffer.copyRect(sx, sy, dx, dy, width, height);
    return true;
  } catch {
    return false;
  }
}

function bell() {
  try {
    console.debug('beep');
    beepAudio.play().catch((e) => {
      console.warn(e);
    });
    return true;
  } catch {
    return false;
  }
}

function clipboard_text(ptr: number, len: number) {
  try {
    let bytes = memory.buffer.slice(ptr, ptr + len);
    const dataArray = new Uint8Array(bytes);
    const text = decoder.decode(dataArray);
    console.log(text);
    return true;
  } catch {
    return false;
  }
}

function led(state: number) {
  try {
    const scroll_lock = (state & 1) != 0;
    const num_lock = (state & 2) != 0;
    const caps_lock = (state & 4) != 0;
    console.log(`LED: scroll_lock=${scroll_lock} num_lock=${num_lock} caps_lock=${caps_lock}`);
    return true;
  } catch {
    return false;
  }
}

function pointer_mode(mode: number) {
  try {
    console.log(`Poiner mode: ${mode}`);
    if (mode == 0) {
      pointerRelative = true;
    } else if (mode == 1) {
      pointerRelative = false;
    } else {
      return false;
    }
    return true;
  } catch {
    return false;
  }
}

function audio_start() {
  try {
    console.log("Audio start");
    audioSink.start();
    return true;
  } catch (e) {
    console.error(e);
    return false;
  }
}

function audio_stop() {
  try {
    console.log("Audio stop");
    audioSink.stop();
    return true;
  } catch (e) {
    console.error(e);
    return false;
  }
}

function audio_data(ptr: number, len: number) {
  try {
    let bytes = memory.buffer.slice(ptr, ptr + len);
    audioSink.push(bytes);
    return true;
  } catch (e) {
    console.error(e);
    return false;
  }
}

let promise = Promise.resolve();
let ws: WebSocket | null = null;

function socket_open() {
  if (host == null) {
    return 0;
  }
  ws = new WebSocket("ws://" + host);
  ws.addEventListener("open", (event) => {
    console.log(event);
    socket_event(null, 0);
  });
  ws.addEventListener("message", (event) => {
    promise = promise.then(() => {
      return event.data.arrayBuffer();
    }).then((buffer: ArrayBuffer) => {
      const arr = new Uint8Array(buffer);
      // console.log(arr);
      const len = arr.length;
      const ptr = alloc_buffer(len);
      let bytes = new Uint8Array(memory.buffer, ptr, len);
      bytes.set(arr);
      socket_event(ptr, arr.length);
    });
  });
  ws.addEventListener("error", (event) => {
    console.log(event);
  });
  ws.addEventListener("close", (event) => {
    console.log(event);
  });
  return 1;
}

function socket_close() {
  ws!.close();
  ws = null;
  return 1;
}

function socket_send(data: number, len: number) {
  try {
    // console.debug(`send ${data} ${len}`);
    let bytes = memory.buffer.slice(data, data + len);
    const dataArray = new Uint8Array(bytes);
    ws!.send(dataArray);
  } catch (e) {
    return true;
  }
  return true;
}

function password(ptr: number, max_len: number) {
  try {
    const val = window.prompt("Password:");
    if (val === null) {
      return false;
    }
    const encoder = new TextEncoder();
    const bytes = encoder.encode(val);
    const len = Math.min(bytes.length, max_len);
    const view = new Uint8Array(memory.buffer, ptr, max_len);
    view.fill(0);
    view.set(bytes.subarray(0, len));
    return true;
  } catch (e) {
    console.error(e);
    return false;
  }
}

const decoder = new TextDecoder();
const wasmUrl = new URL(
  import.meta.env.DEV ? '../target/wasm32-unknown-unknown/debug/wvnc.wasm' : '../wvnc.wasm',
  import.meta.url);
const obj = await WebAssembly.instantiateStreaming(
  fetch(wasmUrl),
  {
    env: {
      debug_print: debug_print,
      set_title: set_title,
      resize: resize,
      fill_rect: fill_rect,
      update_rect: update_rect,
      copy_rect: copy_rect,
      bell: bell,
      clipboard_text: clipboard_text,
      led: led,
      pointer_mode: pointer_mode,
      audio_start: audio_start,
      audio_stop: audio_stop,
      audio_data: audio_data,
      socket_open: socket_open,
      socket_close: socket_close,
      socket_send: socket_send,
      password: password,
    }
  }
);
const instance = obj.instance;
const memory = instance.exports.memory as WebAssembly.Memory;
const keyboard = instance.exports.keyboard as CallableFunction;
const pointer = instance.exports.pointer as CallableFunction;
const socket_event = instance.exports.socket_event as CallableFunction;
const alloc_buffer = instance.exports.alloc_buffer as CallableFunction;

framebuffer.onkeyboard = (down: boolean, key: number) => {
  keyboard(down, key);
};

framebuffer.onpointer = (button: number, x: number, y: number) => {
  if (pointerRelative) {
    pointer(button, (x - prevX) * 2 + 0x7fff, (y - prevY) * 2 + 0x7fff);
    prevX = x;
    prevY = y;
  } else {
    pointer(button, x, y);
  }
};

function start() {
  const width = framebuffer.width;
  const height = framebuffer.height;
  const res = (instance.exports.start as CallableFunction)(width, height);
  console.log(res);
}

start();
