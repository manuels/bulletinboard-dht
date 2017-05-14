BulletinBoard DHT
=================

[![Build Status](https://travis-ci.org/manuels/bulletinboard-dht.svg?branch=master)](https://travis-ci.org/manuels/bulletinboard-dht)
[![Crates Version](https://img.shields.io/crates/v/bulletinboard.svg)](https://crates.io/crates/bulletinboard)

[https://github.com/manuels/bulletinboard-dht](https://github.com/manuels/bulletinboard-dht)

Introduction
------------

BulletinBoard is a general-purpose Distributed-Hash-Table based on [Kademlia](http://pdos.csail.mit.edu/~petar/papers/maymounkov-kademlia-lncs.pdf).

The interface is provided as a D-Bus service via these commands (see [example below](#usage) or [python example](https://github.com/manuels/bulletinboard-dht/tree/master/examples/example.py)):

    Service: org.manuel.BulletinBoard
      Object Path: /
      Interface:   org.manuel.BulletinBoard
      Commands:
       - Store(app_id: str, key: [u8], value: [u8], lifetime_sec: u64)
       - Put(app_id: str, key: [u8], value: [u8])
       - Get(app_id: str, key: [u8]) -> (values: [[u8]])

Please note that the value must not exceed 2048 bytes!

The lifetime for a value you Put() in the DHT is 15 minutes, so you should call Put() every, say, 10 minutes to make sure it stays in the DHT (or just use Store()).


Installation
------------

1) **Download**

         # Debian/Ubuntu
         wget 'https://github.com/manuels/bulletinboard-dht/releases/download/v0.5.3/bulletinboard_0.5.3_amd64.deb'

         # Fedora
         wget 'https://github.com/manuels/bulletinboard-dht/releases/download/v0.5.3/bulletinboard-0.5.3-1.x86_64.rpm'

2) **Install bulletinboard**

         # Debian/Ubuntu
         sudo dpkg -i bulletinboard_0.5.3_amd64.deb

         # Fedora
         sudo rpm -ivh bulletinboard-0.5.3.x86_64.rpm

Usage
-----

Usually BulletinBoard is used by any third-party applications to store and lookup data.
You can use the DBus interface to do this by hand for example in your shell scripts.

### Storing Data

The Put() command stores data in the DHT.
In this example we store under the key *what did you eat?* the value [8B,AD,F0,0D] using
the application ID *mytestapp*:

         $ dbus-send --session \
            --type=method_call \
            --dest=org.manuel.BulletinBoard / \
            org.manuel.BulletinBoard.Put \
            string:"mytestapp" \
            array:byte:"what did you eat?" \
            array:byte:0x8B,0xAD,0xF0,0x0D

### Retrieving Data

Now we can get the stored data by asking the DHT what is stored under
the key *what did you eat?*.
Using the Get() command, we get back the [8B,AD,F0,0D] value we stored previously:

         $ dbus-send --session \
            --reply-timeout=60000 \
            --print-reply \
            --type=method_call \
            --dest=org.manuel.BulletinBoard / \
            org.manuel.BulletinBoard.Get \
            string:"mytestapp" \
            array:byte:"what did you eat?"

         array [
            array of bytes [
               8B AD F0 0D
            ]
         ]

Developing
----------

1)   Get [Rust](http://www.rust-lang.org/)

2)   Clone

         git clone https://github.com/manuels/bulletinboard-dht.git

3)   Build

         cargo build --release

     (in bulletinboard-dht dir)

