use crate::audio::AudioSink;
use crate::d3des;
use crate::error::Error;
use crate::events::{self, Event};
use crate::framebuffer::FrameBuffer;
use crate::io;
use crate::peer;

#[derive(Default, Debug)]
struct PixelFormat {
    bits_per_pixel: u8,
    depth: u8,
    big_endian_flag: bool,
    true_colour_flag: bool,
    red_max: u16,
    green_max: u16,
    blue_max: u16,
    red_shift: u8,
    green_shift: u8,
    blue_shift: u8,
}

impl PixelFormat {
    pub fn from_bytes(buf: &[u8; 16]) -> PixelFormat {
        PixelFormat {
            bits_per_pixel: buf[0],
            depth: buf[1],
            big_endian_flag: buf[2] != 0,
            true_colour_flag: buf[3] != 0,
            red_max: u16::from_be_bytes(buf[4..6].try_into().unwrap()),
            green_max: u16::from_be_bytes(buf[6..8].try_into().unwrap()),
            blue_max: u16::from_be_bytes(buf[8..10].try_into().unwrap()),
            red_shift: buf[10],
            green_shift: buf[11],
            blue_shift: buf[12],
        }
    }

    pub fn to_bytes(&self) -> [u8; 16] {
        let mut buf = [0_u8; 16];
        buf[0] = self.bits_per_pixel;
        buf[1] = self.depth;
        buf[2] = self.big_endian_flag as u8;
        buf[3] = self.true_colour_flag as u8;
        buf[4..6].copy_from_slice(&self.red_max.to_be_bytes());
        buf[6..8].copy_from_slice(&self.green_max.to_be_bytes());
        buf[8..10].copy_from_slice(&self.blue_max.to_be_bytes());
        buf[10] = self.red_shift;
        buf[11] = self.green_shift;
        buf[12] = self.blue_shift;
        buf
    }
}

#[derive(Default)]
struct RfbClientData {
    version: (u16, u16),
    width: u16,
    height: u16,
    pixel_format: PixelFormat,
    name: Vec<u8>,
}

pub struct HandshakeData {
    client: RfbClientData,
    pub security_types: Vec<u8>,
}

pub async fn handshake(
    rd: &mut io::AsyncReader,
    wr: &mut io::AsyncWriter,
) -> Result<HandshakeData, Error> {
    let mut client = RfbClientData::default();
    receive_protocol_version(&mut client, rd).await?;
    send_protocol_version(&mut client, wr).await?;
    let security_types = receive_security(&mut client, rd).await?;
    Ok(HandshakeData {
        client,
        security_types,
    })
}

pub struct MessageReader {
    client: RfbClientData,
    fb: FrameBuffer,
    audio_sink: AudioSink,
}

pub struct MessageWriter {
    //
    pointer_relative: bool,
}

pub async fn initialize(
    rd: &mut io::AsyncReader,
    wr: &mut io::AsyncWriter,
    mut fb: FrameBuffer,
    audio_sink: AudioSink,
    data: HandshakeData,
) -> Result<(MessageReader, MessageWriter), Error> {
    let mut client = data.client;
    if data.security_types.contains(&1) {
        send_security(&mut client, wr, 1).await?;
    } else if data.security_types.contains(&2) {
        send_security(&mut client, wr, 2).await?;
        let mut msg = [0_u8; 16];
        rd.read_exact(&mut msg).await?;
        let mut key = [0_u8; 8];
        let ok = unsafe { peer::password(key.as_mut_ptr(), key.len()) };
        if !ok {
            return Err(Error::GENERIC);
        }
        unsafe {
            d3des::deskey(key.as_ptr(), 0);
            d3des::des(msg[0..8].as_mut_ptr(), msg[0..8].as_mut_ptr());
            d3des::des(msg[8..16].as_mut_ptr(), msg[8..16].as_mut_ptr());
        }
        wr.write_all(&msg).await?;
    } else {
        return Err(Error::PROTOCOL);
    }
    receive_security_result(&mut client, rd).await?;
    send_client_init(&mut client, wr).await?;
    receive_server_init(&mut client, rd).await?;
    fb.set_title(&client.name)?;
    fb.resize(client.width, client.height)?;

    let mut msgwr = MessageWriter {
        pointer_relative: true,
    };

    client.pixel_format = PixelFormat {
        bits_per_pixel: 32,
        depth: 24,
        big_endian_flag: false,
        true_colour_flag: true,
        red_max: 255,
        green_max: 255,
        blue_max: 255,
        red_shift: 16,
        green_shift: 8,
        blue_shift: 0,
    };
    send_set_pixel_format(wr, &mut msgwr, &client.pixel_format).await?;

    // let encodings = vec![0, 1, -223, -257, -258, -259, -261];
    let encodings = vec![
        0, // Raw
        1, // CopyRect
        2, // RRE
        3, // CoRRE
        4, // Hextile
        // -257, // Rel pointer
        -259, // QEMU Audio Pseudo-encoding
        -223, -261, // LED
    ];

    send_set_encodings(wr, &mut msgwr, &encodings).await?;
    send_framebufer_update_request(wr, &mut msgwr, false, 0, 0, client.width, client.height)
        .await?;

    let msgrd = MessageReader {
        client,
        fb,
        audio_sink,
    };

    Ok((msgrd, msgwr))
}

