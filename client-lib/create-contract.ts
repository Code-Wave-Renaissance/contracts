import {
  Connection,
  Transaction,
  sendAndConfirmTransaction,
  clusterApiUrl,
} from "@solana/web3.js";
import "dotenv/config";
import { getKeypairFromEnvironment } from "@solana-developers/helpers";
import { createContract } from "./contract-client";

const senderKeypair = getKeypairFromEnvironment(process.argv[2]);
const workerKeypair = getKeypairFromEnvironment(process.argv[3]);
const id = process.argv[4];
const quantity = parseInt(process.argv[5]);

console.log(`Quantity: `, quantity);
console.log(`Sender: `, senderKeypair.publicKey.toBase58());

const connection = new Connection(clusterApiUrl("devnet"));
const transaction = new Transaction();
const instruction = createContract(senderKeypair.publicKey, workerKeypair.publicKey, id, quantity);

transaction.add(instruction);

const signature = await sendAndConfirmTransaction(connection, transaction, [senderKeypair]);

console.log(`Transaction signature: ` + signature);
