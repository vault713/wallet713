use std::collections::HashMap;
use grin_core::core::amount_to_hr_string;
use grin_core::libtx::{build, tx_fee};

use super::types::{Error, ErrorKind, Slate, Transaction, NodeClient, Identifier, Keychain, WalletBackend, OutputData, TxLogEntry, TxLogEntryType, Context, ContextType, OutputStatus};
use super::keys;

pub fn build_send_tx_slate<T: ?Sized, C, K>(
    wallet: &mut T,
    num_participants: usize,
    amount: u64,
    current_height: u64,
    minimum_confirmations: u64,
    lock_height: u64,
    max_outputs: usize,
    change_outputs: usize,
    selection_strategy_is_use_all: bool,
    parent_key_id: Identifier,
) -> Result<
    (
        Slate,
        Context,
        impl FnOnce(&mut T, &Transaction) -> Result<(), Error>,
    ),
    Error,
>
    where
        T: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    let (elems, inputs, change_amounts_derivations, amount, fee) = select_send_tx(
        wallet,
        amount,
        1,
        current_height,
        minimum_confirmations,
        lock_height,
        max_outputs,
        change_outputs,
        selection_strategy_is_use_all,
        &parent_key_id,
    )?;

    // Create public slate
    let mut slate = Slate::blank(num_participants);
    slate.amount = amount;
    slate.height = current_height;
    slate.lock_height = lock_height;
    slate.fee = fee;
    let slate_id = slate.id.clone();

    let keychain = wallet.keychain().clone();

    let blinding = slate.add_transaction_elements(&keychain, elems)?;

    // Create our own private context
    let mut context = Context::new(
        wallet.keychain().secp(),
        blinding.secret_key(&keychain.secp()).unwrap(),
        ContextType::Tx,
    );

    // Store our private identifiers for each input
    for input in inputs {
        context.add_input(&input.key_id, &input.mmr_index);
    }

    let mut commits: HashMap<Identifier, Option<String>> = HashMap::new();

    // Store change output(s)
    for (change_amount, id, mmr_index) in &change_amounts_derivations {
        context.add_output(&id, &mmr_index);
        commits.insert(id.clone(), wallet.calc_commit_for_cache(*change_amount, &id)?);
    }

    let lock_inputs = context.get_inputs().clone();
    let _lock_outputs = context.get_outputs().clone();

    // Return a closure to acquire wallet lock and lock the coins being spent
    // so we avoid accidental double spend attempt.
    let update_sender_wallet_fn = move |wallet: &mut T, tx: &Transaction| {
        let mut batch = wallet.batch()?;
        let log_id = batch.next_tx_log_id(&parent_key_id)?;
        let mut t = TxLogEntry::new(parent_key_id.clone(), TxLogEntryType::TxSent, log_id);
        t.tx_slate_id = Some(slate_id);
        t.fee = Some(fee);
        let mut amount_debited = 0;
        t.num_inputs = lock_inputs.len();
        for id in lock_inputs {
            let mut coin = wallet.get_output(&id.0, &id.1).unwrap();
            coin.tx_log_entry = Some(log_id);
            amount_debited = amount_debited + coin.value;
            batch.lock_output(&mut coin)?;
        }

        t.amount_debited = amount_debited;

        // write the output representing our change
        for (change_amount, id, _) in &change_amounts_derivations {
            t.num_outputs += 1;
            t.amount_credited += change_amount;
            let commit = commits.get(&id).unwrap().clone();
            batch.save_output(&OutputData {
                root_key_id: parent_key_id.clone(),
                key_id: id.clone(),
                n_child: id.to_path().last_path_index(),
                commit,
                mmr_index: None,
                value: change_amount.clone(),
                status: OutputStatus::Unconfirmed,
                height: current_height,
                lock_height: 0,
                is_coinbase: false,
                tx_log_entry: Some(log_id),
            })?;
        }
        batch.save_tx_log_entry(&t)?;
        batch.store_tx(&slate_id.to_string(), &tx)?;
        batch.commit()?;
        Ok(())
    };

    Ok((slate, context, update_sender_wallet_fn))
}

