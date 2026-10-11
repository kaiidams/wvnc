const SOURCE_BUFFER_SIZE = 4096 * 80;
const PROCESS_BUFFER_SIZE = 4096 * 4;
const MAX_BUFFER_SIZE = 200;

export class AudioSink {
  readonly audioContext: AudioContext;
  source: AudioBufferSourceNode | null;
  samplingRate: number;
  numberOfChannels: number;
  readonly queue: ArrayBuffer[];
  currentBuffer: Int16Array | null;
  currentPosition: number;

  constructor() {
    this.audioContext = new AudioContext();
    this.source = null;
    this.samplingRate = 48000; // 44100;
    this.numberOfChannels = 2;
    this.queue = [];
    this.currentBuffer = null;
    this.currentPosition = 0;
  }

  public start() {
    console.debug("start");
    const buffer = this.audioContext.createBuffer(
      1, SOURCE_BUFFER_SIZE, this.samplingRate,
    );
    const data = buffer.getChannelData(0);
    for (let i = 0; i < data.length; i++) {
      data[i] = 0.0;
    }
    this.source = this.audioContext.createBufferSource();
    this.source.buffer = buffer;
    this.source.loop = true;

    this.queue.length = 0;

    const processor = this.audioContext.createScriptProcessor(PROCESS_BUFFER_SIZE, 1, this.numberOfChannels);
    if (this.numberOfChannels == 1) {
      processor.onaudioprocess = this.handleAudioProcessMono.bind(this);
    } else if (this.numberOfChannels == 2) {
      processor.onaudioprocess = this.handleAudioProcessStereo.bind(this);
    } else {
      throw new Error();
    }
    processor.connect(this.audioContext.destination);
    this.source.connect(processor);
    this.source.start();
  }

  public push(array: ArrayBuffer) {
    if (this.queue.length > MAX_BUFFER_SIZE) {
      console.warn("Audio buffer is full.");
      this.queue.shift();
    }
    this.queue.push(array);
  }

  public setFormat() {
  }

  public stop() {
    console.debug("stop");
    this.source!.stop();
    this.source = null;
  }

  handleAudioProcessMono(ev: AudioProcessingEvent) {
    const data = ev.outputBuffer.getChannelData(0);
    let i;
    let buffer = this.currentBuffer;
    let position = this.currentPosition;
    for (i = 0; i < data.length; i++) {
      if (buffer === null || position >= buffer.length) {
        const array = this.queue.shift();
        if (array == null) {
          this.currentBuffer = null;
          break;
        }
        buffer = new Int16Array(array);
        this.currentBuffer = buffer;
        position = 0;
      }
      data[i] = buffer[position++] / 32767.0;
    }
    this.currentPosition = position;
    for (; i < data.length; i++) {
      data[i] = 0.0;
    }
  }

  handleAudioProcessStereo(ev: AudioProcessingEvent) {
    const data0 = ev.outputBuffer.getChannelData(0);
    const data1 = ev.outputBuffer.getChannelData(1);
    let i;
    let buffer = this.currentBuffer;
    let position = this.currentPosition;
    for (i = 0; i < data0.length; i++) {
      if (buffer === null || position >= buffer.length) {
        const array = this.queue.shift();
        if (array == null) {
          this.currentBuffer = null;
          break;
        }
        buffer = new Int16Array(array);
        this.currentBuffer = buffer;
        position = 0;
      }
      data0[i] = buffer[position++] / 32767.0;
      data1[i] = buffer[position++] / 32767.0;
    }
    this.currentPosition = position;
    for (; i < data0.length; i++) {
      data0[i] = 0.0;
      data1[i] = 0.0;
    }
  }
}
