use mio::{Token, Events, Interest, Poll};
use std::collections::HashMap;
use std::io::{Read, Write};
use mio::net::{TcpListener};

static RESPONSE: &str = "HTTP/1.1 200 OK
Content-Type: text/html
Connection: close
Content-Length: 6

hello
";

fn is_double_crnl(window: &[u8]) -> bool {
    window.len() >= 4 &&
        (window[0] == '\r' as u8) &&
        (window[1] == '\n' as u8) &&
        (window[2] == '\r' as u8) &&
        (window[3] == '\n' as u8)
}

#[cfg(not(windows))]
fn get_first_listen_fd_listener() -> Option<std::net::TcpListener> {
    #[cfg(unix)]
    use std::os::unix::io::FromRawFd;
    #[cfg(target_os = "wasi")]
    use std::os::wasi::io::FromRawFd;

    Some(unsafe { std::net::TcpListener::from_raw_fd(3) })
}

#[cfg(windows)]
fn get_first_listen_fd_listener() -> Option<std::net::TcpListener> {
    // Windows does not support `LISTEN_FDS`
    None
}

fn main() {
    env_logger::init();

    std::env::var("LISTEN_FDS").expect("LISTEN_FDS environment variable unset");

    // Setup the TCP server socket.
    let mut listener = {
        let stdlistener = get_first_listen_fd_listener().unwrap();
        stdlistener.set_nonblocking(true).unwrap();
        println!("Using preopened socket FD 3");
        TcpListener::from_std(stdlistener)
    };

    let mut poll = Poll::new().unwrap();
    poll.registry().register(
        &mut listener,
        Token(0),
        Interest::READABLE).unwrap();

    let mut counter: usize = 0;
    let mut sockets = HashMap::new();
    let mut requests: HashMap<Token, Vec<u8>> = HashMap::new();
    let mut buffer = [0 as u8; 1024];

    let mut events = Events::with_capacity(1024);
    loop {
        poll.poll(&mut events, None).unwrap();
        for event in &events {
            match event.token() {
                Token(0) => {
                    loop {
                        match listener.accept() {
                            Ok((mut socket, _)) => {
                                counter += 1;
                                let token = Token(counter);

                                poll.registry().register(
                                    &mut socket,
                                    token,
                                    Interest::READABLE).unwrap();

                                sockets.insert(token, socket);
                                requests.insert(token, Vec::with_capacity(192));
                            },
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock =>
                                break,
                            Err(_) => break
                        }
                    }
                },
                token if event.is_readable() => {
                    loop {
                        let read = sockets.get_mut(&token).unwrap().read(&mut buffer);
                        match read {
                            Ok(0) => {
                                sockets.remove(&token);
                                break
                            },
                            Ok(n) => {
                                let req = requests.get_mut(&token).unwrap();
                                for b in &buffer[0..n] {
                                    req.push(*b);
                                }
                            },
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock =>
                                break,
                            Err(_) => break
                        }
                    }

                    let ready = requests.get(&token).unwrap()
                        .windows(4)
                        .find(|window| is_double_crnl(*window))
                        .is_some();

                    if ready {
                        let socket = sockets.get_mut(&token).unwrap();
                        poll.registry().reregister(
                            socket,
                            token,
                            Interest::WRITABLE).unwrap();
                    }
                },
                token if event.is_writable() => {
                    requests.get_mut(&token).unwrap().clear();
                    sockets.get_mut(&token).unwrap().write_all(RESPONSE.as_bytes()).unwrap();

                    if let Some(mut socket) = sockets.remove(&token) {
                        poll.registry().deregister(&mut socket).unwrap();
                    }
                },
                _ => {
                    unreachable!()
                }
            }
        }
    }
}
