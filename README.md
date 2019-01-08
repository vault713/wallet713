# wallet713

[![Join the chat at https://gitter.im/vault713/wallet713](https://badges.gitter.im/vault713/wallet713.svg)](https://gitter.im/vault713/wallet713?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

wallet713 is:

- a wallet for Grin.

   [Grin](https://github.com/mimblewimble/grin) is a blockchain-powered cryptocurrency that is an implementation of the MimbleWimble protocol, with a focus on privacy and scalability. In MimbleWimble, transactions are interactive, requiring the Sender and Recipient to interact over a single round trip in order to build the transaction.

- an extension of the official grin wallet

   wallet713 makes it easy to store, send and soon also swap grins seamlessly through a single interface. Built on top of the standard Grin wallet reference implementation, wallet713 extends its functionality to improve usability and reduce friction. 

- is integrated with the 713.grinbox messaging relay

   For better privacy and usability, the 713.grinbox messaging relay allows the steps to build transactions (partial transactions, often called "slates") to be routed via the relay, protecting the user from exposing their IP address, and with no impact to the safety of your funds.

<p align="center">
  <img width="600" src="demo.svg">
</p>

## Features

* **Get up and running fast.** Listen, send and receive using the same instance of the wallet.
* **Use your public key as your address.** 713.grinbox relies on public/private keypairs to authenticate yourself and prevent unauthorized parties to listen to your messages.
* **Process transactions easily.** Send to a recipient's 713.grinbox or keybase profile and it takes care of itself. No need to deal with IP addresses, port forwarding, or manual file transfers.
* **Receive transactions while you are offline.** Transactions persist, waiting for you to fetch them the next time you come online.
* **Contacts.** No need to keep track of 713.grinbox addresses or keybase account names. Add addresses to contacts stored locally on your machine, and sending 10 grin becomes as easy as `send 10 --to @alice`.
* **Remain in full control.** Only you have access to your private keys and your wallet balance, only you can read or sign your own transactions.

## Status

Running on Floonet. Under heavy development ahead of Grin Mainnet Launch. Contributions are welcomed.

## Roadmap

* Multi-sig support.
* P2P Atomic Swaps with Bitcoin directly from within the wallet.
* Transaction aggregation with other users before broadcasting to the network.
* Privacy enhancements.
* Graphical User Interface on Mobile, Desktop and Web.

...and much more. We are only getting started!

## Getting started

* To build and get up and running, see the [build documentation](docs/build.md).
* For specific functionality, see the [usage documentation](docs/usage.md).

## Privacy considerations

* **The relay does not store data.** 713.grinbox does not store any data on completed transactions by design, but it would be possible for the relay to do so and as a result build a graph of activity between addresses.

* **Your IP is your responsibility.** When you communicate to the 713.grinbox relay service, you are exposing your IP to the relay. You can obfuscate your real IP address using services such as a VPN and/or TOR or i2p.

## Credits

All the [Grin contributors](https://github.com/mimblewimble/grin/graphs/contributors)

## License

Apache License v2.0. 
