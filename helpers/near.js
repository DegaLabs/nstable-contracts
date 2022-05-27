const { connect, transactions, keyStores } = require("near-api-js");
const fs = require("fs");
const path = require("path");
const homedir = require("os").homedir();

const CREDENTIALS_DIR = ".near-credentials";

let credentialsPath = path.join(homedir, CREDENTIALS_DIR);
const keyStore = new keyStores.UnencryptedFileSystemKeyStore(credentialsPath);

const NEAR = {
    getNearModule: async (mainnet) => {
        let config = NEAR.getConfig(mainnet)
        return await connect({ ...config, keyStore });
    },
    connectAccount: async (accountId, mainnet) => {
        let near = await NEAR.getNearModule(mainnet);
        return await near.account(accountId);
    },
    getConfig: (mainnet) => {
        let network = NEAR.getNetwork(mainnet)
        return {
            keyStore,
            networkId: network,
            nodeUrl: `https://rpc.${network}.near.org`,
        };
    },
    getNetwork: (mainnet) => {
        return mainnet ? "mainnet" : "testnet"
    }
}

module.exports = NEAR
