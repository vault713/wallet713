use prettytable;
use std::io::prelude::Write;
use term;

use grin_util::secp::pedersen;
use grin_core::global;
use grin_core::core::{self, amount_to_hr_string};

use super::types::{AcctPathMapping, OutputData, OutputStatus, TxLogEntry, WalletInfo, Error};

/// Display outputs in a pretty way
pub fn outputs(
    account: &str,
    cur_height: u64,
    validated: bool,
    outputs: Vec<(OutputData, pedersen::Commitment)>,
    dark_background_color_scheme: bool,
) -> Result<(), Error> {
    let title = format!(
        "Wallet Outputs - Account '{}' - Block Height: {}",
        account, cur_height
    );
    println!();
    let mut t = term::stdout().unwrap();
    t.fg(term::color::MAGENTA).unwrap();
    writeln!(t, "{}", title).unwrap();
    t.reset().unwrap();

    let mut table = table!();

    table.set_titles(row![
		bMG->"Output Commitment",
        bMG->"MMR Index",
		bMG->"Block Height",
		bMG->"Locked Until",
		bMG->"Status",
		bMG->"Coinbase?",
		bMG->"# Confirms",
		bMG->"Value",
		bMG->"Tx"
	]);

    for (out, commit) in outputs {
        let commit = format!("{}", grin_util::to_hex(commit.as_ref().to_vec()));
        let index = match out.mmr_index {
            None => "None".to_owned(),
            Some(t) => t.to_string(),
        };
        let height = format!("{}", out.height);
        let lock_height = format!("{}", out.lock_height);
        let is_coinbase = format!("{}", out.is_coinbase);

        // Mark unconfirmed coinbase outputs as "Mining" instead of "Unconfirmed"
        let status = match out.status {
            OutputStatus::Unconfirmed if out.is_coinbase => "Mining".to_string(),
            _ => format!("{}", out.status),
        };

        let num_confirmations = format!("{}", out.num_confirmations(cur_height));
        let value = format!("{}", core::amount_to_hr_string(out.value, false));
        let tx = match out.tx_log_entry {
            None => "".to_owned(),
            Some(t) => t.to_string(),
        };

        if dark_background_color_scheme {
            table.add_row(row![
				bFC->commit,
				bFB->index,
				bFB->height,
				bFB->lock_height,
				bFR->status,
				bFY->is_coinbase,
				bFB->num_confirmations,
				bFG->value,
				bFC->tx,
			]);
        } else {
            table.add_row(row![
				bFD->commit,
				bFB->index,
				bFB->height,
				bFB->lock_height,
				bFR->status,
				bFD->is_coinbase,
				bFB->num_confirmations,
				bFG->value,
				bFD->tx,
			]);
        }
    }

    table.set_format(*prettytable::format::consts::FORMAT_NO_COLSEP);
    table.printstd();
    println!();

    if !validated {
        println!(
            "\nWARNING: Wallet failed to verify data. \
			 The above is from local cache and possibly invalid! \
			 (is your `grin server` offline or broken?)"
        );
    }
    Ok(())
}

/// Display transaction log in a pretty way
pub fn txs(
    account: &str,
    cur_height: u64,
    validated: bool,
    txs: Vec<TxLogEntry>,
    include_status: bool,
    dark_background_color_scheme: bool,
) -> Result<(), Error> {
    let title = format!(
        "Transaction Log - Account '{}' - Block Height: {}",
        account, cur_height
    );
    println!();
    let mut t = term::stdout().unwrap();
    t.fg(term::color::MAGENTA).unwrap();
    writeln!(t, "{}", title).unwrap();
    t.reset().unwrap();

    let mut table = table!();

    table.set_titles(row![
		bMG->"Id",
		bMG->"Type",
		bMG->"TXID",
		bMG->"Address",
		bMG->"Creation Time",
		bMG->"Confirmed?",
		bMG->"Confirmation Time",
		bMG->"Net \nDifference",
	]);

    for t in txs {
        let id = format!("{}", t.id);
        let slate_id = match t.tx_slate_id {
            Some(m) => grin_util::to_hex(m.as_bytes()[..4].to_vec()),
            None => String::from(""),
        };
        let address = match t.address {
            Some(a) => a.clone(),
            None => String::from(""),
        };
        let entry_type = format!("{}", t.tx_type);
        let creation_ts = format!("{}", t.creation_ts.format("%Y-%m-%d %H:%M:%S"));
        let confirmation_ts = match t.confirmation_ts {
            Some(m) => format!("{}", m.format("%Y-%m-%d %H:%M:%S")),
            None => String::from(""),
        };
        let confirmed = match t.confirmed {
            true => "yes",
            false => "no",
        };
        let net_diff = if t.amount_credited >= t.amount_debited {
            core::amount_to_hr_string(t.amount_credited - t.amount_debited, true)
        } else {
            format!(
                "-{}",
                core::amount_to_hr_string(t.amount_debited - t.amount_credited, true)
            )
        };
        if dark_background_color_scheme {
            table.add_row(row![
				bFC->id,
				bFC->entry_type,
				bFB->slate_id,
				bFC->address,
				bFB->creation_ts,
				bFC->confirmed,
				bFB->confirmation_ts,
				bFY->net_diff,
			]);
        } else {
            if t.confirmed {
                table.add_row(row![
					bFD->id,
					bFb->entry_type,
					bFD->slate_id,
					bFD->address,
					bFB->creation_ts,
					bFg->confirmed,
					bFB->confirmation_ts,
					bFG->net_diff,
				]);
            } else {
                table.add_row(row![
					bFD->id,
					bFb->entry_type,
					bFD->slate_id,
					bFD->address,
					bFB->creation_ts,
					bFR->confirmed,
					bFB->confirmation_ts,
					bFG->net_diff,
				]);
            }
        }
    }

    table.set_format(*prettytable::format::consts::FORMAT_NO_COLSEP);
    table.printstd();
    println!();

    if !validated && include_status {
        println!(
            "\nWARNING: Wallet failed to verify data. \
			 The above is from local cache and possibly invalid! \
			 (is your `grin server` offline or broken?)"
        );
    }
    Ok(())
}

