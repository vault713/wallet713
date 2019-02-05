# Using wallet713

While running, wallet713 works with an internal command prompt. You type commands in the same way as the CLI version of the grin wallet. **Ensure you are running a fully synced Grin node before using the wallet.**

## Contents

  * [Common use cases](#common-use-cases)
    + [Getting started](#getting-started)
    + [Transacting](#transacting)
      - [Transacting using grinbox](#transacting-using-grinbox)
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
    + [Transaction proofs (grinbox only)](#transaction-proofs-grinbox-only)
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
  * [Configuring Foreign & Owner APIs](#configuring_foreign_and_owner_apis)
    + [Foreign API](#foreign-api)
    + [Owner API](#owner-api)
  * [Recovering your wallet](#recovering-your-wallet)
    + [Recovering a wallet from seed file](#recovering-a-wallet-from-seed-file)
    + [Recovering a wallet using your mnemonic BIP-39 phrase](#recovering-a-wallet-using-your-mnemonic-bip-39-phrase)
    + [Displaying existing BIP-39 mnemonic](#displaying-existing-bip-39-mnemonic)
  * [Supported address formats](#supported-address-formats)
    + [Grinbox](#grinbox)
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

In order to receive grins from others you need to listen for transactions coming to your grinbox address:
```
wallet713> $ listen
```
This will also display your grinbox address.

To exit the wallet:
```
wallet713> $ exit
```

### Transacting

#### Transacting using grinbox 

Standard mainnet grinbox addresses begin with `g`.
Standard floonet grinbox addressses begin with `x`. 

To send a 10 grin transaction to the address `xd6p24toTTDj7sxCCM4WGpBVcegVjGi9q5jquq6VWZA1BJroX514`:
```
wallet713> $ send 10 --to xd6p24toTTDj7sxCCM4WGpBVcegVjGi9q5jquq6VWZA1BJroX514
```

To receive grins you simply keep wallet713 running and transactions are processed automatically. Any transactions received while being offline are fetched once you initiate `listen`. 


#### Transacting using Keybase

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

#### Transacting using https

##### Sending via https

wallet713 supports sending transactions to listening wallets via https. Only https is enabled for security reasons. 

To send 10 grins to https://some.wallet.713.mw:13415:
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
Generates the file `transaction.tx` in the designated path that sends 10 grins to a recipient.

##### Receiving a file-based transaction
Once `transaction.tx` is received from a sender, the command:
```
wallet713> $ receive --file ~/path/to/transaction.tx
```
...will process the received `transaction.tx` and generate `transaction.tx.response` in the same directory that should then be returned to the sender wallet.

##### Finalizing a file-based transaction
Having received back `transaction.tx.response`, the sender can then issue:
```
wallet713> $ finalize --file ~/path/to/transaction.tx.response
```
...which will finalize the transaction and broadcast it.

### Send configurations

#### Input selection strategy

Set the input selection strategy [`all`, `smallest`] with the `-s` option: 

To send a transaction using "all" as input selection strategy:
```
wallet713> $ send 10 --to xd6p24toTTDj7sxCCM4WGpBVcegVjGi9q5jquq6VWZA1BJroX514 -s all
```

#### Minimum number of confirmations

Set the minimum number of confirmation for inputs with the `-c` option, the default is `10`:

To send a transaction with 3 required confirmations: 
```
wallet713> $ send 10 --to xd6p24toTTDj7sxCCM4WGpBVcegVjGi9q5jquq6VWZA1BJroX514 -c 3
```

### Transaction proofs (grinbox only)

Thanks to the use of grinbox, wallet713 supports proving that a particular amount was sent in a transaction to a particular grinbox recipient address. It relies on the fact that a recipient needs to return a message to the sender in order to build a valid transaction. As part of that, the recipient need their private key to receive and process the sender's original message, as well as in order to sign and send back the response to the sender. The sender can then use this information to generate a proof that can be sent to Bob or a third party, (say Carol) that says that if a particular transaction kernel is visible on the blockchain, a certain grinbox address has received a transaction of a certain amount. **This can only be used for transactions that have been sent using grinbox and you need wallet713 to generate and validate a transaction proof.**

In the below example,
1. Alice wants to send Bob 1.337 grins and prove to Carol that this transaction has occurred.
1. Bob has grinbox address: `xd7sCQ9bQuQXp4yCn8GSELcuSxnpcPrPoEWJzvPBc5vxyXPQz6PJ`;

#### Creating a transaction proof

1. Alice uses grinbox to send Bob grins using grinbox and broadcasts the transaction to the blockchain:
   ```
   wallet713> $ send 0.233232 --to xd7sCQ9bQuQXp4yCn8GSELcuSxnpcPrPoEWJzvPBc5vxyXPQz6PJ
   ```
1. Alice runs `txs` command to display the transaction log and to identify which ID her transaction has:
   ```
   wallet713> $ txs
   ```
   The transaction in question should show a `yes` in the `proof` column. Example output:
   ```
    23  Sent Tx      4b6ede9f  grinbox://xd7sCQ9bQuQXp4yCn8GSELcuSxnpcPrPoEWJzvPBc5vxyXPQz6PJ                 2019-01-27 20:45:01  yes         2019-01-31 01:02:18  -0.234232      yes 
   ```

1. Alice now generates a proof for this transaction:
   ```
   wallet713> $ export-proof -i <number> -f <filename>
   ```
   ...where `<number>` is the ID in question (in our example `23`), and `<filename>` is the file name that the proof should be saved as (such as `proof.txt`).

1. If successful, Alice receives a confirmation message. Example output:
   ```
   wallet713> $ export-proof -i 23 -f proof.txt
   proof written to proof.txt
   this file proves that [0.233232000] grins was sent to [xd7sCQ9bQuQXp4yCn8GSELcuSxnpcPrPoEWJzvPBc5vxyXPQz6PJ] from [xd7auPddUmmEzSte48a2aZ9tWkjjCppgn41pemUfcVSqjxHHZ6cT]

   outputs:
      08710be0b3fffa79b9423f8e007709a815f237dcfd31340cfa1fdfefd823dca30e
   kernel:
      099c8a166acd426481c1b09707b9e6cdabb69718ee3ca86694579bf98a42c0c80d

   WARNING: this proof should only be considered valid if the kernel is actually on-chain with sufficient confirmations
   please use a grin block explorer to verify this is the case. for example:
      https://floonet.grinscan.net/kernel/099c8a166acd426481c1b09707b9e6cdabb69718ee3ca86694579bf98a42c0c80d
   ```
1. Alice can now send `proof.txt` to Carol, who then can use it to verify the proof. As per the output note above, the proof **is only valid if the kernel in question is found on-chain**. One way to verify this is to locate the specific kernel in a block using a blockchain explorer.

**IMPORTANT NOTE:** When sending to older versions of the wallet, the address of the sender might be missing. In this case the proof only proves that the address of the receiving party was the one receiving the noted grins. Anyone in possession of this proof can claim they were the sender. If the sender field is missing, a warning will be displayed.

#### Verifying a transaction proof

In the example above, Alice has now sent the proof to Carol, who can then verify that file she received from Alice is indeed an untampered proof by validating it from her own wallet713 instance:
```
wallet713> $ verify-proof -f <filename>
```
...where `<filename>` is the file path to the proof that should be verified (such as `proof.txt`). Example output:
```
wallet713> $ verify-proof -f proof.txt
this file proves that [0.233232000] grins was sent to [xd7sCQ9bQuQXp4yCn8GSELcuSxnpcPrPoEWJzvPBc5vxyXPQz6PJ] from [xd7auPddUmmEzSte48a2aZ9tWkjjCppgn41pemUfcVSqjxHHZ6cT]

outputs:
  08710be0b3fffa79b9423f8e007709a815f237dcfd31340cfa1fdfefd823dca30e
kernel:
  099c8a166acd426481c1b09707b9e6cdabb69718ee3ca86694579bf98a42c0c80d

WARNING: this proof should only be considered valid if the kernel is actually on-chain with sufficient confirmations
please use a grin block explorer to verify this is the case. for example:
  https://floonet.grinscan.net/kernel/099c8a166acd426481c1b09707b9e6cdabb69718ee3ca86694579bf98a42c0c80d
```
Once again, as per the output note above, the proof **is only valid if the kernel in question is found on-chain**. One way to verify this is to locat the specific kernel in a block using a blockchain explorer.

**IMPORTANT NOTE:** When sending to older versions of the wallet, the address of the sender might be missing. In this case the proof only proves that the address of the receiving party. Anyone in posession of this proof can claim they were the sender. If the sender field is missing, a warning will be displayed.

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

## Configuring Foreign and Owner APIs

Wallet713 provides a *variant* of grin's default wallet foreign and owner APIs.

The APIs are not exposed by default. You can turn each of them on by setting specific values in the `wallet713.toml` configuration file.

### Foreign API

Wallet713 Foreign API supports the default grin's wallet foreign API, allowing it to receive incoming slates and to build coinbase outputs.

In addition, wallet713 foreign API implementation supports a new route for receiving invoice slates: `/v1/wallet/foreign/receive_invoice`.

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

Note, however, that setting up a such a secret on the foreign requires the sending party to know the secret in order to communicate with your wallet for sending in grins.

```
foreign_api_secret = "<some secret string>"
```

### Owner API

Wallet713 support setting up an owner API listener. This API allows access to the wallet (for sending grins, retrieving info, etc.) via http requests.
It is important to never expose the owner API externally as it may compromise funds in your wallet! Also important to ensure there's a secret set on the API so that calls to the API are authenticated against the secret.

```
owner_api = true
owner_api_address = "127.0.0.1:13420"
owner_api_secret = "<some secret string>"
owner_api_include_foreign = <true|false>
``` 

Wallet713 Owner API supports the default grin's wallet owner API. Additionally `issue_send_tx` supports `grinbox` method where `dest` argument is a grinbox address.

Note that in order to utilize `keybase` and `grinbox` methods, the grinbox and keybase listeners must be initialized automatically at start by using the following configuration parameters in `wallet713.toml`:

```
grinbox_listener_auto_start = true
keybase_listener_auto_start = true
```

## Recovering your wallet

### Recovering a wallet from seed file
```
wallet713> $ restore
```
Remember to include the `-p yourpassphrase` if your seed is password protected.

### Recovering a wallet using your mnemonic BIP-39 phrase
```
wallet713> $ recover -m word1 word2 ...
```
If you would like to set a passphrase, remember to include the `-p yourpassphrase` as you run the command.

### Displaying existing BIP-39 mnemonic
```
wallet713> $ recover -d
```
Remember to include the `-p yourpassphrase` if your seed is password protected.

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
