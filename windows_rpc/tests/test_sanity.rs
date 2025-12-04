use windows_rpc::client_binding::ProtocolSequence;
use windows_rpc_macros::rpc_interface;

#[rpc_interface(guid(0x7a98c250_6808_11cf_b73b_00aa00b677a7), version(0.0))]
trait TestRpcIface {
    fn NoParams() -> u64;
    fn SingleParamReturn(param: i32) -> i32;
    fn Sum(a: i32, b: i32) -> i32;
}

#[test]
fn test() {
    let client = TestRpcIface::new(ClientBinding::new(ProtocolSequence::Alpc, "foobar").unwrap());

    assert_eq!(client.NoParams(), 0xffffffff + 1);
    assert_eq!(client.SingleParamReturn(10), 14);
    assert_eq!(client.Sum(10, 20), 30);
}
