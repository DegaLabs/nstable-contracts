const axios = require('axios')

let ExchangeHelper = {
    getPriceFromBinance: async (token) => {
        url = `https://api.binance.com/api/v3/ticker/price?symbol=${token.toUpperCase()}USDT`
        // url = 'https://api.binance.com/api/v3/trades?symbol=BTCUSDT'

        let price = await axios.get(url)
        return parseFloat(price.data.price)
    },
    getPriceFromBitfinex: async (token) => {
        url = `https://api-pub.bitfinex.com/v2/tickers?symbols=t${token.toUpperCase()}USD`
        // url = 'https://api-pub.bitfinex.com/v2/tickers?symbols=tBTCUSD,tLTCUSD,fUSD'

        let price = await axios.get(url)
        if (price.data.length === 0) {
            return null
        }
        price = price.data[0]
        let bid = parseFloat(price[1])
        let bidAmount = parseFloat(price[2])
        let ask = parseFloat(price[3])
        let askAmount = parseFloat(price[4])
        return (bid + ask) / 2
    },
    getPriceFromBitmart: async (token) => {
        url = `https://api-cloud.bitmart.com/spot/v1/ticker?symbol=${token.toUpperCase()}_USDT`
        let price = await axios.get(url)
        let tickers = price.data.data
        if (tickers.tickers.length === 0) {
            return null
        }
        return parseFloat(tickers.tickers[0].last_price)
    },
    getPriceFromCoinbase: async (token) => {
        try {
            url = `https://api.coinbase.com/v2/prices/${token.toUpperCase()}-USD/spot`

            let price = await axios.get(url)
            return parseFloat(price.data.data.amount)
        } catch (e) {
            return null
        }
    },
    getPriceFromFtx: async (token) => {
        try {
            url = `https://ftx.com/api/markets/${token.toUpperCase()}/USD`

            let price = await axios.get(url)
            return parseFloat(price.data.result.price)
        } catch (e) {
            return null
        }
    },
    getPriceFromHuobi: async (token) => {
        try {
            url = `https://api.huobi.pro/market/detail/merged?symbol=${token.toLowerCase()}usdt`

            let price = await axios.get(url)
            let bid = price.data.tick.bid[0]
            let ask = price.data.tick.ask[0]
            return (bid + ask) / 2
        } catch (e) {
            return null
        }
    },
    getPriceFromOkex: async (token) => {
        url = `https://aws.okex.com/api/spot/v3/instruments/${token.toUpperCase()}-USDT/book?size=1`
        try {
            let price = await axios.get(url)
            let bid = parseFloat(price.data.bids[0][0])
            let ask = parseFloat(price.data.asks[0][0])
            return (bid + ask) / 2
        } catch (e) {
            return null
        }
    },
}

module.exports = ExchangeHelper
