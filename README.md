BulletinBoard DHT
=================

BulletinBoard is a general-purpose Distributed-Hash-Table based on Kademlia [1].
The interface is provided as a D-Bus service via these commands:

    Service: org.manuel.BulletinBoard
      Object Path: /
      Interface:   org.manuel.BulletinBoard
      Commands:
       - Get(app_id: String, key: Array of Bytes)
            -> (values: Array of [Array of Bytes])
       - Put(app_id: String, key: Array of Bytes, value: Array of Bytes)
            -> ()

where `app_id` is an string specific to your application (e.g. `myfilesharingapp`). You can choose any `app_id` you want and you do not have to register you own `app_id` somewhere.
The `key` is hashed together with `app_id` using SHA-1 and the `value` must not
exceed 2048 bytes.
Note that you cannot assume that a value returned by the `Get` command was
really published by an instance of your application.
The lifetime for a value you `Put` in the DHT is 15 minutes, so you should call `Put` every, say, 10 minutes to make sure it stays in the DHT.

[1] http://pdos.csail.mit.edu/~petar/papers/maymounkov-kademlia-lncs.pdf

Getting Started
---------------

1.   Get [Rust](http://www.rust-lang.org/)

2.   Clone
     
         git clone https://github.com/manuels/bulletinboard-dht.git

3.   Build
     
         cargo build --release

     (in bulletinboard-dht dir)

4.   Join the DHT

         ./target/release/bulletinboard -j 94.23.110.187:6666

     94.23.110.187 is currently the only supernode

5.   Access DBus service

         # Put() and Get() for key=[0xDE,0xAD] and value=[0xBE, 0xEF] (app_id="test")
         $ dbus-send --session --type=method_call \
            --dest=org.manuel.BulletinBoard / \
            org.manuel.BulletinBoard.Put string:"test" \
            array:byte:0xDE,0xAD array:byte:0xBE,0xEF

         $ dbus-send --session --reply-timeout=60000 --print-reply \
            --type=method_call --dest=org.manuel.BulletinBoard / \
            org.manuel.BulletinBoard.Get string:"test" array:byte:0xDE,0xAD
         array [
            array of bytes [
               be ef
            ]
         ]