pub async fn handle_messages(
    rd: &mut io::AsyncReader,
    msgrd: &mut MessageReader,
) -> Result<(), Error> {
    let res = inner_handle_messages(rd, msgrd).await;
    events::post(events::Event::FrameBufferUpdate(2, 0, 0));
    msgrd.fb.fill_rect(0, 0, 0, 1024, 768)?;
    res
}

async fn inner_handle_messages(
    rd: &mut io::AsyncReader,
    msgrd: &mut MessageReader,
) -> Result<(), Error> {
    let mut msg_type = [0_u8; 1];
    loop {
        rd.read_exact(&mut msg_type).await?;
        log_debug!("msg_type: {}", msg_type[0]);
        match msg_type[0] {
            0 => {
                receive_framebuffer_update(rd, msgrd).await?;
            }
            1 => {
                // SetColorMapEntries
            }
            2 => {
                receive_bell(rd, msgrd).await?;
            }
            3 => {
                receive_server_cut_text(rd, msgrd).await?;
            }
            255 => {
                // 7.5.9   QEMU Server Message
                receive_qemu_server_message(rd, msgrd).await?;
            }
            _ => return Err(Error::PROTOCOL),
        }
    }
}

pub async fn handle_events(
    wr: &mut io::AsyncWriter,
    msgwr: &mut MessageWriter,
) -> Result<(), Error> {
    loop {
        match events::get_next().await? {
            Event::Keyboard(down, key) => {
                // log_debug!("keyboard {} {}", down, key);
                send_keyboard(wr, msgwr, down, key).await?;
            }
            Event::Pointer(button, x, y) => {
                // log_debug!("Event::Pointer button:{} x:{} y:{}", button, x, y);
                send_pointer(wr, msgwr, button, x, y).await?;
            }
            Event::Clipboard() => {
                log_debug!("clipboard");
                send_client_cut_text(wr, msgwr, &"hello".as_bytes()).await?;
            }
            Event::FrameBufferUpdate(flags, width, height) => {
                // log_debug!("Event::FrameBufferUpdate({}, {})", width, height);
                let incremental = true;
                let x = 0;
                let y = 0;
                send_framebufer_update_request(wr, msgwr, incremental, x, y, width, height).await?;
                if flags & 1 != 0 {
                    log_debug!("Audio capture start");
                    send_audio_format(wr, 3, 2, 48000).await?;
                    send_enable_audio_capture(wr, 0).await?;
                }
                if flags & 2 != 0 {
                    break;
                }
            }
        };
    }
    Ok(())
}

async fn send_set_pixel_format(
    wr: &mut io::AsyncWriter,
    msgwr: &mut MessageWriter,
    pixel_format: &PixelFormat,
) -> Result<(), Error> {
    // 7.4.1   SetPixelFormat
    let mut msg = vec![0_u8; 20];
    msg[0] = 0;
    msg[4..20].copy_from_slice(&pixel_format.to_bytes());
    wr.write_all(&msg).await?;
    Ok(())
}

pub async fn send_set_encodings(
    wr: &mut io::AsyncWriter,
    msgwr: &mut MessageWriter,
    encodings: &[i32],
) -> Result<(), Error> {
    // 7.4.2   SetEncodings
    let mut msg = vec![0_u8; 4 + 4 * encodings.len()];
    msg[0] = 2;
    msg[2..4].copy_from_slice(&(encodings.len() as u16).to_be_bytes());
    for (i, encoding) in encodings.iter().enumerate() {
        msg[4 * i + 4..4 * i + 8].copy_from_slice(&encoding.to_be_bytes());
    }
    wr.write_all(&msg).await?;
    Ok(())
}

