//! UDP implementation for the Tokio backend
// https://github.com/ryankurte/rust-coap-client
// Copyright 2021 ryan kurte <ryan@kurte.nz>

use std::collections::HashMap;
use std::io::Error;

use log::{debug, error, trace};

use tokio::sync::mpsc::channel;

use super::{Ctl, Tokio};
use crate::COAP_MTU;

impl Tokio {
    // Helper for creating a UDP client instance
    pub(crate) async fn new_udp(bind_addr: &str) -> Result<Self, Error> {
        debug!("Creating UDP listener");

        // Bind to local socket
        let udp_sock = tokio::net::UdpSocket::bind(bind_addr).await.map_err(|e| {
            error!("Error binding local socket: {:?}", e);
            e
        })?;

        // Setup control channel
        let (ctl_tx, mut ctl_rx) = channel::<Ctl>(1000);

        // Run listener thread
        let l_ctl_tx = ctl_tx.clone();
        let _listener = tokio::task::spawn(async move {
            let mut buff = [0u8; COAP_MTU];
            let mut handles = HashMap::new();

            loop {
                tokio::select!(
                    // Receive from control channel
                    ctl = ctl_rx.recv() => {
                        match ctl {
                            Some(Ctl::Register(token, rx)) => {
                                debug!("Register handler: {:x}", token);
                                handles.insert(token, rx);
                            },
                            Some(Ctl::Deregister(token)) => {
                                debug!("Deregister handler: {:x}", token);
                                handles.remove(&token);
                            },
                            Some(Ctl::Send(data, peer_addr)) => {
                                trace!("Tx: {:02x?}", data);
                                if let Err(e) = udp_sock.send_to(&data[..], peer_addr).await {
                                    error!("net transmit error: {:?}", e);
                                    break;
                                }
                            }
                            Some(Ctl::Exit) => {
                                debug!("Exiting client");
                                break;
                            },
                            _ => (),
                        }
                    }
                    // Receive from the bound socket
                    r = udp_sock.recv_from(&mut buff) => {
                        let (data, peer_addr) = match r {
                            Ok((n, a)) => (&buff[..n], a),
                            Err(e) => {
                                error!("net receive error: {:?}", e);
                                break;
                            }
                        };

                        trace!("Rx: {:02x?}", data);

                        // Handle received data
                        if let Err(e) = Self::handle_rx(&mut handles, data, peer_addr, l_ctl_tx.clone()).await {
                            error!("net handle error: {:?}", e);
                            break;
                        }
                    },
                );
            }

            debug!("Exit coap UDP handler");
            Ok(())
        });

        Ok(Self { ctl_tx, _listener })
    }
}
