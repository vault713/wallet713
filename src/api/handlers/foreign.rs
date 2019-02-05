use futures::future;
use futures::{Future, Stream};
use gotham::handler::{HandlerFuture, IntoHandlerError};
use gotham::state::{FromState, State};
use hyper::body::Chunk;
use hyper::{Body, Response, StatusCode};

use crate::api::error::ApiError;
use crate::api::router::{trace_create_response, trace_state_and_body, WalletContainer};
use common::Result;
use wallet::types::{BlockFees, Slate};

pub fn receive_tx(mut state: State) -> Box<HandlerFuture> {
    let future = Body::take_from(&mut state)
        .concat2()
        .then(|body| match body {
            Ok(body) => match handle_receive_tx(&state, &body) {
                Ok(res) => future::ok((state, res)),
                Err(e) => future::err((state, ApiError::new(e).into_handler_error())),
            },
            Err(e) => future::err((state, e.into_handler_error())),
        });

    Box::new(future)
}

fn handle_receive_tx(state: &State, body: &Chunk) -> Result<Response<Body>> {
    trace_state_and_body(state, body);
    let mut slate: Slate = serde_json::from_slice(&body)?;
    let wallet = WalletContainer::borrow_from(&state).lock()?;
    wallet.process_sender_initiated_slate(None, &mut slate)?;
    Ok(trace_create_response(
        &state,
        StatusCode::OK,
        mime::APPLICATION_JSON,
        serde_json::to_string(&slate)?,
    ))
}

pub fn build_coinbase(mut state: State) -> Box<HandlerFuture> {
    let future = Body::take_from(&mut state)
        .concat2()
        .then(|body| match body {
            Ok(body) => match handle_build_coinbase(&state, &body) {
                Ok(res) => future::ok((state, res)),
                Err(e) => future::err((state, ApiError::new(e).into_handler_error())),
            },
            Err(e) => future::err((state, e.into_handler_error())),
        });

    Box::new(future)
}

fn handle_build_coinbase(state: &State, body: &Chunk) -> Result<Response<Body>> {
    trace_state_and_body(state, body);
    let block_fees: BlockFees = serde_json::from_slice(&body)?;
    let wallet = WalletContainer::borrow_from(&state).lock()?;
    let cb_data = wallet.build_coinbase(&block_fees)?;
    Ok(trace_create_response(
        &state,
        StatusCode::OK,
        mime::APPLICATION_JSON,
        serde_json::to_string(&cb_data)?,
    ))
}

pub fn receive_invoice(mut state: State) -> Box<HandlerFuture> {
    let future = Body::take_from(&mut state)
        .concat2()
        .then(|body| match body {
            Ok(body) => match handle_receive_invoice(&state, &body) {
                Ok(res) => future::ok((state, res)),
                Err(e) => future::err((state, ApiError::new(e).into_handler_error())),
            },
            Err(e) => future::err((state, e.into_handler_error())),
        });

    Box::new(future)
}

fn handle_receive_invoice(state: &State, body: &Chunk) -> Result<Response<Body>> {
    trace_state_and_body(state, body);
    let mut slate: Slate = serde_json::from_slice(&body)?;
    let wallet = WalletContainer::borrow_from(&state).lock()?;
    wallet.process_receiver_initiated_slate(&mut slate)?;
    Ok(trace_create_response(
        &state,
        StatusCode::OK,
        mime::APPLICATION_JSON,
        serde_json::to_string(&slate)?,
    ))
}
