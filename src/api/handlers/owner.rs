use failure::Error;
use futures::future;
use futures::{Future, Stream};
use gotham::handler::{HandlerFuture, IntoHandlerError, IntoResponse};
use gotham::helpers::http::response::create_empty_response;
use gotham::state::{FromState, State};
use hyper::body::Chunk;
use hyper::{Body, Response, StatusCode};
use std::fs::File;
use std::io::Write;
use std::str::FromStr;
use url::Url;
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::api::router::{
    trace_create_response, trace_state, trace_state_and_body, WalletContainer,
};
use crate::broker::Publisher;
use crate::common::ErrorKind;
use crate::contacts::{Address, GrinboxAddress, KeybaseAddress};
use crate::wallet::types::Slate;

pub fn retrieve_outputs(state: State) -> (State, Response<Body>) {
    let res = match handle_retrieve_outputs(&state) {
        Ok(res) => res,
        Err(e) => ApiError::new(e).into_handler_error().into_response(&state),
    };
    (state, res)
}

#[derive(Deserialize, StateData, StaticResponseExtender)]
pub struct RetrieveOutputsQueryParams {
    refresh: Option<bool>,
    show_spent: Option<bool>,
    tx_id: Option<u32>,
}

fn handle_retrieve_outputs(state: &State) -> Result<Response<Body>, Error> {
    trace_state(state);
    let &RetrieveOutputsQueryParams {
        refresh,
        show_spent,
        tx_id,
    } = RetrieveOutputsQueryParams::borrow_from(&state);
    let wallet = WalletContainer::borrow_from(&state).lock()?;
    let response =
        wallet.retrieve_outputs(show_spent.unwrap_or(false), refresh.unwrap_or(false), tx_id)?;
    Ok(trace_create_response(
        &state,
        StatusCode::OK,
        mime::APPLICATION_JSON,
        serde_json::to_string(&response)?,
    ))
}

pub fn retrieve_txs(state: State) -> (State, Response<Body>) {
    let res = match handle_retrieve_txs(&state) {
        Ok(res) => res,
        Err(e) => ApiError::new(e).into_handler_error().into_response(&state),
    };
    (state, res)
}

#[derive(Deserialize, StateData, StaticResponseExtender)]
pub struct RetrieveTransactionsQueryParams {
    refresh: Option<bool>,
    id: Option<u32>,
    tx_id: Option<String>,
}

fn handle_retrieve_txs(state: &State) -> Result<Response<Body>, Error> {
    trace_state(state);
    let &RetrieveTransactionsQueryParams {
        refresh,
        id,
        ref tx_id,
    } = RetrieveTransactionsQueryParams::borrow_from(&state);
    let wallet = WalletContainer::borrow_from(&state).lock()?;
    let response = wallet.retrieve_txs(
        refresh.unwrap_or(false),
        id,
        tx_id
            .clone()
            .map(|x| Uuid::from_str(&x).unwrap_or(Uuid::default())),
    )?;
    Ok(trace_create_response(
        &state,
        StatusCode::OK,
        mime::APPLICATION_JSON,
        serde_json::to_string(&response)?,
    ))
}

pub fn retrieve_stored_tx(state: State) -> (State, Response<Body>) {
    let res = match handle_retrieve_stored_tx(&state) {
        Ok(res) => res,
        Err(e) => ApiError::new(e).into_handler_error().into_response(&state),
    };
    (state, res)
}

#[derive(Deserialize, StateData, StaticResponseExtender)]
pub struct RetrieveStoredTransactionQueryParams {
    id: u32,
}

fn handle_retrieve_stored_tx(state: &State) -> Result<Response<Body>, Error> {
    trace_state(state);
    let &RetrieveStoredTransactionQueryParams { id } =
        RetrieveStoredTransactionQueryParams::borrow_from(&state);
    let wallet = WalletContainer::borrow_from(&state).lock()?;
    let (_, txs) = wallet.retrieve_txs(true, Some(id), None)?;
    if txs.len() != 1 {
        return Err(ErrorKind::ModelNotFound.into());
    }

    if txs[0].tx_slate_id.is_none() {
        return Err(ErrorKind::ModelNotFound.into());
    }

    let stored_tx = wallet.get_stored_tx(&txs[0].tx_slate_id.unwrap().to_string())?;
    let response = (txs[0].confirmed, Some(stored_tx));
    Ok(trace_create_response(
        &state,
        StatusCode::OK,
        mime::APPLICATION_JSON,
        serde_json::to_string(&response)?,
    ))
}

