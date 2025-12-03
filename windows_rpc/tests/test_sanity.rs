use windows_rpc_macros::gen_interface;

gen_interface!();

#[test]
fn test() {
    let client = Hello::new(
        ClientBinding::new(
            windows_rpc::client_binding::ProtocolSequence::Alpc,
            "foobar",
        )
        .unwrap(),
    );

    assert_eq!(client.NoParams(), 0xffffffff + 1);
    assert_eq!(client.SingleParamReturn(10), 14);
    assert_eq!(client.Sum(10, 20), 30);
}
