#!/bin/bash

case "${TRAVIS_OS_NAME}" in
    "linux")
        wget https://www.openssl.org/source/openssl-1.1.0j.tar.gz
        tar -xf openssl-1.1.0j.tar.gz
        cd openssl-1.1.0j
        ./config
        make
        cd ..
        cargo clean && \
        OPENSSL_LIB_DIR=./openssl-1.1.0 OPENSSL_INCLUDE_DIR=./openssl-1.1.0/include cargo build --release && \
        ./.auto-release.sh
        ;;
    "osx")
        brew update
        cargo clean && \
        cargo build --release && \
        ./.auto-release.sh
        ;;
    *)
        printf "Error! Unknown \$TRAVIS_OS_NAME: \`%s\`" "${TRAVIS_OS_NAME}"
        exit 1
esac