# wallet713

[![Join the chat at https://gitter.im/vault713/wallet713](https://badges.gitter.im/vault713/wallet713.svg)](https://gitter.im/vault713/wallet713?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

[Grin](https://github.com/mimblewimble/grin) is a blockchain-powered cryptocurrency that is an implementation of the MimbleWimble protocol, with a focus on privacy and scalability. In MimbleWimble, transactions are interactive, requiring the Sender and Recipient to interact over a single round trip in order to build the transaction.

wallet713 is a non-custodial wallet for Grin that aims to make it easy to store, send and swap grins seamlessly through a single interface. Built on top of the standard Grin wallet reference implementation, wallet713 extends functionality to improve usability and reduce friction. Integrated with the 713.grinbox messaging relay, partial transactions are routed via the relay with no impact to the safety of your funds.

<p align="center">
  <img width="600" src="demo.svg">
</p>

## Features

* **Get up and running fast.** Listen, send and receive using the same instance of the wallet.
* **Use your public key as your address.** 713.grinbox relies on public/private keypairs to authenticate yourself and prevent unauthorized parties to listen to your messages.
* **Process transactions easily.** Send to a recipient's 713.grinbox and it takes care of itself. No need to deal with IP addresses, port forwarding, or manual file transfers.
* **Receive transactions while you are offline.** Transactions are sent to your 713.grinbox, waiting for you to fetch them the next time you come online.
* **Remain in full control.** Only you have access to your private keys and your wallet balance, only you can read or sign your own transactions.

## Roadmap

* Multi-sig support.
* P2P Atomic Swaps with Bitcoin directly from within the wallet.
* Transaction aggregation with other users before broadcasting to the network.
* Privacy enhancements.
* Graphical User Interface on Mobile, Desktop and Web.

...and much more. We are only getting started!

## Status

Under heavy development ahead of Grin Mainnet Launch. Contributions are welcomed.

## Installation and usage

### Requirements
wallet713 has the [same requirements](https://github.com/mimblewimble/grin/blob/master/doc/build.md#requirements) as Grin and also requires a fully synced Grin node to be running in order to be operational.

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

### Usage

While running, wallet713 works with an internal command prompt. You type commands in the same way as the CLI version of the grin wallet.

When you run the wallet for the first time, the wallet will create a config file for you and also generate your public/private keypairs. Running `config` displays your current configuration. 

Iniate a new wallet:
```
wallet713> $ init
```

Display wallet info:
```
wallet713> $ info
```

In order to receive grins from others you need to listen for transactions coming to your 713.grinbox address:
```
wallet713> $ listen
```
Standard 713.grinbox addressses always start with `x`. 

To send a 10 grin transaction to the address `xd8q4wgBBwdg75vD2J1VswdT4x7bJE6P5o1hcoht99ebc6C1wxxq`:
```
wallet713> $ send 10 --to xd8q4wgBBwdg75vD2J1VswdT4x7bJE6P5o1hcoht99ebc6C1wxxq
```

To receive grins you simply keep wallet713 running and transactions are processed automatically. Any transactions received while being offline are fetched once you initiate `listen`. 

To exit the wallet:
```
wallet713> $ exit
```

#### Importing an existing wallet

To import an existing grin wallet to use in wallet713 follow these steps:
1. Ensure you have the previous wallet's `wallet.seed`. In the default config of the grin wallet, this is stored in `~/.grin/wallet_data`.
1. Build wallet713, run it, run `init`. Exit the wallet.  
1. Copy and replace `wallet713/wallet713_data/wallet.seed` with the `wallet.seed` of the wallet you want to restore.
1. Run wallet713, and then run `restore`.
1. Your previous wallet should now have been restored, and you can validate this by running `info`.

## Privacy considerations

* **The relay does not store data.** 713.grinbox does not store any data on completed transactions by design, but it would be possible for the relay to do so and as a result build a graph of activity between addresses.

* **Your IP is your responsibility.** When you communicate to the 713.grinbox relay service, you are exposing your IP to the relay. You can obfuscate your real IP address using services such as a VPN and/or TOR or i2p.

## Credits

All the [Grin contributors](https://github.com/mimblewimble/grin/graphs/contributors)

## License

Apache License v2.0. 
