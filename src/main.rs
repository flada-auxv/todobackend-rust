extern crate iron;
use iron::prelude::*;
use iron::status;
use iron::headers;
use iron::method::{Options, Get, Post, Delete};
use iron::AfterMiddleware;
use iron::typemap::Key;
extern crate router;
use router::Router;
extern crate persistent;
extern crate unicase;
use unicase::UniCase;

extern crate rustc_serialize;
use rustc_serialize::json;

extern crate postgres;
extern crate r2d2;
extern crate r2d2_postgres;

struct CorsSupport;
impl AfterMiddleware for CorsSupport {
    fn after(&self, _: &mut Request, mut res: Response) -> IronResult<Response> {
        res.headers.set(headers::AccessControlAllowOrigin::Any);
        res.headers.set(headers::AccessControlAllowHeaders(vec![UniCase("content-type".to_owned()), UniCase("accept".to_owned())]));
        res.headers.set(headers::AccessControlAllowMethods(vec![Options, Get, Post, Delete]));
        Ok(res)
    }
}

type Pool = r2d2::Pool<r2d2_postgres::PostgresConnectionManager>;
type PooledConnection = r2d2::PooledConnection<r2d2_postgres::PostgresConnectionManager>;
struct DbConnectionPool;
impl Key for DbConnectionPool {
    type Value = Pool;
}
impl DbConnectionPool {
    fn get_connection(req: &mut iron::request::Request) -> PooledConnection {
        let pool = req.get::<persistent::Read<DbConnectionPool>>().unwrap();
        pool.get().unwrap()
    }
}

#[derive(RustcDecodable, RustcEncodable)]
struct Todo {
    title: String,
}
impl Todo {
    fn new(row: postgres::rows::Row) -> Todo {
        Todo { title: row.get("title") }
    }
}

fn main() {
    let mut router = Router::new();
    router.get("/", |_: &mut Request| {
        Ok(Response::with((status::Ok, "hi")))
    });

    router.get("/todos", |req: &mut Request| {
        let conn = DbConnectionPool::get_connection(req);

        let stmt = conn.prepare("SELECT id, title FROM todos").unwrap();
        let rows = stmt.query(&[]).unwrap();

        let todos = rows.iter().map(|row| Todo::new(row)).collect::<Vec<_>>();

        Ok(Response::with((status::Ok, json::encode(&todos).unwrap())))
    });

    router.get("/todos/:id", |req: &mut Request| {
        let conn = DbConnectionPool::get_connection(req);

        let params = req.extensions.get::<Router>().unwrap();
        let id = params.find("id").unwrap().parse::<i32>().unwrap();

        let stmt = conn.prepare("SELECT id, title FROM todos WHERE id = $1").unwrap();
        let result = stmt.query(&[&id]).unwrap();
        let row = result.iter().next().unwrap();

        Ok(Response::with((status::Ok, json::encode(&Todo::new(row)).unwrap())))
    });

    router.post("/", |_: &mut Request| {
        let todo = Todo { title: "a todo".to_string() };
        let encoded = json::encode(&todo).unwrap();

        Ok(Response::with((status::Ok, encoded)))
    });

    router.delete("/", |_: &mut Request| {
        Ok(Response::with(status::Ok))
    });

    let config = r2d2::Config::default();
    let manager = r2d2_postgres::PostgresConnectionManager::new("postgres://flada@localhost/todobackend-rust", postgres::SslMode::None).unwrap();
    let pool = r2d2::Pool::new(config, manager).unwrap();

    let mut middleware = Chain::new(router);
    middleware.link(persistent::Read::<DbConnectionPool>::both(pool));
    middleware.link_after(CorsSupport);

    Iron::new(middleware).http("localhost:3000").unwrap();
}
