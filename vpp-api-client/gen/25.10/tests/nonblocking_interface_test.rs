use crate::interface::*;
use crate::interface_types::IfStatusFlags;
use crate::ip_types::*;
use vpp_api_transport::shmem::non_blocking::Client;

async fn new_client() -> Client {
    Client::connect("test_blocking_client", None, 32).await.unwrap()
}

#[tokio::test]
async fn test_vpp_functions() {
    let client = new_client().await;
    let vl_msg_id_res = client.get_message_index(&String::from("control_ping_51077d14"));
    assert_eq!(vl_msg_id_res.is_ok(), true);
}

#[tokio::test]
async fn test_sw_interface_add_del_address() {
    let mut client = new_client().await;

    let create_interface: SwInterfaceAddDelAddressReply = client.send_rcv(
        SwInterfaceAddDelAddress {
            client_index: 0,
            context: 0,
            is_add: true,
            del_all: false,
            sw_if_index: 0,
            prefix: AddressWithPrefix {
                address: Address {
                    af: AddressFamily::ADDRESS_IP4,
                    un: AddressUnion::new_Ip4Address([10, 10, 1, 2]),
                },
                len: 24,
            },
        }
    ).await.unwrap();

    assert_ne!(create_interface.context, 0);
    println!("create_interface {:?}", create_interface);
}

#[tokio::test]
async fn test_sw_interface_set_flags() {
    let mut client = new_client().await;

    let create_interface: SwInterfaceSetFlagsReply = client.send_rcv(
        SwInterfaceSetFlags {
            client_index: 0,
            context: 0,
            sw_if_index: 0,
            flags: vec![
                IfStatusFlags::IF_STATUS_API_FLAG_ADMIN_UP,
                IfStatusFlags::IF_STATUS_API_FLAG_LINK_UP,
            ]
            .try_into()
            .unwrap(),
        }
    ).await.unwrap();

    assert_ne!(create_interface.context, 0);
    println!("SwInterfaceSetFlagsReply {:?}", create_interface);
}

#[tokio::test]
async fn test_sw_interface_set_promisc() {
    let mut client = new_client().await;
    
    let set_promisc_reply: SwInterfaceSetPromiscReply = client.send_rcv(
        SwInterfaceSetPromisc {
            client_index: 0,
            context: 0,
            sw_if_index: 0,
            promisc_on: false,
        }
    ).await.unwrap();

    assert_ne!(set_promisc_reply.context, 0);
    println!("SwInterfaceSetPromiscReply {:?}", set_promisc_reply);
}

#[tokio::test]
async fn test_hw_interface_set_mtu() {
    let mut client = new_client().await;

    let set_mtu_reply: HwInterfaceSetMtuReply = client.send_rcv(
        HwInterfaceSetMtu {
            client_index: 0,
            context: 0,
            sw_if_index: 0,
            mtu: 50,
        }
    ).await.unwrap();

    assert_ne!(set_mtu_reply.context, 0);
    println!("HwInterfaceSetMtuReply {:?}", set_mtu_reply);

}

#[tokio::test]
async fn test_sw_interface_set_mtu() {
    let mut client = new_client().await;

    let set_mtu_reply: SwInterfaceSetMtuReply = client.send_rcv(
        SwInterfaceSetMtu {
            client_index: 0,
            context: 0,
            sw_if_index: 0,
            mtu: vec![1500, 0, 0, 0].try_into().unwrap(),
        }
    ).await.unwrap();

    assert_ne!(set_mtu_reply.context, 0);
    println!("SwInterfaceSetMtuReply {:?}", set_mtu_reply);

}

#[tokio::test]
async fn test_sw_interface_set_ip_directed_broadcast() {
    let mut client = new_client().await;

    let set_ip_directed_broadcast: SwInterfaceSetIpDirectedBroadcastReply = client.send_rcv(
        SwInterfaceSetIpDirectedBroadcast {
            client_index: 0,
            context: 0,
            sw_if_index: 0,
            enable: true,
        }
    ).await.unwrap();

    assert_ne!(set_ip_directed_broadcast.context, 0);
    println!("SwInterfaceSetIpDirectedBroadcastReply {:?}", set_ip_directed_broadcast);

}

#[tokio::test]
async fn test_want_interface_events() {
    let mut client = new_client().await;

    let reply: WantInterfaceEventsReply = client.send_rcv(
        WantInterfaceEvents {
            client_index: 0,
            context: 0,
            enable_disable: 32,
            pid: 32,
        }
    ).await.unwrap();

    assert_ne!(reply.context, 0);
    println!("WantInterfaceEventsReply {:?}", reply);

}

