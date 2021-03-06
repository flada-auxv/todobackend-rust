use std::env;

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
extern crate bodyparser;
extern crate unicase;
use unicase::UniCase;

extern crate rustc_serialize;
use rustc_serialize::json::{self, Json, ToJson};
use std::collections::BTreeMap;

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
    fn setup() -> Pool {
        let config = r2d2::Config::default();
        let url = env::var("DATABASE_URL").unwrap_or_else(|e| panic!("{:?}", e));
        let manager = r2d2_postgres::PostgresConnectionManager::new(url.as_ref(), postgres::SslMode::None).unwrap_or_else(|e| panic!("{}", e));
        r2d2::Pool::new(config, manager).unwrap_or_else(|e| panic!("{}", e))
    }

    fn get_connection(req: &mut iron::request::Request) -> PooledConnection {
        let pool = req.get::<persistent::Read<DbConnectionPool>>().unwrap_or_else(|e| panic!("{:?}", e));
        pool.get().unwrap_or_else(|e| panic!("{}", e))
    }
}

#[derive(Debug, Clone, RustcDecodable)]
struct Todo {
    title: String,
    completed: Option<bool>,
    url: Option<String>,
}
impl Todo {
    fn new(row: postgres::rows::Row) -> Todo {
        Todo {
            title: row.get("title"),
            completed: row.get("completed"),
            url: row.get("url"),
        }
    }
    fn to_json_str(&self) -> String {
        self.to_json().to_string()
    }
}
impl ToJson for Todo {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("title".to_string(), self.title.to_json());
        d.insert("completed".to_string(), self.completed.unwrap_or(false).to_json());
        // clone() …？
        d.insert("url".to_string(), self.url.clone().unwrap_or("".to_string()).to_json());
        Json::Object(d)
    }
}

fn main() {
    let mut router = Router::new();
    router.get("/", |_: &mut Request| {
        Ok(Response::with((status::Ok, "hi")))
    });

    router.get("/todos", |req: &mut Request| {
        let conn = DbConnectionPool::get_connection(req);

        let stmt = conn.prepare("SELECT * FROM todos").unwrap_or_else(|e| panic!("{}", e));
        let rows = stmt.query(&[]).unwrap_or_else(|e| panic!("{}", e));

        let todos = rows.iter().map(|row| Todo::new(row).to_json()).collect::<Vec<_>>();

        Ok(Response::with((status::Ok, json::encode(&todos).unwrap_or_else(|e| panic!("{}", e)))))
    });

    router.get("/todos/:id", |req: &mut Request| {
        let conn = DbConnectionPool::get_connection(req);

        let params = req.extensions.get::<Router>().unwrap();
        let id = params.find("id").unwrap().parse::<i32>().unwrap_or_else(|e| panic!("{}", e));

        let stmt = conn.prepare("SELECT * FROM todos WHERE id = $1").unwrap_or_else(|e| panic!("{}", e));
        let result = stmt.query(&[&id]).unwrap_or_else(|e| panic!("{}", e));
        let row = result.iter().next().unwrap();

        Ok(Response::with((status::Ok, Todo::new(row).to_json_str())))
    });

    router.post("/todos", |req: &mut Request| {
        let conn = DbConnectionPool::get_connection(req);

        match req.get::<bodyparser::Struct<Todo>>() {
            Ok(Some(todo)) => {
                conn.execute(
                    "INSERT INTO todos (title, completed, url) VALUES ($1, $2, $3)",
                    &[&todo.title, &todo.completed.unwrap_or(false), &todo.url]
                ).unwrap_or_else(|e| panic!("{}", e));

                Ok(Response::with((status::Ok, todo.to_json_str())))
            },
            Ok(None) => panic!(""),
            Err(err) => panic!("Error: {:?}", err),
        }
    });

    router.delete("/todos", |req: &mut Request| {
        let conn = DbConnectionPool::get_connection(req);

        conn.execute("TRUNCATE todos", &[]).unwrap_or_else(|e| panic!("{}", e));

        Ok(Response::with((status::Ok)))
    });

    let pool = DbConnectionPool::setup();

    const MAX_BODY_LENGTH: usize = 1024 * 1024 * 10;

    let mut middleware = Chain::new(router);
    middleware.link(persistent::Read::<DbConnectionPool>::both(pool));
    middleware.link_before(persistent::Read::<bodyparser::MaxBodyLength>::one(MAX_BODY_LENGTH));
    middleware.link_after(CorsSupport);

    Iron::new(middleware).http("localhost:3000").unwrap_or_else(|e| panic!("{}", e));
}
