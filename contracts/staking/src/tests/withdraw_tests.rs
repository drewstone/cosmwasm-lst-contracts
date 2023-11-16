#[cfg(test)]
mod withdraw_tests {
    use crate::contract::execute;
    use crate::msg::ExecuteMsg;
    use crate::state::{BATCHES, STATE, CONFIG};
    use crate::tests::test_helper::init;
    use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{Addr, Uint128, SubMsg, ReplyOn, CosmosMsg};
    use milky_way::staking::{Batch, LiquidUnstakeRequest};
    use osmosis_std::types::cosmos::bank::v1beta1::MsgSend;
    use osmosis_std::types::cosmos::base::v1beta1::Coin;

    #[test]
    fn withdraw() {
        let mut deps = init();
        let env = mock_env();
        let mut state = STATE.load(&deps.storage).unwrap();

        state.total_liquid_stake_token = Uint128::from(100_000u128);
        STATE.save(&mut deps.storage, &state).unwrap();

        let mut pending_batch: Batch =
            Batch::new(1, Uint128::zero(), env.block.time.seconds() + 10000);
        pending_batch.liquid_unstake_requests.insert(
            "bob".to_string(),
            LiquidUnstakeRequest::new(Addr::unchecked("bob"), Uint128::from(10u128)),
        );
        let res = BATCHES.save(&mut deps.storage, 1, &pending_batch);
        assert!(res.is_ok());

        let pending_batch_2: Batch =
            Batch::new(2, Uint128::zero(), env.block.time.seconds() + 10000);
        let res = BATCHES.save(&mut deps.storage, 2, &pending_batch_2);
        assert!(res.is_ok());

        // batch not ready
        let msg = ExecuteMsg::Withdraw { batch_id: 1 };
        let info = mock_info("bob", &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
        assert!(res.is_err());

        // batch ready
        pending_batch.status = milky_way::staking::BatchStatus::Received;
        let res = BATCHES.save(&mut deps.storage, 1, &pending_batch);
        assert!(res.is_ok());

        // no request in batch
        let msg = ExecuteMsg::Withdraw { batch_id: 2 };
        let info = mock_info("bob", &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
        assert!(res.is_err());

        let msg = ExecuteMsg::Withdraw { batch_id: 1 };
        let info = mock_info("alice", &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
        assert!(res.is_err());

        // success
        let msg = ExecuteMsg::Withdraw {
            batch_id: pending_batch.id,
        };
        let info = mock_info("bob", &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg.clone());
        assert!(res.is_ok());
        let messages = res.unwrap().messages;
        assert!(messages.len() == 1); // send message as sub message
                                                   // TODO
                                                   // let msg: MsgSend = res.unwrap().messages.get(0).unwrap().into();
                                                   // assert!(
                                                   //     msg.amount = [Coin {
                                                   //         amount: Uint128::from(100u128),
                                                   //         denom: config.native_token_denom.clone(),
                                                   //     }]
                                                   // );
                                                   // assert!(msg.to_address = "bob");

        let config = CONFIG.load(&deps.storage).unwrap();
        let coin = Coin {
            denom: config.native_token_denom.clone(),
            amount: "10".to_string(),
        };

        let mut coins = Vec::new();
        coins.push(coin);                  
        assert_eq!(messages[0], SubMsg {
            id: 0,
            msg: <MsgSend as Into<CosmosMsg>>::into(MsgSend {
                from_address: Addr::unchecked(MOCK_CONTRACT_ADDR).to_string(),
                to_address: "bob".to_string(),
                amount: coins
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never,
        });                                            
    }
}