pub fn build_recipient_output_with_slate<T: ?Sized, C, K>(
    wallet: &mut T,
    slate: &mut Slate,
    parent_key_id: Identifier,
) -> Result<
    (
        Identifier,
        Context,
        impl FnOnce(&mut T) -> Result<(), Error>,
    ),
    Error,
>
    where
        T: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    // Create a potential output for this transaction
    let key_id = keys::next_available_key(wallet).unwrap();

    let keychain = wallet.keychain().clone();
    let key_id_inner = key_id.clone();
    let amount = slate.amount;
    let height = slate.height;

    let slate_id = slate.id.clone();
    let blinding =
        slate.add_transaction_elements(&keychain, vec![build::output(amount, key_id.clone())])?;

    // Add blinding sum to our context
    let mut context = Context::new(
        keychain.secp(),
        blinding
            .secret_key(wallet.keychain().clone().secp())
            .unwrap(),
        ContextType::Tx,
    );

    context.add_output(&key_id, &None);

    // Create closure that adds the output to recipient's wallet
    // (up to the caller to decide when to do)
    let wallet_add_fn = move |wallet: &mut T| {
        let commit = wallet.calc_commit_for_cache(amount, &key_id_inner)?;
        let mut batch = wallet.batch()?;
        let log_id = batch.next_tx_log_id(&parent_key_id)?;
        let mut t = TxLogEntry::new(parent_key_id.clone(), TxLogEntryType::TxReceived, log_id);
        t.tx_slate_id = Some(slate_id);
        t.amount_credited = amount;
        t.num_outputs = 1;
        batch.save_output(&OutputData {
            root_key_id: parent_key_id.clone(),
            key_id: key_id_inner.clone(),
            n_child: key_id_inner.to_path().last_path_index(),
            commit,
            mmr_index: None,
            value: amount,
            status: OutputStatus::Unconfirmed,
            height: height,
            lock_height: 0,
            is_coinbase: false,
            tx_log_entry: Some(log_id),
        })?;
        batch.save_tx_log_entry(&t)?;
        batch.commit()?;
        Ok(())
    };
    Ok((key_id, context, wallet_add_fn))
}

fn select_send_tx<T: ?Sized, C, K>(
    wallet: &mut T,
    amount: u64,
    num_outputs: usize,
    current_height: u64,
    minimum_confirmations: u64,
    lock_height: u64,
    max_outputs: usize,
    change_outputs: usize,
    selection_strategy_is_use_all: bool,
    parent_key_id: &Identifier,
) -> Result<
    (
        Vec<Box<build::Append<K>>>,
        Vec<OutputData>,
        Vec<(u64, Identifier, Option<u64>)>, // change amounts and derivations
        u64,                    // amount
        u64,                    // fee
    ),
    Error,
