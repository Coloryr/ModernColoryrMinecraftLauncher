/// 局域网游戏相关
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, UdpSocket},
    sync::{
        Arc, LazyLock,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use mcml_base::events::EventArgHandler;
use regex::Regex;

use mcml_names::{i18, i18_items::{error_type::{CoreResult, ErrorData, ErrorType}, thread_type::ThreadType}};

const PORT: u16 = 4445;

const IPV4: &str = "224.0.2.60";
const IPV6: &str = "FF75:230::60";

static LAN_INFO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[MOTD\](.*?)\[/MOTD\]\[AD\](.*?)\[/AD\]").unwrap());

fn get_motd(text: &str) -> Option<GameMotd> {
    let caps = LAN_INFO_RE.captures(text)?;

    Some(GameMotd {
        motd: caps
            .get(1)
            .map_or(String::from("missing"), |m| m.as_str().to_string()),
        ip: caps
            .get(2)
            .map_or(String::from("missing"), |m| m.as_str().to_string()),
        addr: None,
    })
}

fn build_motd(motd: GameMotd) -> String {
    format!("[MOTD]{}[/MOTD][AD]{}[/AD]", motd.motd, motd.ip)
}

pub struct GameLan {
    socket_v4: Arc<UdpSocket>,
    socket_v6: Arc<Option<UdpSocket>>,
    is_run: Arc<AtomicBool>,
    events: Arc<EventArgHandler<GameMotd>>,
}

pub struct GameMotd {
    pub motd: String,
    pub ip: String,
    pub addr: Option<SocketAddr>,
}

impl GameLan {
    pub fn new_client() -> CoreResult<Self> {
        let interfaces = if_addrs::get_if_addrs().map_err(|err| {
            ErrorType::SocketError(ErrorData {
                error: err.to_string(),
            })
        })?;

        let v6 = interfaces
            .iter()
            .any(|item| !item.is_loopback() && item.ip().is_ipv6());

        let socket_v4 =
            UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, PORT)).map_err(|err| {
                ErrorType::SocketError(ErrorData {
                    error: err.to_string(),
                })
            })?;
        let addr_v4 = IPV4.parse::<Ipv4Addr>().map_err(|err| {
            ErrorType::SocketError(ErrorData {
                error: err.to_string(),
            })
        })?;

        let mut socket_v6 = if v6 {
            Some(
                UdpSocket::bind(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, PORT, 0, 0)).map_err(
                    |err| {
                        ErrorType::SocketError(ErrorData {
                            error: err.to_string(),
                        })
                    },
                )?,
            )
        } else {
            None
        };

        let addr_v6 = IPV6.parse::<Ipv6Addr>().map_err(|err| {
            ErrorType::SocketError(ErrorData {
                error: err.to_string(),
            })
        })?;

        for interface in interfaces.iter() {
            if interface.is_loopback() {
                continue; // 通常跳过回环地址
            }
            match interface.ip() {
                IpAddr::V4(ipv4_addr) => {
                    socket_v4
                        .join_multicast_v4(&addr_v4, &ipv4_addr)
                        .map_err(|err| {
                            ErrorType::SocketError(ErrorData {
                                error: err.to_string(),
                            })
                        })?;
                }
                IpAddr::V6(_) => {
                    if v6 {
                        socket_v6
                            .as_ref()
                            .unwrap()
                            .join_multicast_v6(&addr_v6, interface.index.unwrap())
                            .map_err(|err| {
                                ErrorType::SocketError(ErrorData {
                                    error: err.to_string(),
                                })
                            })?;
                    }
                }
            }
        }

        Ok(Self {
            socket_v4: Arc::new(socket_v4),
            socket_v6: Arc::new(socket_v6.take()),
            is_run: Arc::new(AtomicBool::new(false)),
            events: Arc::new(EventArgHandler::new()),
        })
    }

    pub fn add_event_handler<F>(&self, handler: F) -> u64
    where
        F: Fn(&GameMotd) + Send + Sync + 'static,
    {
        self.events.add_handler(handler)
    }

    pub fn remove_event_handler(&self, id: u64) {
        self.events.remove_handel(id);
    }

    pub fn start_read(&self) -> CoreResult<()> {
        self.is_run.store(true, Ordering::Release);

        let runv4 = self.is_run.clone();
        let socketv4 = self.socket_v4.clone();
        let events = self.events.clone();
        thread::Builder::new()
            .name(i18::get_thread(ThreadType::LanClientV4))
            .spawn(move || {
                let mut temp = vec![0u8; 1024];
                while runv4.load(Ordering::Acquire) {
                    match socketv4.recv_from(&mut temp) {
                        Ok((size, addr)) => {
                            let data = String::from_utf8_lossy(&temp[..size]);
                            if let Some(mut data) = get_motd(&data) {
                                data.addr = Some(addr);
                                events.emit(data);
                            }
                        }
                        Err(err) => {
                            mcml_log::error_type(ErrorType::SocketError(ErrorData {
                                error: err.to_string(),
                            }));
                        }
                    }
                }
            })
            .map_err(|err| {
                ErrorType::ThreadError(ErrorData {
                    error: err.to_string(),
                })
            })?;

        let runv6 = self.is_run.clone();
        let socketv6 = self.socket_v6.clone();
        let events6 = self.events.clone();

        if socketv6.as_ref().is_some() {
            thread::Builder::new()
                .name(i18::get_thread(ThreadType::LanClientV6))
                .spawn(move || {
                    let mut temp = vec![0u8; 1024];
                    while runv6.load(Ordering::Acquire) {
                        match socketv6.as_ref().as_ref().unwrap().recv_from(&mut temp) {
                            Ok((size, addr)) => {
                                let data = String::from_utf8_lossy(&temp[..size]);
                                if let Some(mut data) = get_motd(&data) {
                                    data.addr = Some(addr);
                                    events6.emit(data);
                                }
                            }
                            Err(err) => {
                                mcml_log::error_type(ErrorType::SocketError(ErrorData {
                                    error: err.to_string(),
                                }));
                            }
                        }
                    }
                })
                .map_err(|err| {
                    ErrorType::ThreadError(ErrorData {
                        error: err.to_string(),
                    })
                })?;
        }

        Ok(())
    }
}
