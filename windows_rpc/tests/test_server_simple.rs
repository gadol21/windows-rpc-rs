use windows_rpc_macros::rpc_interface;

#[rpc_interface(guid(0x12345678_1234_1234_1234_123456789abc), version(1.0))]
trait SimpleRpc {
    fn add(a: i32, b: i32) -> i32;
}

struct SimpleRpcImpl;

impl SimpleRpcServerImpl for SimpleRpcImpl {
    fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }
}

#[test]
fn test_server_creation() {
    // Just test that we can create the server without crashing
    let _server = SimpleRpcServer::new(SimpleRpcImpl);
    println!("Server created successfully");
}

#[test]
fn test_server_registration() {
    let mut server = SimpleRpcServer::new(SimpleRpcImpl);
    match server.register("test_simple_endpoint") {
        Ok(_) => {
            println!("Server registered successfully");
            server.stop().ok();
        }
        Err(e) => {
            println!("Failed to register server: {:?}", e);
            panic!("Registration failed: {:?}", e);
        }
    }
}
