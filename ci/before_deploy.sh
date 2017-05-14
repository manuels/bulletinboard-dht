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
    if [ "$FEATURES" = '--features dbus_service' ]; then
        NAME='with_dbus_service'
    else
        NAME='without_dbus_service'
    fi
    tar czf $src/$CRATE_NAME-$TRAVIS_TAG-$TARGET-$NAME.tar.gz *
    #sudo apt-get install -y ruby ruby-dev build-essential
    sudo apt-get install -y ruby-dev build-essential rpm

    git clone https://github.com/rbenv/rbenv.git ~/.rbenv
    cd ~/.rbenv && src/configure && make -C src
    export PATH="$HOME/.rbenv/bin:$PATH"
    #~/.rbenv/bin/rbenv init
    eval "$(rbenv init -)"
    ~/.rbenv/bin/rbenv rehash
    which gem

    if [ $NAME = with_dbus_service ]; then
        gem install --no-ri --no-rdoc ffi
        gem install --no-ri --no-rdoc fpm
        fpm -s dir -t deb -n $CRATE_NAME -v `echo $TRAVIS_TAG | tr -d v` \
            $src/org.manuel.BulletinBoard.service=/usr/share/dbus-1/services/ \
            $src/target/$TARGET/release/bulletinboard=/usr/bin/
        fpm -s dir -t rpm -n $CRATE_NAME -v `echo $TRAVIS_TAG | tr -d v` \
            $src/org.manuel.BulletinBoard.service=/usr/share/dbus-1/services/ \
            $src/target/$TARGET/release/bulletinboard=/usr/bin/
        cp *deb $src
        cp *rpm $src
    fi

    cd $src

    rm -rf $stage
}

main
