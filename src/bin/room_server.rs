//! Persistent room service. Tokens and full decks never enter public projections.
use axum::{
    Json, Router,
    extract::{
        ConnectInfo, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use jarcade::multiplayer::{
    self as mp, ClientMessage, Command, GameKind, MemberView, RoomView, ServerMessage, Session,
    coup, reverie,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::mpsc;
use tower_http::services::ServeDir;

type Shared = Arc<Mutex<Hub>>;
type Tx = mpsc::Sender<ServerMessage>;
static CONNECTION: AtomicU64 = AtomicU64::new(1);
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
struct ConnectionGuard;
impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        ACTIVE.fetch_sub(1, Ordering::Relaxed);
    }
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn secret() -> String {
    rand::random::<[u8; 24]>()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
#[derive(Clone)]
struct Link {
    connection: u64,
    tx: Tx,
}
#[derive(Clone, Serialize, Deserialize)]
struct Seat {
    name: String,
    token: String,
    ready: bool,
    left: bool,
    #[serde(skip)]
    link: Option<Link>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "game", content = "state", rename_all = "snake_case")]
enum Match {
    Court(coup::Game),
    Reverie(reverie::Game),
}
impl Match {
    fn finished(&self) -> bool {
        match self {
            Self::Court(g) => g.finished(),
            Self::Reverie(g) => g.finished(),
        }
    }
    fn phase_key(&self) -> String {
        match self {
            Self::Court(g) => {
                let v = g.view(0);
                serde_json::to_string(&(v.phase, v.prompt, v.pending)).unwrap()
            }
            Self::Reverie(g) => {
                let v = g.view(0);
                format!("{}:{}:{}", v.phase, v.round, v.storyteller)
            }
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
struct Room {
    code: String,
    game: GameKind,
    seats: Vec<Seat>,
    host: usize,
    revision: u64,
    epoch: u64,
    updated: u64,
    board: Option<Match>,
}
impl Room {
    fn view(&self, you: usize) -> RoomView {
        RoomView {
            code: self.code.clone(),
            game: self.game,
            revision: self.revision,
            epoch: self.epoch,
            you,
            host: self.host,
            members: self
                .seats
                .iter()
                .map(|s| MemberView {
                    name: s.name.clone(),
                    connected: s.link.is_some(),
                    ready: s.ready,
                })
                .collect(),
            court: match &self.board {
                Some(Match::Court(g)) => Some(g.view(you)),
                _ => None,
            },
            reverie: match &self.board {
                Some(Match::Reverie(g)) => Some(g.view(you)),
                _ => None,
            },
        }
    }
    fn broadcast(&self) {
        for (i, s) in self.seats.iter().enumerate() {
            if let Some(link) = &s.link {
                let _ = link.tx.try_send(ServerMessage::State {
                    room: Box::new(self.view(i)),
                });
            }
        }
    }
    fn change(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.updated = now();
    }
    fn command(&mut self, you: usize, epoch: u64, command: Command) -> Result<(), &'static str> {
        if epoch != self.epoch {
            return Err("The table changed. Try again");
        }
        let before = self.board.as_ref().map(Match::phase_key);
        match command {
            Command::Ready(ready) if self.board.is_none() => self.seats[you].ready = ready,
            Command::Start if self.board.is_none() => {
                if you != self.host {
                    return Err("Only the host can start");
                }
                let (min, max) = self.game.limits();
                if !(min..=max).contains(&self.seats.len()) {
                    return Err("Invite more players first");
                }
                if self.seats.iter().any(|s| !s.ready || s.link.is_none()) {
                    return Err("Everyone must be connected and ready");
                }
                let names = self.seats.iter().map(|s| s.name.clone()).collect();
                self.board = Some(match self.game {
                    GameKind::Court => Match::Court(coup::Game::new(names, rand::random())?),
                    GameKind::Reverie => Match::Reverie(reverie::Game::new(names, rand::random())?),
                });
            }
            Command::Rematch if self.board.as_ref().is_some_and(Match::finished) => {
                if you != self.host {
                    return Err("Only the host can open a new table");
                }
                let host = self.seats[you].token.clone();
                self.seats.retain(|s| !s.left);
                self.host = self.seats.iter().position(|s| s.token == host).unwrap_or(0);
                self.board = None;
                for s in &mut self.seats {
                    s.ready = false;
                }
            }
            Command::Court(movement) => match &mut self.board {
                Some(Match::Court(g)) => g.play(you, movement)?,
                _ => return Err("This is not an active Coupe game"),
            },
            Command::Reverie(movement) => match &mut self.board {
                Some(Match::Reverie(g)) => g.play(you, movement)?,
                _ => return Err("This is not an active Dicksit game"),
            },
            _ => return Err("That option is not available"),
        }
        if before != self.board.as_ref().map(Match::phase_key) {
            self.epoch = self.epoch.wrapping_add(1);
        }
        self.change();
        Ok(())
    }
}
struct Hub {
    rooms: HashMap<String, Room>,
    path: PathBuf,
    rates: HashMap<std::net::IpAddr, (u64, u32)>,
}
impl Hub {
    fn load(path: PathBuf) -> std::io::Result<Self> {
        let rooms = if path.exists() {
            let data = std::fs::read(&path)?;
            serde_json::from_slice(&data).map_err(std::io::Error::other)?
        } else {
            HashMap::new()
        };
        Ok(Self {
            rooms,
            path,
            rates: HashMap::new(),
        })
    }
    fn save(&self) -> std::io::Result<()> {
        let parent = self.path.parent().unwrap();
        std::fs::create_dir_all(parent)?;
        let tmp = self.path.with_extension("new");
        use std::io::Write;
        let mut options = std::fs::OpenOptions::new();
        options.create(true).truncate(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&tmp)?;
        file.write_all(&serde_json::to_vec(&self.rooms).map_err(std::io::Error::other)?)?;
        file.sync_all()?;
        std::fs::rename(tmp, &self.path)?;
        Ok(())
    }
    fn sweep(&mut self) {
        self.rooms.retain(|_, r| {
            now().saturating_sub(r.updated) < 86400 || r.seats.iter().any(|s| s.link.is_some())
        });
        self.rates
            .retain(|_, (t, _)| now().saturating_sub(*t) < 120);
    }
    fn create(
        &mut self,
        game: GameKind,
        name: String,
        ip: std::net::IpAddr,
    ) -> Result<(String, String), &'static str> {
        self.sweep();
        let rate = self.rates.entry(ip).or_insert((now(), 0));
        if now().saturating_sub(rate.0) > 60 {
            *rate = (now(), 0);
        }
        if rate.1 >= 12 {
            return Err("Too many new rooms. Wait a minute");
        }
        rate.1 += 1;
        if self.rooms.len() >= 500 {
            return Err("The room service is full. Try later");
        }
        let alphabet = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
        let code = loop {
            let random = rand::random::<[u8; 6]>();
            let code: String = random
                .iter()
                .map(|b| alphabet[usize::from(*b) % alphabet.len()] as char)
                .collect();
            if !self.rooms.contains_key(&code) {
                break code;
            }
        };
        let token = secret();
        self.rooms.insert(
            code.clone(),
            Room {
                code: code.clone(),
                game,
                seats: vec![Seat {
                    name,
                    token: token.clone(),
                    ready: true,
                    left: false,
                    link: None,
                }],
                host: 0,
                revision: 0,
                epoch: 0,
                updated: now(),
                board: None,
            },
        );
        Ok((code, token))
    }
}
fn error(tx: &Tx, message: impl Into<String>) {
    let _ = tx.try_send(ServerMessage::Error {
        message: message.into(),
    });
}
fn disconnect(shared: &Shared, binding: &Option<(String, String)>, connection: u64) {
    let Some((code, token)) = binding else {
        return;
    };
    let mut hub = shared.lock().unwrap();
    if let Some(room) = hub.rooms.get_mut(code)
        && let Some(seat) = room.seats.iter_mut().find(|s| s.token == *token)
        && seat
            .link
            .as_ref()
            .is_some_and(|l| l.connection == connection)
    {
        seat.link = None;
        room.change();
        room.broadcast();
    }
}
fn handle(
    shared: &Shared,
    tx: &Tx,
    connection: u64,
    binding: &mut Option<(String, String)>,
    ip: std::net::IpAddr,
    message: ClientMessage,
) {
    let mut hub = shared.lock().unwrap();
    let handshake = match message {
        ClientMessage::Create { game, name } => {
            if binding.is_some() {
                error(tx, "Leave the current table first");
                return;
            }
            let name = mp::clean_text(&name, 24);
            if name.is_empty() {
                error(tx, "Enter your name");
                return;
            }
            match hub.create(game, name, ip) {
                Ok(session) => Some(session),
                Err(e) => {
                    error(tx, e);
                    return;
                }
            }
        }
        ClientMessage::Join { room: code, name } => {
            if binding.is_some() {
                error(tx, "Leave the current table first");
                return;
            }
            let code = code.trim().to_ascii_uppercase();
            let name = mp::clean_text(&name, 24);
            if name.is_empty() {
                error(tx, "Enter your name");
                return;
            }
            let Some(room) = hub.rooms.get_mut(&code) else {
                error(tx, "Room not found");
                return;
            };
            if room.board.is_some() {
                error(tx, "This game has started. Resume your seat instead");
                return;
            }
            if room.seats.len() >= room.game.limits().1 {
                error(tx, "This table is full");
                return;
            }
            let mut unique = name.clone();
            let mut n = 2;
            while room.seats.iter().any(|s| s.name == unique) {
                unique = format!("{} {n}", mp::clean_text(&name, 20));
                n += 1;
            }
            let token = secret();
            room.seats.push(Seat {
                name: unique,
                token: token.clone(),
                ready: false,
                left: false,
                link: None,
            });
            room.epoch += 1;
            Some((code, token))
        }
        ClientMessage::Resume { room, token } => {
            if binding.is_some() {
                error(tx, "Already seated");
                return;
            }
            Some((room.trim().to_ascii_uppercase(), token))
        }
        ClientMessage::Play { revision, command } => {
            let Some((code, token)) = binding else {
                error(tx, "Join a room first");
                return;
            };
            let Some(room) = hub.rooms.get_mut(code) else {
                error(tx, "Room expired");
                return;
            };
            let Some(you) = room.seats.iter().position(|s| {
                s.token == *token
                    && !s.left
                    && s.link.as_ref().is_some_and(|l| l.connection == connection)
            }) else {
                error(tx, "This seat is no longer connected");
                return;
            };
            if let Err(e) = room.command(you, revision, command) {
                error(tx, e);
                let _ = tx.try_send(ServerMessage::State {
                    room: Box::new(room.view(you)),
                });
                return;
            }
            room.broadcast();
            None
        }
        ClientMessage::Leave => {
            let Some((code, token)) = binding.take() else {
                return;
            };
            if let Some(room) = hub.rooms.get_mut(&code)
                && let Some(you) = room.seats.iter().position(|s| {
                    s.token == token && s.link.as_ref().is_some_and(|l| l.connection == connection)
                })
            {
                let before = room.board.as_ref().map(Match::phase_key);
                room.seats[you].link = None;
                room.seats[you].left = true;
                match &mut room.board {
                    Some(Match::Court(g)) => g.forfeit(you),
                    Some(Match::Reverie(g)) => g.end_on_leave(you),
                    None => {
                        room.seats.remove(you);
                        if you < room.host {
                            room.host -= 1;
                        }
                        room.epoch += 1;
                    }
                }
                if before != room.board.as_ref().map(Match::phase_key) {
                    room.epoch += 1;
                }
                if room.host >= room.seats.len()
                    || room.seats.get(room.host).is_some_and(|s| s.left)
                {
                    room.host = room.seats.iter().position(|s| !s.left).unwrap_or(0);
                }
                room.change();
                room.broadcast();
            }
            let _ = tx.try_send(ServerMessage::Left);
            None
        }
    };
    if let Some((code, token)) = handshake {
        let Some(room) = hub.rooms.get_mut(&code) else {
            error(tx, "Room not found or expired");
            return;
        };
        let Some(you) = room.seats.iter().position(|s| s.token == token && !s.left) else {
            error(tx, "The saved seat is not available");
            return;
        };
        if let Some(old) = room.seats[you].link.take() {
            let _ = old.tx.try_send(ServerMessage::Disconnected {
                reason: "Your seat opened in another window".into(),
            });
        }
        room.seats[you].link = Some(Link {
            connection,
            tx: tx.clone(),
        });
        *binding = Some((code.clone(), token.clone()));
        let _ = tx.try_send(ServerMessage::Welcome {
            session: Session {
                room: code,
                token,
                game: room.game,
            },
        });
        room.change();
        room.broadcast();
    }
    if let Err(e) = hub.save() {
        eprintln!("Room persistence failed: {e}");
        error(tx, "Server progress could not be saved");
    }
}
async fn socket(mut socket: WebSocket, shared: Shared, ip: std::net::IpAddr) {
    let id = CONNECTION.fetch_add(1, Ordering::Relaxed);
    let (tx, mut rx) = mpsc::channel(32);
    let mut binding = None;
    let mut heartbeat = tokio::time::interval(std::time::Duration::from_secs(60));
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut rate = (std::time::Instant::now(), 0_u32);
    loop {
        tokio::select! {
                    outgoing=rx.recv()=>{let Some(message)=outgoing else{break;};let close=matches!(message,ServerMessage::Disconnected{..});
                        if socket.send(Message::Text(serde_json::to_string(&message).unwrap().into())).await.is_err()||close{break;}}
                    incoming=socket.next()=>{match incoming{
                        Some(Ok(Message::Text(text)))=>{
                            if rate.0.elapsed().as_secs()>=1{rate=(std::time::Instant::now(),0);}rate.1+=1;if rate.1>20{error(&tx,"Too many commands");continue;}
                            match serde_json::from_str::<ClientMessage>(&text){Ok(msg)=>handle(&shared,&tx,id,&mut binding,ip,msg),Err(_)=>error(&tx,"Invalid message")}
                        }
                        Some(Ok(Message::Ping(data)))=>{if socket.send(Message::Pong(data)).await.is_err(){break;}}
                        Some(Ok(Message::Pong(_)))=>{}
                        _=>break,
                    }}
                    _=heartbeat.tick()=>{if binding.is_none()&&rate.0.elapsed().as_secs()>20{break;}
        if socket.send(Message::Ping(Vec::new().into())).await.is_err(){break;}}
                }
    }
    disconnect(&shared, &binding, id);
    let _ = socket.close().await;
}
async fn upgrade(
    ws: WebSocketUpgrade,
    State(shared): State<Shared>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Response, StatusCode> {
    if let Some(origin) = headers.get("origin").and_then(|v| v.to_str().ok()) {
        let host = headers
            .get("host")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if origin != format!("https://{host}") && origin != format!("http://{host}") {
            return Err(StatusCode::FORBIDDEN);
        }
    }
    ACTIVE
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
            (n < 256).then_some(n + 1)
        })
        .map_err(|_| StatusCode::TOO_MANY_REQUESTS)?;
    let guard = ConnectionGuard;
    Ok(ws
        .max_message_size(16384)
        .max_frame_size(16384)
        .on_upgrade(move |ws| async move {
            let _guard = guard;
            socket(ws, shared, addr.ip()).await
        }))
}
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = std::env::var("JARCADE_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("runtime"));
    let shared = Arc::new(Mutex::new(Hub::load(data.join("rooms.json"))?));
    let assets = std::env::var("JARCADE_STATIC_DIR").unwrap_or_else(|_| "dist".into());
    let app = Router::new()
        .route("/ws", get(upgrade))
        .route(
            "/health",
            get(|| async {
                Json(serde_json::json!({"status":"ok","games":["court","reverie"],"protocol":1}))
            }),
        )
        .fallback_service(ServeDir::new(assets))
        .with_state(shared);
    let bind = std::env::var("JARCADE_BIND").unwrap_or_else(|_| "127.0.0.1:8091".into());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    println!("Jarcade room service listening on {bind}");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn hub() -> Shared {
        Arc::new(Mutex::new(Hub {
            rooms: HashMap::new(),
            path: std::env::temp_dir().join(format!("jarcade-test-{}.json", secret())),
            rates: HashMap::new(),
        }))
    }
    fn attach(
        hub: &Shared,
        id: u64,
        msg: ClientMessage,
    ) -> (mpsc::Receiver<ServerMessage>, Option<(String, String)>) {
        let (tx, rx) = mpsc::channel(32);
        let mut binding = None;
        handle(
            hub,
            &tx,
            id,
            &mut binding,
            "127.0.0.1".parse().unwrap(),
            msg,
        );
        (rx, binding)
    }
    #[test]
    fn lobby_authority_private_views_and_persistent_resume() {
        let shared = hub();
        let (_, a) = attach(
            &shared,
            1,
            ClientMessage::Create {
                game: GameKind::Court,
                name: "Alice".into(),
            },
        );
        let (code, token) = a.unwrap();
        let (_, binding_b) = attach(
            &shared,
            2,
            ClientMessage::Join {
                room: code.clone(),
                name: "Bob".into(),
            },
        );
        let mut hub = shared.lock().unwrap();
        let room = hub.rooms.get_mut(&code).unwrap();
        assert!(room.command(1, room.epoch, Command::Start).is_err());
        room.command(1, room.epoch, Command::Ready(true)).unwrap();
        room.command(0, room.epoch, Command::Start).unwrap();
        let a = room.view(0);
        let b = room.view(1);
        assert!(
            a.court.unwrap().players[1]
                .cards
                .iter()
                .all(|c| c.role.is_none())
        );
        assert!(
            b.court.unwrap().players[0]
                .cards
                .iter()
                .all(|c| c.role.is_none())
        );
        hub.save().unwrap();
        let path = hub.path.clone();
        drop(hub);
        let loaded = Hub::load(path.clone()).unwrap();
        assert!(loaded.rooms[&code].seats.iter().all(|s| s.link.is_none()));
        let restored = Arc::new(Mutex::new(loaded));
        let (mut rx, binding) = attach(
            &restored,
            3,
            ClientMessage::Resume {
                room: code.clone(),
                token: token.clone(),
            },
        );
        assert!(binding.is_some());
        assert!(matches!(
            rx.try_recv().unwrap(),
            ServerMessage::Welcome { .. }
        ));
        assert!(matches!(
            rx.try_recv().unwrap(),
            ServerMessage::State { .. }
        ));
        let (_, wrong) = attach(
            &restored,
            4,
            ClientMessage::Resume {
                room: code,
                token: "bad".into(),
            },
        );
        assert!(wrong.is_none());
        assert!(binding_b.is_some());
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn concurrent_submissions_share_epoch_but_old_phase_moves_fail() {
        let mut seats = vec![];
        for i in 0..4 {
            let (tx, _rx) = mpsc::channel(32);
            seats.push(Seat {
                name: format!("P{i}"),
                token: format!("token{i}"),
                ready: true,
                left: false,
                link: Some(Link { connection: i, tx }),
            });
        }
        let mut r = Room {
            code: "ABCDEF".into(),
            game: GameKind::Reverie,
            seats,
            host: 0,
            revision: 0,
            epoch: 0,
            updated: now(),
            board: None,
        };
        r.command(0, 0, Command::Start).unwrap();
        let card = r.view(0).reverie.unwrap().hand[0];
        r.command(
            0,
            r.epoch,
            Command::Reverie(reverie::Move::Story {
                card,
                clue: "Moon".into(),
            }),
        )
        .unwrap();
        let epoch = r.epoch;
        for i in 1..4 {
            let card = r.view(i).reverie.unwrap().hand[0];
            r.command(
                i,
                epoch,
                Command::Reverie(reverie::Move::Submit { cards: vec![card] }),
            )
            .unwrap();
        }
        assert!(r.epoch > epoch);
        assert!(
            r.command(1, epoch, Command::Reverie(reverie::Move::Vote { card }))
                .is_err()
        );
    }
    #[test]
    fn replacing_connection_revokes_old_commands_and_leave() {
        let shared = hub();
        let (mut old_rx, mut binding) = attach(
            &shared,
            1,
            ClientMessage::Create {
                game: GameKind::Court,
                name: "Alice".into(),
            },
        );
        let (code, token) = binding.clone().unwrap();
        let (_, new) = attach(
            &shared,
            2,
            ClientMessage::Resume {
                room: code.clone(),
                token,
            },
        );
        assert!(new.is_some());
        while old_rx.try_recv().is_ok() {}
        let (tx, mut rx) = mpsc::channel(32);
        handle(
            &shared,
            &tx,
            1,
            &mut binding,
            "127.0.0.1".parse().unwrap(),
            ClientMessage::Leave,
        );
        assert!(matches!(rx.try_recv().unwrap(), ServerMessage::Left));
        let hub = shared.lock().unwrap();
        assert_eq!(hub.rooms[&code].seats.len(), 1);
        assert_eq!(
            hub.rooms[&code].seats[0].link.as_ref().unwrap().connection,
            2
        );
        let path = hub.path.clone();
        drop(hub);
        std::fs::remove_file(path).unwrap();
    }
}
