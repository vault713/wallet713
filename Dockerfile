# Multistage docker build, requires docker 17.05
# based on: https://github.com/mimblewimble/grin/blob/master/etc/Dockerfile

# builder stage
FROM rust:1.31 as builder

RUN set -ex && \
    apt-get update && \
    apt-get --no-install-recommends --yes install \
    clang \
    libclang-dev \
    llvm-dev \
    libncurses5 \
    libncursesw5 \
    cmake \
    git

WORKDIR /usr/src/wallet713

# Copying
COPY . .

# Building
RUN cargo build --release

# runtime stage
FROM debian:9.4

RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y locales openssl ca-certificates

RUN sed -i -e 's/# en_US.UTF-8 UTF-8/en_US.UTF-8 UTF-8/' /etc/locale.gen && \
    dpkg-reconfigure --frontend=noninteractive locales && \
    update-locale LANG=en_US.UTF-8

ENV LANG en_US.UTF-8

COPY --from=builder /usr/src/wallet713/target/release/wallet713 /usr/local/bin/wallet713

WORKDIR /root/.wallet713

VOLUME ["/root/.wallet713"]

ENTRYPOINT ["wallet713"]