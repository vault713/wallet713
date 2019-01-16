# Using wallet713

While running, wallet713 works with an internal command prompt. You type commands in the same way as the CLI version of the grin wallet. **Ensure you are running a fully synced Grin node before using the wallet.**

## Contents
- [Common use cases](#common-use-cases)
  * [Getting started](#getting-started)
  * [Transacting using Keybase](#transacting-using-keybase)
  * [Transacting using files](#transacting-using-files)
    + [Creating a file-based transaction](#creating-a-file-based-transaction)
    + [Receiving a file-based transaction](#receiving-a-file-based-transaction)
    + [Finalizing a file-based transaction](#finalizing-a-file-based-transaction)
  * [Using Contacts](#using-contacts)
  * [Using a passphrase](#using-a-passphrase)
    + [Set a passphrase](#set-a-passphrase)
    + [Locking & unlocking the wallet](#locking---unlocking-the-wallet)
  * [Using invoice](#using-invoice)
    + [Issuing invoices](#issuing-invoices)
    + [Paying invoices](#paying-invoices)
  * [Splitting your outputs](#splitting-your-outputs)
- [Running your own node](#running-your-own-node)
- [Restoring your wallet](#restoring-your-wallet)
    + [Restoring a wallet using your mnemonic BIP-39 phrase](#restoring-a-wallet-using-your-mnemonic-bip-39-phrase)
    + [Manually importing a .seed](#manually-importing-a-seed)
- [Supported address formats](#supported-address-formats)
  * [Grinbox](#grinbox)
  * [Keybase](#keybase)
- [Command documentation](#command-documentation)

## Common use cases

### Getting started

When you run the wallet for the first time, the wallet will create a config file for you. Running `config` displays your current configuration.
Configuration files will be created by default under ~/.wallet713/ under a dedicated folder for each chain type (/main or /floo).

Running against mainnet:
```
$ ./wallet713
```

Running against floonet:
```
$ ./wallet713 --floonet
```

Initiate a new wallet:
```
wallet713> $ init
```

Display wallet info:
```
wallet713> $ info
```

In order to receive grins from others you need to listen for transactions coming to your grinbox address:
```
wallet713> $ listen
```
This will also display your grinbox address.

Standard floonet grinbox addressses always start with `x`. 

To send a 10 grin transaction to the address `xd6p24toTTDj7sxCCM4WGpBVcegVjGi9q5jquq6VWZA1BJroX514`:
```
wallet713> $ send 10 --to xd6p24toTTDj7sxCCM4WGpBVcegVjGi9q5jquq6VWZA1BJroX514
```

To receive grins you simply keep wallet713 running and transactions are processed automatically. Any transactions received while being offline are fetched once you initiate `listen`. 

To exit the wallet:
```
wallet713> $ exit
```

### Transacting using Keybase

First ensure you are logged into your account on keybase.io via the keybase command line interface or their desktop client.

Start a keybase listener on wallet713:
```
wallet713> $ listen --keybase
```

You are now ready to receive grins to your keybase @username, by having senders send to `keybase://username`.
If you are currently offline, the wallet will process your transactions the next time you run a listener.

To send 10 grins to Igno on keybase:
```
wallet713> $ send 10 --to keybase://ignotus
```

### Transacting using files

#### Creating a file-based transaction
```
wallet713> $ send 10 --file ~/path/to/transaction.tx
```
Generates the file `transaction.tx` in the designated path that sends 10 grins to a recipient.

#### Receiving a file-based transaction
Once `transaction.tx` is received from a sender, the command:
```
wallet713> $ receive --file ~/path/to/transaction.tx
```
...will process the received `transaction.tx` and generate `transaction.tx.response` in the same directory that should then be returned to the sender wallet.

#### Finalizing a file-based transaction
Having received back `transaction.tx.response`, the sender can then issue:
```
wallet713> $ finalize --file ~/path/to/transaction.tx.response
```
...which will finalize the transaction and broadcast it.

### Using Contacts

To make it easier to transact with parties without having to deal with their grinbox addresses or keybase profiles, you can assign them nicknames that are stored locally in your contacts. **These contacts are stored locally on your machine and are not synced or shared with us.**

To add the grinbox address `xd6p24toTTDj7sxCCM4WGpBVcegVjGi9q5jquq6VWZA1BJroX514` to your contacts as `faucet`:
```
wallet713> $ contacts add faucet grinbox://xd6p24toTTDj7sxCCM4WGpBVcegVjGi9q5jquq6VWZA1BJroX514
```

Similarly, to add the keybase address `keybase://ignotus` to your contacts as `igno`:
```
wallet713> $ contacts add igno keybase://ignotus
```

You can list your contacts:
```
wallet713> $ contacts
```

You can now send 10 grins to either of these contacts by their nicknames, preceded by @:
```
wallet713> $ send 10 --to @igno
```

### Using a passphrase

#### Set a passphrase
You can set the passphrase `yourpassphrase` when you initiate a new wallet: 
```
wallet713> $ init -p yourpassphrase
```

#### Locking & unlocking the wallet
Once you have a passphrase set, it will be required to `unlock` when you want to use the wallet after its been locked or when you launch the wallet:
```
wallet713> $ unlock -p yourpassphrase
```

### Using invoice

The `invoice` command reverses the default transaction flow. This allows you as a recipient to specify an amount you expect to be paid and send this over to a particular sender. Once the sender has returned the slate to you, you can then finalize the transaction and broadcast it to the network. This is very useful for merchant related flows. For a related discussion see [this forum post](https://www.grin-forum.org/t/reverse-transaction-building/482).

#### Issuing invoices

The command works very similar to send. The following command raises a request to be paid 10 grins from @faucet:
```
wallet713> $ invoice 10 --to @faucet
```

#### Paying invoices

Paying inbound payment requests are turned off by default. 

Currently, only blindly auto-accepting any inbound invoice from any user is supported. To enable this for an invoice amount that is 50 grin or less, you add the following line to your `wallet713.toml` configuration file:
```
max_auto_accept_invoice = 50000000000
```

More powerful payment flows will be supported in upcoming versions of wallet713.

### Splitting your outputs

When building Grin transactions, the outputs (UTXOs) used become locked and cannot be used until the transaction is finalized. Ensuring you have available outputs helps you transact with multiple parties concurrently without having to wait for UTXOs to become available again. 

Breaking down UTXOs can also help you protect your privacy as it makes it harder to determine which of those that belong to you.

As part of `send` you can determine how many change outputs you would like to receive, through the `-o` option. If you were sending @igno 10 grins from a single UTXO of 25 grins, the following transaction would generate 3 change outputs of 5 grins each:
```
wallet713> $ send 10 --to @igno -o 3
```

Similarly, as part of `invoice` you can specify in how many outputs you would like the payment to be received in. The following would allow you to receive 10 grins in total from @faucet, split in two outputs of 5 grins each:  
```
wallet713> $ invoice 10 --to @faucet -o 2
```

## Running your own node

Set corresponding `grin_node_uri` and `grin_node_secret` in your `~/.wallet713/XXX/wallet713.toml` where `XXX` is `floo` or `main` depending on which network you run the wallet for.

## Restoring your wallet

#### Restoring a wallet using your mnemonic BIP-39 phrase
```
wallet713> $ restore -m word1 word2 ...
```
If you had a passphrase, remember to include the `-p yourpassphrase` as you run the command.

#### Manually importing a .seed

To import an existing grin wallet to use in wallet713 follow these steps:
1. Ensure you have the previous wallet's `wallet.seed`. In the default config of the grin wallet, this is stored in `~/.grin/wallet_data`.
1. Build wallet713, run it, run `init`. Exit the wallet.  
1. Copy and replace `wallet713/target/release/wallet713_data/wallet.seed` with the `wallet.seed` of the wallet you want to restore.
1. Run wallet713, and then run `restore`.
1. Your previous wallet should now have been restored, and you can validate this by running `info`.

## Supported address formats

The following transaction addresses are currently supported.

### Grinbox
Assigned to you when you run the wallet for the first time. The address is derived from your seed. Mainnet grinbox addresses begin with `g`, floonet addresses begin with `x`.
Typical address format: `grinbox://gVuDBqXYZekdpQ8EeT1bQXSk8KHKTZqFFiQwAecVCyyqZX8UwKZq`

####  Address derivation
Addresses are derived from your wallet seed. A single seed can generate up to `2^32` different addresses. Each of your addresses is specified by an index, which defaults to 0.

#### Switching address
1. Stop the grinbox listener by using the `stop` command
1. Run `config -g` to switch to the next address. This will display your new address.
If instead you would like more control over which address to use, you can specify an index with the `-i` flag. For example, switching to the address with index `10` is done by running `config -g -i 10`.
1. Start the grinbox listener again by running `listen`.

The index will persist in between wallet713 sessions and is stored in your configuration file.

### Keybase
Your username on [Keybase](https://keybase.io).
Typical address format: `keybase://ignotus`

## Command documentation

For the most recent up to date documentation about specific commands, please refer to the documentation in wallet713 itself.

To list all available commands:
```
wallet713> $ help
```  

For help about a specific command `<command>`:
```
wallet713> $ <command> --help
```
