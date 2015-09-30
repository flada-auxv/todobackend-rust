extern crate iron;

use iron::prelude::*;

fn hello_world(_: &mut Request) -> IronResult<Response> {
    Ok(Response::with((iron::status::Ok, "Hello world!!")))
}

fn main() {
    Iron::new(hello_world).http("localhost:3000").unwrap();
}
