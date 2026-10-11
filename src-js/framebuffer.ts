const KEYCODE_MAP: { [key: string]: number } = {
  'Backspace': 0xff08,
  'Tab': 0xff09,
  'Enter': 0xff0d,
  'Escape': 0xff1b,
  'Insert': 0xff63,
  'Delete': 0xffff,
  'Home': 0xff50,
  'End': 0xff57,
  'PageUp': 0xff55,
  'PageDown': 0xff56,
  'ArrowLeft': 0xff51,
  'ArrowUp': 0xff52,
  'ArrowRight': 0xff53,
  'ArrowDown': 0xff54,
  // F1..F12
  'Shift': 0xffe1,
  'Control': 0xffe3,
  'Meta': 0xffe7,
  'Alt': 0xffe9,
};

for (let i = 1; i <= 12; i++) {
  KEYCODE_MAP[`F${i}`] = 0xffbe + i - 1;
}

console.assert(KEYCODE_MAP['F12'] == 0xffc9);

const KEYCODE_MAP_LEFT: { [key: string]: number } = {
  'Shift': 0xffe1,
  'Control': 0xffe3,
  'Meta': 0xffe7,
  'Alt': 0xffe9,
};

const KEYCODE_MAP_RIGHT: { [key: string]: number } = {
  'Shift': 0xffe2,
  'Control': 0xffe4,
  'Meta': 0xffe8,
  'Alt': 0xffea,
};

function getKeyCodeFromEvent(event: KeyboardEvent): number | null {
  const key = event.key;
  if (event.location == KeyboardEvent.DOM_KEY_LOCATION_LEFT) {
    const code = KEYCODE_MAP_LEFT[key];
    if (code) {
      return code;
    }
  } else if (event.location == KeyboardEvent.DOM_KEY_LOCATION_RIGHT) {
    const code = KEYCODE_MAP_RIGHT[key];
    if (code) {
      return code;
    }
  }
  const code = KEYCODE_MAP[key];
  if (code) {
    return code;
  }
  if (key.length == 1) {
    const code = key.charCodeAt(0);
    if (code >= 0x20 && code <= 0x7f) {
      return code;
    }
  }
  return null;
}

/**
 * A frame buffer component options.
 */
interface FrameBufferComponentOptions {

  /**
   * The width of the frame buffer in pixels.
   */
  width?: number;

  /**
   * The height of the frame buffer in pixels.
   */
  height?: number;

  /**
   * The physical pixels / the frame buffer pixels.
   */
  scale?: number;

  /**
   * Allow fractional scaling.
   */
  fractionalScale?: boolean;
}

/**
 * A keyboard event handler.
 * @param down True if the key is pressed down, false if released.
 * @param key The key code.
 */
export type KeyboardEventHandler = (down: boolean, key: number) => void;

/**
 * A pointer event handler.
 * @param button The button state.
 * @param x The pointer x position in pixels.
 * @param y The pointer y position in pixels.
 */
export type PointerEventHandler = (button: number, x: number, y: number) => void;

/**
 * A user event handler.
 * @param event The user event.
 */
export type UserEventHandler = ((event: any) => void);

interface FrameBufferResizeResult {
  /**
   * The width of the frame buffer in pixels.
   */
  width: number;

  /**
   * The height of the frame buffer in pixels.
   */
  height: number;
  scale: number
}

/**
 * Frame buffer API based on HTML Canvas element.
 */
export class FrameBufferComponent {
  private readonly container: HTMLElement;
  private readonly options: FrameBufferComponentOptions;
  private readonly canvas: HTMLCanvasElement;
  private readonly context: CanvasRenderingContext2D;
  private readonly resizeEventHandler: () => void;
  private readonly blurEventHandler: () => void;
  private readonly focusEventHandler: () => void;
  private readonly visibilityChangeHandler: () => void;
  private readonly pressedKeys = new Set<number>();
  private isBlurred = false;
  private button: number;

  /**
   * The width of the frame buffer in pixels.
   */
  public width: number;

  /**
   * The height of the frame buffer in pixels.
   */
  public height: number;

  /**
   * The physical pixels / the frame buffer pixels.
   */
  public computedScale: number;

  /**
   * The keyboard event handler.
   */
  public onkeyboard: KeyboardEventHandler | null;

  /**
   * The pointer event handler.
   */
  public onpointer: PointerEventHandler | null;

  /**
   * The user event handler.
   */
  public onuser: UserEventHandler | null;