>
    where
        T: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    // select some spendable coins from the wallet
    let (max_outputs, coins) = select_coins(
        wallet,
        amount,
        current_height,
        minimum_confirmations,
        max_outputs,
        selection_strategy_is_use_all,
        parent_key_id,
    );

    // sender is responsible for setting the fee on the partial tx
    // recipient should double check the fee calculation and not blindly trust the
    // sender

    // TODO - Is it safe to spend without a change output? (1 input -> 1 output)
    // TODO - Does this not potentially reveal the senders private key?
    //
    // First attempt to spend without change
    let mut fee = tx_fee(coins.len(), num_outputs, 1, None);
    let mut total: u64 = coins.iter().map(|c| c.value).sum();
    let mut amount_with_fee = amount + fee;

    if total == 0 {
        return Err(ErrorKind::NotEnoughFunds {
            available: 0,
            available_disp: amount_to_hr_string(0, false),
            needed: amount_with_fee as u64,
            needed_disp: amount_to_hr_string(amount_with_fee as u64, false),
        })?;
    }

    // The amount with fee is more than the total values of our max outputs
    if total < amount_with_fee && coins.len() == max_outputs {
        return Err(ErrorKind::NotEnoughFunds {
            available: total,
            available_disp: amount_to_hr_string(total, false),
            needed: amount_with_fee as u64,
            needed_disp: amount_to_hr_string(amount_with_fee as u64, false),
        })?;
    }

    let num_outputs = change_outputs + num_outputs;

    // We need to add a change address or amount with fee is more than total
    if total != amount_with_fee {
        fee = tx_fee(coins.len(), num_outputs, 1, None);
        amount_with_fee = amount + fee;

        // Here check if we have enough outputs for the amount including fee otherwise
        // look for other outputs and check again
        while total < amount_with_fee {
            // End the loop if we have selected all the outputs and still not enough funds
            if coins.len() == max_outputs {
                return Err(ErrorKind::NotEnoughFunds {
                    available: total as u64,
                    available_disp: amount_to_hr_string(total, false),
                    needed: amount_with_fee as u64,
                    needed_disp: amount_to_hr_string(amount_with_fee as u64, false),
                })?;
            }

            // select some spendable coins from the wallet
            let (_, coins) = select_coins(
                wallet,
                amount_with_fee,
                current_height,
                minimum_confirmations,
                max_outputs,
                selection_strategy_is_use_all,
                parent_key_id,
            );
            fee = tx_fee(coins.len(), num_outputs, 1, None);
            total = coins.iter().map(|c| c.value).sum();
            amount_with_fee = amount + fee;
        }
    }

    // build transaction skeleton with inputs and change
    let (mut parts, change_amounts_derivations) =
        inputs_and_change(&coins, wallet, amount, fee, change_outputs)?;

    // This is more proof of concept than anything but here we set lock_height
    // on tx being sent (based on current chain height via api).
    parts.push(build::with_lock_height(lock_height));

    Ok((parts, coins, change_amounts_derivations, amount, fee))
}

pub fn inputs_and_change<T: ?Sized, C, K>(
    coins: &Vec<OutputData>,
    wallet: &mut T,
    amount: u64,
    fee: u64,
    num_change_outputs: usize,
) -> Result<(Vec<Box<build::Append<K>>>, Vec<(u64, Identifier, Option<u64>)>), Error>
    where
        T: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    let mut parts = vec![];

    // calculate the total across all inputs, and how much is left
    let total: u64 = coins.iter().map(|c| c.value).sum();

    parts.push(build::with_fee(fee));

    // if we are spending 10,000 coins to send 1,000 then our change will be 9,000
    // if the fee is 80 then the recipient will receive 1000 and our change will be
    // 8,920
    let change = total - amount - fee;

    // build inputs using the appropriate derived key_ids
    for coin in coins {
        if coin.is_coinbase {
            parts.push(build::coinbase_input(coin.value, coin.key_id.clone()));
        } else {
            parts.push(build::input(coin.value, coin.key_id.clone()));
        }
    }

    let mut change_amounts_derivations = vec![];

    if change != 0 {
        let part_change = change / num_change_outputs as u64;
        let remainder_change = change % part_change;

        for x in 0..num_change_outputs {
            // n-1 equal change_outputs and a final one accounting for any remainder
            let change_amount = if x == (num_change_outputs - 1) {
                part_change + remainder_change
            } else {
                part_change
            };

            let change_key = wallet.derive_next().unwrap();

            change_amounts_derivations.push((change_amount, change_key.clone(), None));
            parts.push(build::output(change_amount, change_key));
        }
    }

    Ok((parts, change_amounts_derivations))
}

