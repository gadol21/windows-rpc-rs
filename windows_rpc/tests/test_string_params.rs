use std::time::Duration;
use windows_rpc::client_binding::{ClientBinding, ProtocolSequence};
use windows_rpc_macros::rpc_interface;

#[rpc_interface(guid(0xabcdef12_3456_7890_abcd_ef1234567890), version(1.0))]
trait StringTestRpc {
    fn echo_string(message: &str) -> i32;
    fn get_length(text: &str) -> u32;
}

struct StringTestRpcImpl;

impl StringTestRpcServerImpl for StringTestRpcImpl {
    fn echo_string(&self, message: &str) -> i32 {
        println!("Server received: {}", message);
        if message == "Hello from Rust!" { 42 } else { 0 }
    }

    fn get_length(&self, text: &str) -> u32 {
        text.len() as u32
    }
}

#[test]
fn test_string_parameters() {
    println!("Starting string parameter test...");
    let endpoint = "test_string_endpoint";

    // Start server in a background thread
    let endpoint_clone = endpoint.to_string();
    let server_handle = std::thread::spawn(move || {
        println!("Server: Creating server...");
        let mut server = StringTestRpcServer::new(StringTestRpcImpl);

        println!("Server: Registering...");
        server
            .register(&endpoint_clone)
            .expect("Failed to register server");

        println!("Server: Starting to listen...");
        server.listen_async().expect("Failed to start listening");

        // Keep server alive
        std::thread::sleep(Duration::from_secs(5));

        println!("Server: Stopping...");
        server.stop().expect("Failed to stop server");
    });

    // Give server time to start
    println!("Client: Waiting for server to start...");
    std::thread::sleep(Duration::from_millis(500));

    // Create client and call methods with string parameters
    println!("Client: Creating client...");
    let client = StringTestRpcClient::new(
        ClientBinding::new(ProtocolSequence::Alpc, endpoint)
            .expect("Failed to create client binding"),
    );

    println!("Client: Calling echo_string...");
    let result = client.echo_string("Hello from Rust!");
    assert_eq!(result, 42, "echo_string should return 42");
    println!("Client: echo_string returned {}", result);

    println!("Client: Calling get_length...");
    let length = client.get_length("Test");
    assert_eq!(length, 4, "get_length should return 4");
    println!("Client: get_length returned {}", length);

    // Wait for server thread to finish
    server_handle.join().expect("Server thread panicked");
    println!("Test completed successfully!");
}
