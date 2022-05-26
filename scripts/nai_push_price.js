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
    await account.signAndSendTransaction({
        receiverId: subAcc("naivaultv6"),
        actions: [
            transactions.functionCall("push_price_data", Buffer.from(JSON.stringify({price_data: { recency_duration_sec: 0, timestamp: "0", prices: [{asset_id: "usdc.fakes.testnet", price: { multiplier: "100000000", decimals: 8 }}, {asset_id: "wrap.testnet", price: { multiplier: "550000000", decimals: 8 }} ]}})), 100000000000000, "1")
        ],
    });

    console.log(result);
}