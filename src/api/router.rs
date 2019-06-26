use easy_jsonrpc::{Handler, MaybeReply};
use failure::Error;
use futures::future;
use futures::{Future, Stream};
use gotham::handler::{HandlerFuture, IntoHandlerError};
use gotham::helpers::http::response::create_response;
use gotham::middleware::state::StateMiddleware;
use gotham::middleware::{NewMiddleware, Middleware};
use gotham::pipeline::chain::PipelineHandleChain;
use gotham::pipeline::new_pipeline;
use gotham::pipeline::single::single_pipeline;
use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::{FromState, State, StateData};
use hyper::{Body, Chunk, HeaderMap, Method, Response, StatusCode, Uri, Version};
use mime::Mime;
use std::panic::RefUnwindSafe;

use crate::api::auth::BasicAuthMiddleware;
use crate::api::error::ApiError;
use crate::api::handlers::{foreign, owner};
use crate::broker::{GrinboxPublisher, GrinboxSubscriber, KeybasePublisher, KeybaseSubscriber};
use crate::common::{ErrorKind, Keychain};
use crate::wallet::api::Foreign;
use crate::wallet::types::{Arc, NodeClient, Mutex, MutexGuard, WalletBackend};
use crate::wallet::Container;
use super::rpc::ForeignRpc;

pub struct ApiMiddleware<W, C, K>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    api: Foreign<W, C, K>,
}

impl<W, C, K> RefUnwindSafe for ApiMiddleware<W, C, K>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{}

impl<W, C, K> ApiMiddleware<W, C, K>
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


impl<W, C, K> Middleware for ApiMiddleware<W, C, K>
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

impl<W, C, K> NewMiddleware for ApiMiddleware<W, C, K>
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

/*fn build_owner_api<C, P>(route: &mut RouterBuilder<C, P>, owner_api_include_foreign: Option<bool>)
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: RefUnwindSafe + Send + Sync + 'static,
{
    route
        .get("/v1/wallet/owner/retrieve_outputs")
        .with_query_string_extractor::<owner::RetrieveOutputsQueryParams>()
        .to(owner::retrieve_outputs);

    route
        .get("/v1/wallet/owner/retrieve_txs")
        .with_query_string_extractor::<owner::RetrieveTransactionsQueryParams>()
        .to(owner::retrieve_txs);

    route
        .get("/v1/wallet/owner/retrieve_stored_tx")
        .with_query_string_extractor::<owner::RetrieveStoredTransactionQueryParams>()
        .to(owner::retrieve_stored_tx);

    route
        .get("/v1/wallet/owner/node_height")
        .to(owner::node_height);

    route
        .get("/v1/wallet/owner/retrieve_summary_info")
        .to(owner::retrieve_summary_info);

    route
        .post("/v1/wallet/owner/finalize_tx")
        .to(owner::finalize_tx);

    route
        .post("/v1/wallet/owner/cancel_tx")
        .with_query_string_extractor::<owner::CancelTransactionQueryParams>()
        .to(owner::cancel_tx);

    route
        .post("/v1/wallet/owner/post_tx")
        .with_query_string_extractor::<owner::PostTransactionQueryParams>()
        .to(owner::post_tx);

    route
        .post("/v1/wallet/owner/issue_send_tx")
        .to(owner::issue_send_tx);

    if owner_api_include_foreign.is_some() && owner_api_include_foreign.unwrap() == true {
        build_foreign_api(route);
    }
}*/

/*pub fn build_owner_api_router(
    wallet: Arc<Mutex<Wallet>>,
    grinbox_broker: Option<(GrinboxPublisher, GrinboxSubscriber)>,
    keybase_broker: Option<(KeybasePublisher, KeybaseSubscriber)>,
    owner_api_secret: Option<String>,
    owner_api_include_foreign: Option<bool>,
) -> Router {
    let grinbox_publisher = grinbox_broker.map(|(p, _)| p);
    let keybase_publisher = keybase_broker.map(|(p, _)| p);

    let (chain, pipelines) = single_pipeline(
        new_pipeline()
            .add(BasicAuthMiddleware::new(owner_api_secret))
            .add(StateMiddleware::new(WalletContainer::new(
                wallet,
                grinbox_publisher,
                keybase_publisher,
            )))
            .build(),
    );

    build_router(chain, pipelines, |route| {
        build_owner_api(route, owner_api_include_foreign);
    })
}*/

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
            .add(ApiMiddleware::new(container))
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

    let val: serde_json::Value = serde_json::from_reader(&body.to_vec()[..])?;
    let api = Foreign::<W, C, K>::borrow_from(&state);

    let foreign_api = api as &dyn ForeignRpc;
    let res = match foreign_api.handle_request(val) {
        MaybeReply::Reply(r) => r,
        MaybeReply::DontReply => {
            // Since it's http, we need to return something. We return [] because jsonrpc
            // clients will parse it as an empty batch response.
            serde_json::json!([])
        }
    };

    Ok(create_response(
        state,
        StatusCode::OK,
        mime::APPLICATION_JSON,
        res.to_string()
    ))
}

/*pub fn trace_state(state: &State) {
    let method = Method::borrow_from(state);
    let uri = Uri::borrow_from(state);
    let http_version = Version::borrow_from(state);
    let headers = HeaderMap::borrow_from(state);
    trace!(
        "REQUEST Method: {} URI: {} HTTP Version: {:?} Headers: {:?}",
        method,
        uri,
        http_version,
        headers
    );
}*/

pub fn trace_state_and_body(state: &State, body: &Chunk) {
    let method = Method::borrow_from(state);
    let uri = Uri::borrow_from(state);
    let http_version = Version::borrow_from(state);
    let headers = HeaderMap::borrow_from(state);
    let body = String::from_utf8(body.to_vec()).unwrap();
    println!(
        "REQUEST Method: {} URI: {} HTTP Version: {:?} Headers: {:?} Body: {}",
        method,
        uri,
        http_version,
        headers,
        body
    );
}

/*pub fn trace_create_response(
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
}*/
