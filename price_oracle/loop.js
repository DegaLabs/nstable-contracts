const exchangeHelper = require('./readPrice')
const nearHelper = require('../helpers/near')
const { transactions } = require("near-api-js");
sleep = async (time) => new Promise((resolve) => setTimeout(resolve, time))
now = () => {
    return Math.floor(Date.now() / 1000)
}
let config = require('config')
let targetContracts = config.targetContracts
let dataValidPeriod = config.feed_period //300 seconds
async function main() {
    while (true) {
        for (const vaultContractID of targetContracts) {
            let account = await nearHelper.connectAccount("pricefeeder.deganstable.testnet")
            let priceData = await account.viewFunction(vaultContractID, "get_price_data");

            let now_time = now();

            if (now_time > parseInt(priceData.timestamp) + priceData.recency_duration_sec) {
                //reading price feed info
                console.log('start', new Date())

                let priceMap = await exchangeHelper.readPrices()
                console.log(priceMap)
                console.log('end', new Date())

                let tokenList = await account.viewFunction(vaultContractID, "get_token_list")
                let nearMap = config.nearMap[config.network]
                let prices = []
                for (const t of tokenList) {
                    let mainTokenName = Object.keys(nearMap).find(key => nearMap[key] === t)
                    if (mainTokenName) {
                        let price = {
                            asset_id: t,
                            price: {
                                multiplier: `${priceMap[mainTokenName]}`,
                                decimals: 8
                            }
                        }
                        prices.push(price)
                    }
                }

                let priceDataToPush = {
                    timestamp: `${now_time}`,
                    recency_duration_sec: dataValidPeriod,
                    prices: prices
                }
                console.log(JSON.stringify(priceDataToPush))

                await account.signAndSendTransaction({
                    receiverId: vaultContractID,
                    actions: [
                        transactions.functionCall("push_price_data", Buffer.from(JSON.stringify({ price_data: priceDataToPush })), 100000000000000, "100000000000000000000000")
                    ],
                });
                console.log('done')
            } else {
                console.log('data still valid')
            }
        }

        await sleep(10 * 1000)
    }

}

main()