pub async fn send_framebufer_update_request(
    wr: &mut io::AsyncWriter,
    msgwr: &mut MessageWriter,
    incremental: bool,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
) -> Result<(), Error> {
    let mut msg = [0_u8; 10];
    msg[0] = 3;
    msg[1] = incremental as u8;
    msg[2..4].copy_from_slice(&x.to_be_bytes());
    msg[4..6].copy_from_slice(&y.to_be_bytes());
    msg[6..8].copy_from_slice(&width.to_be_bytes());
    msg[8..10].copy_from_slice(&height.to_be_bytes());
    wr.write_all(&msg).await?;
    Ok(())
}

pub async fn send_keyboard(
    wr: &mut io::AsyncWriter,
    msgwr: &mut MessageWriter,
    down: bool,
    key: u32,
) -> Result<(), Error> {
    let mut msg = [0_u8; 8];
    msg[0] = 4;
    msg[1] = down as u8;
    msg[4..8].copy_from_slice(&key.to_be_bytes());
    wr.write_all(&msg).await?;
    Ok(())
}

pub async fn send_pointer(
    wr: &mut io::AsyncWriter,
    msgwr: &mut MessageWriter,
    button: u8,
    x: u16,
    y: u16,
) -> Result<(), Error> {
    let mut msg = [0_u8; 6];
    msg[0] = 5;
    msg[1] = button;
    msg[2..4].copy_from_slice(&x.to_be_bytes());
    msg[4..6].copy_from_slice(&y.to_be_bytes());
    wr.write_all(&msg).await?;
    Ok(())
}

pub async fn send_client_cut_text(
    wr: &mut io::AsyncWriter,
    msgwr: &mut MessageWriter,
    text: &[u8],
) -> Result<(), Error> {
    let mut msg = vec![0_u8; 8 + text.len()];
    msg[0] = 6;
    msg[4..8].copy_from_slice(&text.len().to_be_bytes());
    msg[8..].copy_from_slice(text);
    wr.write_all(&msg).await?;
    Ok(())
}

async fn receive_protocol_version(
    client: &mut RfbClientData,
    rd: &mut io::AsyncReader,
) -> Result<(), Error> {
    let mut msg = [0_u8; 12];
    rd.read_exact(&mut msg).await?;
    let mut version = parse_rfb_hello(&msg)?;
    if version.0 != 3 {
        return Err(Error::PROTOCOL);
    }
    version.1 = match version.1 {
        8.. => 8,
        7 => 7,
        _ => 3,
    };
    client.version = version;
    log_debug!(
        "Server RFB protocol version: {}.{}",
        client.version.0,
        client.version.1
    );
    Ok(())
}

fn rfb_version_from_bytes(buf: &[u8; 3]) -> Result<u16, Error> {
    let mut ver = 0_u16;
    for ch in buf.iter() {
        if !(*ch >= b'0' && *ch <= b'9') {
            return Err(Error::PROTOCOL);
        }
        ver = ver * 10 + (*ch - b'0') as u16;
    }
    Ok(ver)
}

fn rfb_version_to_bytes(version: u16) -> Result<[u8; 3], Error> {
    if version >= 1000 {
        return Err(Error::GENERIC);
    }
    let mut res = [0_u8; 3];
    let mut n = version;
    res[2] = (n % 10) as u8 + b'0';
    n /= 10;
    res[1] = (n % 10) as u8 + b'0';
    n /= 10;
    res[0] = n as u8 + b'0';
    Ok(res)
}

fn parse_rfb_hello(msg: &[u8; 12]) -> Result<(u16, u16), Error> {
    if !(msg[..4].eq(b"RFB ") && msg[7] == b'.' && msg[11] == b'\n') {
        return Err(Error::PROTOCOL);
    }
    let major_version = rfb_version_from_bytes(&msg[4..7].try_into()?)?;
    let minor_version = rfb_version_from_bytes(&msg[8..11].try_into()?)?;
    Ok((major_version, minor_version))
}

