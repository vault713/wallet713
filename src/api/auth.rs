use futures::future;
use gotham::handler::HandlerFuture;
use gotham::helpers::http::response::create_empty_response;
use gotham::middleware::{Middleware, NewMiddleware};
use gotham::state::{FromState, State};
use grin_util::to_base64;
use hyper::header::{HeaderMap, AUTHORIZATION};
use hyper::StatusCode;
use ring::constant_time::verify_slices_are_equal;
use std::io;

pub struct BasicAuthMiddleware {
    api_basic_auth: Option<String>,
}

impl BasicAuthMiddleware {
    pub fn new(api_basic_auth: Option<String>) -> Self {
        Self {
            api_basic_auth: api_basic_auth
                .map(|x| String::from("Basic ") + &to_base64(&(String::from("grin:") + &x))),
        }
    }
}

impl Middleware for BasicAuthMiddleware {
    fn call<C>(self, state: State, chain: C) -> Box<HandlerFuture>
    where
        C: FnOnce(State) -> Box<HandlerFuture>,
    {
        if self.api_basic_auth.is_none() {
            return chain(state);
        }

        let auth: Option<String> = HeaderMap::borrow_from(&state)
            .get(AUTHORIZATION)
            .map(|x| x.to_str().unwrap().to_string());

        if auth
            .map(|x| {
                verify_slices_are_equal(self.api_basic_auth.unwrap().as_bytes(), x.as_bytes())
                    .is_ok()
            })
            .unwrap_or(false)
        {
            chain(state)
        } else {
            let res = create_empty_response(&state, StatusCode::UNAUTHORIZED);
            Box::new(future::ok((state, res)))
        }
    }
}

impl NewMiddleware for BasicAuthMiddleware {
    type Instance = BasicAuthMiddleware;

    fn new_middleware(&self) -> io::Result<Self::Instance> {
        Ok(BasicAuthMiddleware {
            api_basic_auth: self.api_basic_auth.clone(),
        })
    }
}