pub fn select_coins<T: ?Sized, C, K>(
    wallet: &mut T,
    amount: u64,
    current_height: u64,
    minimum_confirmations: u64,
    max_outputs: usize,
    select_all: bool,
    parent_key_id: &Identifier,
) -> (usize, Vec<OutputData>)
//    max_outputs_available, Outputs
    where
        T: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    // first find all eligible outputs based on number of confirmations
    let mut eligible = wallet
        .outputs()
        .filter(|out| {
            out.root_key_id == *parent_key_id
                && out.eligible_to_spend(current_height, minimum_confirmations)
        })
        .collect::<Vec<OutputData>>();

    let max_available = eligible.len();

    // sort eligible outputs by increasing value
    eligible.sort_by_key(|out| out.value);

    // use a sliding window to identify potential sets of possible outputs to spend
    // Case of amount > total amount of max_outputs(500):
    // The limit exists because by default, we always select as many inputs as
    // possible in a transaction, to reduce both the Output set and the fees.
    // But that only makes sense up to a point, hence the limit to avoid being too
    // greedy. But if max_outputs(500) is actually not enough to cover the whole
    // amount, the wallet should allow going over it to satisfy what the user
    // wants to send. So the wallet considers max_outputs more of a soft limit.
    if eligible.len() > max_outputs {
        for window in eligible.windows(max_outputs) {
            let windowed_eligibles = window.iter().cloned().collect::<Vec<_>>();
            if let Some(outputs) = select_from(amount, select_all, windowed_eligibles) {
                return (max_available, outputs);
            }
        }
        // Not exist in any window of which total amount >= amount.
        // Then take coins from the smallest one up to the total amount of selected
        // coins = the amount.
        if let Some(outputs) = select_from(amount, false, eligible.clone()) {
            return (max_available, outputs);
        }
    } else {
        if let Some(outputs) = select_from(amount, select_all, eligible.clone()) {
            return (max_available, outputs);
        }
    }

    // we failed to find a suitable set of outputs to spend,
    // so return the largest amount we can so we can provide guidance on what is
    // possible
    eligible.reverse();
    (
        max_available,
        eligible.iter().take(max_outputs).cloned().collect(),
    )
}

fn select_from(amount: u64, select_all: bool, outputs: Vec<OutputData>) -> Option<Vec<OutputData>> {
    let total = outputs.iter().fold(0, |acc, x| acc + x.value);
    if total >= amount {
        if select_all {
            return Some(outputs.iter().cloned().collect());
        } else {
            let mut selected_amount = 0;
            return Some(
                outputs
                    .iter()
                    .take_while(|out| {
                        let res = selected_amount < amount;
                        selected_amount += out.value;
                        res
                    })
                    .cloned()
                    .collect(),
            );
        }
    } else {
        None
    }
}

pub fn build_receive_tx_slate<T: ?Sized, C, K>(
    wallet: &mut T,
    num_participants: usize,
    amount: u64,
    num_outputs: usize,
    current_height: u64,
    lock_height: u64,
    parent_key_id: Identifier,
) -> Result<
    (
        Slate,
        Context,
        impl FnOnce(&mut T, &Transaction) -> Result<(), Error>,
    ),
    Error,
>
    where
        T: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    let mut slate = Slate::blank(num_participants);
    slate.amount = amount;
    slate.height = current_height;
    slate.lock_height = lock_height;

    let mut elems = vec![];
    let mut key_ids_and_amounts = vec![];

    let mut remaining_amount = amount;
    for i in 0..num_outputs {
        let key_id = keys::next_available_key(wallet).unwrap();
        let output_amount: u64 = if i == num_outputs - 1 {
            remaining_amount
        } else {
            amount / (num_outputs as u64)
        };
        if output_amount > 0 {
            key_ids_and_amounts.push((key_id.clone(), output_amount));
            elems.push(build::output(output_amount, key_id.clone()));
            remaining_amount -= output_amount;
        }
    }

    let keychain = wallet.keychain().clone();
    let blinding =
        slate.add_transaction_elements(&keychain, elems)?;

    let mut context = Context::new(
        keychain.secp(),
        blinding
            .secret_key(wallet.keychain().clone().secp())
            .unwrap(),
        ContextType::Tx,
    );

    let key_ids_and_amounts_inner = key_ids_and_amounts.clone();
    let mut commits: HashMap<Identifier, Option<String>> = HashMap::new();
    for (id, amount) in key_ids_and_amounts {
        context.add_output(&id.clone(), &None);
        commits.insert(id.clone(), wallet.calc_commit_for_cache(amount, &id)?);
    }

    let slate_id = slate.id.clone();
    let wallet_add_fn = move |wallet: &mut T, tx: &Transaction| {
        let mut batch = wallet.batch()?;
        let log_id = batch.next_tx_log_id(&parent_key_id)?;
        let mut t = TxLogEntry::new(parent_key_id.clone(), TxLogEntryType::TxSent, log_id);
        t.tx_slate_id = Some(slate_id);
        t.amount_credited = amount;
        t.num_outputs = num_outputs;
        for (id, amount) in key_ids_and_amounts_inner {
            let commit = commits.get(&id).unwrap().clone();
            batch.save_output(&OutputData {
                root_key_id: parent_key_id.clone(),
                key_id: id.clone(),
                n_child: id.to_path().last_path_index(),
                commit,
                mmr_index: None,
                value: amount,
                status: OutputStatus::Unconfirmed,
                height: current_height,
                lock_height: 0,
                is_coinbase: false,
                tx_log_entry: Some(log_id),
            })?;
        }
        batch.save_tx_log_entry(&t)?;
        batch.store_tx(&slate_id.to_string(), &tx)?;
        batch.commit()?;
        Ok(())
    };
    Ok((slate, context, wallet_add_fn))
}

