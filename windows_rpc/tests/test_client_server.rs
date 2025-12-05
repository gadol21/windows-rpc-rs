use windows_rpc::client_binding::{ClientBinding, ProtocolSequence};
use windows_rpc::rpc_interface;

#[rpc_interface(guid(0x12345678_1234_1234_1234_123456789abc), version(1.0))]
trait TestRpc {
    fn add(a: i32, b: i32) -> i32;
    fn multiply(x: i32, y: i32) -> i32;
    fn strlen(string: &str) -> u64;
}

struct TestRpcImpl;
impl TestRpcServerImpl for TestRpcImpl {
    fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }

    fn multiply(&self, x: i32, y: i32) -> i32 {
        x * y
    }

    fn strlen(&self, string: &str) -> u64 {
        string.len() as u64
    }
}

#[test]
fn test_client_server_integration() {
    let endpoint = "test_endpoint_12345";

    // Start server in a background thread
    let mut server = TestRpcServer::new(TestRpcImpl);
    server
        .register(&endpoint)
        .expect("Failed to register server");
    server.listen_async().expect("Failed to start listening");

    // Create client and call methods
    let client = TestRpcClient::new(
        ClientBinding::new(ProtocolSequence::Alpc, endpoint)
            .expect("Failed to create client binding"),
    );

    // Test the methods
    assert_eq!(client.add(10, 20), 30, "add(10, 20) should return 30");
    assert_eq!(client.multiply(5, 6), 30, "multiply(5, 6) should return 30");
    assert_eq!(
        client.strlen("hello"),
        "hello".len() as u64,
        "strlen() should return len of param"
    );

    server.stop().expect("Failed to stop server");
}
