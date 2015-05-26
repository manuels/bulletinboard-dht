BulletinBoard DHT
=================

BulletinBoard is a general-purpose Distributed-Hash-Table based on Kademlia [1].
The interface is provided as a D-Bus service via these commands:

    Service: org.manuel.BulletinBoard
      Object Path: /
      Interface:   org.manuel.BulletinBoard
      Commands:
       - Get(app_id: String, key: Array of [Byte])
            -> (values: Array of [Array of [Byte]])
       - Put(app_id: String, key: Array of [Byte], value: Array of [Byte])
            -> ()
       - Remove(app_id: String, key: Array of [Byte], value: Array of [Byte])
            -> ()
       - RemoveKey(app_id: String, key: Array of [Byte])
            -> ()

where `app_id` is an string specific to your application (e.g. `myfilesharingapp`). You can choose any `app_id` you want and you do not have to register you own `app_id` somewhere.
The `key` is hashed together with `app_id` using SHA-1 and the `value` may not
exceed 2048 bytes.
Note that you cannot assume that a value returned by the `Get` command was
really published by an instance of your application.

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

         ./target/release/bulletinboardd -j 94.23.110.187

     94.23.110.187 is currently the only supernode

5.   Access DBus service
