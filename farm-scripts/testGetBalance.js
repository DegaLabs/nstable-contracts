const nearAPI = require("near-api-js");
const { connect, transactions } = nearAPI;
const { utils } = nearAPI;
const amountInYocto = utils.format.parseNearAmount("1");


// creates keyStore from a provided file
// you will need to pass the location of the .json key pair

const { KeyPair, keyStores } = require("near-api-js");
const fs = require("fs");
const { off } = require("process");
const { SSL_OP_COOKIE_EXCHANGE } = require("constants");
const homedir = require("os").homedir();

const ACCOUNT_ID = "farm_token.dongvc.testnet";  // NEAR account tied to the keyPair
const ACCOUNT_ID2 = "exchange.dongvc.testnet";  // NEAR account tied to the keyPair
const NETWORK_ID = "testnet";
// path to your custom keyPair location (ex. function access key for example account)
const KEY_PATH_farm_token = '/.near-credentials/testnet/farm_token.dongvc.testnet.json';
const KEY_PATH_farm_token2 = '/.near-credentials/testnet/exchange.dongvc.testnet.json';


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



async function loadAcc1() {

    //CHECK BALANCE
    const near = await connect(config);
    const account = await near.account("farm_token.dongvc.testnet");
    console.log(await account.getAccountBalance());
    
    //CHECK BALANCE
    const near1 = await connect(config);
    const account1 = await near1.account("exchange.dongvc.testnet");
    console.log(await account1.getAccountBalance());

    const contract = new nearAPI.Contract(
        account, // the account object that is connecting
        "farm_token.dongvc.testnet",
        {
            // name of contract you're connecting to
            viewMethods: ["ft_balance_of", "storage_balance_bounds"], // view methods do not change state but usually return a value
            changeMethods: ["storage_deposit", "ft_transfer", "ft_transfer_call"], // change methods modify state
            sender: account, // account object to initialize and sign transactions.
        }
    );



    const balance = await contract.ft_balance_of({"account_id" : "dongvc.testnet"});
    console.log('Balance :  ------ ', balance)
    console.log('Balance :  ------ ', await contract.ft_balance_of({"account_id" : ACCOUNT_ID}))
    
    
  //  await transactions.functionCall("storage_deposit", Buffer.from(JSON.stringify({ "account_id": contract, "registration_only": false})), 0.01, "0.000000000000000000000001")
   // await transactions.functionCall("ft_transfer", Buffer.from(JSON.stringify({ "receiver_id": "exchange.dongvc.testnet", "amount" : "80000000000000"})), 2)
    
       console.log( await contract.storage_balance_bounds())

        await contract.ft_transfer( {
            args : {"receiver_id" : "exchange.dongvc.testnet" , "amount" : "100000"},
            amount : 1

        })
        console.log("Start deposit")

        await contract.storage_deposit({
            args : {"account_id" : "test-token.dongvc.testnet" , "registration_only" : false},
            amount : "1250000000000000000000"
        })

        console.log("Finish deposit")

        // await contract.ft_transfer_call({
        //     args : {"receiver_id" : "test-token.dongvc.testnet" , "amount" : "80000", "msg" : ""},
        //     amount : 1
        // })



   //await contract.ft_transfer({"receiver_id" : "exchange.dongvc.testnet" , "amount" : "60000000000000", "msg": ""}, "300000000000000",  amountInYocto)
    console.log(" Good Job")
    // await contract.ft_transfer({"receiver_id" : "exchange.dongvc.testnet" , "amount" : "60000000000000" }, "300000000000000", "1000000000000000000000000" )
    // console.log(B)
    //await contract.storage_deposit({"account_id" : "exchange.dongvc.testnet" , "registration_only" : false, amount : 0.01})
    console.log('Balance :  ------ ', await contract.ft_balance_of({"account_id" : "exchange.dongvc.testnet"}))


}
loadAcc1()