#[tokio::test]
async fn test_sw_interface_address_replace_begin() {
    let mut client = new_client().await;

    let reply: SwInterfaceAddressReplaceBeginReply = client.send_rcv(
        SwInterfaceAddressReplaceBegin {
            client_index: 0,
            context: 0,
        }
    ).await.unwrap();

    assert_ne!(reply.context, 0);
    println!("SwInterfaceAddressReplaceBeginReply {:?}", reply);

}

#[tokio::test]
async fn test_sw_interface_address_replace_end() {
    let mut client = new_client().await;

    let reply: SwInterfaceAddressReplaceEndReply = client.send_rcv(
        SwInterfaceAddressReplaceEnd {
            client_index: 0,
            context: 0,
        }
    ).await.unwrap();

    assert_ne!(reply.context, 0);
    println!("SwInterfaceAddressReplaceEndReply {:?}", reply);

}

#[tokio::test]
async fn test_sw_interface_set_table() {
    let mut client = new_client().await;

    let reply: SwInterfaceSetTableReply = client.send_rcv(
        SwInterfaceSetTable {
            client_index: 0,
            context: 0,
            sw_if_index: 1,
            is_ipv6: false,
            vrf_id: 32,
        }
    ).await.unwrap();
    println!("SwInterfaceSetTableReply {:?}", reply);
    assert_ne!(reply.context, 0);

}

#[tokio::test]
async fn test_sw_interface_get_table() {
    let mut client = new_client().await;
    let reply: SwInterfaceGetTableReply = client.send_rcv(
        SwInterfaceGetTable {
            client_index: 0,
            context: 0,
            sw_if_index: 1,
            is_ipv6: false,
        }
    ).await.unwrap();
    println!("SwInterfaceGetTableReply {:?}", reply);
    assert_ne!(reply.context, 0);

}

#[tokio::test]
async fn test_sw_interface_set_unnumbered() {
    let mut client = new_client().await;
    let reply: SwInterfaceSetUnnumberedReply = client.send_rcv(
        SwInterfaceSetUnnumbered {
            client_index: 0,
            context: 0,
            sw_if_index: 1,
            unnumbered_sw_if_index: 2,
            is_add: false,
        }
    ).await.unwrap();
    println!("SwInterfaceSetUnnumberedReply {:?}", reply);
    assert_ne!(reply.context, 0);

}

#[tokio::test]
async fn test_sw_interface_clear_stats() {
    let mut client = new_client().await;
    let reply: SwInterfaceClearStatsReply = client.send_rcv(
        SwInterfaceClearStats {
            client_index: 0,
            context: 0,
            sw_if_index: 1,
        }
    ).await.unwrap();
    println!("SwInterfaceClearStatsReply {:?}", reply);
    assert_ne!(reply.context, 0);

}

#[tokio::test]
async fn test_sw_interface_tag_add_del() {
    let mut client = new_client().await;
    let reply: SwInterfaceTagAddDelReply = client.send_rcv(
        SwInterfaceTagAddDel {
            client_index: 0,
            context: 0,
            sw_if_index: 1,
            is_add: false,
            tag: "Faisal".try_into().unwrap(),
        }
    ).await.unwrap();
    println!("SwInterfaceTagAddDelReply {:?}", reply);
    assert_ne!(reply.context, 0);

}

#[tokio::test]
async fn test_sw_interface_add_del_mac_address() {
    let mut client = new_client().await;
    let reply: SwInterfaceAddDelMacAddressReply = client.send_rcv(
        SwInterfaceAddDelMacAddress {
            client_index: 0,
            context: 0,
            sw_if_index: 1,
            is_add: 0,
            addr: [0, 0x01, 0x02, 0x03, 0x04, 0x05],
        }
    ).await.unwrap();
    println!("SwInterfaceAddDelMacAddressReply {:?}", reply);
    assert_ne!(reply.context, 0);

}

#[tokio::test]
async fn test_sw_interface_set_mac_address() {
    let mut client = new_client().await;
    let reply: SwInterfaceSetMacAddressReply = client.send_rcv(
        SwInterfaceSetMacAddress {
            client_index: 0,
            context: 0,
            sw_if_index: 1,
            mac_address: [0, 0x01, 0x02, 0x03, 0x04, 0x05],
        }
    ).await.unwrap();
    println!("SwInterfaceSetMacAddressReply {:?}", reply);
    assert_ne!(reply.context, 0);

}

#[tokio::test]
async fn test_sw_interface_get_mac_address() {
    let mut client = new_client().await;
    let reply: SwInterfaceGetMacAddressReply = client.send_rcv(
        SwInterfaceGetMacAddress {
            client_index: 0,
            context: 0,
            sw_if_index: 1,
        }
    ).await.unwrap();
    println!("SwInterfaceGetMacAddressReply {:?}", reply);
    assert_ne!(reply.context, 0);

}
