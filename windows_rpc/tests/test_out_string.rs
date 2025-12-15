use windows_rpc::rpc_interface;
use windows_rpc::{ProtocolSequence, client_binding::ClientBinding};

#[rpc_interface(guid(0x12345678_1234_1234_1234_123456789abc), version(1.0))]
trait TestRpc {
    fn return_string(param: &str) -> String;
}

struct TestRpcImpl;
impl TestRpcServerImpl for TestRpcImpl {
    fn return_string(param: &str) -> String {
        format!("Got {param}")
    }
}

#[test]
fn test_client_server_integration() {
    let endpoint = "test_endpoint_out_string";

    // Start server in a background thread
    let mut server = TestRpcServer::<TestRpcImpl>::new();
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
    assert_eq!(
        client.return_string("t e s t"),
        "Got t e s t",
        "return_string() should return 'Got t e s t'"
    );

    server.stop().expect("Failed to stop server");
}
