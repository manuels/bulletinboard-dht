#!/usr/bin/env python3

import dbus

APP_ID = 'mytestapp'

def put(key, value):
    bus = dbus.SessionBus()

    proxy = bus.get_object('org.manuel.BulletinBoard', '/')
    iface = dbus.Interface(proxy, 'org.manuel.BulletinBoard')
    iface.Put(APP_ID, key.encode('utf-8'), value.encode('utf-8'))


def get(key):
    bus = dbus.SessionBus()

    proxy = bus.get_object('org.manuel.BulletinBoard', '/')
    iface = dbus.Interface(proxy, 'org.manuel.BulletinBoard')
    values = iface.Get(APP_ID, key.encode('utf-8'))

    return [bytearray(v).decode('utf-8') for v in values]


def main():
    key = 'what is love?'
    value = "Baby don't hurt me, don't hurt me no more."
    put(key, value)

    print(key, get(key))


if __name__ == '__main__':
    main()

