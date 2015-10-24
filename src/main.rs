extern crate iron;
extern crate router;
use iron::prelude::*;
use iron::status;
use iron::headers;
use iron::method::{Options, Get, Post, Delete};
use iron::AfterMiddleware;

extern crate unicase;
use unicase::UniCase;

extern crate rustc_serialize;
use rustc_serialize::json;

struct CorsSupport;

impl AfterMiddleware for CorsSupport {
    fn after(&self, _: &mut Request, mut res: Response) -> IronResult<Response> {
        res.headers.set(headers::AccessControlAllowOrigin::Any);
        res.headers.set(headers::AccessControlAllowHeaders(vec![UniCase("content-type".to_owned()), UniCase("accept".to_owned())]));
        res.headers.set(headers::AccessControlAllowMethods(vec![Options, Get, Post, Delete]));
        Ok(res)
    }
}

#[derive(RustcDecodable, RustcEncodable)]
struct Todo {
    title: String,
}

fn main() {
    let mut router = router::Router::new();
    router.get("/", |_: &mut Request| {
        let res = Response::with((status::Ok, "Hello World!"));
        Ok(res)
    });

    router.post("/", |_: &mut Request| {
        let todo = Todo { title: "a todo".to_string() };
        let encoded = json::encode(&todo).unwrap();

        Ok(Response::with((status::Ok, encoded)))
    });

    router.delete("/", |_: &mut Request| {
        Ok(Response::with(status::Ok))
    });

    let mut middleware = Chain::new(router);
    middleware.link_after(CorsSupport);

    Iron::new(middleware).http("localhost:3000").unwrap();
}
