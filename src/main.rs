extern crate iron;
extern crate router;
extern crate unicase;

use iron::prelude::*;
use iron::status;
use iron::headers;
use iron::method::{Options, Get, Post};
use iron::AfterMiddleware;

use unicase::UniCase;

struct CorsSupport;

impl AfterMiddleware for CorsSupport {
    fn after(&self, _: &mut Request, mut res: Response) -> IronResult<Response> {
        res.headers.set(headers::AccessControlAllowOrigin::Any);
        res.headers.set(headers::AccessControlAllowHeaders(vec![UniCase("content-type".to_owned()), UniCase("accept".to_owned())]));
        res.headers.set(headers::AccessControlAllowMethods(vec![Options, Get, Post]));
        Ok(res)
    }
}

fn main() {
    let mut router = router::Router::new();
    router.get("/", |_: &mut Request| {
        let res = Response::with((status::Ok, "Hello World!"));
        Ok(res)
    });

    let mut chain = Chain::new(router);
    chain.link_after(CorsSupport);

    Iron::new(chain).http("localhost:3000").unwrap();
}
