use failure::Error;
use gotham::helpers::http::response::create_response;
use gotham::middleware::state::StateMiddleware;
use gotham::pipeline::chain::PipelineHandleChain;
use gotham::pipeline::new_pipeline;
use gotham::pipeline::single::single_pipeline;
use gotham::router::builder::*;
use gotham::router::Router;
use gotham::state::{FromState, State};
use hyper::{Body, Chunk, HeaderMap, Method, Response, StatusCode, Uri, Version};
use mime::Mime;
use std::panic::RefUnwindSafe;

use crate::api::auth::BasicAuthMiddleware;
use crate::api::handlers::{foreign, owner};
use crate::broker::{GrinboxPublisher, GrinboxSubscriber, KeybasePublisher, KeybaseSubscriber};
use crate::wallet::types::{Arc, Mutex, MutexGuard};
use crate::wallet::Wallet;
use common::ErrorKind;

#[derive(Clone, StateData)]
pub struct WalletContainer {
    pub wallet: Arc<Mutex<Wallet>>,
    grinbox_publisher: Option<GrinboxPublisher>,
    keybase_publisher: Option<KeybasePublisher>,
}

impl RefUnwindSafe for WalletContainer {}

impl WalletContainer {
    fn new(
        wallet: Arc<Mutex<Wallet>>,
        grinbox_publisher: Option<GrinboxPublisher>,
        keybase_publisher: Option<KeybasePublisher>,
    ) -> Self {
        Self {
            wallet,
            grinbox_publisher,
            keybase_publisher,
        }
    }

    pub fn lock(&self) -> Result<MutexGuard<Wallet>, Error> {
        Ok(self.wallet.lock())
    }

    pub fn grinbox_publisher(&self) -> Result<&GrinboxPublisher, Error> {
        self.grinbox_publisher.as_ref().ok_or_else(|| {
            ErrorKind::GenericError(String::from("missing grinbox publisher")).into()
        })
    }

    pub fn keybase_publisher(&self) -> Result<&KeybasePublisher, Error> {
        self.keybase_publisher.as_ref().ok_or_else(|| {
            ErrorKind::GenericError(String::from("missing keybase publisher")).into()
        })
    }
}

fn build_owner_api<C, P>(route: &mut RouterBuilder<C, P>, owner_api_include_foreign: Option<bool>)
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
}

fn build_foreign_api<C, P>(route: &mut RouterBuilder<C, P>)
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: RefUnwindSafe + Send + Sync + 'static,
{
    route
        .post("/v1/wallet/foreign/receive_tx")
        .to(foreign::receive_tx);

    route
        .post("/v1/wallet/foreign/build_coinbase")
        .to(foreign::build_coinbase);

    route
        .post("/v1/wallet/foreign/receive_invoice")
        .to(foreign::receive_invoice);
}

pub fn build_owner_api_router(
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
}

pub fn build_foreign_api_router(
    wallet: Arc<Mutex<Wallet>>,
    grinbox_broker: Option<(GrinboxPublisher, GrinboxSubscriber)>,
    keybase_broker: Option<(KeybasePublisher, KeybaseSubscriber)>,
    foreign_api_secret: Option<String>,
) -> Router {
    let grinbox_publisher = grinbox_broker.map(|(p, _)| p);
    let keybase_publisher = keybase_broker.map(|(p, _)| p);

    let (chain, pipelines) = single_pipeline(
        new_pipeline()
            .add(BasicAuthMiddleware::new(foreign_api_secret))
            .add(StateMiddleware::new(WalletContainer::new(
                wallet,
                grinbox_publisher,
                keybase_publisher,
            )))
            .build(),
    );

    build_router(chain, pipelines, |route| {
        build_foreign_api(route);
    })
}

pub fn trace_state(state: &State) {
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
}

pub fn trace_state_and_body(state: &State, body: &Chunk) {
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

pub fn trace_create_response(
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
