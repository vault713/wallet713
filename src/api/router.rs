use easy_jsonrpc::{Handler, MaybeReply};
use failure::Error;
use futures::future;
use futures::{Future, Stream};
use gotham::handler::{HandlerFuture, IntoHandlerError};
use gotham::helpers::http::response::create_response;
use gotham::middleware::{NewMiddleware, Middleware};
use gotham::pipeline::new_pipeline;
use gotham::pipeline::single::single_pipeline;
use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::{FromState, State};
use hyper::{Body, Chunk, HeaderMap, Method, Response, StatusCode, Uri, Version};
use mime::Mime;
use serde_json::{Value, json};
use std::panic::RefUnwindSafe;

use crate::api::auth::BasicAuthMiddleware;
use crate::api::error::ApiError;
use crate::common::Keychain;
use crate::wallet::api::{Foreign, Owner};
use crate::wallet::types::{Arc, NodeClient, Mutex, WalletBackend};
use crate::wallet::Container;
use super::rpc::{ForeignRpc, OwnerRpc};

pub struct ForeignApiMiddleware<W, C, K>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    api: Foreign<W, C, K>,
}

impl<W, C, K> RefUnwindSafe for ForeignApiMiddleware<W, C, K>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{}

impl<W, C, K> ForeignApiMiddleware<W, C, K>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    fn new(
        container: Arc<Mutex<Container<W, C, K>>>
    ) -> Self {
        Self {
            api: Foreign::new(container),
        }
    }
}


impl<W, C, K> Middleware for ForeignApiMiddleware<W, C, K>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        state.put(self.api);
        chain(state)
    }
}

impl<W, C, K> NewMiddleware for ForeignApiMiddleware<W, C, K>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    type Instance = Self;

    /// Clones the current middleware to a new instance.
    fn new_middleware(&self) -> std::io::Result<Self::Instance> {
        Ok(Self {
            api: self.api.clone()
        })
    }
}

pub struct OwnerApiMiddleware<W, C, K>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    api: Owner<W, C, K>,
}

impl<W, C, K> RefUnwindSafe for OwnerApiMiddleware<W, C, K>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{}

impl<W, C, K> OwnerApiMiddleware<W, C, K>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    fn new(
        container: Arc<Mutex<Container<W, C, K>>>
    ) -> Self {
        Self {
            api: Owner::new(container),
        }
    }
}


impl<W, C, K> Middleware for OwnerApiMiddleware<W, C, K>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    fn call<Chain>(self, mut state: State, chain: Chain) -> Box<HandlerFuture>
    where
        Chain: FnOnce(State) -> Box<HandlerFuture>,
    {
        state.put(self.api);
        chain(state)
    }
}

impl<W, C, K> NewMiddleware for OwnerApiMiddleware<W, C, K>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    type Instance = Self;

    /// Clones the current middleware to a new instance.
    fn new_middleware(&self) -> std::io::Result<Self::Instance> {
        Ok(Self {
            api: self.api.clone()
        })
    }
}

pub fn build_foreign_api_router<W, C, K>(
    container: Arc<Mutex<Container<W, C, K>>>,
    foreign_api_secret: Option<String>,
) -> Router
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    let (chain, pipelines) = single_pipeline(
        new_pipeline()
            .add(BasicAuthMiddleware::new(foreign_api_secret))
            .add(ForeignApiMiddleware::new(container))
            .build(),
    );

    build_router(chain, pipelines, |route| {
        route.request(vec![Method::POST], "/v2/foreign").to(foreign_api_handler::<W, C, K>);
    })
}

fn foreign_api_handler<W, C, K>(mut state: State) -> Box<HandlerFuture>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    let future = Body::take_from(&mut state)
        .concat2()
        .then(|body| match body {
            Ok(body) => match foreign_api_handler_inner::<W, C, K>(&state, &body) {
                Ok(res) => future::ok((state, res)),
                Err(e) => future::err((state, ApiError::new(e).into_handler_error())),
            },
            Err(e) => future::err((state, e.into_handler_error())),
        });

    Box::new(future)
}

fn foreign_api_handler_inner<W, C, K>(state: &State, body: &Chunk) -> Result<Response<Body>, Error>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    trace_state_and_body(state, body);

    let val: Value = serde_json::from_reader(&body.to_vec()[..])?;
    let api = Foreign::<W, C, K>::borrow_from(&state);

    let foreign_api = api as &dyn ForeignRpc;
    let res = match foreign_api.handle_request(val) {
        MaybeReply::Reply(r) => r,
        MaybeReply::DontReply => {
            // Since it's http, we need to return something. We return [] because jsonrpc
            // clients will parse it as an empty batch response.
            json!([])
        }
    };

    Ok(trace_create_response(
        state,
        StatusCode::OK,
        mime::APPLICATION_JSON,
        res.to_string()
    ))
}

pub fn build_owner_api_router<W, C, K>(
    container: Arc<Mutex<Container<W, C, K>>>,
    owner_api_secret: Option<String>,
) -> Router
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    let (chain, pipelines) = single_pipeline(
        new_pipeline()
            .add(BasicAuthMiddleware::new(owner_api_secret))
            .add(OwnerApiMiddleware::new(container))
            .build(),
    );

    build_router(chain, pipelines, |route| {
        route.request(vec![Method::POST], "/v2/owner").to(owner_api_handler::<W, C, K>);
    })
}

fn owner_api_handler<W, C, K>(mut state: State) -> Box<HandlerFuture>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    let future = Body::take_from(&mut state)
        .concat2()
        .then(|body| match body {
            Ok(body) => match owner_api_handler_inner::<W, C, K>(&state, &body) {
                Ok(res) => future::ok((state, res)),
                Err(e) => future::err((state, ApiError::new(e).into_handler_error())),
            },
            Err(e) => future::err((state, e.into_handler_error())),
        });

    Box::new(future)
}

fn owner_api_handler_inner<W, C, K>(state: &State, body: &Chunk) -> Result<Response<Body>, Error>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    trace_state_and_body(state, body);

    let val: Value = serde_json::from_reader(&body.to_vec()[..])?;
    let api = Owner::<W, C, K>::borrow_from(&state);

    let owner_api = api as &dyn OwnerRpc;
    let res = match owner_api.handle_request(val) {
        MaybeReply::Reply(r) => r,
        MaybeReply::DontReply => {
            // Since it's http, we need to return something. We return [] because jsonrpc
            // clients will parse it as an empty batch response.
            json!([])
        }
    };

    Ok(trace_create_response(
        state,
        StatusCode::OK,
        mime::APPLICATION_JSON,
        res.to_string()
    ))
}

fn trace_state_and_body(state: &State, body: &Chunk) {
    let method = Method::borrow_from(state);
    let uri = Uri::borrow_from(state);
    let http_version = Version::borrow_from(state);
    let headers = HeaderMap::borrow_from(state);
    let body = String::from_utf8(body.to_vec()).unwrap();
    trace!(
        "REQUEST Method: {} URI: {} HTTP Version: {:?} Headers: {:?} Body: {}",
        method,
        uri,
        http_version,
        headers,
        body
    );
}

fn trace_create_response(
    state: &State,
    status: StatusCode,
    mime: Mime,
    body: String,
) -> Response<Body> {
    let headers = HeaderMap::borrow_from(state);
    trace!(
        "RESPONSE ({}) Headers: {:?} Body: {}",
        status,
        headers,
        body
    );
    create_response(state, status, mime, body)
}
