use crate::audio::AudioSink;
use crate::framebuffer::FrameBuffer;
use crate::io;
use crate::rfb;
use crate::task;
use crate::{Error, log_debug, log_error, log_info};

pub(crate) async fn appmain() -> Result<(), Error> {
    log_debug!("start");
    let fb = FrameBuffer::new();
    let audio_sink = AudioSink::new();
    let (mut rd, mut wr) = io::connect().await;
    log_debug!("handshake");
    let data = rfb::handshake(&mut rd, &mut wr).await?;
    log_debug!("handshake done");
    for v in data.security_types.iter() {
        log_debug!("security_type: {}", v);
    }

    log_debug!("initialize");
    let (mut msgrd, mut msgwr) = rfb::initialize(&mut rd, &mut wr, fb, audio_sink, data).await?;
    log_debug!("initialize done");

    task::spawn(async move {
        let _res = rfb::handle_messages(&mut rd, &mut msgrd).await;
    });

    rfb::handle_events(&mut wr, &mut msgwr).await.ok();
    Ok(())
}