async fn send_protocol_version(
    client: &mut RfbClientData,
    wr: &mut io::AsyncWriter,
) -> Result<(), Error> {
    let mut msg = [0_u8; 12];
    msg.copy_from_slice(b"RFB 000.000\n");
    let version = client.version;
    msg[4..7].copy_from_slice(&rfb_version_to_bytes(version.0)?);
    msg[8..11].copy_from_slice(&rfb_version_to_bytes(version.1)?);
    log_debug!(
        "Server RFB protocol version: {}.{}",
        client.version.0,
        client.version.1
    );
    wr.write_all(&msg).await?;
    Ok(())
}

async fn receive_security(
    _client: &mut RfbClientData,
    rd: &mut io::AsyncReader,
) -> Result<Vec<u8>, Error> {
    let mut msg = [0_u8; 1];
    rd.read_exact(&mut msg).await?;
    let number_of_security_types = msg[0] as usize;
    let mut msg = vec![0_u8; number_of_security_types];
    rd.read_exact(&mut msg).await?;
    let security_types = msg;
    Ok(security_types)
}

async fn send_security(
    _client: &mut RfbClientData,
    wr: &mut io::AsyncWriter,
    security_type: u8,
) -> Result<(), Error> {
    let msg = [security_type];
    wr.write_all(&msg).await?;
    Ok(())
}

async fn receive_security_result(
    _client: &mut RfbClientData,
    rd: &mut io::AsyncReader,
) -> Result<u32, Error> {
    let mut msg = [0_u8; 4];
    rd.read_exact(&mut msg).await?;
    let status = u32::from_be_bytes(msg[0..4].try_into().unwrap());
    log_debug!("SecurityResult(status={})", status);
    Ok(status)
}

async fn send_client_init(
    _client: &mut RfbClientData,
    wr: &mut io::AsyncWriter,
) -> Result<(), Error> {
    let msg = [0_u8];
    wr.write_all(&msg).await?;
    Ok(())
}

async fn receive_server_init(
    client: &mut RfbClientData,
    rd: &mut io::AsyncReader,
) -> Result<(), Error> {
    let mut msg = [0_u8; 24];
    rd.read_exact(&mut msg).await?;
    client.width = u16::from_be_bytes(msg[0..2].try_into().unwrap());
    client.height = u16::from_be_bytes(msg[2..4].try_into().unwrap());
    client.pixel_format = PixelFormat::from_bytes(msg[4..20].try_into().unwrap());
    let name_length = u32::from_be_bytes(msg[20..24].try_into().unwrap());
    let name_length = name_length as usize;
    client.name = vec![0_u8; name_length];
    rd.read_exact(&mut client.name).await?;
    log_debug!(
        "ServerInit(width={}, height={}, pixel_format={:?}, name=\"{}\")",
        client.width,
        client.height,
        client.pixel_format,
        String::from_utf8(client.name.clone())?,
    );
    Ok(())
}

