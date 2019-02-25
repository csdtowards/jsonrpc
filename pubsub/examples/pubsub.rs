use std::{time, thread};
use std::sync::{Arc, atomic};

use jsonrpc_core::*;
use jsonrpc_pubsub::{PubSubHandler, Session, Subscriber, SubscriptionId};
use jsonrpc_tcp_server::{ServerBuilder, RequestContext};

use jsonrpc_core::futures::Future;

/// To test the server:
///
/// ```bash
/// $ netcat localhost 3030 -
/// {"id":1,"jsonrpc":"2.0","method":"hello_subscribe","params":[10]}
///
/// ```
fn main() {
	let mut io = PubSubHandler::new(MetaIoHandler::default());
	io.add_method("say_hello", |_params: Params| {
		Ok(Value::String("hello".to_string()))
	});

	let is_done = Arc::new(atomic::AtomicBool::default());
	let is_done2 = is_done.clone();
	io.add_subscription(
		"hello",
		("subscribe_hello", move |params: Params, _, subscriber: Subscriber| {
			if params != Params::None {
				subscriber.reject(Error {
					code: ErrorCode::ParseError,
					message: "Invalid parameters. Subscription rejected.".into(),
					data: None,
				}).unwrap();
				return;
			}

			let sink = subscriber.assign_id(SubscriptionId::Number(5)).unwrap();
			// or subscriber.reject(Error {} );
			// or drop(subscriber)
			let is_done = is_done.clone();
			thread::spawn(move || {
				loop {
					if is_done.load(atomic::Ordering::AcqRel) {
						return;
					}

					thread::sleep(time::Duration::from_millis(100));
					if sink.notify(Params::Array(vec![Value::Number(10.into())])).wait().is_err() {
						println!("Subscription has ended, finishing.");
						break;
					}
				}
			});
		}),
		("remove_hello", move |_id: SubscriptionId, _| {
			println!("Closing subscription");
			is_done2.store(true, atomic::Ordering::AcqRel);
			futures::future::ok(Value::Bool(true))
		}),
	);

	let server = ServerBuilder::new(io)
		.session_meta_extractor(|context: &RequestContext| Some(Arc::new(Session::new(context.sender.clone()))))
		.start(&"127.0.0.1:3030".parse().unwrap())
		.expect("Unable to start RPC server");

	server.wait();
}