pub fn build_recipient_input_with_slate<T: ?Sized, C, K>(
    wallet: &mut T,
    slate: &mut Slate,
    minimum_confirmations: u64,
    max_outputs: usize,
    num_change_outputs: usize,
    selection_strategy_is_use_all: bool,
    parent_key_id: Identifier,
) -> Result<
    (
        Context,
        impl FnOnce(&mut T, &Transaction) -> Result<(), Error>,
    ),
    Error,
>
    where
        T: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    let current_height = wallet.w2n_client().get_chain_height()?;

    let lock_height = slate.lock_height;
    let amount = slate.amount;
    let num_outputs = slate.tx.outputs().len();

    let (elems, inputs, change_amounts_derivations, _amount, fee) = select_send_tx(
        wallet,
        amount,
        num_outputs,
        current_height,
        minimum_confirmations,
        lock_height,
        max_outputs,
        num_change_outputs,
        selection_strategy_is_use_all,
        &parent_key_id,
    )?;

    slate.fee = fee;
    let slate_id = slate.id.clone();

    let keychain = wallet.keychain().clone();

    let blinding = slate.add_transaction_elements(&keychain, elems)?;

    let mut context = Context::new(
        wallet.keychain().secp(),
        blinding.secret_key(&keychain.secp()).unwrap(),
        ContextType::Tx,
    );

    for input in inputs {
        context.add_input(&input.key_id, &input.mmr_index);
    }

    let mut commits: HashMap<Identifier, Option<String>> = HashMap::new();

    for (change_amount, id, mmr_index) in &change_amounts_derivations {
        context.add_output(&id, &mmr_index);
        commits.insert(id.clone(), wallet.calc_commit_for_cache(*change_amount, &id)?);
    }

    let lock_inputs = context.get_inputs().clone();
    let _lock_outputs = context.get_outputs().clone();

    let update_sender_wallet_fn = move |wallet: &mut T, tx: &Transaction| {
        let mut batch = wallet.batch()?;
        let log_id = batch.next_tx_log_id(&parent_key_id)?;
        let mut t = TxLogEntry::new(parent_key_id.clone(), TxLogEntryType::TxReceived, log_id);
        t.tx_slate_id = Some(slate_id);
        t.fee = Some(fee);
        let mut amount_debited = 0;
        t.num_inputs = lock_inputs.len();
        for id in lock_inputs {
            let mut coin = wallet.get_output(&id.0, &id.1).unwrap();
            coin.tx_log_entry = Some(log_id);
            amount_debited = amount_debited + coin.value;
            batch.lock_output(&mut coin)?;
        }

        t.amount_debited = amount_debited;

        for (change_amount, id, _) in &change_amounts_derivations {
            t.num_outputs += 1;
            t.amount_credited += change_amount;
            let commit = commits.get(&id).unwrap().clone();
            batch.save_output(&OutputData {
                root_key_id: parent_key_id.clone(),
                key_id: id.clone(),
                n_child: id.to_path().last_path_index(),
                commit,
                mmr_index: None,
                value: change_amount.clone(),
                status: OutputStatus::Unconfirmed,
                height: current_height,
                lock_height: 0,
                is_coinbase: false,
                tx_log_entry: Some(log_id),
            })?;
        }
        batch.save_tx_log_entry(&t)?;
        batch.store_tx(&slate_id.to_string(), &tx)?;
        batch.commit()?;
        Ok(())
    };
    Ok((context, update_sender_wallet_fn))
}
