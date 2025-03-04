use crate::contract::{execute, reply, sudo, IBC_TIMEOUT};
use crate::msg::{ExecuteMsg, IBCLifecycleComplete, SudoMsg};
use crate::query::query_ibc_queue;
use crate::state::{ibc, IbcWaitingForReply, IBC_WAITING_FOR_REPLY, INFLIGHT_PACKETS};
use crate::tests::test_helper::{init, CELESTIA1, CHANNEL_ID, NATIVE_TOKEN, OSMO1, OSMO3};
use cosmwasm_std::testing::{message_info, mock_env};
use cosmwasm_std::{
    attr, coins, Addr, Binary, CosmosMsg, IbcTimeout, MsgResponse, Reply, ReplyOn, SubMsg,
    SubMsgResponse, SubMsgResult, Timestamp,
};
use osmosis_std::types::cosmos::base::v1beta1::Coin;
use osmosis_std::types::ibc::applications::transfer::v1::MsgTransfer;
use osmosis_std::types::ibc::applications::transfer::v1::MsgTransferResponse;
use std::vec::Vec;

#[test]
fn success_ibc_queue() {
    let mut deps = init();
    let env = mock_env();
    let info = message_info(&Addr::unchecked(OSMO3), &coins(1000, NATIVE_TOKEN));
    let msg = ExecuteMsg::LiquidStake {
        mint_to: None,
        expected_mint_amount: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone());

    let timeout = IbcTimeout::with_timestamp(Timestamp::from_nanos(
        env.block.time.nanos() + IBC_TIMEOUT.nanos(),
    ));

    let ibc_coin = Coin {
        denom: NATIVE_TOKEN.to_string(),
        amount: "1000".to_string(),
    };

    let ibc_sub_msg_id = env.block.time.nanos() + env.transaction.unwrap().index as u64;
    match res {
        Ok(ref result) => {
            assert_eq!(
                result.attributes,
                vec![
                    attr("action", "liquid_stake"),
                    attr("sender", OSMO3),
                    attr("in_amount", "1000"),
                    attr("mint_amount", "1000"),
                ]
            );
            assert_eq!(
                result.messages[2],
                SubMsg {
                    payload: Binary::new(vec![]),
                    id: ibc_sub_msg_id,
                    msg: <MsgTransfer as Into<CosmosMsg>>::into(MsgTransfer {
                        source_channel: CHANNEL_ID.to_string(),
                        source_port: "transfer".to_string(),
                        sender: env.contract.address.to_string(),
                        receiver: Addr::unchecked(CELESTIA1).to_string(),
                        token: Some(ibc_coin),
                        timeout_height: None,
                        timeout_timestamp: timeout.timestamp().unwrap().nanos(),
                        memo: format!("{{\"ibc_callback\":\"{}\"}}", env.contract.address),
                    }),
                    gas_limit: None,
                    reply_on: ReplyOn::Always,
                }
            );
        }
        Err(e) => {
            panic!("Unexpected error: {:?}", e);
        }
    }
    let sequence = 1;

    let ibc_waiting_for_reply = IBC_WAITING_FOR_REPLY
        .may_load(&deps.storage, ibc_sub_msg_id)
        .unwrap();
    println!("test {:?}", ibc_waiting_for_reply);
    assert_eq!(
        ibc_waiting_for_reply,
        Some(IbcWaitingForReply { amount: 1000 })
    );

    let inflight_packet = INFLIGHT_PACKETS.may_load(&deps.storage, sequence).unwrap();
    assert_eq!(Some(inflight_packet), Some(None));

    let res = query_ibc_queue(deps.as_ref(), None, None);
    assert!(res.unwrap().ibc_queue.is_empty());

    // Reply
    let _result = reply(
        deps.as_mut(),
        mock_env(),
        Reply {
            gas_used: 0,
            payload: Binary::new(vec![]),
            id: ibc_sub_msg_id,
            result: SubMsgResult::Ok(SubMsgResponse {
                data: Some(Binary::from(MsgTransferResponse { sequence })),
                events: Vec::new(), // No events
                msg_responses: vec![MsgResponse {
                    type_url: MsgTransferResponse::TYPE_URL.to_string(),
                    value: Binary::from(MsgTransferResponse { sequence }),
                }],
            }),
        },
    );

    // Check the status
    let ibc_waiting_for_reply = IBC_WAITING_FOR_REPLY
        .may_load(&deps.storage, ibc_sub_msg_id)
        .unwrap();
    println!("test {:?}", ibc_waiting_for_reply);
    assert_eq!(Some(ibc_waiting_for_reply), Some(None));

    let inflight_packet = INFLIGHT_PACKETS.may_load(&deps.storage, sequence).unwrap();
    assert_eq!(
        inflight_packet,
        Some(ibc::IBCTransfer {
            sequence,
            amount: 1000,
            status: ibc::PacketLifecycleStatus::Sent
        })
    );

    let res = query_ibc_queue(deps.as_ref(), None, None);
    assert!(res.unwrap().ibc_queue.len() == 1);

    // send recover message
    let msg = ExecuteMsg::RecoverPendingIbcTransfers {
        paginated: None,
        selected_packets: None,
    };
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg);

    // still the same
    let inflight_packet = INFLIGHT_PACKETS.may_load(&deps.storage, sequence).unwrap();
    assert_eq!(
        inflight_packet,
        Some(ibc::IBCTransfer {
            sequence,
            amount: 1000,
            status: ibc::PacketLifecycleStatus::Sent
        })
    );

    let res = query_ibc_queue(deps.as_ref(), None, None);
    assert!(res.unwrap().ibc_queue.len() == 1);

    let _result = sudo(
        deps.as_mut(),
        mock_env(),
        SudoMsg::IBCLifecycleComplete(IBCLifecycleComplete::IBCAck {
            channel: CHANNEL_ID.to_string(),
            sequence,
            ack: "ack".to_string(),
            success: true,
        }),
    );

    let inflight_packet = INFLIGHT_PACKETS.may_load(&deps.storage, sequence).unwrap();
    assert_eq!(Some(inflight_packet), Some(None));

    let res = query_ibc_queue(deps.as_ref(), None, None);
    assert!(res.unwrap().ibc_queue.is_empty());
}

