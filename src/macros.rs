//! Provides a macro for creating an rpc service and client stub.

#[macro_export]
macro_rules! rpc_service { ($server:ident: 
    $( $fn_name:ident( $( $arg:ident : $in_:ty ),* ) -> $out:ty;)*) => {
        #[allow(dead_code)]
        pub mod $server {
            use std::net::ToSocketAddrs;
            use std::io;
            use std::sync::Arc;
            use $crate::protocol::{
                self,
                ServeHandle,
                serve_async,
            };

            #[doc="An RPC error that occurred during serving an RPC request."]
            #[derive(Debug)]
            pub enum Error {
                #[doc="An IO error occurred."]
                Io(io::Error),

                #[doc="An unexpected internal error. Typically a bug in the server impl."]
                InternalError,
            }

            impl ::std::convert::From<protocol::Error> for Error {
                fn from(err: protocol::Error) -> Error {
                    match err {
                        protocol::Error::Io(err) => Error::Io(err), 
                        _ => Error::InternalError,
                    }
                }
            }

            impl ::std::convert::From<io::Error> for Error {
                fn from(err: io::Error) -> Error {
                    Error::Io(err)
                }
            }

            #[doc="The result of an RPC call; either the successful result or the error."]
            pub type Result<T> = ::std::result::Result<T, Error>;

            #[doc="The provided RPC service."]
            pub trait Service: Send + Sync {
                $(
                    fn $fn_name(&self, $($arg:$in_),*) -> $out;
                )*
            }

            #[allow(non_camel_case_types)]
            #[derive(Debug, Serialize, Deserialize)]
            enum Request {
                $(
                    $fn_name($($in_),*),
                )*
            }

            #[allow(non_camel_case_types)]
            #[derive(Debug, Serialize, Deserialize)]
            enum Reply {
                $(
                    $fn_name($out),
                )*
                Impossible,
            }

            #[doc="The client stub that makes RPC calls to the server."]
            pub struct Client(protocol::Client<Request, Reply>);

            impl Client {
                #[doc="Create a new client that connects to the given address."]
                pub fn new<A>(addr: A) -> Result<Self>
                    where A: ToSocketAddrs,
                {
                    let inner = try!(protocol::Client::new(addr));
                    Ok(Client(inner))
                }

                $(
                    pub fn $fn_name(&self, $($arg: $in_),*) -> Result<$out> {
                        let reply = try!((self.0).rpc(&Request::$fn_name($($arg),*)));
                        if let Reply::$fn_name(reply) = reply {
                            Ok(reply)
                        } else {
                            Err(Error::InternalError)
                        }
                    }
                )*
            }

            struct Server<S: 'static + Service>(S);

            impl<S> protocol::Serve<Request, Reply> for Server<S>
                where S: 'static + Service
            {
                fn serve(&self, request: Request) -> Reply {
                    match request {
                        $(
                            Request::$fn_name($($arg),*) =>
                                Reply::$fn_name((self.0).$fn_name($($arg),*)),
                         )*
                    }
                }
            }

            #[doc="Start a running service."]
            pub fn serve<A, S>(addr: A, service: S) -> Result<ServeHandle>
                where A: ToSocketAddrs,
                      S: 'static + Service
            {
                let server = Arc::new(Server(service));
                Ok(try!(serve_async(addr, server)))
            }
        }
    }
}

#[cfg(test)]
mod test {

    rpc_service!(my_server:
        hello(foo: super::Foo) -> super::Foo;

        add(x: i32, y: i32) -> i32;
    );

    use self::my_server::*;

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    pub struct Foo {
        message: String
    }

    impl Service for () {
        fn hello(&self, s: Foo) -> Foo {
            Foo{message: format!("Hello, {}", &s.message)}
        }

        fn add(&self, x: i32, y: i32) -> i32 {
            x + y
        }
    }

    #[test]
    fn simple_test() {
        println!("Starting");
        let addr = "127.0.0.1:9000";
        let shutdown = my_server::serve(addr, ()).unwrap();
        let client = Client::new(addr).unwrap();
        assert_eq!(3, client.add(1, 2).unwrap());
        let foo = Foo{message: "Adam".into()};
        let want = Foo{message: format!("Hello, {}", &foo.message)};
        assert_eq!(want, client.hello(Foo{message: "Adam".into()}).unwrap());
        drop(client);
        shutdown.shutdown();
    }
}