/// Display summary info in a pretty way
pub fn info(
    account: &str,
    wallet_info: &WalletInfo,
    validated: bool,
    dark_background_color_scheme: bool,
) {
    println!(
        "\n____ Wallet Summary Info - Account '{}' as of height {} ____\n",
        account, wallet_info.last_confirmed_height,
    );

    let mut table = table!();

    if dark_background_color_scheme {
        table.add_row(row![
			bFG->"Total",
			FG->amount_to_hr_string(wallet_info.total, false)
		]);
        // Only dispay "Immature Coinbase" if we have related outputs in the wallet.
        // This row just introduces confusion if the wallet does not receive coinbase rewards.
        if wallet_info.amount_immature > 0 {
            table.add_row(row![
				bFY->format!("Immature Coinbase (< {})", global::coinbase_maturity()),
				FY->amount_to_hr_string(wallet_info.amount_immature, false)
			]);
        }
        table.add_row(row![
			bFY->format!("Awaiting Confirmation (< {})", wallet_info.minimum_confirmations),
			FY->amount_to_hr_string(wallet_info.amount_awaiting_confirmation, false)
		]);
        table.add_row(row![
			Fr->"Locked by previous transaction",
			Fr->amount_to_hr_string(wallet_info.amount_locked, false)
		]);
        table.add_row(row![
			Fw->"--------------------------------",
			Fw->"-------------"
		]);
        table.add_row(row![
			bFG->"Currently Spendable",
			FG->amount_to_hr_string(wallet_info.amount_currently_spendable, false)
		]);
    } else {
        table.add_row(row![
			bFG->"Total",
			FG->amount_to_hr_string(wallet_info.total, false)
		]);
        // Only dispay "Immature Coinbase" if we have related outputs in the wallet.
        // This row just introduces confusion if the wallet does not receive coinbase rewards.
        if wallet_info.amount_immature > 0 {
            table.add_row(row![
				bFB->format!("Immature Coinbase (< {})", global::coinbase_maturity()),
				FB->amount_to_hr_string(wallet_info.amount_immature, false)
			]);
        }
        table.add_row(row![
			bFB->format!("Awaiting Confirmation (< {})", wallet_info.minimum_confirmations),
			FB->amount_to_hr_string(wallet_info.amount_awaiting_confirmation, false)
		]);
        table.add_row(row![
			Fr->"Locked by previous transaction",
			Fr->amount_to_hr_string(wallet_info.amount_locked, false)
		]);
        table.add_row(row![
			Fw->"--------------------------------",
			Fw->"-------------"
		]);
        table.add_row(row![
			bFG->"Currently Spendable",
			FG->amount_to_hr_string(wallet_info.amount_currently_spendable, false)
		]);
    };
    table.set_format(*prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.printstd();
    println!();
    if !validated {
        println!(
            "\nWARNING: Wallet failed to verify data against a live chain. \
			 The above is from local cache and only valid up to the given height! \
			 (is your `grin server` offline or broken?)"
        );
    }
}

/// Display list of wallet accounts in a pretty way
pub fn accounts(acct_mappings: Vec<AcctPathMapping>) {
    println!("\n____ Wallet Accounts ____\n", );
    let mut table = table!();

    table.set_titles(row![
		mMG->"Name",
		bMG->"Parent BIP-32 Derivation Path",
	]);
    for m in acct_mappings {
        table.add_row(row![
			bFC->m.label,
			bGC->m.path.to_bip_32_string(),
		]);
    }
    table.set_format(*prettytable::format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.printstd();
    println!();
}
