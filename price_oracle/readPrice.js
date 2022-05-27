const config = require('config')
const exchangeHelper = require('./exchange')

function capitalize(string) {
    return string[0].toUpperCase() + string.slice(1);
}
let sleep = (time) => new Promise((resolve) => setTimeout(resolve, time))

async function readPrices() {
    let tokenMap = config.tokens.map(async token => {
        let exchangeMap = config.exchanges.map(async exchange => {
            let func = 'getPriceFrom' + capitalize(exchange)
            return await exchangeHelper[func](token)
        })
        let data = await Promise.all(exchangeMap)
        return {token: token, price: data}
    })
    let result = await Promise.all(tokenMap)
    let priceMap = {}
    result.forEach(item => {
        let price = (item.price.reduce((a, b) => a + b, 0)) / item.price.filter(x => x !== null).length
        price = Math.floor(price*100000000)
        priceMap[item.token.toLowerCase()] = price
    })
    return priceMap
}

module.exports = {
    readPrices
}