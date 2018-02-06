#![feature(proc_macro, conservative_impl_trait, generators, catch_expr)]

extern crate rand;
extern crate futures_await as futures;
#[macro_use]
extern crate log;
extern crate tokio_core;
extern crate tokio_timer;
#[macro_use]
extern crate error_chain;

use std::net::SocketAddr;

mod kademlia;
mod server;
mod messages;

error_chain! {
    errors {
        ValueTooLong
    }

    foreign_links {
        Fmt(::std::fmt::Error);
        Io(::std::io::Error);
        Canceled(futures::Canceled);
        SendInbound(futures::unsync::mpsc::SendError<messages::Response>);
        SendOutbound(futures::unsync::mpsc::SendError<(SocketAddr, messages::Message)>);
    }
}

fn main() {
    println!("Hello, world!");
}