#[test]
fn fail_ibc_queue() {
    let mut deps = init();
    let env = mock_env();
    let info = message_info(&Addr::unchecked(OSMO3), &coins(1000, NATIVE_TOKEN));
    let msg = ExecuteMsg::LiquidStake {
        mint_to: None,
        expected_mint_amount: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone());
    let ibc_sub_msg_id = env.block.time.nanos() + env.transaction.unwrap().index as u64;
    match res {
        Ok(ref result) => {
            assert_eq!(
                result.attributes,
                vec![
                    attr("action", "liquid_stake"),
                    attr("sender", OSMO3),
                    attr("in_amount", "1000"),
                    attr("mint_amount", "1000"),
                ]
            );
        }
        Err(e) => {
            panic!("Unexpected error: {:?}", e);
        }
    }
    let sequence = 1;

    // Reply
    let _result = reply(
        deps.as_mut(),
        mock_env(),
        Reply {
            id: ibc_sub_msg_id,
            result: SubMsgResult::Ok(SubMsgResponse {
                data: Some(Binary::from(MsgTransferResponse { sequence })),
                events: Vec::new(), // No events
                msg_responses: vec![MsgResponse {
                    type_url: MsgTransferResponse::TYPE_URL.to_string(),
                    value: cosmwasm_std::Binary::from(MsgTransferResponse { sequence }),
                }],
            }),
            gas_used: 0,                              // Add missing gas_used field
            payload: cosmwasm_std::Binary::default(), // Add missing payload field
        },
    );

    let _result = sudo(
        deps.as_mut(),
        mock_env(),
        SudoMsg::IBCLifecycleComplete(IBCLifecycleComplete::IBCAck {
            channel: CHANNEL_ID.to_string(),
            sequence,
            ack: "ack".to_string(),
            success: false,
        }),
    );

    let inflight_packet = INFLIGHT_PACKETS.may_load(&deps.storage, sequence).unwrap();
    assert_eq!(
        inflight_packet,
        Some(ibc::IBCTransfer {
            sequence,
            amount: 1000,
            status: ibc::PacketLifecycleStatus::AckFailure
        })
    );

    let res = query_ibc_queue(deps.as_ref(), None, None);
    assert!(res.unwrap().ibc_queue.len() == 1);

    // send recover message
    let msg = ExecuteMsg::RecoverPendingIbcTransfers {
        paginated: None,
        selected_packets: None,
    };
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg);

    let inflight_packet = INFLIGHT_PACKETS.may_load(&deps.storage, sequence).unwrap();
    assert_eq!(Some(inflight_packet), Some(None));

    let res = query_ibc_queue(deps.as_ref(), None, None);
    assert!(res.unwrap().ibc_queue.is_empty());
}

