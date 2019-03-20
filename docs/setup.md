# Setting up wallet713

## Option 1: Using the official script

### Download and install latest version
From your terminal window run:
```
curl https://wallet.713.mw/install.sh -sSf | sh
```

### Run

Once installed, run wallet713 anywhere from your command prompt. You may need to restart your terminal window.
```
$ wallet713
```

If you'd like to run against floonet, use:
```
$ wallet713 --floonet
```
I

## Option 2: Building your own binary

### Requirements
1. All the [current requirements](https://github.com/mimblewimble/grin/blob/master/doc/build.md#requirements) of Grin.
1. [OpenSSL](https://www.openssl.org).
   * macOS with Homebrew:
      ```
      $ brew install openssl
      ``` 
   * Linux:
      ```
      $ sudo apt-get install openssl
      ```

### Installation

```
$ git clone https://github.com/vault713/wallet713
$ cd wallet713
$ cargo build --release
```
And then to run:
```
$ cd target/release
$ ./wallet713
```

If you'd like to run against floonet, use:
```
$ cd target/release
$ ./wallet713 --floonet
```

## Option 3: Build and run via docker

```
$ docker build -t my/wallet713 .
$ docker run --rm --name wallet713 -ti -v $PWD/data:/root/.wallet713 my/wallet713
```
You can use `ctrl+p q` to detach and `docker attach wallet713` to reattach to the running container.