pub fn node_height(state: State) -> (State, Response<Body>) {
    let res = match handle_node_height(&state) {
        Ok(res) => res,
        Err(e) => ApiError::new(e).into_handler_error().into_response(&state),
    };
    (state, res)
}

fn handle_node_height(state: &State) -> Result<Response<Body>, Error> {
    trace_state(state);
    let wallet = WalletContainer::borrow_from(&state).lock()?;
    let response = wallet.node_height()?;
    Ok(trace_create_response(
        &state,
        StatusCode::OK,
        mime::APPLICATION_JSON,
        serde_json::to_string(&response)?,
    ))
}

pub fn retrieve_summary_info(state: State) -> (State, Response<Body>) {
    let res = match handle_retrieve_summary_info(&state) {
        Ok(res) => res,
        Err(e) => ApiError::new(e).into_handler_error().into_response(&state),
    };
    (state, res)
}

fn handle_retrieve_summary_info(state: &State) -> Result<Response<Body>, Error> {
    trace_state(state);
    let wallet = WalletContainer::borrow_from(&state).lock()?;
    let response = wallet.retrieve_summary_info(true)?;
    Ok(trace_create_response(
        &state,
        StatusCode::OK,
        mime::APPLICATION_JSON,
        serde_json::to_string(&response)?,
    ))
}

pub fn finalize_tx(mut state: State) -> Box<HandlerFuture> {
    let future = Body::take_from(&mut state)
        .concat2()
        .then(|body| match body {
            Ok(body) => match handle_finalize_tx(&state, &body) {
                Ok(res) => future::ok((state, res)),
                Err(e) => future::err((state, ApiError::new(e).into_handler_error())),
            },
            Err(e) => future::err((state, e.into_handler_error())),
        });

    Box::new(future)
}

pub fn handle_finalize_tx(state: &State, body: &Chunk) -> Result<Response<Body>, Error> {
    trace_state_and_body(state, body);
    let mut slate: Slate = serde_json::from_slice(&body)?;
    let container = WalletContainer::borrow_from(&state);
    let wallet = container.lock()?;

    wallet.finalize_slate(&mut slate, None)?;

    Ok(create_empty_response(&state, StatusCode::OK))
}

pub fn cancel_tx(state: State) -> (State, Response<Body>) {
    let res = match handle_cancel_tx(&state) {
        Ok(res) => res,
        Err(e) => ApiError::new(e).into_handler_error().into_response(&state),
    };
    (state, res)
}

#[derive(Deserialize, StateData, StaticResponseExtender)]
pub struct CancelTransactionQueryParams {
    id: u32,
}

fn handle_cancel_tx(state: &State) -> Result<Response<Body>, Error> {
    trace_state(state);
    let &CancelTransactionQueryParams { id } = CancelTransactionQueryParams::borrow_from(&state);
    let wallet = WalletContainer::borrow_from(&state).lock()?;
    let response = wallet.cancel(id)?;
    Ok(trace_create_response(
        &state,
        StatusCode::OK,
        mime::APPLICATION_JSON,
        serde_json::to_string(&response)?,
    ))
}

pub fn post_tx(mut state: State) -> Box<HandlerFuture> {
    let future = Body::take_from(&mut state)
        .concat2()
        .then(|body| match body {
            Ok(body) => match handle_post_tx(&state, &body) {
                Ok(res) => future::ok((state, res)),
                Err(e) => future::err((state, ApiError::new(e).into_handler_error())),
            },
            Err(e) => future::err((state, e.into_handler_error())),
        });

    Box::new(future)
}

#[derive(Deserialize, StateData, StaticResponseExtender)]
pub struct PostTransactionQueryParams {
    fluff: Option<bool>,
}

pub fn handle_post_tx(state: &State, body: &Chunk) -> Result<Response<Body>, Error> {
    trace_state_and_body(state, body);
    let slate: Slate = serde_json::from_slice(&body)?;
    let &PostTransactionQueryParams { fluff } = PostTransactionQueryParams::borrow_from(&state);
    let container = WalletContainer::borrow_from(&state);
    let wallet = container.lock()?;
    wallet.post_tx(&slate.tx, fluff.unwrap_or(false))?;
    Ok(create_empty_response(&state, StatusCode::OK))
}

#[derive(Serialize, Deserialize, Debug)]
enum IssueSendMethod {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "grinbox")]
    Grinbox,
    #[serde(rename = "keybase")]
    Keybase,
    #[serde(rename = "http")]
    Http,
    #[serde(rename = "file")]
    File,
}

