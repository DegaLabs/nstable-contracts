const { exec } = require('node:child_process');
const fs = require('fs');
const { mainModule } = require('node:process');
const sleep = async (time) => new Promise((resolve) => setTimeout(resolve, time))
let target = "deganstable.testnet"
async function main() {
    while (true) {
        console.log("start creating account")
        exec("near dev-deploy", (err, stdout, stderr) => {
            fs.readdir("/Users/campv/.near-credentials/testnet", (err, files) => {
                files.forEach(file => {
                    if (file.startsWith("dev-") && file.endsWith(".json")) {
                        console.log("deleting account", file.substring(0, file.length - 5))
                        exec(`near delete ${file.substring(0, file.length - 5)} ${target}`, (err, stdout, stderr) => {
                            exec(`rm -rf /Users/campv/.near-credentials/testnet/${file}`)
                            console.log("transferred to ", target)
                        })
                    }
                });
            });
        })
        await sleep(30 * 1000)
        console.log('waiting')
    }
}

main()