#[test]
fn timeout_ibc_queue() {
    let mut deps = init();
    let env = mock_env();
    let info = message_info(&Addr::unchecked(OSMO3), &coins(1000, NATIVE_TOKEN));
    let msg = ExecuteMsg::LiquidStake {
        mint_to: None,
        expected_mint_amount: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone());
    let ibc_sub_msg_id = env.block.time.nanos() + env.transaction.unwrap().index as u64;
    match res {
        Ok(ref result) => {
            assert_eq!(
                result.attributes,
                vec![
                    attr("action", "liquid_stake"),
                    attr("sender", OSMO3),
                    attr("in_amount", "1000"),
                    attr("mint_amount", "1000"),
                ]
            );
        }
        Err(e) => {
            panic!("Unexpected error: {:?}", e);
        }
    }
    let sequence = 1;

    // Reply
    let _result = reply(
        deps.as_mut(),
        mock_env(),
        Reply {
            id: ibc_sub_msg_id,
            result: SubMsgResult::Ok(SubMsgResponse {
                data: Some(Binary::from(MsgTransferResponse { sequence })),
                events: Vec::new(), // No events
                msg_responses: vec![MsgResponse {
                    type_url: MsgTransferResponse::TYPE_URL.to_string(),
                    value: cosmwasm_std::Binary::from(MsgTransferResponse { sequence }),
                }],
            }),
            gas_used: 0,
            payload: cosmwasm_std::Binary::default(),
        },
    );

    let _result = sudo(
        deps.as_mut(),
        mock_env(),
        SudoMsg::IBCLifecycleComplete(IBCLifecycleComplete::IBCTimeout {
            channel: CHANNEL_ID.to_string(),
            sequence,
        }),
    );

    let inflight_packet = INFLIGHT_PACKETS.may_load(&deps.storage, sequence).unwrap();
    assert_eq!(
        inflight_packet,
        Some(ibc::IBCTransfer {
            sequence,
            amount: 1000,
            status: ibc::PacketLifecycleStatus::TimedOut
        })
    );

    let res = query_ibc_queue(deps.as_ref(), None, None);
    assert!(res.unwrap().ibc_queue.len() == 1);

    // send recover message
    let msg = ExecuteMsg::RecoverPendingIbcTransfers {
        paginated: None,
        selected_packets: None,
    };
    let _res = execute(deps.as_mut(), mock_env(), info.clone(), msg);

    let inflight_packet = INFLIGHT_PACKETS.may_load(&deps.storage, sequence).unwrap();
    assert_eq!(Some(inflight_packet), Some(None));

    let res = query_ibc_queue(deps.as_ref(), None, None);
    assert!(res.unwrap().ibc_queue.is_empty());
}

