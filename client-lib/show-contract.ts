import {
  Connection,
  clusterApiUrl,
} from "@solana/web3.js";
import "dotenv/config";
import { getKeypairFromEnvironment } from "@solana-developers/helpers";
import { getContractData } from "./contract-client";

const senderKeypair = getKeypairFromEnvironment(process.argv[2]);
const workerKeypair = getKeypairFromEnvironment(process.argv[3]);
const id = process.argv[4];

console.log(`Sender: `, senderKeypair.publicKey.toBase58());

const connection = new Connection(clusterApiUrl("devnet"));

const contractData = await getContractData(
  connection,
  senderKeypair.publicKey,
  workerKeypair.publicKey,
  id
);

console.log(`ContractId: ` + contractData.contractId);
console.log(`Owner: ` + contractData.owner);
console.log(`Worker: ` + contractData.worker);
console.log(`Total Quantity: ` + contractData.totalQuantity);
console.log(`Actual Step: ` + contractData.actualStep);
