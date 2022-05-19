const { connect, transactions, keyStores } = require("near-api-js");
const fs = require("fs");
const path = require("path");
const homedir = require("os").homedir();

const CREDENTIALS_DIR = ".near-credentials";
// NOTE: replace "example" with your accountId
const CONTRACT_NAME = "venst.deganstable.testnet";
const SENDER_ACCOUNT_ID = "nst.deganstable.testnet";

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
    await account.signAndSendTransaction({
        receiverId: subAcc("nst"),
        actions: [
            transactions.functionCall("ft_transfer_call", Buffer.from(JSON.stringify({ receiver_id: subAcc("venst"), amount: "40000000000", msg: JSON.stringify({days: 20, action_name: "IncreaseAmountAndUnlockTime"}) })), 100000000000000, "1")
        ],
    });

    console.log(result);
}