pub async fn receive_framebuffer_update(
    rd: &mut io::AsyncReader,
    msgrd: &mut MessageReader,
) -> Result<(), Error> {
    let mut msg = [0_u8; 4];
    rd.read_exact(&mut msg[1..]).await?;
    let number_of_encodings = u16::from_be_bytes(msg[2..4].try_into().unwrap());
    let mut flags = 0;
    for i in 0..number_of_encodings {
        let mut msg = [0_u8; 12];
        rd.read_exact(&mut msg).await?;
        let x = u16::from_be_bytes(msg[0..2].try_into().unwrap());
        let y = u16::from_be_bytes(msg[2..4].try_into().unwrap());
        let width = u16::from_be_bytes(msg[4..6].try_into().unwrap());
        let height = u16::from_be_bytes(msg[6..8].try_into().unwrap());
        let encoding_type = i32::from_be_bytes(msg[8..12].try_into().unwrap());
        log_debug!(
            "  [{}]=Rectangle(x={}, y={}, width={}, height={}, encoding_type={})",
            i,
            x,
            y,
            width,
            height,
            encoding_type,
        );
        match encoding_type {
            0 => {
                // 7.6.1   Raw Encoding
                let mut data = vec![0_u8; width as usize * height as usize * 4];
                rd.read_exact(&mut data).await?;
                for j in 0..(width as usize * height as usize) {
                    // BGRA (little endian) -> RGBA
                    data.swap(j * 4, j * 4 + 2);
                    data[j * 4 + 3] = 255;
                }
                log_debug!("  raw=..");
                msgrd.fb.update_rect(&data, x, y, width, height)?;
            }
            1 => {
                // 7.6.2   CopyRect Encoding
                let mut msg = [0_u8; 4];
                rd.read_exact(&mut msg).await?;
                let sx = u16::from_be_bytes(msg[0..2].try_into().unwrap());
                let sy = u16::from_be_bytes(msg[2..4].try_into().unwrap());
                log_debug!("  CopyRect({}, {})", sx, sy);
                msgrd.fb.copy_rect(sx, sy, x, y, width, height)?;
            }
            2 => {
                // 7.6.3   RRE Encoding
                let mut msg = [0_u8; 8];
                rd.read_exact(&mut msg).await?;
                let number_of_subrectangles = u32::from_be_bytes(msg[0..4].try_into().unwrap());
                let pixel = u32::from_be_bytes(msg[4..8].try_into().unwrap());
                msgrd.fb.fill_rect(pixel, x, y, width, height)?;
                log_debug!("  RRE: {}", number_of_subrectangles);
                for _ in 0..number_of_subrectangles {
                    let mut msg = [0_u8; 12];
                    rd.read_exact(&mut msg).await?;
                    let pixel = u32::from_be_bytes(msg[0..4].try_into().unwrap());
                    let x = u16::from_be_bytes(msg[4..6].try_into().unwrap());
                    let y = u16::from_be_bytes(msg[6..8].try_into().unwrap());
                    let width = u16::from_be_bytes(msg[8..10].try_into().unwrap());
                    let height = u16::from_be_bytes(msg[10..12].try_into().unwrap());
                    msgrd.fb.fill_rect(pixel, x, y, width, height)?;
                }
            }
            3 => {
                // 7.6.4   CoRRE Encoding
                let mut msg = [0_u8; 8];
                rd.read_exact(&mut msg).await?;
                let number_of_subrectangles = u32::from_be_bytes(msg[0..4].try_into().unwrap());
                let pixel = u32::from_be_bytes(msg[4..8].try_into().unwrap());
                msgrd.fb.fill_rect(pixel, x, y, width, height)?;
                log_debug!("  RRE: {}", number_of_subrectangles);
                for _ in 0..number_of_subrectangles {
                    let mut msg = [0_u8; 12];
                    rd.read_exact(&mut msg).await?;
                    let pixel = u32::from_be_bytes(msg[0..4].try_into().unwrap());
                    let x = msg[4] as u16;
                    let y = msg[5] as u16;
                    let width = msg[6] as u16;
                    let height = msg[7] as u16;
                    msgrd.fb.fill_rect(pixel, x, y, width, height)?;
                }
            }
            4 => {
                // 7.6.5   Hextile Encoding
                for dy in (0..height).step_by(16) {
                    for dx in (0..width).step_by(16) {
                        let dw = std::cmp::min(width - dx, 16);
                        let dh = std::cmp::min(height - dy, 16);
                        let mut msg = [0_u8; 1];
                        rd.read_exact(&mut msg).await?;
                        let subencoding_mask = msg[0];
                        if subencoding_mask & 1 != 0 {
                            // raw
                            let mut msg = [0_u8; 4 * 256];
                            rd.read_exact(&mut msg[..(4 * dw * dh) as usize]).await?;
                        } else {
                            if subencoding_mask & 2 != 0 {
                                let mut msg = [0_u8; 4];
                                rd.read_exact(&mut msg).await?;
                            }
                            if subencoding_mask & 4 != 0 {
                                let mut msg = [0_u8; 4];
                                rd.read_exact(&mut msg).await?;
                            }
                            if subencoding_mask & 8 != 0 {
                                let mut msg = [0_u8; 1];
                                rd.read_exact(&mut msg).await?;
                                if subencoding_mask & 16 != 0 {
                                    let mut msg = [0_u8; 6];
                                    rd.read_exact(&mut msg).await?;
                                } else {
                                    let mut msg = [0_u8; 2];
                                    rd.read_exact(&mut msg).await?;
                                }
                            }
                        }
                    }
                }
            }
            -223 => {
                // 7.7.4   DesktopSize Pseudo-encoding
                msgrd.client.width = width;
                msgrd.client.height = height;
                msgrd.fb.resize(width, height)?;
            }
            -257 => {
                // 7.7.7   QEMU Pointer Motion Change Pseudo-encoding
                log_error!("QEMU Pointer Motion Change: relative={}", x == 0);
                let mode = match x {
                    0 => 0, // relative
                    1 => 1, // relative
                    _ => {
                        return Err(Error::PROTOCOL);
                    }
                };
                msgrd.fb.pointer_mode(mode)?
            }
            -258 => {
                // 7.7.8   QEMU Extended Key Event Pseudo-encoding
                log_error!("QEMU Extended Key Event");
                return Err(Error::PROTOCOL);
            }
            -259 => {
                // 7.7.9   QEMU Audio Pseudo-encoding
                log_error!("Audio enabled");
                flags |= 1;
            }
            -261 => {
                // 7.7.10   QEMU LED State Pseudo-encoding
                let mut msg = [0_u8; 1];
                rd.read_exact(&mut msg).await?;
                let state = msg[0];
                msgrd.fb.led(state)?;
            }
            _ => {
                log_error!("Unknown encoding type: {}", encoding_type);
                return Err(Error::PROTOCOL);
            }
        }
    }
    events::post(Event::FrameBufferUpdate(
        flags,
        msgrd.client.width,
        msgrd.client.height,
    ));
    Ok(())
}

