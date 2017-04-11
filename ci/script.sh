# This script takes care of testing your crate

set -ex

# TODO This is the "test phase", tweak it as you see fit
main() {
    if [ $TARGET = x86_64-unknown-linux-gnu ]; then
        FEATURES='--features dbus_service'
        COMPILER=cargo
        eval `dbus-launch --sh-syntax`
    else
        FEATURES='--no-default-features'
        COMPILER=cross
    fi

    $COMPILER build --target $TARGET $FEATURES
    $COMPILER build --target $TARGET --release $FEATURES

    if [ ! -z $DISABLE_TESTS ]; then
        return
    fi

    $COMPILER test --target $TARGET $FEATURES
    $COMPILER test --target $TARGET --release $FEATURES
}

# we don't run the "test phase" when doing deploys
if [ -z $TRAVIS_TAG ]; then
    main
fi