  constructor(
    container: HTMLElement,
    options?: FrameBufferComponentOptions) {
    this.container = container;
    this.options = Object.assign({
      width: 640, height: 480, scale: 0, resize: true, fractionalScale: true,
    }, options);
    this.canvas = document.createElement('canvas');
    this.canvas.tabIndex = 0;
    if (this.options.width) {
      this.canvas.width = this.options.width;
    }
    if (this.options.height) {
      this.canvas.height = this.options.height;
    }
    this.width = 0;
    this.height = 0;
    this.computedScale = 1;
    this.canvas.style.margin = 'auto';
    this.canvas.style.imageRendering = 'pixelated';
    this.canvas.style.backgroundColor = 'black';
    this.canvas.style.outline = 'none';
    this.canvas.style.userSelect = 'none';
    this.canvas.style.cursor = 'none';
    this.onpointer = null;
    this.onkeyboard = null;
    this.onuser = null;
    this.button = 0;
    this.context = this.canvas.getContext('2d') as CanvasRenderingContext2D;
    this.context.imageSmoothingEnabled = false;
    this.container.appendChild(this.canvas);

    this.resizeEventHandler = this.resizeCanvas.bind(this);
    window.addEventListener('resize', this.resizeEventHandler);
    this.blurEventHandler = this.handleBlur.bind(this);
    this.focusEventHandler = () => { this.isBlurred = false; };
    this.visibilityChangeHandler = () => {
      if (document.hidden) {
        this.handleBlur();
      }
    };
    window.addEventListener('blur', this.blurEventHandler);
    window.addEventListener('focus', this.focusEventHandler);
    window.addEventListener('pagehide', this.blurEventHandler);
    document.addEventListener('visibilitychange', this.visibilityChangeHandler);
    this.canvas.addEventListener('blur', this.blurEventHandler);
    this.canvas.addEventListener('focus', this.focusEventHandler);
    this.canvas.addEventListener('pointerdown', this.handlePointerEvent.bind(this));
    this.canvas.addEventListener('pointerup', this.handlePointerEvent.bind(this));
    this.canvas.addEventListener('pointermove', this.handlePointerEvent.bind(this));
    this.canvas.addEventListener('wheel', this.handleWheelEvent.bind(this), { passive: true });
    this.canvas.addEventListener('contextmenu', (event) => {
      // Prevents context menu.
      event.preventDefault();
    });
    this.canvas.addEventListener('touchstart', (event) => {
      // Prevents haptic feedback on long touch press.
      event.preventDefault();
    });
    this.canvas.addEventListener('keydown', this.handleKeyboardEvent.bind(this));
    this.canvas.addEventListener('keyup', this.handleKeyboardEvent.bind(this));
    this.resizeCanvas();
  }

  public resize(width?: number, height?: number) {
    console.log("hoge");
    this.options.width = width;
    this.options.height = height;
    if (this.options.width) {
      this.canvas.width = this.options.width;
    }
    if (this.options.height) {
      this.canvas.height = this.options.height;
    }
    this.resizeCanvas();
  }

  /**
   * Focuses the frame buffer component.
   */
  public focus() {
    this.canvas.focus();
  }

  public computeCanvasSize(displayWidth: number, displayHeight: number, dpi: number, options: FrameBufferComponentOptions): FrameBufferResizeResult {
    let { width, height, scale, fractionalScale } = options;
    if (!scale) {
      if (width == null && height == null) {
        console.info("The default size as no restriction is given.");
        width = 640;
        height = 480;
        scale = 1.0;
      } else if (height == null) {
        console.info("Height is resized to fit the window.");
        scale = displayWidth * dpi / width!;
      } else if (width == null) {
        console.info("Width is resized to fit the window.");
        scale = displayHeight * dpi / height!;
      } else if (height * displayWidth >= displayHeight * width) {
        console.info("Frame buffer size constrained by height.");
        scale = displayHeight * dpi / height;
      } else {
        console.info("Frame buffer size constrained by width.");
        scale = displayWidth * dpi / width;
      }
      if (!fractionalScale) {
        scale = Math.floor(scale);
      }
    } else {
      if (width == null) {
        width = 640;
      }
      if (height == null) {
        height = 480;
      }
    }

    scale = scale / dpi;

    if (width == null) {
      width = Math.floor(displayWidth / scale);
    }

    if (height == null) {
      height = Math.floor(displayHeight / scale);
    }

    return { width, height, scale };
  }

  private resizeCanvas() {
    console.log("resize");
    const displayWidth = window.innerWidth;
    const displayHeight = window.innerHeight;
    const dpi = window.devicePixelRatio;
    const size = this.computeCanvasSize(displayWidth, displayHeight, dpi, this.options);
    this.canvas.style.width = Math.floor(size.width * size.scale) + "px";
    this.canvas.style.height = Math.floor(size.height * size.scale) + "px";
    this.computedScale = size.scale;
    if (size.width != this.width || size.height != this.height) {
      this.width = size.width;
      this.height = size.height;
      this.canvas.width = size.width;
      this.canvas.height = size.height;
    }
  }

