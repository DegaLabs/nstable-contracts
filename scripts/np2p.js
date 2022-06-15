const { connect, transactions, keyStores } = require("near-api-js");
const fs = require("fs");
const path = require("path");
const homedir = require("os").homedir();
const BigNumber = require("bignumber.js")

function toBigNumber(n, pow) {
    let ret = new BigNumber(`1e${pow}`).multipliedBy(`${n}`).toFixed(0)
    return ret
}

const CREDENTIALS_DIR = ".near-credentials";
// NOTE: replace "example" with your accountId
const GOVERNANCE = "deganstable.testnet";
const BORROWER1 = "borrower1.deganstable.testnet";
const LENDER = "deganstable.testnet"
const nP2P = "np2pv3.deganstable.testnet"

const USDC = "usdc.fakes.testnet"
const WETH = "weth.fakes.testnet"
const WBTC = "wbtc.fakes.testnet"
const WNEAR = "wrap.testnet"

let decimals = {}
decimals[USDC] = 6
decimals[WETH] = 18
decimals[WBTC] = 8
decimals[WNEAR] = 24

function getTokens() {
    return [USDC, WETH, WBTC, WNEAR]
}

function getDecimals() {
    return getTokens().map(t => decimals[t])
}

let credentialsPath = path.join(homedir, CREDENTIALS_DIR);
const keyStore = new keyStores.UnencryptedFileSystemKeyStore(credentialsPath);

let mainAccount = "deganstable.testnet"
function subAcc(sub) {
    return sub + "." + mainAccount
}

const config = {
    keyStore,
    networkId: "testnet",
    nodeUrl: "https://rpc.testnet.near.org",
};



sendTransactions();

async function sendTransactions() {
    console.log('getTokens', getTokens())
    console.log('getDecimals', getDecimals())
    console.log('getTokens', getTokens())
    console.log('getDecimals', getDecimals())
    const near = await connect({ ...config, keyStore });
    let account = await near.account(GOVERNANCE);
    // console.log(JSON.stringify({days: 10, action_name: "CreateLock"}) )
    // console.log('adding supported tokens')
    // await account.signAndSendTransaction({
    //     receiverId: nP2P,
    //     actions: [
    //         transactions.functionCall("add_new_supported_tokens", { token_ids: getTokens(), decimals: getDecimals() }, 10000000000000, toBigNumber(1, decimals[WNEAR]))
    //     ],
    // });
    // console.log('here')
    // {
    //     for (t of getTokens()) {
    //         console.log('registering ', t)
    //         await account.signAndSendTransaction({
    //             receiverId: t,
    //             actions: [
    //                 transactions.functionCall("storage_deposit", { account_id: nP2P, registration_only: true }, 100000000000000, toBigNumber(1, decimals[WNEAR]))
    //             ],
    //         });
    //     }
    // }

    // console.log('register for ', GOVERNANCE)
    // await account.signAndSendTransaction({
    //     receiverId: nP2P,
    //     actions: [
    //         transactions.functionCall("storage_deposit", { account_id: GOVERNANCE }, 100000000000000, toBigNumber(1, decimals[WNEAR]))
    //     ],
    // });

    // console.log('register for ', BORROWER1)
    // await account.signAndSendTransaction({
    //     receiverId: nP2P,
    //     actions: [
    //         transactions.functionCall("storage_deposit", { account_id: BORROWER1 }, 100000000000000, toBigNumber(1, decimals[WNEAR]))
    //     ],
    // });

    // console.log('creating pool 0')
    // await account.signAndSendTransaction({
    //     receiverId: nP2P,
    //     actions: [
    //         transactions.functionCall("create_new_pool", { lend_token_id: USDC, collateral_token_id: WNEAR }, 100000000000000, toBigNumber(11, decimals[WNEAR]))
    //     ],
    // });

    // //deposit 
    // console.log('deposit pool 0')
    // await account.signAndSendTransaction({
    //     receiverId: USDC,
    //     actions: [
    //         transactions.functionCall("ft_transfer_call", { receiver_id: nP2P, amount: toBigNumber(1000, decimals[USDC]), msg: JSON.stringify({ pool_id: 0 }) }, 100000000000000, "1")
    //     ],
    // });

    // account = await near.account(BORROWER1);
    // console.log('deposit pool 0', WNEAR)
    // await account.signAndSendTransaction({
    //     receiverId: WNEAR,
    //     actions: [
    //         transactions.functionCall("ft_transfer_call", { receiver_id: nP2P, amount: toBigNumber(300, decimals[WNEAR]), msg: JSON.stringify({ pool_id: 0 }) }, 100000000000000, "1")
    //     ],
    // });

    account = await near.account(BORROWER1);
    console.log('deposit pool 0', WNEAR)
    await account.signAndSendTransaction({
        receiverId: WNEAR,
        actions: [
            transactions.functionCall("ft_transfer_call", { receiver_id: nP2P, amount: toBigNumber(20000, decimals[WNEAR]), msg: JSON.stringify({ pool_id: 0 }) }, 100000000000000, "1")
        ],
    });

    // await account.signAndSendTransaction({
    //     receiverId: "usdc.fakes.testnet",
    //     actions: [
    //         transactions.functionCall("ft_transfer_call", Buffer.from(JSON.stringify({ receiver_id: subAcc("np2pv4"), amount: "5000000000", msg: JSON.stringify({pool_id: 1})})), 100000000000000, "1")
    //         //deposit only
    //         //transactions.functionCall("ft_transfer_call", Buffer.from(JSON.stringify({ receiver_id: subAcc("naivaultv6"), amount: "500000000", msg: ""})), 100000000000000, "1")
    //     ],
    // });
    // console.log('borrowing')
    // await account.signAndSendTransaction({
    //     receiverId: "naistable.deganstable.testnet",
    //     actions: [
    //         transactions.functionCall("borrow", Buffer.from(JSON.stringify({ collateral_token_id: collateral_token_id, borrow_amount: "200000000000000000000" })), 100000000000000, "1")
    //         //deposit only
    //         //transactions.functionCall("ft_transfer_call", Buffer.from(JSON.stringify({ receiver_id: subAcc("naivaultv6"), amount: "500000000", msg: ""})), 100000000000000, "1")
    //     ],
    // });

    console.log(result);
}