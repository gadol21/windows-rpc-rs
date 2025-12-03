use windows::core::GUID;
use windows_rpc::{BaseType, Interface, Method, Parameter, compile_client};

use windows::Win32::System::Rpc::RPC_CLIENT_INTERFACE;

trait Hello {
    const IFSPEC: RPC_CLIENT_INTERFACE;
}

fn main() {
    println!(
        "{}",
        prettyplease::unparse(
            &syn::parse2(compile_client(Interface {
                name: "Hello".to_string(),
                uuid: GUID::from_u128(0x7a98c250_6808_11cf_b73b_00aa00b677a7),
                methods: vec![
                    Method {
                        return_type: None,
                        name: "WriteSomething".to_string(),
                        parameters: vec![]
                    } // Method {
                      //     return_type: Some(windows_rpc::Type::Simple(BaseType::U32)),
                      //     name: "WriteSomething".to_string(),
                      //     parameters: vec![]
                      // },
                      // Method {
                      //     return_type: None,
                      //     name: "SaySomething".to_string(),
                      //     parameters: vec![
                      //         Parameter {
                      //             r#type: windows_rpc::Type::String,
                      //             name: "something".to_string(),
                      //             is_in: true,
                      //             is_out: false,
                      //         },
                      //         Parameter {
                      //             r#type: windows_rpc::Type::String,
                      //             name: "something_else".to_string(),
                      //             is_in: true,
                      //             is_out: false,
                      //         }
                      //     ]
                      // }
                ],
                ..Default::default()
            }))
            .unwrap(),
        )
    );
}
