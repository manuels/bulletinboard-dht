# This script takes care of building your crate and packaging it for release

set -ex

main() {
    local src=$(pwd) \
          stage=

    case $TRAVIS_OS_NAME in
        linux)
            stage=$(mktemp -d)
            ;;
        osx)
            stage=$(mktemp -d -t tmp)
            ;;
    esac

    test -f Cargo.lock || cargo generate-lockfile

    if [ $TARGET = x86_64-unknown-linux-gnu ]; then
        FEATURES='--features dbus_service'
        COMPILER=cargo
        eval `dbus-launch --sh-syntax`
    else
        FEATURES='--no-default-features'
        COMPILER=cross
    fi

    # TODO Update this to build the artifacts that matter to you
    #cross rustc --bin bulletinboard --target $TARGET --release -- -C lto
    $COMPILER build --target $TARGET $FEATURES --release

    # TODO Update this to package the right artifacts
    cp target/$TARGET/release/bulletinboard $stage/

    cd $stage
    tar czf $src/$CRATE_NAME-$TRAVIS_TAG-$TARGET.tar.gz *
    cd $src

    rm -rf $stage
}

main
