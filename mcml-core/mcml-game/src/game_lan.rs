/// 局域网游戏相关
use std::{
    mem::MaybeUninit,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    sync::{
        Arc, LazyLock,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use mcml_base::events::EventArgHandler;
use regex::Regex;

use mcml_names::{
    i18,
    i18_items::{
        error_type::{CoreResult, ErrorData, ErrorType},
        thread_type::ThreadType,
    },
};
use socket2::{Domain, Protocol, SockAddr, Socket, Type};

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
        port: caps
            .get(2)
            .map_or(String::from("missing"), |m| m.as_str().to_string()),
        addr: None,
    })
}

fn build_motd(motd: GameMotd) -> String {
    format!("[MOTD]{}[/MOTD][AD]{}[/AD]", motd.motd, motd.port)
}

pub struct GameLan {
    socket_v4: Arc<Socket>,
    socket_v6: Arc<Option<Socket>>,
    is_run: Arc<AtomicBool>,
    events: Option<Arc<EventArgHandler<GameMotd>>>,
    send_v4: Option<SockAddr>,
    send_v6: Option<SockAddr>,
}

pub struct GameMotd {
    pub motd: String,
    pub port: String,
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
            Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).map_err(|err| {
                ErrorType::SocketError(ErrorData {
                    error: err.to_string(),
                })
            })?;
        socket_v4.set_reuse_address(true).map_err(|err| {
            ErrorType::SocketError(ErrorData {
                error: err.to_string(),
            })
        })?;
        let bind_addr_v4 = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, PORT);
        socket_v4
            .bind(&SockAddr::from(bind_addr_v4))
            .map_err(|err| {
                ErrorType::SocketError(ErrorData {
                    error: err.to_string(),
                })
            })?;
        let addr_v4 = IPV4.parse::<Ipv4Addr>().map_err(|err| {
            ErrorType::SocketError(ErrorData {
                error: err.to_string(),
            })
        })?;

        let mut socket_v6 = None;
        if v6 {
            let sock_v6 =
                Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP)).map_err(|err| {
                    ErrorType::SocketError(ErrorData {
                        error: err.to_string(),
                    })
                })?;

            sock_v6.set_reuse_address(true).map_err(|err| {
                ErrorType::SocketError(ErrorData {
                    error: err.to_string(),
                })
            })?;

            sock_v6.set_only_v6(true).map_err(|err| {
                ErrorType::SocketError(ErrorData {
                    error: err.to_string(),
                })
            })?;

            let bind_addr_v6 = SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, PORT, 0, 0);
            sock_v6.bind(&SockAddr::from(bind_addr_v6)).map_err(|err| {
                ErrorType::SocketError(ErrorData {
                    error: err.to_string(),
                })
            })?;

            socket_v6 = Some(sock_v6);
        }

        let addr_v6 = IPV6.parse::<Ipv6Addr>().map_err(|err| {
            ErrorType::SocketError(ErrorData {
                error: err.to_string(),
            })
        })?;

        for interface in interfaces.iter() {
            if interface.is_loopback() {
                continue;
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
                IpAddr::V6(_ipv6_addr) => {
                    if v6 {
                        if let Some(sock_v6) = socket_v6.as_ref() {
                            sock_v6
                                .join_multicast_v6(&addr_v6, interface.index.unwrap_or(0))
                                .map_err(|err| {
                                    ErrorType::SocketError(ErrorData {
                                        error: err.to_string(),
                                    })
                                })?;
                        }
                    }
                }
            }
        }

        Ok(Self {
            socket_v4: Arc::new(socket_v4),
            socket_v6: Arc::new(socket_v6.take()),
            is_run: Arc::new(AtomicBool::new(false)),
            events: Some(Arc::new(EventArgHandler::new())),
            send_v4: None,
            send_v6: None,
        })
    }

    pub fn new_server() -> CoreResult<Self> {
        let interfaces = if_addrs::get_if_addrs().map_err(|err| {
            ErrorType::SocketError(ErrorData {
                error: err.to_string(),
            })
        })?;

        let v6 = interfaces
            .iter()
            .any(|item| !item.is_loopback() && item.ip().is_ipv6());

        let socket_v4 =
            Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).map_err(|err| {
                ErrorType::SocketError(ErrorData {
                    error: err.to_string(),
                })
            })?;

        socket_v4.set_reuse_address(true).map_err(|err| {
            ErrorType::SocketError(ErrorData {
                error: err.to_string(),
            })
        })?;

        socket_v4.set_broadcast(true).map_err(|err| {
            ErrorType::SocketError(ErrorData {
                error: err.to_string(),
            })
        })?;

        let bind_addr_v4 = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0);
        socket_v4
            .bind(&SockAddr::from(bind_addr_v4))
            .map_err(|err| {
                ErrorType::SocketError(ErrorData {
                    error: err.to_string(),
                })
            })?;

        let ipv4 = IPV4.parse::<Ipv4Addr>().map_err(|err| {
            ErrorType::SocketError(ErrorData {
                error: err.to_string(),
            })
        })?;
        let addr_v4 = SocketAddr::V4(SocketAddrV4::new(ipv4, PORT));

        let mut socket_v6 = None;
        let mut addr_v6 = None;

        if v6 {
            let sock_v6 =
                Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP)).map_err(|err| {
                    ErrorType::SocketError(ErrorData {
                        error: err.to_string(),
                    })
                })?;

            sock_v6.set_reuse_address(true).map_err(|err| {
                ErrorType::SocketError(ErrorData {
                    error: err.to_string(),
                })
            })?;

            sock_v6.set_only_v6(true).map_err(|err| {
                ErrorType::SocketError(ErrorData {
                    error: err.to_string(),
                })
            })?;

            let bind_addr_v6 = SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0);
            sock_v6.bind(&SockAddr::from(bind_addr_v6)).map_err(|err| {
                ErrorType::SocketError(ErrorData {
                    error: err.to_string(),
                })
            })?;

            let ipv6 = IPV6.parse::<Ipv6Addr>().map_err(|err| {
                ErrorType::SocketError(ErrorData {
                    error: err.to_string(),
                })
            })?;
            addr_v6 = Some(SocketAddr::V6(SocketAddrV6::new(ipv6, PORT, 0, 0)));

            socket_v6 = Some(sock_v6);
        }

        Ok(Self {
            socket_v4: Arc::new(socket_v4),
            socket_v6: Arc::new(socket_v6),
            is_run: Arc::new(AtomicBool::new(false)),
            events: None,
            send_v4: Some(SockAddr::from(addr_v4)),
            send_v6: addr_v6.map(SockAddr::from),
        })
    }

    /// 添加接受回调
    pub fn add_event_handler<F>(&self, handler: F) -> u64
    where
        F: Fn(&GameMotd) + Send + Sync + 'static,
    {
        self.events
            .as_ref()
            .map(|item| item.add_handler(handler))
            .unwrap_or(u64::MAX)
    }

    /// 删除接受回调
    pub fn remove_event_handler(&self, id: u64) {
        if let Some(handle) = self.events.as_ref() {
            handle.remove_handel(id);
        }
    }

    /// 启动发送组播（服务端）
    pub fn start_send(&self, motd: GameMotd) -> CoreResult<()> {
        self.is_run.store(true, Ordering::Release);

        let run = self.is_run.clone();
        let socket_v4 = self.socket_v4.clone();
        let socket_v6 = self.socket_v6.clone();

        let addr_v4 = self.send_v4.clone().ok_or_else(|| {
            ErrorType::SocketError(ErrorData {
                error: "IPv4 send address not set".to_string(),
            })
        })?;

        let addr_v6 = self.send_v6.clone();

        thread::Builder::new()
            .name(i18::get_thread(ThreadType::LanServer))
            .spawn(move || {
                let motd = build_motd(motd);
                let data = motd.as_bytes();

                while run.load(Ordering::Acquire) {
                    // 发送 IPv4 组播
                    if let Err(err) = socket_v4.send_to(data, &addr_v4) {
                        mcml_log::error_type(ErrorType::SocketError(ErrorData {
                            error: format!("Failed to send IPv4 multicast: {}", err),
                        }));
                    }

                    // 发送 IPv6 组播
                    if let Some(sock_v6) = socket_v6.as_ref()
                        && let Some(addr) = addr_v6.as_ref()
                    {
                        if let Err(err) = sock_v6.send_to(data, addr) {
                            mcml_log::error_type(ErrorType::SocketError(ErrorData {
                                error: format!("Failed to send IPv6 multicast: {}", err),
                            }));
                        }
                    }

                    thread::sleep(std::time::Duration::from_secs(3));
                }
            })
            .map_err(|err| {
                ErrorType::ThreadError(ErrorData {
                    error: err.to_string(),
                })
            })?;

        Ok(())
    }

    /// 启动接收组播（客户端）
    pub fn start_read(&self) -> CoreResult<()> {
        self.is_run.store(true, Ordering::Release);

        let run_v4 = self.is_run.clone();
        let socket_v4 = self.socket_v4.clone();
        let events = self.events.clone();

        // IPv4 接收线程
        thread::Builder::new()
            .name(i18::get_thread(ThreadType::LanClientV4))
            .spawn(move || {
                let mut buffer = [MaybeUninit::<u8>::new(0); 1024];
                while run_v4.load(Ordering::Acquire) {
                    match socket_v4.recv_from(&mut buffer) {
                        Ok((size, addr)) => {
                            let data = unsafe {
                                let slice = &buffer[..size];
                                std::slice::from_raw_parts(slice.as_ptr() as *const u8, size)
                            };

                            if let Ok(text) = std::str::from_utf8(data) {
                                if let Some(mut motd) = get_motd(text) {
                                    motd.addr = Some(addr.as_socket().unwrap());
                                    if let Some(events) = events.as_ref() {
                                        events.emit(motd);
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            mcml_log::error_type(ErrorType::SocketError(ErrorData {
                                error: err.to_string(),
                            }));
                        }
                    }
                }

                unsafe {
                    buffer.assume_init_drop();
                }
            })
            .map_err(|err| {
                ErrorType::ThreadError(ErrorData {
                    error: err.to_string(),
                })
            })?;

        // IPv6 接收线程
        let run_v6 = self.is_run.clone();
        let socket_v6 = self.socket_v6.clone();
        let events6 = self.events.clone();

        if socket_v6.as_ref().is_some() {
            thread::Builder::new()
                .name(i18::get_thread(ThreadType::LanClientV6))
                .spawn(move || {
                    let mut buffer = [MaybeUninit::<u8>::new(0); 1024];
                    while run_v6.load(Ordering::Acquire) {
                        match socket_v6.as_ref().as_ref().unwrap().recv_from(&mut buffer) {
                            Ok((size, addr)) => {
                                let data = unsafe {
                                    let slice = &buffer[..size];
                                    std::slice::from_raw_parts(slice.as_ptr() as *const u8, size)
                                };

                                if let Ok(text) = std::str::from_utf8(data) {
                                    if let Some(mut motd) = get_motd(text) {
                                        motd.addr = Some(addr.as_socket().unwrap());
                                        if let Some(events) = events6.as_ref() {
                                            events.emit(motd);
                                        }
                                    }
                                }
                            }
                            Err(err) => {
                                mcml_log::error_type(ErrorType::SocketError(ErrorData {
                                    error: err.to_string(),
                                }));
                            }
                        }
                    }

                    unsafe {
                        buffer.assume_init_drop();
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

    pub fn stop(&self) {
        self.is_run.store(false, Ordering::Release);
    }
}
