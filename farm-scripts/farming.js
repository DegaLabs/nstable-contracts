const nearAPI = require("near-api-js");
const { connect, transactions } = nearAPI;

// creates keyStore from a provided file
// you will need to pass the location of the .json key pair

const { KeyPair, keyStores } = require("near-api-js");
const fs = require("fs");
const { off } = require("process");
const homedir = require("os").homedir();

const ACCOUNT_ID = "v2.dongvc.testnet";
const ACCOUNT_ID2 = "dongvc.testnet";  // NEAR account tied to the keyPair
const NETWORK_ID = "testnet";
// path to your custom keyPair location (ex. function access key for example account)
const KEY_PATH_farm_token = '/.near-credentials/testnet/v2.dongvc.testnet.json';
const KEY_PATH_farm_token2 = '/.near-credentials/testnet/v2.dongvc.testnet.json';


const credentials = JSON.parse(fs.readFileSync(homedir + KEY_PATH_farm_token));
const credentials2 = JSON.parse(fs.readFileSync(homedir + KEY_PATH_farm_token2));

const keyStore = new keyStores.InMemoryKeyStore();
keyStore.setKey(NETWORK_ID, ACCOUNT_ID, KeyPair.fromString(credentials.private_key));
keyStore.setKey(NETWORK_ID, ACCOUNT_ID2, KeyPair.fromString(credentials2.private_key));

console.log(keyStore)
//console.log('Keystores -----     ', keyStores)


const config = {
    networkId: "testnet",
    keyStore,
    nodeUrl: "https://rpc.testnet.near.org",
    walletUrl: "https://wallet.testnet.near.org",
    helperUrl: "https://helper.testnet.near.org",
    explorerUrl: "https://explorer.testnet.near.org",
};



async function create_farm() {
    console.log("A")
    const near = await connect(config);
    console.log("B")
    const account = await near.account("v2.dongvc.testnet");
    const account2 = await near.account("dongvc.testnet");



    const contract = new nearAPI.Contract(
        account, // the account object that is connecting
        "farm_token.dongvc.testnet",
        {
            // name of contract you're connecting to
            viewMethods: ["ft_balance_of"], // view methods do not change state but usually return a value
            // changeMethods: ["addMessage"], // change methods modify state
            sender: account, // account object to initialize and sign transactions.
        }
    );

    const balance = await contract.ft_balance_of({ "account_id": ACCOUNT_ID });
    console.log('Balance :  ------ ', balance)
    const balance2 = await contract.ft_balance_of({ "account_id": ACCOUNT_ID2 });
    console.log('Balance :  ------ ', balance2)



    const contract2 = new nearAPI.Contract(
        account, // the account object that is connecting
        "v2.dongvc.testnet",
        {
            // name of contract you're connecting to
            viewMethods: ["get_metadata"], // view methods do not change state but usually return a value
            changeMethods: ["create_simple_farm"], // change methods modify state
            sender: account, // account object to initialize and sign transactions.
        }
    );

    const abc = await contract2.create_simple_farm({ "terms": { "seed_id": "3", 'reward_token': "farm_token.dongvc.testnet", "start_at": 5, "reward_per_session": "2", "session_interval": 3000} , gas : 0.1 , amount: 3000})
    const metadata = await contract2.get_metadata()
    console.log(metadata)

    await contract2.create_simple_farm({ "terms": { "seed_id": "4", 'reward_token': "farm_token.dongvc.testnet", "start_at": 5, "reward_per_session": "2", "session_interval": 3000} })
    //const metadata = await contract2.get_metadata()
    console.log(await contract2.get_metadata())


}
create_farm()