  private handlePointerEvent(event: PointerEvent) {
    event.preventDefault();
    let button = event.button;
    if (button == 3) {
      // back
      button = 7;
    } else if (button >= 4) {
      // forward...
      button = 0;
    }
    if (event.type == "pointerdown") {
      this.canvas.focus();
      this.button |= 1 << button;
      this.canvas.setPointerCapture(event.pointerId);
    } else if (event.type == "pointerup") {
      this.button &= ~(1 << button);
      this.canvas.releasePointerCapture(event.pointerId);
    }
    const x = Math.floor(event.offsetX / this.computedScale);
    const y = Math.floor(event.offsetY / this.computedScale);
    if (this.onpointer) {
      this.onpointer(this.button, x, y);
    }
  }

  private handleWheelEvent(event: WheelEvent) {
    let button = this.button;
    if (event.deltaY < 0) {
      // Scroll up
      button |= 1 << 3;
    } else if (event.deltaY > 0) {
      // Scroll down
      button |= 1 << 4;
    }
    if (event.deltaX < 0) {
      // Scroll left
      button |= 1 << 5;
    } else if (event.deltaX > 0) {
      // Scroll right
      button |= 1 << 6;
    }
    const x = Math.floor(event.offsetX / this.computedScale);
    const y = Math.floor(event.offsetY / this.computedScale);
    if (this.onpointer) {
      this.onpointer(button, x, y);
      this.onpointer(this.button, x, y);
    }
  }

  private handleKeyboardEvent(event: KeyboardEvent) {
    event.preventDefault();
    this.isBlurred = false;
    const key = getKeyCodeFromEvent(event);
    if (key !== null) {
      const down = event.type == "keydown";
      if (down) {
        this.pressedKeys.add(key);
      } else {
        this.pressedKeys.delete(key);
      }
      if (this.onkeyboard)
        this.onkeyboard(down, key);
    } else {
      console.error(`Unknown key: ${event.key}`);
    }
  }

  public releaseAllKeys() {
    for (const key of this.pressedKeys) {
      if (this.onkeyboard) {
        this.onkeyboard(false, key);
      }
    }
    this.pressedKeys.clear();
  }

  private handleBlur() {
    if (this.isBlurred && this.pressedKeys.size === 0) {
      return;
    }
    this.isBlurred = true;
    this.releaseAllKeys();
  }

  /**
   * Set the title.
   * @param text Title text.
   */
  public setTitle(text: string) {
    document.title = text;
  }

  /**
   * Updates the rectangle with the specified color.
   * @param pixel The color to fill the rectangle with.
   * @param x The x-coordinate of the rectangle.
   * @param y The y-coordinate of the rectangle.
   * @param width The width of the rectangle.
   * @param height The height of the rectangle.
   */
  public fillRect(pixel: number, x: number, y: number, width: number, height: number) {
    const r = (pixel >> 16) & 255;
    const g = (pixel >> 8) & 255;
    const b = pixel & 255;
    const style = `rgb(${r}, ${g}, ${b})`;
    this.context.fillStyle = style;
    this.context.fillRect(x, y, width, height);
  }

  /**
   * Updates the rectangle with the pixel data.
   * @param data The pixel data.
   * @param x The x-coordinate of the rectangle.
   * @param y The y-coordinate of the rectangle.
   * @param width The width of the rectangle.
   * @param height The height of the rectangle.
   */
  public updateRect(dataArray: ImageDataArray, x: number, y: number, width: number, height: number) {
    const imageData = new ImageData(dataArray, width, height);
    this.context.putImageData(imageData, x, y);
  }

  /**
   * Copy the rectangle.
   * @param sx The x-coordinate of the source.
   * @param sy The y-coordinate of the source.
   * @param dx The x-coordinate of the destination.
   * @param dy The y-coordinate of the destination.
   * @param width The width of the rectangle.
   * @param height The height of the rectangle.
   */
  public copyRect(sx: number, sy: number, dx: number, dy: number, width: number, height: number) {
    const imageData = this.context.getImageData(sx, sy, width, height);
    this.context.putImageData(imageData, dx, dy);
  }

  public postUserEvent(event: any) {
    if (this.onuser)
      this.onuser(event);
  }

  /**
   * Requests to ehter the full screen mode.
   */
  public requestFullScreen(options?: FullscreenOptions) {
    this.container.requestFullscreen(options);
  }

  /**
   * Disposes the frame buffer component.
   */
  public dispose() {
    window.removeEventListener('resize', this.resizeEventHandler);
    window.removeEventListener('blur', this.blurEventHandler);
    window.removeEventListener('focus', this.focusEventHandler);
    window.removeEventListener('pagehide', this.blurEventHandler);
    document.removeEventListener('visibilitychange', this.visibilityChangeHandler);
    this.canvas.removeEventListener('blur', this.blurEventHandler);
    this.canvas.removeEventListener('focus', this.focusEventHandler);
    this.container.removeChild(this.canvas);
  }
}