#[test]
fn recover_non_paginated() {
    let mut deps = init();
    let info = message_info(&Addr::unchecked(OSMO3), &[]);

    for i in 1..=15 {
        let res = INFLIGHT_PACKETS.save(
            &mut deps.storage,
            i,
            &ibc::IBCTransfer {
                sequence: i,
                amount: 1000,
                status: ibc::PacketLifecycleStatus::AckFailure,
            },
        );
        assert!(res.is_ok());
    }

    // send recover message
    let msg = ExecuteMsg::RecoverPendingIbcTransfers {
        paginated: None,
        selected_packets: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
    assert!(res.is_ok());
    let res = res.unwrap();
    assert_eq!(res.attributes[1], attr("packets", "15"));
}

#[test]
fn recover_paginated() {
    let mut deps = init();
    let info = message_info(&Addr::unchecked(OSMO3), &[]);

    for i in 1..=15 {
        let res = INFLIGHT_PACKETS.save(
            &mut deps.storage,
            i,
            &ibc::IBCTransfer {
                sequence: i,
                amount: 1000,
                status: ibc::PacketLifecycleStatus::AckFailure,
            },
        );
        assert!(res.is_ok());
    }

    // send recover message
    let msg = ExecuteMsg::RecoverPendingIbcTransfers {
        paginated: Some(true),
        selected_packets: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
    assert!(res.is_ok());
    let res = res.unwrap();
    assert_eq!(res.attributes[1], attr("packets", "10"));
}

#[test]
fn recover_forced() {
    let mut deps = init();

    for i in 1..=15 {
        let res = INFLIGHT_PACKETS.save(
            &mut deps.storage,
            i,
            &ibc::IBCTransfer {
                sequence: i,
                amount: 1000,
                status: ibc::PacketLifecycleStatus::Sent,
            },
        );
        assert!(res.is_ok());
    }

    // send recover message
    let msg = ExecuteMsg::RecoverPendingIbcTransfers {
        paginated: Some(true),
        selected_packets: Some(vec![1, 2, 3]),
    };
    let info = message_info(&Addr::unchecked(OSMO1), &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone());
    assert!(res.is_err()); // not an admin

    let info = message_info(&Addr::unchecked(OSMO3), &[]);
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone());
    assert!(res.is_ok());
    let res = res.unwrap();
    assert_eq!(res.attributes[1], attr("packets", "3"));
}

#[test]
fn recover_multiple() {
    let mut deps = init();
    let env = mock_env();
    let info = message_info(&Addr::unchecked(OSMO3), &coins(1000, NATIVE_TOKEN));

    let res = INFLIGHT_PACKETS.save(
        &mut deps.storage,
        1,
        &ibc::IBCTransfer {
            sequence: 1,
            amount: 1000,
            status: ibc::PacketLifecycleStatus::TimedOut,
        },
    );
    assert!(res.is_ok());
    let res = INFLIGHT_PACKETS.save(
        &mut deps.storage,
        2,
        &ibc::IBCTransfer {
            sequence: 2,
            amount: 2000,
            status: ibc::PacketLifecycleStatus::AckFailure,
        },
    );
    assert!(res.is_ok());

    // send recover message
    let msg = ExecuteMsg::RecoverPendingIbcTransfers {
        paginated: None,
        selected_packets: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
    assert!(res.is_ok());
    let res = res.unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::from(MsgTransfer {
            source_channel: CHANNEL_ID.to_string(),
            source_port: "transfer".to_string(),
            sender: env.contract.address.to_string(),
            receiver: Addr::unchecked(CELESTIA1).to_string(),
            token: Some(Coin {
                denom: NATIVE_TOKEN.to_string(),
                amount: "3000".to_string(),
            }),
            timeout_height: None,
            timeout_timestamp: env.block.time.nanos() + IBC_TIMEOUT.nanos(),
            memo: format!("{{\"ibc_callback\":\"{}\"}}", env.contract.address),
        })
    );

    let inflight_packet =
        INFLIGHT_PACKETS.range(&deps.storage, None, None, cosmwasm_std::Order::Ascending);
    assert_eq!(inflight_packet.count(), 0);
}

#[test]
fn recover_recursive() {
    let mut deps = init();
    let info = message_info(&Addr::unchecked(OSMO3), &coins(1000, NATIVE_TOKEN));

    let res = INFLIGHT_PACKETS.save(
        &mut deps.storage,
        1,
        &ibc::IBCTransfer {
            sequence: 1,
            amount: 1000,
            status: ibc::PacketLifecycleStatus::TimedOut,
        },
    );
    assert!(res.is_ok());

    // send recover message
    let msg = ExecuteMsg::RecoverPendingIbcTransfers {
        paginated: None,
        selected_packets: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
    assert!(res.is_ok());

    // Reply
    let _result = reply(
        deps.as_mut(),
        mock_env(),
        Reply {
            id: 2,
            result: SubMsgResult::Ok(SubMsgResponse {
                data: Some(Binary::from(MsgTransferResponse { sequence: 2 })), // No data returned
                events: Vec::new(),                                            // No events
                msg_responses: vec![MsgResponse {
                    type_url: MsgTransferResponse::TYPE_URL.to_string(),
                    value: cosmwasm_std::Binary::from(MsgTransferResponse { sequence: 2 }),
                }],
            }),
            gas_used: 0,
            payload: cosmwasm_std::Binary::default(),
        },
    );

    let inflight_packet =
        INFLIGHT_PACKETS.range(&deps.storage, None, None, cosmwasm_std::Order::Ascending);
    assert_eq!(inflight_packet.count(), 1);

    let _result = sudo(
        deps.as_mut(),
        mock_env(),
        SudoMsg::IBCLifecycleComplete(IBCLifecycleComplete::IBCAck {
            channel: CHANNEL_ID.to_string(),
            sequence: 2,
            ack: "ack".to_string(),
            success: false,
        }),
    );

    // send recover message
    let msg = ExecuteMsg::RecoverPendingIbcTransfers {
        paginated: None,
        selected_packets: None,
    };
    let res = execute(deps.as_mut(), mock_env(), info.clone(), msg);
    assert!(res.is_ok());
    let res = res.unwrap();
    assert_eq!(res.messages.len(), 1);
}
