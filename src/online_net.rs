//! Event-driven platform transport. No network polling timer or hidden game rules.
use jarcade::multiplayer::{ClientMessage, ServerMessage};
#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn jarcade_online_connect();
    fn jarcade_online_close();
    fn jarcade_online_send(p: *const u8, n: usize);
    fn jarcade_online_poll(p: *mut u8, n: usize) -> usize;
}
pub struct Network {
    #[cfg(not(target_arch = "wasm32"))]
    tx: Option<tokio::sync::mpsc::Sender<String>>,
    #[cfg(not(target_arch = "wasm32"))]
    rx: std::sync::mpsc::Receiver<ServerMessage>,
}
impl Network {
    pub fn new() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (_, rx) = std::sync::mpsc::channel();
            Self { tx: None, rx }
        }
        #[cfg(target_arch = "wasm32")]
        Self {}
    }
    pub fn connect(&mut self) {
        self.close();
        #[cfg(target_arch = "wasm32")]
        // SAFETY: scalar call to the bundled transport plugin.
        unsafe {
            jarcade_online_connect();
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (tx, mut out) = tokio::sync::mpsc::channel::<String>(32);
            let (events, rx) = std::sync::mpsc::channel();
            self.tx = Some(tx);
            self.rx = rx;
            std::thread::spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("network runtime");
                runtime.block_on(async move{
                    use futures_util::{SinkExt,StreamExt};use tokio_tungstenite::{connect_async,tungstenite::Message};
                    let _=rustls::crypto::ring::default_provider().install_default();
                    let url=std::env::var("JARCADE_SERVER_URL").unwrap_or_else(|_|"wss://jarcade.jaypussy.site/ws".into());
                    let result=tokio::time::timeout(std::time::Duration::from_secs(12),connect_async(url)).await;
                    let mut socket=match result{Ok(Ok((s,_)))=>s,_=>{emit(&events,ServerMessage::Disconnected{reason:"Could not connect. Check your connection and retry.".into()});return;}};
                    emit(&events,ServerMessage::Connected);
                    loop{tokio::select!{
                        message=out.recv()=>{let Some(text)=message else{let _=socket.close(None).await;return;};if socket.send(Message::Text(text.into())).await.is_err(){break;}}
                        message=socket.next()=>match message{
                            Some(Ok(Message::Text(text)))=>if text.len()<=131072&&let Ok(msg)=serde_json::from_str(&text){emit(&events,msg);},
                            Some(Ok(Message::Ping(p)))=>{if socket.send(Message::Pong(p)).await.is_err(){break;}},
                            Some(Ok(Message::Pong(_)))=>{},_=>break,
                        }
                    }}emit(&events,ServerMessage::Disconnected{reason:"Connection lost. Reconnect to keep playing.".into()});
                });
            });
        }
    }
    pub fn close(&mut self) {
        #[cfg(target_arch = "wasm32")]
        // SAFETY: closes the plugin-owned socket, retaining no Rust pointers.
        unsafe {
            jarcade_online_close();
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.tx = None;
        }
    }
    pub fn send(&self, msg: &ClientMessage) {
        let text = serde_json::to_string(msg).expect("wire message");
        #[cfg(target_arch = "wasm32")]
        // SAFETY: plugin copies this valid UTF-8 slice synchronously.
        unsafe {
            jarcade_online_send(text.as_ptr(), text.len());
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(tx) = &self.tx {
            let _ = tx.try_send(text);
        }
    }
    pub fn poll(&mut self) -> Option<ServerMessage> {
        #[cfg(target_arch = "wasm32")]
        {
            let mut data = vec![0u8; 131072];
            // SAFETY: synchronous copy into the allocated buffer, bounded by capacity.
            let n = unsafe { jarcade_online_poll(data.as_mut_ptr(), data.len()) }.min(data.len());
            if n == 0 {
                None
            } else {
                serde_json::from_slice(&data[..n]).ok()
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.rx.try_recv().ok()
        }
    }
}
impl Drop for Network {
    fn drop(&mut self) {
        self.close();
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn emit(tx: &std::sync::mpsc::Sender<ServerMessage>, msg: ServerMessage) {
    if tx.send(msg).is_ok() {
        macroquad::miniquad::window::schedule_update();
    }
}
