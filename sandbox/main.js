const { connect, KeyPair, keyStores, utils, Contract } = require("near-api-js");

const BN = require("bn.js");
const fs = require("fs").promises;
const assert = require("assert").strict;
const path = require("path");
const homedir = require("os").homedir();

const CREDENTIALS_DIR = ".near-credentials";
const ACCOUNT_ID = "nekocoin.testnet";
const CONTRACT_ID = "ft.nekocoin.testnet";
const VAULT_ID = "vault.nekocoin.testnet";
const WASM_PATH = "./contracts/main.wasm";
const credentialsPath = path.join(homedir, CREDENTIALS_DIR);
const keyStore = new keyStores.UnencryptedFileSystemKeyStore(credentialsPath);
const config = {
	keyStore,
	networkId: "testnet",
	nodeUrl: "https://rpc.testnet.near.org",
	headers: {},
};
const getConfig = (env) => {
	switch (env) {
		case "sandbox":
		case "local":
			return {
				networkId: "sandbox",
				nodeUrl: "http://localhost:3030",
				masterAccount: "nekocoin.near",
				contractAccount: "ft.nekocoin.near",
				keyPath: "/tmp/near-sandbox/validator_key.json",
			};
	}
};
main();

async function clean() {
	const near = await connect(config);
	const response = await near.connection.provider.query({
		request_type: "view_state",
		finality: "final",
		account_id: CONTRACT_ID,
		prefix_base64: "",
	});
	console.log(
		JSON.stringify({
			// TODO add calc size of data for limit burning 200TGas for one call on contract
			keys: response.values.map((it) => it.key),
		})
	);
}
async function main() {
	const contract = await initNekoContract(CONTRACT_ID);
	switch (process.argv[2]) {
		case "deploy":
			deployContract(CONTRACT_ID, WASM_PATH);
			break;
		case "init":
			await contract.new_default_meta({
				owner_id: ACCOUNT_ID,
				vault_id: VAULT_ID,
				factory_id: "factory.nekocoin.testnet",
				fee_percent: 5,
				cookie_reward_rate: 1,
			});
			break;
		case "metadata":
			const metadata = await contract.ft_metadata({});
			console.log(metadata);
			break;
		case "balance":
			const balance = await contract.ft_balance_of({ account_id: process.argv[3] });
			console.log("BALANCE:", balance);
			break;
		case "mint":
			const result_mint = await contract.ft_mint({
				args: {
					to: process.argv[3],
					amount: parseInt(process.argv[4]),
				},
			});
			console.log(result_mint);
			break;
		case "update-vault":
			const result_transfer = await contract.update_vault({
				args: { vault_id: process.argv[3] },
			});
			console.log(result_transfer);
			break;
		case "stake":
			const stakeAmount = 100;
			await contract.stake({ args: { amount: stakeAmount }, amount: 2 });
			break;
		case "clean":
			const near = await connect(config);
			const response = await near.connection.provider.query({
				request_type: "view_state",
				finality: "final",
				account_id: CONTRACT_ID,
				prefix_base64: "",
			});
			console.log(
				JSON.stringify({
					// TODO add calc size of data for limit burning 200TGas for one call on contract
					keys: response.values.map((it) => it.key),
				})
			);
			break;
		default:
	}
}

async function initNekoContract(contractId) {
	const near = await connect(config);
	const account = await near.account(ACCOUNT_ID);
	const methodOptions = {
		viewMethods: ["ft_balance_of", "ft_metadata"],
		changeMethods: ["new_default_meta", "ft_mint", "ft_transfer", "update_vault", "stake", "clean"],
	};
	return new Contract(account, contractId, methodOptions);
}

async function deployContract(contractId, wasmPath) {
	const near = await connect(config);
	const account = await near.account(contractId);
	const file = await fs.readFile(wasmPath);
	const result = await account.deployContract(file);
	console.log(result);
}

function toNear(amount) {
	return utils.format.formatNearAmount(amount);
}

function fromNear(amount) {
	return utils.format.parseNearAmount(amount);
}
