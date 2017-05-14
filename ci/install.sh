set -ex

main() {
    curl https://sh.rustup.rs -sSf | \
        sh -s -- -y --default-toolchain $TRAVIS_RUST_VERSION

    local target=
    if [ $TRAVIS_OS_NAME = linux ]; then
        target=x86_64-unknown-linux-gnu
        sort=sort
        echo 'APT::Default-Release "trusty";' | sudo tee /etc/apt/apt.conf.d/01ubuntu
        echo 'deb http://archive.ubuntu.com/ubuntu xenial main restricted universe multiverse' | sudo tee -a /etc/apt/sources.list
        echo <<EOF | sudo tee -a /etc/apt/preferences
Package: libdbus-1-dev
Pin: release n=trusty
Pin-Priority: -10

Package: libdbus-1-dev
Pin: release n=xenial
Pin-Priority: 900

Package: dbus-x11
Pin: release n=trusty
Pin-Priority: -10

Package: dbus-x11
Pin: release n=xenial
Pin-Priority: 900
EOF
        sudo apt-get update
        sudo apt-get install -t xenial -y binutils libdbus-1-dev dbus-x11
    else
        target=x86_64-apple-darwin
        sort=gsort  # for `sort --sort-version`, from brew's coreutils.
    fi

    # This fetches latest stable release
    local tag=$(git ls-remote --tags --refs --exit-code https://github.com/japaric/cross \
                       | cut -d/ -f3 \
                       | grep -E '^v[0-9.]+$' \
                       | $sort --version-sort \
                       | tail -n1)
    echo cross version: $tag
    curl -LSfs https://japaric.github.io/trust/install.sh | \
        sh -s -- \
           --force \
           --git japaric/cross \
           --tag $tag \
           --target $target
}

main
