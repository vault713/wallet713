# Using wallet713

While running, wallet713 works with an internal command prompt. You type commands in the same way as the CLI version of the epic wallet. **Ensure you are running a fully synced Epic node before using the wallet.**

## Contents

  * [Common use cases](#common-use-cases)
    + [Getting started](#getting-started)
    + [Transacting](#transacting)
      - [Transacting using epicbox](#transacting-using-epicbox)
      - [Transacting using Keybase](#transacting-using-keybase)
      - [Transacting using https](#transacting-using-https)
        * [Sending via https](#sending-via-https)
        * [Receiving via http](#receiving-via-http)
      - [Transacting using files](#transacting-using-files)
        * [Creating a file-based transaction](#creating-a-file-based-transaction)
        * [Receiving a file-based transaction](#receiving-a-file-based-transaction)
        * [Finalizing a file-based transaction](#finalizing-a-file-based-transaction)
    + [Send configurations](#send-configurations)
      - [Input selection strategy](#input-selection-strategy)
      - [Minimum number of confirmations](#minimum-number-of-confirmations)
    + [Transaction proofs (epicbox only)](#transaction-proofs-epicbox-only)
      - [Creating a transaction proof](#creating-a-transaction-proof)
      - [Verifying a transaction proof](#verifying-a-transaction-proof)
    + [Using Contacts](#using-contacts)
    + [Using a passphrase](#using-a-passphrase)
      - [Set a passphrase](#set-a-passphrase)
      - [Locking & unlocking the wallet](#locking---unlocking-the-wallet)
    + [Using invoice](#using-invoice)
      - [Issuing invoices](#issuing-invoices)
      - [Paying invoices](#paying-invoices)
    + [Splitting your outputs](#splitting-your-outputs)
  * [Running your own node](#running-your-own-node)
  * [Configuring Foreign & Owner APIs](#configuring-foreign-and-owner-apis)
    + [Foreign API](#foreign-api)
    + [Owner API](#owner-api)
  * [Recovering your wallet](#recovering-your-wallet)
    + [Recovering a wallet from seed file](#recovering-a-wallet-from-seed-file)
    + [Recovering a wallet using your mnemonic BIP-39 phrase](#recovering-a-wallet-using-your-mnemonic-bip-39-phrase)
    + [Displaying existing BIP-39 mnemonic](#displaying-existing-bip-39-mnemonic)
  * [Supported address formats](#supported-address-formats)
    + [Epicbox](#epicbox)
      - [Address derivation](#address-derivation)
      - [Switching address](#switching-address)
    + [Keybase](#keybase)
  * [Command documentation](#command-documentation)

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

In order to receive epics from others you need to listen for transactions coming to your epicbox address:
```
wallet713> $ listen
```
This will also display your epicbox address.

To exit the wallet:
```
wallet713> $ exit
```

### Transacting

#### Transacting using epicbox

Standard mainnet epicbox addresses begin with `e`.
Standard floonet epicbox addressses begin with `z`.

To send a 10 epic transaction to the address `zd6p24toTTDj7sxCCM4WGpBVcegVjGi9q5jquq6VWZA1BJroX514`:
```
wallet713> $ send 10 --to zd6p24toTTDj7sxCCM4WGpBVcegVjGi9q5jquq6VWZA1BJroX514
```

To receive epics you simply keep wallet713 running and transactions are processed automatically. Any transactions received while being offline are fetched once you initiate `listen`.


#### Transacting using Keybase

First ensure you are logged into your account on keybase.io via the keybase command line interface or their desktop client.

Start a keybase listener on wallet713:
```
wallet713> $ listen keybase
```

You are now ready to receive epics to your keybase @username, by having senders send to `keybase://username`.
If you are currently offline, the wallet will process your transactions the next time you run a listener.

To send 10 epics to Igno on keybase:
```
wallet713> $ send 10 --to keybase://ignotus
```

#### Transacting using https

##### Sending via https

wallet713 supports sending transactions to listening wallets via http(s).

To send 10 epics to https://some.wallet.713.mw:13415:
```
wallet713> $ send 10 --to https://some.wallet.713.mw:13415
```

##### Receiving via http

Wallet713 supports receiving transactions via http. In order to set this up you need the foreign api listener running.

For instructions on how to set this up please refer to the section: [Foreign API](#foreign-api)

Note that in otder to set up https access to the foreign API, which is highly recommended, you would need to install a reverse proxy and on a registered domain with a proper SSL certificate.

#### Transacting using files

##### Creating a file-based transaction
```
wallet713> $ send 10 --file ~/path/to/transaction.tx
```
Generates the file `transaction.tx` in the designated path that sends 10 epics to a recipient.

##### Receiving a file-based transaction
Once `transaction.tx` is received from a sender, the command:
```
wallet713> $ receive ~/path/to/transaction.tx
```
...will process the received `transaction.tx` and generate `transaction.tx.response` in the same directory that should then be returned to the sender wallet.

##### Finalizing a file-based transaction
Having received back `transaction.tx.response`, the sender can then issue:
```
wallet713> $ finalize ~/path/to/transaction.tx.response
```
...which will finalize the transaction and broadcast it.

### Send configurations

#### Input selection strategy

Set the input selection strategy [`all`, `smallest`] with the `-s` option:

To send a transaction using "all" as input selection strategy:
```
wallet713> $ send 10 --to zd6p24toTTDj7sxCCM4WGpBVcegVjGi9q5jquq6VWZA1BJroX514 -s all
```

#### Minimum number of confirmations

Set the minimum number of confirmation for inputs with the `-c` option, the default is `10`:

To send a transaction with 3 required confirmations:
```
wallet713> $ send 10 --to zd6p24toTTDj7sxCCM4WGpBVcegVjGi9q5jquq6VWZA1BJroX514 -c 3
```

### Transaction proofs (epicbox only)

Thanks to the use of epicbox, wallet713 supports proving that a particular amount was sent in a transaction to a particular epicbox recipient address. It relies on the fact that a recipient needs to return a message to the sender in order to build a valid transaction. As part of that, the recipient need their private key to receive and process the sender's original message, as well as in order to sign and send back the response to the sender. The sender can then use this information to generate a proof that can be sent to Bob or a third party, (say Carol) that says that if a particular transaction kernel is visible on the blockchain, a certain epicbox address has received a transaction of a certain amount. **This can only be used for transactions that have been sent using epicbox and you need wallet713 to generate and validate a transaction proof.**

In the below example,
1. Alice wants to send Bob 1.337 epics and prove to Carol that this transaction has occurred.
1. Bob has epicbox address: `zd7sCQ9bQuQXp4yCn8GSELcuSxnpcPrPoEWJzvPBc5vxyXPQz6PJ`;

#### Creating a transaction proof

1. Alice uses epicbox to send Bob epics using epicbox and broadcasts the transaction to the blockchain:
   ```
   wallet713> $ send 0.233232 --to zd7sCQ9bQuQXp4yCn8GSELcuSxnpcPrPoEWJzvPBc5vxyXPQz6PJ
   ```
1. Alice runs `txs` command to display the transaction log and to identify which ID her transaction has:
   ```
   wallet713> $ txs
   ```
   The transaction in question should show a `yes` in the `proof` column. Example output:
   ```
    23  Sent Tx      4b6ede9f  zd7sCQ9bQuQXp4yCn8GSELcuSxnpcPrPoEWJzvPBc5vxyXPQz6PJ                 2019-01-27 20:45:01  yes         2019-01-31 01:02:18  -0.234232      yes
   ```

1. Alice now exports a proof for this transaction:
   ```
   wallet713> $ proof export <number> <filename>
   ```
   ...where `<number>` is the ID in question (in our example `23`), and `<filename>` is the file name that the proof should be saved as (such as `proof.txt`).

1. If successful, Alice receives a confirmation message. Example output:
   ```
   wallet713> $ proof export 23 proof.txt
   proof written to proof.txt
   this file proves that [0.233232000] epics was sent to [zd7sCQ9bQuQXp4yCn8GSELcuSxnpcPrPoEWJzvPBc5vxyXPQz6PJ] from [zd7auPddUmmEzSte48a2aZ9tWkjjCppgn41pemUfcVSqjxHHZ6cT]

   outputs:
      08710be0b3fffa79b9423f8e007709a815f237dcfd31340cfa1fdfefd823dca30e
   kernel:
      099c8a166acd426481c1b09707b9e6cdabb69718ee3ca86694579bf98a42c0c80d

   WARNING: this proof should only be considered valid if the kernel is actually on-chain with sufficient confirmations
   please use a epic block explorer to verify this is the case. for example:
      https://floonet.epicscan.net/kernel/099c8a166acd426481c1b09707b9e6cdabb69718ee3ca86694579bf98a42c0c80d
   ```
1. Alice can now send `proof.txt` to Carol, who then can use it to verify the proof. As per the output note above, the proof **is only valid if the kernel in question is found on-chain**. One way to verify this is to locate the specific kernel in a block using a blockchain explorer.

**IMPORTANT NOTE:** When sending to older versions of the wallet, the address of the sender might be missing. In this case the proof only proves that the address of the receiving party was the one receiving the noted epics. Anyone in possession of this proof can claim they were the sender. If the sender field is missing, a warning will be displayed.

#### Verifying a transaction proof

In the example above, Alice has now sent the proof to Carol, who can then verify that file she received from Alice is indeed an untampered proof by validating it from her own wallet713 instance:
```
wallet713> $ proof verify <filename>
```
...where `<filename>` is the file path to the proof that should be verified (such as `proof.txt`). Example output:
```
wallet713> $ proof verify proof.txt
this file proves that [0.233232000] epics was sent to [zd7sCQ9bQuQXp4yCn8GSELcuSxnpcPrPoEWJzvPBc5vxyXPQz6PJ] from [zd7auPddUmmEzSte48a2aZ9tWkjjCppgn41pemUfcVSqjxHHZ6cT]

outputs:
  08710be0b3fffa79b9423f8e007709a815f237dcfd31340cfa1fdfefd823dca30e
kernel:
  099c8a166acd426481c1b09707b9e6cdabb69718ee3ca86694579bf98a42c0c80d

WARNING: this proof should only be considered valid if the kernel is actually on-chain with sufficient confirmations
please use a epic block explorer to verify this is the case. for example:
  https://floonet.epicscan.net/kernel/099c8a166acd426481c1b09707b9e6cdabb69718ee3ca86694579bf98a42c0c80d
```
Once again, as per the output note above, the proof **is only valid if the kernel in question is found on-chain**. One way to verify this is to locat the specific kernel in a block using a blockchain explorer.

**IMPORTANT NOTE:** When sending to older versions of the wallet, the address of the sender might be missing. In this case the proof only proves that the address of the receiving party. Anyone in posession of this proof can claim they were the sender. If the sender field is missing, a warning will be displayed.

### Using Contacts

To make it easier to transact with parties without having to deal with their epicbox addresses or keybase profiles, you can assign them nicknames that are stored locally in your contacts. **These contacts are stored locally on your machine and are not synced or shared with us.**

To add the epicbox address `zd6p24toTTDj7sxCCM4WGpBVcegVjGi9q5jquq6VWZA1BJroX514` to your contacts as `faucet`:
```
wallet713> $ contacts add faucet zd6p24toTTDj7sxCCM4WGpBVcegVjGi9q5jquq6VWZA1BJroX514
```

Similarly, to add the keybase address `keybase://ignotus` to your contacts as `igno`:
```
wallet713> $ contacts add igno keybase://ignotus
```

You can list your contacts:
```
wallet713> $ contacts
```

You can now send 10 epics to either of these contacts by their nicknames, preceded by @:
```
wallet713> $ send 10 --to @igno
```

### Using invoice

The `invoice` command is temporarily disabled in v2.0.0. It will be back in v2.0.1.

### Splitting your outputs

When building Epic transactions, the outputs (UTXOs) used become locked and cannot be used until the transaction is finalized. Ensuring you have available outputs helps you transact with multiple parties concurrently without having to wait for UTXOs to become available again.

Breaking down UTXOs can also help you protect your privacy as it makes it harder to determine which of those that belong to you.

As part of `send` you can determine how many change outputs you would like to receive, through the `-o` option. If you were sending @igno 10 epics from a single UTXO of 25 epics, the following transaction would generate 3 change outputs of 5 epics each:
```
wallet713> $ send 10 --to @igno -o 3
```

Similarly, as part of `invoice` you can specify in how many outputs you would like the payment to be received in. The following would allow you to receive 10 epics in total from @faucet, split in two outputs of 5 epics each:  
```
wallet713> $ invoice 10 --to @faucet -o 2
```

## Running your own node

Set corresponding `epic_node_uri` and `epic_node_secret` in your `~/.wallet713/XXX/wallet713.toml` where `XXX` is `floo` or `main` depending on which network you run the wallet for.

## Configuring Foreign and Owner APIs

Wallet713 provides a *variant* of epic's default wallet foreign and owner APIs.

The APIs are not exposed by default. You can turn each of them on by setting specific values in the `wallet713.toml` configuration file.

### Foreign API

Wallet713 Foreign API supports the default epic's wallet foreign API, allowing it to receive incoming slates and to build coinbase outputs.

In order to turn on foreign API support you need to set the following configuration option:

```
foreign_api = true
```

With this option, whenever you run wallet713 it would automatically start the foreign API listener.

By default the foreign api will bind to *0.0.0.0:3415* for mainnet and *0.0.0.0:13415* for floonet, however this can be configured with the following option:

```
foreign_api_address = "0.0.0.0:5555"
```

If you would like to secure access to the foreign api, you can set up a secret by using the following configuration option.

Note, however, that setting up a such a secret on the foreign requires the sending party to know the secret in order to communicate with your wallet for sending in epics.

```
foreign_api_secret = "<some secret string>"
```

### Owner API

Wallet713 support setting up an owner API listener. This API allows access to the wallet (for sending epics, retrieving info, etc.) via http requests.
It is important to never expose the owner API externally as it may compromise funds in your wallet! Also important to ensure there's a secret set on the API so that calls to the API are authenticated against the secret.

```
owner_api = true
owner_api_address = "127.0.0.1:13420"
owner_api_secret = "<some secret string>"
owner_api_include_foreign = <true|false>
```

Wallet713 Owner API supports the default epic's wallet owner API. Additionally `issue_send_tx` supports `epicbox` method where `dest` argument is a epicbox address.

Note that in order to utilize `keybase` and `epicbox` methods, the epicbox and keybase listeners must be initialized automatically at start by using the following configuration parameters in `wallet713.toml`:

```
epicbox_listener_auto_start = true
keybase_listener_auto_start = true
```

## Recovering your wallet

### Recovering a wallet from seed file
```
wallet713> $ restore
```

Note that this command will scan the chain for your UTXO's so it might take a few minutes to complete.

### Recovering a wallet using your mnemonic BIP-39 phrase
```
wallet713> $ seed recover
```
This will prompt for your mnemonic and allows you to set an optional password.

Note that this command will scan the chain for your UTXO's so it might take a few minutes to complete.

### Displaying existing BIP-39 mnemonic
```
wallet713> $ seed display
```

## Supported address formats

The following transaction addresses are currently supported.

### Epicbox
Assigned to you when you run the wallet for the first time. The address is derived from your seed. Mainnet epicbox addresses begin with `e`, floonet addresses begin with `x`.
Typical address format: `eVuDBqXYZekdpQ8EeT1bQXSk8KHKTZqFFiQwAecVCyyqZX8UwKZq`

####  Address derivation
Addresses are derived from your wallet seed. A single seed can generate up to `2^32` different addresses. Each of your addresses is specified by an index, which defaults to 0.

#### Switching address
Switching address is as simple as running the `address --next` command. This will switch to next index.

It is also possible to switch to the previous one with `address --prev` or to a specific index with `address --index <index>`.

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