#[derive(Serialize, Deserialize, Debug)]
struct IssueSendBody {
    method: IssueSendMethod,
    dest: Option<String>,
    amount: u64,
    minimum_confirmations: u64,
    max_outputs: usize,
    num_change_outputs: usize,
    selection_strategy_is_use_all: bool,
    message: Option<String>,
    version: Option<u16>,
}

pub fn issue_send_tx(mut state: State) -> Box<HandlerFuture> {
    let future = Body::take_from(&mut state)
        .concat2()
        .then(|body| match body {
            Ok(body) => match handle_issue_send_tx(&state, &body) {
                Ok(res) => future::ok((state, res)),
                Err(e) => future::err((state, ApiError::new(e).into_handler_error())),
            },
            Err(e) => future::err((state, e.into_handler_error())),
        });

    Box::new(future)
}

pub fn handle_issue_send_tx(state: &State, body: &Chunk) -> Result<Response<Body>, Error> {
    trace_state_and_body(state, body);
    let body: IssueSendBody = serde_json::from_slice(&body)?;
    let container = WalletContainer::borrow_from(&state);
    let wallet = container.lock()?;
    let selection_strategy = match body.selection_strategy_is_use_all {
        true => "all",
        false => "",
    };

    let res = match body.method {
        IssueSendMethod::None => {
            let slate = wallet.initiate_send_tx(
                None,
                body.amount,
                body.minimum_confirmations,
                selection_strategy,
                body.num_change_outputs,
                body.max_outputs,
                body.message,
                body.version,
            )?;
            slate.serialize_to_original_version()?
        }
        IssueSendMethod::Grinbox => {
            let address = GrinboxAddress::from_str(
                body.dest
                    .ok_or_else(|| ErrorKind::GrinboxAddressParsingError(String::from("")))?
                    .as_str(),
            )?;
            let publisher = container.grinbox_publisher()?;
            let slate = wallet.initiate_send_tx(
                Some(address.to_string()),
                body.amount,
                body.minimum_confirmations,
                selection_strategy,
                body.num_change_outputs,
                body.max_outputs,
                body.message,
                body.version,
            )?;
            publisher.post_slate(&slate, &address)?;
            slate.serialize_to_original_version()?
        }
        IssueSendMethod::Keybase => {
            let address = KeybaseAddress::from_str(
                body.dest
                    .ok_or_else(|| ErrorKind::KeybaseAddressParsingError(String::from("")))?
                    .as_str(),
            )?;
            let publisher = container.keybase_publisher()?;
            let slate = wallet.initiate_send_tx(
                Some(address.to_string()),
                body.amount,
                body.minimum_confirmations,
                selection_strategy,
                body.num_change_outputs,
                body.max_outputs,
                body.message,
                body.version,
            )?;
            publisher.post_slate(&slate, &address)?;
            slate.serialize_to_original_version()?
        }
        IssueSendMethod::Http => {
            let destination = body
                .dest
                .ok_or_else(|| ErrorKind::GrinboxAddressParsingError(String::from("")))?;
            let url = Url::parse(&format!("{}/v1/wallet/foreign/receive_tx", destination))?;
            let slate = wallet.initiate_send_tx(
                Some(destination),
                body.amount,
                body.minimum_confirmations,
                selection_strategy,
                body.num_change_outputs,
                body.max_outputs,
                body.message,
                body.version,
            )?;
            let slate_ser = slate.serialize_to_original_version()?;
            let slate: String = grin_api::client::post(url.as_str(), None, &slate_ser)?;
            let mut slate = Slate::deserialize_upgrade(&slate)?;
            wallet.finalize_slate(&mut slate, None)?;
            slate.serialize_to_original_version()?
        }
        IssueSendMethod::File => {
            let mut file = File::create(
                body.dest
                    .ok_or_else(|| {
                        ErrorKind::GenericError(String::from("filename not specified in `dest`"))
                    })?
                    .as_str(),
            )?;
            let slate = wallet.initiate_send_tx(
                None,
                body.amount,
                body.minimum_confirmations,
                selection_strategy,
                body.num_change_outputs,
                body.max_outputs,
                body.message,
                body.version,
            )?;
            let json = slate.serialize_to_original_version()?;
            file.write_all(json.as_bytes())?;
            json
        }
    };

    Ok(trace_create_response(
        &state,
        StatusCode::OK,
        mime::APPLICATION_JSON,
        res,
    ))
}
