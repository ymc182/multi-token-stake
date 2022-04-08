const nearAPI = require("near-api-js");
const BN = require("bn.js");
const fs = require("fs").promises;
const assert = require("assert").strict;

function getConfig(env) {
	switch (env) {
		case "sandbox":
		case "local":
			return {
				networkId: "sandbox",
				nodeUrl: "http://localhost:3030",
				masterAccount: "test.near",
				contractAccount: "neko.test.near",
				factoryAccount: "factory.test.near",
				keyPath: "/tmp/near-sandbox/validator_key.json",
			};
	}
}
const factory_methods = {
	viewMethods: ["ft_balance_of"],
	changeMethods: ["init_factory", "ft_mint"],
};
const contractMethods = {
	viewMethods: ["get_fee_rate"],
	changeMethods: ["set_bake_fee", "new_default_meta"],
};
let config;
let masterAccount;
let masterKey;
let pubKey;
let keyStore;
let near;

async function initNear() {
	config = getConfig(process.env.NEAR_ENV || "sandbox");
	const keyFile = require(config.keyPath);
	masterKey = nearAPI.utils.KeyPair.fromString(keyFile.secret_key || keyFile.private_key);
	pubKey = masterKey.getPublicKey();
	keyStore = new nearAPI.keyStores.InMemoryKeyStore();
	keyStore.setKey(config.networkId, config.masterAccount, masterKey);
	near = await nearAPI.connect({
		deps: {
			keyStore,
		},
		networkId: config.networkId,
		nodeUrl: config.nodeUrl,
	});
	masterAccount = new nearAPI.Account(near.connection, config.masterAccount);
	console.log("Finish init NEAR");
}

async function createContractUser(accountPrefix, contractAccountId, contractMethods) {
	let accountId = accountPrefix + "." + config.masterAccount;
	await masterAccount.createAccount(accountId, pubKey, new BN(10).pow(new BN(25)));
	keyStore.setKey(config.networkId, accountId, masterKey);
	const account = new nearAPI.Account(near.connection, accountId);
	const accountUseContract = new nearAPI.Contract(account, contractAccountId, contractMethods);
	return [account, accountUseContract];
}
async function createFactoryUser(account, contractAccountId, contractMethods) {
	const accountUseFactory = new nearAPI.Contract(account, contractAccountId, contractMethods);
	return accountUseFactory;
}
async function initTest() {
	const contract = await fs.readFile("./contracts/main.wasm");
	const factory = await fs.readFile("./contracts/factory.wasm");
	const _contractAccount = await masterAccount.createAndDeployContract(
		config.contractAccount,
		pubKey,
		contract,
		new BN(10).pow(new BN(25))
	);
	const _factoryAccount = await masterAccount.createAndDeployContract(
		config.factoryAccount,
		pubKey,
		factory,
		new BN(10).pow(new BN(25))
	);
	const [alice, aliceUseContract] = await createContractUser("alice", config.contractAccount, contractMethods);
	const aliceUseFactory = await createFactoryUser(alice, config.factoryAccount, factory_methods);
	const [bob, bobUseContract] = await createContractUser("bob", config.contractAccount, contractMethods);
	const bobUseFactory = await createFactoryUser(bob, config.factoryAccount, factory_methods);
	console.log("Finish deploy contracts and create test accounts");
	return { aliceUseContract, bobUseContract, aliceUseFactory, bobUseFactory };
}

async function test() {
	// 1. Creates testing accounts and deploys a contract
	await initNear();
	const { aliceUseContract, bobUseContract, aliceUseFactory, bobUseFactory } = await initTest();

	// 2. Performs a `set_status` transaction signed by Alice and then calls `get_status` to confirm `set_status` worked

	await aliceUseContract.new_default_meta({
		args: {
			owner_id: "alice.test.near",
			vault_id: "vault.test.near",
			factory_id: "factory.test.near",
			fee_percent: 10,
			cookie_reward_rate: 10,
		},
	});
	await aliceUseFactory.init_factory({
		args: {
			owner_id: "alice.test.near",
			neko_id: "neko.test.near",
			vault_id: "vault.test.near",
		},
	});
	const fee = await aliceUseContract.get_fee_rate({});
	const factory_balance = await aliceUseFactory.ft_balance_of({ account_id: "alice.test.near" });
	assert.equal(fee, 10);
	console.log("Factory Balance:", factory_balance);
}

test();