pub async fn receive_bell(
    rd: &mut io::AsyncReader,
    msgrd: &mut MessageReader,
) -> Result<(), Error> {
    msgrd.fb.bell()?;
    Ok(())
}

pub async fn receive_server_cut_text(
    rd: &mut io::AsyncReader,
    msgrd: &mut MessageReader,
) -> Result<(), Error> {
    let mut msg = [0_u8; 8];
    rd.read_exact(&mut msg[1..]).await?;
    let length = u32::from_be_bytes(msg[4..8].try_into().unwrap());
    if length > 0 {
        let mut data = vec![0_u8; length as usize];
        rd.read_exact(&mut data).await?;
        msgrd.fb.clipboard_text(&data)?;
    }
    Ok(())
}

pub async fn receive_qemu_server_message(
    rd: &mut io::AsyncReader,
    msgrd: &mut MessageReader,
) -> Result<(), Error> {
    let mut msg = [0_u8; 4];
    rd.read_exact(&mut msg[1..]).await?;
    let submsg_type = msg[1];
    let operation = u16::from_be_bytes(msg[2..4].try_into().unwrap());
    // log_debug!("submsg_type={} operation={}", submsg_type, operation);
    match submsg_type {
        1 => match operation {
            0 => {
                log_debug!("audio stop");
                msgrd.audio_sink.stop()?;
            }
            1 => {
                log_debug!("audio start");
                msgrd.audio_sink.start()?;
            }
            2 => {
                let mut msg = [0_u8; 4];
                rd.read_exact(&mut msg).await?;
                let length = u32::from_be_bytes(msg);
                if length > 0 {
                    let mut data = vec![0_u8; length as usize];
                    rd.read_exact(&mut data).await?;
                    msgrd.audio_sink.data(&data)?;
                }
            }
            _ => {
                log_error!("Unknown operation: {}", operation);
                return Err(Error::PROTOCOL);
            }
        },
        _ => {
            log_error!("Unknown submsg_type: {}", submsg_type);
            return Err(Error::PROTOCOL);
        }
    }
    Ok(())
}

async fn send_enable_audio_capture(wr: &mut io::AsyncWriter, operation: u16) -> Result<(), Error> {
    // 7.4.12.2   QEMU Audio Client Message
    let mut msg = [0_u8; 4];
    msg[0] = 255;
    msg[1] = 1;
    msg[2..4].copy_from_slice(&operation.to_be_bytes());
    wr.write_all(&msg).await?;
    Ok(())
}

async fn send_audio_format(
    wr: &mut io::AsyncWriter,
    sample_format: u8,
    nchannels: u8,
    frequency: u32,
) -> Result<(), Error> {
    // 7.4.12.2   QEMU Audio Client Message
    let mut msg = [0_u8; 10];
    msg[0] = 255;
    msg[1] = 1;
    msg[2..4].copy_from_slice(&u16::to_be_bytes(2));
    msg[4] = sample_format;
    msg[5] = nchannels;
    msg[6..10].copy_from_slice(&frequency.to_be_bytes());
    wr.write_all(&msg).await?;
    Ok(())
}
