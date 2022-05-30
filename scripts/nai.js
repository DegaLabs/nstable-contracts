const { connect, transactions, keyStores } = require("near-api-js");
const fs = require("fs");
const path = require("path");
const homedir = require("os").homedir();

const CREDENTIALS_DIR = ".near-credentials";
// NOTE: replace "example" with your accountId
const SENDER_ACCOUNT_ID = "deganstable.testnet";

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
    const near = await connect({ ...config, keyStore });
    console.log(config)
    const account = await near.account(SENDER_ACCOUNT_ID);
    //console.log(JSON.stringify({days: 10, action_name: "CreateLock"}) )
    let collateral_token_id = "wrap.testnet"
    console.log('depositing')
    await account.signAndSendTransaction({
        receiverId: collateral_token_id,
        actions: [
            transactions.functionCall("ft_transfer_call", Buffer.from(JSON.stringify({ receiver_id: subAcc("naistable"), amount: "100000000000000000000000000", msg: ""})), 100000000000000, "1")
            //deposit only
            //transactions.functionCall("ft_transfer_call", Buffer.from(JSON.stringify({ receiver_id: subAcc("naivaultv6"), amount: "500000000", msg: ""})), 100000000000000, "1")
        ],
    });
    console.log('borrowing')
    await account.signAndSendTransaction({
        receiverId: "naistable.deganstable.testnet",
        actions: [
            transactions.functionCall("borrow", Buffer.from(JSON.stringify({ collateral_token_id: collateral_token_id, borrow_amount: "200000000000000000000" })), 100000000000000, "1")
            //deposit only
            //transactions.functionCall("ft_transfer_call", Buffer.from(JSON.stringify({ receiver_id: subAcc("naivaultv6"), amount: "500000000", msg: ""})), 100000000000000, "1")
        ],
    });

    console.log(result);
}